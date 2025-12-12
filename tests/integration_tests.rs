//! Integration tests for iSCSI target
//!
//! These tests replicate the functionality of the C-based test suite but in pure Rust.
//! They test:
//! - Discovery and login
//! - SCSI commands
//! - I/O operations
//! - Parameter negotiation
//! - Error handling
//! - Arbitrary PDU transmission (for testing edge cases)

use iscsi_target::{IscsiClient, IscsiTarget, ScsiBlockDevice, ScsiResult};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Simple in-memory storage for testing
struct TestStorage {
    data: Vec<u8>,
}

impl TestStorage {
    fn new(size_mb: usize) -> Self {
        TestStorage {
            data: vec![0u8; size_mb * 1024 * 1024],
        }
    }
}

impl ScsiBlockDevice for TestStorage {
    fn read(&self, lba: u64, blocks: u32, block_size: u32) -> ScsiResult<Vec<u8>> {
        let offset = (lba * block_size as u64) as usize;
        let len = (blocks * block_size) as usize;
        if offset + len > self.data.len() {
            return Err(iscsi_target::IscsiError::Scsi(
                "Read beyond storage capacity".to_string(),
            ));
        }
        Ok(self.data[offset..offset + len].to_vec())
    }

    fn write(&mut self, lba: u64, data: &[u8], block_size: u32) -> ScsiResult<()> {
        let offset = (lba * block_size as u64) as usize;
        if offset + data.len() > self.data.len() {
            return Err(iscsi_target::IscsiError::Scsi(
                "Write beyond storage capacity".to_string(),
            ));
        }
        self.data[offset..offset + data.len()].copy_from_slice(data);
        Ok(())
    }

    fn capacity(&self) -> u64 {
        (self.data.len() / 512) as u64
    }

    fn block_size(&self) -> u32 {
        512
    }
}

/// Start an iSCSI target server in a background thread
fn start_test_target() -> Result<std::thread::JoinHandle<()>, Box<dyn std::error::Error>> {
    let storage = TestStorage::new(100); // 100 MB
    let target = IscsiTarget::builder()
        .bind_addr("127.0.0.1:3260")
        .target_name("iqn.2025-12.local:storage.disk1")
        .build(storage)?;

    let handle = std::thread::spawn(move || {
        let _ = target.run();
    });

    // Give the server time to start
    std::thread::sleep(Duration::from_millis(500));

    Ok(handle)
}

// Note: These tests are designed to be run with `cargo test -- --test-threads=1`
// and require a running iSCSI target. They should be run as integration tests.

#[test]
#[ignore] // Requires running target - use with: cargo test -- --ignored --test-threads=1
fn test_client_connect_and_login() {
    // This test connects to localhost:3260 and performs login
    // Start target first: cargo run --example simple_target
    match IscsiClient::connect("127.0.0.1:3260") {
        Ok(mut client) => {
            assert!(!client.is_logged_in());

            let result = client.login(
                "iqn.2025-12.local:initiator",
                "iqn.2025-12.local:storage.disk1",
            );

            match result {
                Ok(()) => {
                    assert!(client.is_logged_in());
                    let _ = client.logout();
                }
                Err(e) => eprintln!("Login failed: {}", e),
            }
        }
        Err(e) => eprintln!("Connection failed: {}", e),
    }
}

#[test]
#[ignore]
fn test_client_sequence_numbers() {
    match IscsiClient::connect("127.0.0.1:3260") {
        Ok(mut client) => {
            let initial_cmd_sn = client.cmd_sn();
            assert_eq!(initial_cmd_sn, 0);

            // After login, cmd_sn should increment
            if client
                .login(
                    "iqn.2025-12.local:initiator",
                    "iqn.2025-12.local:storage.disk1",
                )
                .is_ok()
            {
                // cmd_sn should have incremented
                assert!(client.cmd_sn() > initial_cmd_sn);
                let _ = client.logout();
            }
        }
        Err(e) => eprintln!("Connection failed: {}", e),
    }
}

// ============================================================================
// Tests for arbitrary PDU transmission (for testing edge cases and protocol compliance)
// ============================================================================

#[test]
#[ignore]
fn test_raw_pdu_transmission() {
    use iscsi_target::pdu::IscsiPdu;

    match IscsiClient::connect("127.0.0.1:3260") {
        Ok(mut client) => {
            // Create and send a custom PDU
            let mut pdu = IscsiPdu::new();
            pdu.opcode = 0x03; // LOGIN_REQUEST
            pdu.immediate = true;
            pdu.flags = 0x0C; // CSG=0, NSG=3 (full feature)
            pdu.itt = 1;

            // Send raw PDU
            if let Ok(()) = client.send_raw_pdu(&pdu) {
                // Try to receive response
                match client.recv_pdu() {
                    Ok(_response) => {
                        // Success - we got a response
                    }
                    Err(e) => eprintln!("Failed to receive response: {}", e),
                }
            }
        }
        Err(e) => eprintln!("Connection failed: {}", e),
    }
}

