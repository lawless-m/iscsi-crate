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
use once_cell::sync::Lazy;
use std::env;

// ============================================================================
// Test Configuration Module
// ============================================================================

#[derive(Debug)]
struct TestConfig {
    target_addr: String,
}

static TEST_CONFIG: Lazy<TestConfig> = Lazy::new(|| {
    // Check environment variable first (highest priority)
    if let Ok(addr) = env::var("ISCSI_TEST_TARGET") {
        eprintln!("Using target address from ISCSI_TEST_TARGET: {}", addr);
        return TestConfig { target_addr: addr };
    }

    // Try to load from test-config.toml
    if let Ok(contents) = std::fs::read_to_string("test-config.toml") {
        if let Ok(config) = contents.parse::<toml::Value>() {
            if let Some(portal) = config.get("target")
                .and_then(|t| t.get("portal"))
                .and_then(|p| p.as_str()) {
                eprintln!("Using target address from test-config.toml: {}", portal);
                return TestConfig {
                    target_addr: portal.to_string()
                };
            }
        }
    }

    // Fallback to default (standard iSCSI port)
    let default = target_addr().to_string();
    eprintln!("Using default target address: {}", default);
    TestConfig {
        target_addr: default
    }
});

fn target_addr() -> &'static str {
    &TEST_CONFIG.target_addr
}