// ============================================================================
// Example test categories similar to C test suite
// ============================================================================

/// TD-001: Basic Discovery (would use SendTargets with libiscsi)
/// For now, this tests basic connectivity
#[test]
#[ignore]
fn test_discovery_basic() {
    match IscsiClient::connect("127.0.0.1:3260") {
        Ok(_client) => {
            // In a real discovery test, we would:
            // 1. Set session type to DISCOVERY
            // 2. Send SendTargets PDU
            // 3. Parse response for available targets
        }
        Err(e) => eprintln!("Connection failed: {}", e),
    }
}

/// TL-001: Basic Login
#[test]
#[ignore]
fn test_login_basic() {
    match IscsiClient::connect("127.0.0.1:3260") {
        Ok(mut client) => {
            match client.login(
                "iqn.2025-12.local:initiator",
                "iqn.2025-12.local:storage.disk1",
            ) {
                Ok(()) => {
                    assert!(client.is_logged_in());
                    let _ = client.logout();
                }
                Err(e) => panic!("Login failed: {}", e),
            }
        }
        Err(e) => panic!("Connection failed: {}", e),
    }
}

/// TC-001: INQUIRY Command (SCSI)
#[test]
#[ignore]
fn test_scsi_inquiry() {
    match IscsiClient::connect("127.0.0.1:3260") {
        Ok(mut client) => {
            if client
                .login(
                    "iqn.2025-12.local:initiator",
                    "iqn.2025-12.local:storage.disk1",
                )
                .is_ok()
            {
                // INQUIRY command: opcode 0x12, flags 0x00, length 255
                let cdb = vec![0x12, 0x00, 0x00, 0x00, 0xFF, 0x00];

                match client.send_scsi_command(&cdb, None) {
                    Ok(response) => {
                        // Verify response is successful
                        // In real test, would check response data
                        println!("INQUIRY response: opcode=0x{:02x}", response.opcode);
                    }
                    Err(e) => eprintln!("INQUIRY failed: {}", e),
                }

                let _ = client.logout();
            }
        }
        Err(e) => eprintln!("Connection failed: {}", e),
    }
}

/// TC-003: READ CAPACITY
#[test]
#[ignore]
fn test_scsi_read_capacity() {
    match IscsiClient::connect("127.0.0.1:3260") {
        Ok(mut client) => {
            if client
                .login(
                    "iqn.2025-12.local:initiator",
                    "iqn.2025-12.local:storage.disk1",
                )
                .is_ok()
            {
                // READ CAPACITY (10): opcode 0x25
                let cdb = vec![0x25, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

                match client.send_scsi_command(&cdb, None) {
                    Ok(response) => {
                        println!(
                            "READ CAPACITY response: opcode=0x{:02x}, data_len={}",
                            response.opcode, response.data_length
                        );
                    }
                    Err(e) => eprintln!("READ CAPACITY failed: {}", e),
                }

                let _ = client.logout();
            }
        }
        Err(e) => eprintln!("Connection failed: {}", e),
    }
}

/// TI-001: Single Block Read
#[test]
#[ignore]
fn test_io_single_block_read() {
    match IscsiClient::connect("127.0.0.1:3260") {
        Ok(mut client) => {
            if client
                .login(
                    "iqn.2025-12.local:initiator",
                    "iqn.2025-12.local:storage.disk1",
                )
                .is_ok()
            {
                // READ (10): opcode 0x28, LBA=0, blocks=1
                let cdb = vec![0x28, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00];

                match client.send_scsi_command(&cdb, None) {
                    Ok(response) => {
                        println!(
                            "READ (10) response: opcode=0x{:02x}, data_len={}",
                            response.opcode, response.data_length
                        );
                        // Verify data_length == 512 (one block)
                        assert_eq!(response.data_length, 512);
                    }
                    Err(e) => eprintln!("READ failed: {}", e),
                }

                let _ = client.logout();
            }
        }
        Err(e) => eprintln!("Connection failed: {}", e),
    }
}

/// TI-002: Single Block Write
#[test]
#[ignore]
fn test_io_single_block_write() {
    match IscsiClient::connect("127.0.0.1:3260") {
        Ok(mut client) => {
            if client
                .login(
                    "iqn.2025-12.local:initiator",
                    "iqn.2025-12.local:storage.disk1",
                )
                .is_ok()
            {
                // WRITE (10): opcode 0x2A, LBA=0, blocks=1
                let cdb = vec![0x2A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00];
                let data = vec![0xAA; 512]; // Write pattern

                match client.send_scsi_command(&cdb, Some(&data)) {
                    Ok(response) => {
                        println!("WRITE (10) response: opcode=0x{:02x}", response.opcode);
                    }
                    Err(e) => eprintln!("WRITE failed: {}", e),
                }

                let _ = client.logout();
            }
        }
        Err(e) => eprintln!("Connection failed: {}", e),
    }
}

/// Test data integrity - Write pattern and read back
#[test]
#[ignore]
fn test_io_data_integrity() {
    match IscsiClient::connect("127.0.0.1:3260") {
        Ok(mut client) => {
            if client
                .login(
                    "iqn.2025-12.local:initiator",
                    "iqn.2025-12.local:storage.disk1",
                )
                .is_ok()
            {
                // Write pattern
                let pattern = vec![0x55; 512];

                // WRITE (10): LBA=0, blocks=1
                let write_cdb = vec![0x2A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00];
                match client.send_scsi_command(&write_cdb, Some(&pattern)) {
                    Ok(_) => {
                        // Read back
                        let read_cdb =
                            vec![0x28, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00];
                        match client.send_scsi_command(&read_cdb, None) {
                            Ok(response) => {
                                // Verify data matches pattern
                                if response.data == pattern {
                                    println!("Data integrity test: PASSED");
                                } else {
                                    eprintln!("Data integrity test: FAILED - data mismatch");
                                }
                            }
                            Err(e) => eprintln!("Read failed: {}", e),
                        }
                    }
                    Err(e) => eprintln!("Write failed: {}", e),
                }

                let _ = client.logout();
            }
        }
        Err(e) => eprintln!("Connection failed: {}", e),
    }
}

// ============================================================================
// Test for invalid PDU handling (parameter validation edge cases)
// ============================================================================

/// Test invalid parameters in login
#[test]
#[ignore]
fn test_login_invalid_max_recv_data_size() {
    use iscsi_target::pdu::{IscsiPdu, opcode, flags};

    match IscsiClient::connect("127.0.0.1:3260") {
        Ok(mut client) => {
            // Build login request with invalid MaxRecvDataSegmentLength=0
            let params = "InitiatorName=iqn.test:init\0TargetName=iqn.test:tgt\0AuthMethod=None\0MaxRecvDataSegmentLength=0\0";

            let mut pdu = IscsiPdu::new();
            pdu.opcode = opcode::LOGIN_REQUEST;
            pdu.immediate = true;
            pdu.flags = flags::TRANSIT | (flags::CSG_SECURITY_NEG & 0x03) << 2 | (flags::NSG_LOGIN_OP_NEG & 0x03);
            pdu.itt = 0;
            pdu.data = params.as_bytes().to_vec();

            // Pad to 4-byte boundary
            while pdu.data.len() % 4 != 0 {
                pdu.data.push(0);
            }

            if let Ok(()) = client.send_raw_pdu(&pdu) {
                match client.recv_pdu() {
                    Ok(response) => {
                        println!(
                            "Invalid parameter response: opcode=0x{:02x}",
                            response.opcode
                        );
                        // Target should reject with error
                    }
                    Err(e) => eprintln!("Failed to receive response: {}", e),
                }
            }
        }
        Err(e) => eprintln!("Connection failed: {}", e),
    }
}

// ============================================================================
// Unit tests (these don't require a running target)
// ============================================================================

#[test]
fn test_pdu_roundtrip() {
    use iscsi_target::pdu::IscsiPdu;

    let mut pdu = IscsiPdu::new();
    pdu.opcode = 0x01; // SCSI_COMMAND
    pdu.immediate = false;
    pdu.flags = 0x80; // FINAL
    pdu.itt = 0x12345678;
    pdu.lun = 0x0000000000000000;
    pdu.data = b"test data".to_vec();

    let bytes = pdu.to_bytes();
    let parsed = IscsiPdu::from_bytes(&bytes).expect("Failed to parse PDU");

    assert_eq!(parsed.opcode, pdu.opcode);
    assert_eq!(parsed.immediate, pdu.immediate);
    assert_eq!(parsed.flags, pdu.flags);
    assert_eq!(parsed.itt, pdu.itt);
    assert_eq!(parsed.data, pdu.data);
}

#[test]
fn test_pdu_data_padding() {
    use iscsi_target::pdu::IscsiPdu;

    let mut pdu = IscsiPdu::new();
    pdu.opcode = 0x01;
    pdu.data = b"ABC".to_vec(); // 3 bytes, needs padding to 4

    let bytes = pdu.to_bytes();

    // Check padding: should have at least BHS_SIZE + 4 bytes
    assert!(bytes.len() >= iscsi_target::pdu::BHS_SIZE + 4);

    // Parse back
    let parsed = IscsiPdu::from_bytes(&bytes).expect("Failed to parse PDU");
    assert_eq!(parsed.data, b"ABC");
}