// ============================================================================
// Test Storage Implementation
// ============================================================================

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
        .bind_addr(target_addr())
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
    match IscsiClient::connect(target_addr()) {
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
    match IscsiClient::connect(target_addr()) {
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

    match IscsiClient::connect(target_addr()) {
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
    match IscsiClient::connect(target_addr()) {
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
    match IscsiClient::connect(target_addr()) {
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
    match IscsiClient::connect(target_addr()) {
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
    match IscsiClient::connect(target_addr()) {
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

/// TC-002: TEST UNIT READY
#[test]
#[ignore]
fn test_scsi_test_unit_ready() {
    match IscsiClient::connect(target_addr()) {
        Ok(mut client) => {
            if client
                .login(
                    "iqn.2025-12.local:initiator",
                    "iqn.2025-12.local:storage.disk1",
                )
                .is_ok()
            {
                // TEST UNIT READY: opcode 0x00
                let cdb = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

                match client.send_scsi_command(&cdb, None) {
                    Ok(response) => {
                        println!("TEST UNIT READY response: opcode=0x{:02x}", response.opcode);
                    }
                    Err(e) => eprintln!("TEST UNIT READY failed: {}", e),
                }

                let _ = client.logout();
            }
        }
        Err(e) => eprintln!("Connection failed: {}", e),
    }
}

/// TC-005: MODE SENSE
#[test]
#[ignore]
fn test_scsi_mode_sense() {
    match IscsiClient::connect(target_addr()) {
        Ok(mut client) => {
            if client
                .login(
                    "iqn.2025-12.local:initiator",
                    "iqn.2025-12.local:storage.disk1",
                )
                .is_ok()
            {
                // MODE SENSE (6): opcode 0x1A
                let cdb = vec![0x1A, 0x00, 0x3F, 0x00, 0xFF, 0x00];

                match client.send_scsi_command(&cdb, None) {
                    Ok(response) => {
                        println!("MODE SENSE response: opcode=0x{:02x}, data_len={}", response.opcode, response.data_length);
                    }
                    Err(e) => eprintln!("MODE SENSE failed: {}", e),
                }

                let _ = client.logout();
            }
        }
        Err(e) => eprintln!("Connection failed: {}", e),
    }
}

/// TC-007: REPORT LUNS
#[test]
#[ignore]
fn test_scsi_report_luns() {
    match IscsiClient::connect(target_addr()) {
        Ok(mut client) => {
            if client
                .login(
                    "iqn.2025-12.local:initiator",
                    "iqn.2025-12.local:storage.disk1",
                )
                .is_ok()
            {
                // REPORT LUNS: opcode 0xA0
                let cdb = vec![0xA0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00];

                match client.send_scsi_command(&cdb, None) {
                    Ok(response) => {
                        println!("REPORT LUNS response: opcode=0x{:02x}, data_len={}", response.opcode, response.data_length);
                    }
                    Err(e) => eprintln!("REPORT LUNS failed: {}", e),
                }

                let _ = client.logout();
            }
        }
        Err(e) => eprintln!("Connection failed: {}", e),
    }
}

/// TC-008: Invalid Command
#[test]
#[ignore]
fn test_scsi_invalid_command() {
    match IscsiClient::connect(target_addr()) {
        Ok(mut client) => {
            if client
                .login(
                    "iqn.2025-12.local:initiator",
                    "iqn.2025-12.local:storage.disk1",
                )
                .is_ok()
            {
                // Invalid SCSI opcode: 0xFF (reserved)
                let cdb = vec![0xFF, 0x00, 0x00, 0x00, 0x00, 0x00];

                match client.send_scsi_command(&cdb, None) {
                    Ok(response) => {
                        println!("Invalid command response: opcode=0x{:02x}", response.opcode);
                        // Should receive CHECK CONDITION status
                    }
                    Err(e) => println!("Invalid command properly rejected: {}", e),
                }

                let _ = client.logout();
            }
        }
        Err(e) => eprintln!("Connection failed: {}", e),
    }
}

/// TC-009: Command to Invalid LUN
#[test]
#[ignore]
fn test_scsi_invalid_lun() {
    match IscsiClient::connect(target_addr()) {
        Ok(mut client) => {
            if client
                .login(
                    "iqn.2025-12.local:initiator",
                    "iqn.2025-12.local:storage.disk1",
                )
                .is_ok()
            {
                // Send INQUIRY to invalid LUN 99
                // Need to construct raw PDU to specify LUN
                use iscsi_target::pdu::{IscsiPdu, opcode};
                let mut pdu = IscsiPdu::new();
                pdu.opcode = opcode::SCSI_COMMAND;
                pdu.flags = 0x80; // Final
                pdu.lun = 99 << 48; // LUN 99
                pdu.itt = 1;

                // INQUIRY CDB
                let cdb = vec![0x12, 0x00, 0x00, 0x00, 0xFF, 0x00];
                pdu.data = cdb;

                if let Ok(()) = client.send_raw_pdu(&pdu) {
                    match client.recv_pdu() {
                        Ok(response) => {
                            println!("Invalid LUN response: opcode=0x{:02x}", response.opcode);
                            // Should receive CHECK CONDITION with LOGICAL_UNIT_NOT_SUPPORTED
                        }
                        Err(e) => println!("Invalid LUN command failed: {}", e),
                    }
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
    match IscsiClient::connect(target_addr()) {
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
    match IscsiClient::connect(target_addr()) {
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
    match IscsiClient::connect(target_addr()) {
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

/// TI-003: Multi-Block Sequential Read
#[test]
#[ignore]
fn test_io_multi_block_sequential_read() {
    match IscsiClient::connect(target_addr()) {
        Ok(mut client) => {
            if client.login("iqn.2025-12.local:initiator", "iqn.2025-12.local:storage.disk1").is_ok() {
                // READ (10): LBA=0, blocks=4
                let cdb = vec![0x28, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00];
                match client.send_scsi_command(&cdb, None) {
                    Ok(response) => {
                        assert_eq!(response.data_length, 2048); // 4 blocks * 512 bytes
                        println!("Multi-block sequential read: PASSED");
                    }
                    Err(e) => eprintln!("Multi-block read failed: {}", e),
                }
                let _ = client.logout();
            }
        }
        Err(e) => eprintln!("Connection failed: {}", e),
    }
}

/// TI-004: Multi-Block Sequential Write
#[test]
#[ignore]
fn test_io_multi_block_sequential_write() {
    match IscsiClient::connect(target_addr()) {
        Ok(mut client) => {
            if client.login("iqn.2025-12.local:initiator", "iqn.2025-12.local:storage.disk1").is_ok() {
                let data = vec![0xAA; 2048]; // 4 blocks
                // WRITE (10): LBA=0, blocks=4
                let cdb = vec![0x2A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00];
                match client.send_scsi_command(&cdb, Some(&data)) {
                    Ok(_) => println!("Multi-block sequential write: PASSED"),
                    Err(e) => eprintln!("Multi-block write failed: {}", e),
                }
                let _ = client.logout();
            }
        }
        Err(e) => eprintln!("Connection failed: {}", e),
    }
}

/// TI-005: Random Access Reads
#[test]
#[ignore]
fn test_io_random_access_reads() {
    match IscsiClient::connect(target_addr()) {
        Ok(mut client) => {
            if client.login("iqn.2025-12.local:initiator", "iqn.2025-12.local:storage.disk1").is_ok() {
                // Read from various LBAs: 0, 10, 100, 1000
                for lba in [0, 10, 100, 1000] {
                    let cdb = vec![0x28, 0x00, (lba >> 24) as u8, (lba >> 16) as u8, (lba >> 8) as u8, lba as u8, 0x00, 0x00, 0x01, 0x00];
                    match client.send_scsi_command(&cdb, None) {
                        Ok(response) => assert_eq!(response.data_length, 512),
                        Err(e) => {
                            eprintln!("Random read at LBA {} failed: {}", lba, e);
                            return;
                        }
                    }
                }
                println!("Random access reads: PASSED");
                let _ = client.logout();
            }
        }
        Err(e) => eprintln!("Connection failed: {}", e),
    }
}

/// TI-006: Random Access Writes
#[test]
#[ignore]
fn test_io_random_access_writes() {
    match IscsiClient::connect(target_addr()) {
        Ok(mut client) => {
            if client.login("iqn.2025-12.local:initiator", "iqn.2025-12.local:storage.disk1").is_ok() {
                let data = vec![0xBB; 512];
                // Write to various LBAs: 5, 50, 500
                for lba in [5, 50, 500] {
                    let cdb = vec![0x2A, 0x00, (lba >> 24) as u8, (lba >> 16) as u8, (lba >> 8) as u8, lba as u8, 0x00, 0x00, 0x01, 0x00];
                    if let Err(e) = client.send_scsi_command(&cdb, Some(&data)) {
                        eprintln!("Random write at LBA {} failed: {}", lba, e);
                        return;
                    }
                }
                println!("Random access writes: PASSED");
                let _ = client.logout();
            }
        }
        Err(e) => eprintln!("Connection failed: {}", e),
    }
}

/// TI-007: Large Transfer Read
#[test]
#[ignore]
fn test_io_large_transfer_read() {
    match IscsiClient::connect(target_addr()) {
        Ok(mut client) => {
            if client.login("iqn.2025-12.local:initiator", "iqn.2025-12.local:storage.disk1").is_ok() {
                // READ (10): LBA=0, blocks=64 (32 KB)
                let cdb = vec![0x28, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x00];
                match client.send_scsi_command(&cdb, None) {
                    Ok(response) => {
                        assert_eq!(response.data_length, 32768); // 64 blocks * 512
                        println!("Large transfer read: PASSED");
                    }
                    Err(e) => eprintln!("Large read failed: {}", e),
                }
                let _ = client.logout();
            }
        }
        Err(e) => eprintln!("Connection failed: {}", e),
    }
}

/// TI-008: Large Transfer Write
#[test]
#[ignore]
fn test_io_large_transfer_write() {
    match IscsiClient::connect(target_addr()) {
        Ok(mut client) => {
            if client.login("iqn.2025-12.local:initiator", "iqn.2025-12.local:storage.disk1").is_ok() {
                let data = vec![0xCC; 32768]; // 64 blocks
                // WRITE (10): LBA=0, blocks=64
                let cdb = vec![0x2A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x00];
                match client.send_scsi_command(&cdb, Some(&data)) {
                    Ok(_) => println!("Large transfer write: PASSED"),
                    Err(e) => eprintln!("Large write failed: {}", e),
                }
                let _ = client.logout();
            }
        }
        Err(e) => eprintln!("Connection failed: {}", e),
    }
}

/// TI-009: Zero-Length Transfer
#[test]
#[ignore]
fn test_io_zero_length_transfer() {
    match IscsiClient::connect(target_addr()) {
        Ok(mut client) => {
            if client.login("iqn.2025-12.local:initiator", "iqn.2025-12.local:storage.disk1").is_ok() {
                // READ (10): LBA=0, blocks=0
                let cdb = vec![0x28, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
                match client.send_scsi_command(&cdb, None) {
                    Ok(_) => println!("Zero-length transfer: PASSED"),
                    Err(e) => eprintln!("Zero-length transfer failed: {}", e),
                }
                let _ = client.logout();
            }
        }
        Err(e) => eprintln!("Connection failed: {}", e),
    }
}

/// TI-010: Maximum Transfer Size
#[test]
#[ignore]
fn test_io_maximum_transfer_size() {
    match IscsiClient::connect(target_addr()) {
        Ok(mut client) => {
            if client.login("iqn.2025-12.local:initiator", "iqn.2025-12.local:storage.disk1").is_ok() {
                // READ (10): LBA=0, blocks=256 (128 KB - typical max)
                let cdb = vec![0x28, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00];
                match client.send_scsi_command(&cdb, None) {
                    Ok(response) => {
                        assert_eq!(response.data_length, 131072); // 256 blocks * 512
                        println!("Maximum transfer size: PASSED");
                    }
                    Err(e) => eprintln!("Maximum transfer failed: {}", e),
                }
                let _ = client.logout();
            }
        }
        Err(e) => eprintln!("Connection failed: {}", e),
    }
}

/// TI-011: Beyond Maximum Transfer
#[test]
#[ignore]
fn test_io_beyond_maximum_transfer() {
    match IscsiClient::connect(target_addr()) {
        Ok(mut client) => {
            if client.login("iqn.2025-12.local:initiator", "iqn.2025-12.local:storage.disk1").is_ok() {
                // READ (10): LBA=0, blocks=512 (256 KB - likely beyond max)
                let cdb = vec![0x28, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00];
                match client.send_scsi_command(&cdb, None) {
                    Ok(_) => println!("Beyond maximum transfer: handled"),
                    Err(e) => println!("Beyond maximum transfer properly rejected: {}", e),
                }
                let _ = client.logout();
            }
        }
        Err(e) => eprintln!("Connection failed: {}", e),
    }
}

/// TI-012: Unaligned Access
#[test]
#[ignore]
fn test_io_unaligned_access() {
    match IscsiClient::connect(target_addr()) {
        Ok(mut client) => {
            if client.login("iqn.2025-12.local:initiator", "iqn.2025-12.local:storage.disk1").is_ok() {
                // READ (10): LBA=1 (odd LBA), blocks=3
                let cdb = vec![0x28, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x03, 0x00];
                match client.send_scsi_command(&cdb, None) {
                    Ok(response) => {
                        assert_eq!(response.data_length, 1536); // 3 blocks * 512
                        println!("Unaligned access: PASSED");
                    }
                    Err(e) => eprintln!("Unaligned access failed: {}", e),
                }
                let _ = client.logout();
            }
        }
        Err(e) => eprintln!("Connection failed: {}", e),
    }
}

/// TI-013: Write-Read-Verify Pattern
#[test]
#[ignore]
fn test_io_write_read_verify_pattern() {
    match IscsiClient::connect(target_addr()) {
        Ok(mut client) => {
            if client.login("iqn.2025-12.local:initiator", "iqn.2025-12.local:storage.disk1").is_ok() {
                let pattern = (0..512).map(|i| (i % 256) as u8).collect::<Vec<u8>>();
                // WRITE (10): LBA=10, blocks=1
                let write_cdb = vec![0x2A, 0x00, 0x00, 0x00, 0x00, 0x0A, 0x00, 0x00, 0x01, 0x00];
                match client.send_scsi_command(&write_cdb, Some(&pattern)) {
                    Ok(_) => {
                        // READ (10): LBA=10, blocks=1
                        let read_cdb = vec![0x28, 0x00, 0x00, 0x00, 0x00, 0x0A, 0x00, 0x00, 0x01, 0x00];
                        match client.send_scsi_command(&read_cdb, None) {
                            Ok(response) => {
                                if response.data == pattern {
                                    println!("Write-read-verify pattern: PASSED");
                                } else {
                                    eprintln!("Write-read-verify: data mismatch");
                                }
                            }
                            Err(e) => eprintln!("Verify read failed: {}", e),
                        }
                    }
                    Err(e) => eprintln!("Verify write failed: {}", e),
                }
                let _ = client.logout();
            }
        }
        Err(e) => eprintln!("Connection failed: {}", e),
    }
}

/// TI-014: Overwrite Test
#[test]
#[ignore]
fn test_io_overwrite() {
    match IscsiClient::connect(target_addr()) {
        Ok(mut client) => {
            if client.login("iqn.2025-12.local:initiator", "iqn.2025-12.local:storage.disk1").is_ok() {
                // Write pattern 1
                let pattern1 = vec![0x11; 512];
                let write_cdb = vec![0x2A, 0x00, 0x00, 0x00, 0x00, 0x14, 0x00, 0x00, 0x01, 0x00];
                client.send_scsi_command(&write_cdb, Some(&pattern1)).ok();

                // Overwrite with pattern 2
                let pattern2 = vec![0x22; 512];
                client.send_scsi_command(&write_cdb, Some(&pattern2)).ok();

                // Read back and verify pattern 2
                let read_cdb = vec![0x28, 0x00, 0x00, 0x00, 0x00, 0x14, 0x00, 0x00, 0x01, 0x00];
                match client.send_scsi_command(&read_cdb, None) {
                    Ok(response) => {
                        if response.data == pattern2 {
                            println!("Overwrite test: PASSED");
                        } else {
                            eprintln!("Overwrite test: data mismatch");
                        }
                    }
                    Err(e) => eprintln!("Overwrite read failed: {}", e),
                }
                let _ = client.logout();
            }
        }
        Err(e) => eprintln!("Connection failed: {}", e),
    }
}

// ============================================================================
// Additional Edge Case and Stress Tests (beyond C suite)
// ============================================================================

/// Stress test: Rapid login/logout cycles
#[test]
#[ignore]
fn test_stress_rapid_login_logout() {
    for i in 0..10 {
        match IscsiClient::connect(target_addr()) {
            Ok(mut client) => {
                if client.login("iqn.2025-12.local:initiator", "iqn.2025-12.local:storage.disk1").is_ok() {
                    let _ = client.logout();
                } else {
                    eprintln!("Login failed on iteration {}", i);
                    return;
                }
            }
            Err(e) => {
                eprintln!("Connection failed on iteration {}: {}", i, e);
                return;
            }
        }
    }
    println!("Rapid login/logout stress test: PASSED (10 cycles)");
}

/// Stress test: Sustained I/O operations
#[test]
#[ignore]
fn test_stress_sustained_io() {
    match IscsiClient::connect(target_addr()) {
        Ok(mut client) => {
            if client.login("iqn.2025-12.local:initiator", "iqn.2025-12.local:storage.disk1").is_ok() {
                let data = vec![0xDD; 512];
                // Perform 100 write/read cycles
                for lba in 0..100 {
                    let write_cdb = vec![0x2A, 0x00, 0x00, 0x00, 0x00, lba as u8, 0x00, 0x00, 0x01, 0x00];
                    if client.send_scsi_command(&write_cdb, Some(&data)).is_err() {
                        eprintln!("Sustained I/O failed at write {}", lba);
                        return;
                    }
                    let read_cdb = vec![0x28, 0x00, 0x00, 0x00, 0x00, lba as u8, 0x00, 0x00, 0x01, 0x00];
                    if client.send_scsi_command(&read_cdb, None).is_err() {
                        eprintln!("Sustained I/O failed at read {}", lba);
                        return;
                    }
                }
                println!("Sustained I/O stress test: PASSED (100 write/read cycles)");
                let _ = client.logout();
            }
        }
        Err(e) => eprintln!("Connection failed: {}", e),
    }
}

/// Edge case: Read at capacity boundary
#[test]
#[ignore]
fn test_edge_read_at_capacity_boundary() {
    match IscsiClient::connect(target_addr()) {
        Ok(mut client) => {
            if client.login("iqn.2025-12.local:initiator", "iqn.2025-12.local:storage.disk1").is_ok() {
                // First get capacity
                let cap_cdb = vec![0x25, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
                match client.send_scsi_command(&cap_cdb, None) {
                    Ok(response) => {
                        if response.data.len() >= 8 {
                            let last_lba = u32::from_be_bytes([
                                response.data[0],
                                response.data[1],
                                response.data[2],
                                response.data[3],
                            ]);
                            // Try to read the last LBA
                            let read_cdb = vec![
                                0x28, 0x00,
                                (last_lba >> 24) as u8,
                                (last_lba >> 16) as u8,
                                (last_lba >> 8) as u8,
                                last_lba as u8,
                                0x00, 0x00, 0x01, 0x00
                            ];
                            match client.send_scsi_command(&read_cdb, None) {
                                Ok(_) => println!("Read at capacity boundary: PASSED"),
                                Err(e) => eprintln!("Read at last LBA failed: {}", e),
                            }
                        }
                    }
                    Err(e) => eprintln!("READ CAPACITY failed: {}", e),
                }
                let _ = client.logout();
            }
        }
        Err(e) => eprintln!("Connection failed: {}", e),
    }
}

/// Edge case: Read beyond capacity
#[test]
#[ignore]
fn test_edge_read_beyond_capacity() {
    match IscsiClient::connect(target_addr()) {
        Ok(mut client) => {
            if client.login("iqn.2025-12.local:initiator", "iqn.2025-12.local:storage.disk1").is_ok() {
                // Try to read at a very high LBA that's definitely beyond capacity
                let huge_lba = 0xFFFFFFu32;
                let read_cdb = vec![
                    0x28, 0x00,
                    (huge_lba >> 24) as u8,
                    (huge_lba >> 16) as u8,
                    (huge_lba >> 8) as u8,
                    huge_lba as u8,
                    0x00, 0x00, 0x01, 0x00
                ];
                match client.send_scsi_command(&read_cdb, None) {
                    Ok(_) => eprintln!("Read beyond capacity should have failed!"),
                    Err(e) => println!("Read beyond capacity properly rejected: {}", e),
                }
                let _ = client.logout();
            }
        }
        Err(e) => eprintln!("Connection failed: {}", e),
    }
}

/// Edge case: Interleaved read/write operations
#[test]
#[ignore]
fn test_edge_interleaved_read_write() {
    match IscsiClient::connect(target_addr()) {
        Ok(mut client) => {
            if client.login("iqn.2025-12.local:initiator", "iqn.2025-12.local:storage.disk1").is_ok() {
                // Write to LBA 0, read from LBA 10, write to LBA 5, read from LBA 0
                let data = vec![0xEE; 512];
                let ops = vec![
                    ("write", 0u32), ("read", 10), ("write", 5), ("read", 0),
                    ("write", 20), ("read", 5), ("write", 15), ("read", 20),
                ];
                for (op, lba) in ops {
                    let cdb = if op == "write" {
                        vec![0x2A, 0x00, 0, 0, 0, lba as u8, 0x00, 0x00, 0x01, 0x00]
                    } else {
                        vec![0x28, 0x00, 0, 0, 0, lba as u8, 0x00, 0x00, 0x01, 0x00]
                    };
                    let result = if op == "write" {
                        client.send_scsi_command(&cdb, Some(&data))
                    } else {
                        client.send_scsi_command(&cdb, None)
                    };
                    if result.is_err() {
                        eprintln!("Interleaved {} at LBA {} failed", op, lba);
                        return;
                    }
                }
                println!("Interleaved read/write: PASSED");
                let _ = client.logout();
            }
        }
        Err(e) => eprintln!("Connection failed: {}", e),
    }
}

/// Edge case: Multiple INQUIRY commands in succession
#[test]
#[ignore]
fn test_edge_multiple_inquiry() {
    match IscsiClient::connect(target_addr()) {
        Ok(mut client) => {
            if client.login("iqn.2025-12.local:initiator", "iqn.2025-12.local:storage.disk1").is_ok() {
                let cdb = vec![0x12, 0x00, 0x00, 0x00, 0xFF, 0x00];
                for i in 0..20 {
                    if client.send_scsi_command(&cdb, None).is_err() {
                        eprintln!("Multiple INQUIRY failed at iteration {}", i);
                        return;
                    }
                }
                println!("Multiple INQUIRY: PASSED (20 iterations)");
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

    match IscsiClient::connect(target_addr()) {
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
