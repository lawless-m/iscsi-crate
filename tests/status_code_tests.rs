//! Tests for RFC 3720 login status code coverage
//!
//! These tests verify that:
//! 1. The status code decoder handles all documented codes
//! 2. The server returns appropriate status codes for error conditions
//! 3. Clients receive and decode status codes correctly

use iscsi_target::error::decode_login_status;

// ============================================================================
// Unit Tests for Status Code Decoder
// ============================================================================

#[test]
fn test_decode_success() {
    let msg = decode_login_status(0x00, 0x00);
    assert!(msg.contains("success"), "Success message should mention success");
}

#[test]
fn test_decode_target_moved_temporarily() {
    let msg = decode_login_status(0x01, 0x01);
    assert!(msg.contains("moved temporarily"), "Should mention temporary move");
    assert!(msg.contains("portal"), "Should mention portal");
}

#[test]
fn test_decode_target_moved_permanently() {
    let msg = decode_login_status(0x01, 0x02);
    assert!(msg.contains("moved permanently"), "Should mention permanent move");
    assert!(msg.contains("configuration"), "Should suggest config update");
}

#[test]
fn test_decode_auth_failure() {
    let msg = decode_login_status(0x02, 0x01);
    assert!(msg.contains("Authentication failed"), "Should indicate auth failure");
    assert!(msg.contains("username") || msg.contains("password"), "Should mention credentials");
}

#[test]
fn test_decode_authorization_failure() {
    let msg = decode_login_status(0x02, 0x02);
    assert!(msg.contains("Authorization failure"), "Should indicate authz failure");
    assert!(msg.contains("ACL"), "Should mention ACL");
    assert!(msg.contains("tgtadm"), "Should provide TGTD example");
}

#[test]
fn test_decode_target_not_found() {
    let msg = decode_login_status(0x02, 0x03);
    assert!(msg.contains("Target not found") || msg.contains("doesn't exist"), "Should indicate target not found");
    assert!(msg.contains("discovery") || msg.contains("discover"), "Should suggest running discovery");
}

#[test]
fn test_decode_target_removed() {
    let msg = decode_login_status(0x02, 0x04);
    assert!(msg.contains("removed"), "Should mention removal");
}

#[test]
fn test_decode_unsupported_version() {
    let msg = decode_login_status(0x02, 0x05);
    assert!(msg.contains("version") || msg.contains("Unsupported"), "Should mention version");
}

#[test]
fn test_decode_too_many_connections() {
    let msg = decode_login_status(0x02, 0x06);
    assert!(msg.contains("Too many connections") || msg.contains("maximum"), "Should indicate connection limit");
    assert!(msg.contains("MaxConnections"), "Should mention MaxConnections parameter");
}

#[test]
fn test_decode_missing_parameter() {
    let msg = decode_login_status(0x02, 0x07);
    assert!(msg.contains("Missing") || msg.contains("required"), "Should indicate missing parameter");
    assert!(msg.contains("InitiatorName"), "Should list InitiatorName");
    assert!(msg.contains("TargetName"), "Should list TargetName");
}

#[test]
fn test_decode_cannot_include_in_session() {
    let msg = decode_login_status(0x02, 0x08);
    assert!(msg.contains("session") || msg.contains("include"), "Should mention session");
}

#[test]
fn test_decode_session_type_not_supported() {
    let msg = decode_login_status(0x02, 0x09);
    assert!(msg.contains("Session type") || msg.contains("not supported"), "Should indicate unsupported type");
    // Message mentions "discovery" or "SendTargets" as troubleshooting
    assert!(msg.contains("discovery") || msg.contains("SendTargets") || msg.contains("TargetName"),
        "Should provide troubleshooting hints, got: {}", msg);
}

#[test]
fn test_decode_session_does_not_exist() {
    let msg = decode_login_status(0x02, 0x0A);
    assert!(msg.contains("does not exist") || msg.contains("Session"), "Should indicate session missing");
}

#[test]
fn test_decode_invalid_request_during_login() {
    let msg = decode_login_status(0x02, 0x0B);
    assert!(msg.contains("Invalid") || msg.contains("login"), "Should indicate invalid request");
}

#[test]
fn test_decode_target_error() {
    let msg = decode_login_status(0x03, 0x00);
    assert!(msg.contains("Target error"), "Should indicate target error");
}

#[test]
fn test_decode_service_unavailable() {
    let msg = decode_login_status(0x03, 0x01);
    assert!(msg.contains("unavailable") || msg.contains("service"), "Should indicate unavailable");
    assert!(msg.contains("retry") || msg.contains("Wait"), "Should suggest retry");
}

#[test]
fn test_decode_out_of_resources() {
    let msg = decode_login_status(0x03, 0x02);
    assert!(msg.contains("resources") || msg.contains("out of"), "Should indicate resource exhaustion");
}

#[test]
fn test_decode_unknown_status() {
    let msg = decode_login_status(0xFF, 0xFF);
    assert!(msg.contains("Unknown") || msg.contains("unrecognized"), "Should indicate unknown code");
    assert!(msg.contains("0xff") || msg.contains("0xFF"), "Should show the code");
    assert!(msg.contains("RFC 3720"), "Should reference RFC");
}

// ============================================================================
// Decoder Coverage Summary Test
// ============================================================================

#[test]
fn test_all_rfc_3720_status_codes_have_messages() {
    // This test documents all RFC 3720 status codes and verifies the decoder handles them
    let test_cases = vec![
        (0x00, 0x00, "Success"),
        (0x01, 0x01, "Target moved temporarily"),
        (0x01, 0x02, "Target moved permanently"),
        (0x02, 0x00, "Authentication failure"),
        (0x02, 0x01, "Authentication failed"),
        (0x02, 0x02, "Authorization failure"),
        (0x02, 0x03, "Target not found"),
        (0x02, 0x04, "Target removed"),
        (0x02, 0x05, "Unsupported version"),
        (0x02, 0x06, "Too many connections"),
        (0x02, 0x07, "Missing parameter"),
        (0x02, 0x08, "Cannot include in session"),
        (0x02, 0x09, "Session type not supported"),
        (0x02, 0x0A, "Session does not exist"),
        (0x02, 0x0B, "Invalid request during login"),
        (0x03, 0x00, "Target error"),
        (0x03, 0x01, "Service unavailable"),
        (0x03, 0x02, "Out of resources"),
    ];

    for (class, detail, description) in test_cases {
        let msg = decode_login_status(class, detail);
        assert!(!msg.is_empty(),
            "Status code 0x{:02x}{:02x} ({}) should have a non-empty message",
            class, detail, description);

        // Should not return "Unknown" for documented codes
        assert!(!msg.contains("Unknown"),
            "Status code 0x{:02x}{:02x} ({}) should not return 'Unknown' message, got: {}",
            class, detail, description, msg);
    }
}

// ============================================================================
// Integration Tests for Server-Implemented Status Codes
// ============================================================================
// Note: These require a running target and are marked #[ignore]
// Run with: cargo test -- --ignored

#[cfg(test)]
mod integration {
    use iscsi_target::IscsiClient;

    fn target_addr() -> &'static str {
        "127.0.0.1:3261"
    }

    /// Test that wrong target name returns TARGET_NOT_FOUND (0x0203)
    #[test]
    fn test_server_returns_target_not_found() {
        let _ = env_logger::builder().is_test(true).try_init();
        use iscsi_target::{IscsiTarget, ScsiBlockDevice, ScsiResult};
        use std::thread;
        use std::time::Duration;

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
                    return Err(iscsi_target::IscsiError::Scsi("Out of bounds".into()));
                }
                Ok(self.data[offset..offset + len].to_vec())
            }

            fn write(&mut self, lba: u64, data: &[u8], block_size: u32) -> ScsiResult<()> {
                let offset = (lba * block_size as u64) as usize;
                if offset + data.len() > self.data.len() {
                    return Err(iscsi_target::IscsiError::Scsi("Out of bounds".into()));
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

        // Start target with specific IQN
        let storage = TestStorage::new(10);
        let target = IscsiTarget::builder()
            .bind_addr("127.0.0.1:13261")
            .target_name("iqn.2025-12.test:correct-name")
            .build(storage)
            .expect("Failed to create target");

        let target = std::sync::Arc::new(target);
        let target_clone = target.clone();

        let target_thread = thread::spawn(move || {
            target_clone.run()
        });

        // Give target time to start
        thread::sleep(Duration::from_millis(500));

        // Try to login with wrong target name
        let mut client = IscsiClient::connect("127.0.0.1:13261")
            .expect("Failed to connect");

        let result = client.login(
            "iqn.test:initiator",
            "iqn.2025-12.test:wrong-name", // Wrong target name
        );

        assert!(result.is_err(), "Login with wrong target should fail");
        let err = result.unwrap_err().to_string();

        // Should mention "Target not found" and status code 0x0203
        assert!(err.contains("Target not found") || err.contains("0203"),
            "Error should indicate target not found: {}", err);

        // Clean up
        target.stop();
        target_thread.join().ok();
    }

    /// Test that missing InitiatorName returns MISSING_PARAMETER (0x0207)
    #[test]
    fn test_server_returns_missing_parameter() {
        let _ = env_logger::builder().is_test(true).try_init();
        use iscsi_target::{IscsiTarget, ScsiBlockDevice, ScsiResult};
        use iscsi_target::pdu::IscsiPdu;
        use std::io::{Read as IoRead, Write as IoWrite};
        use std::net::TcpStream;
        use std::thread;
        use std::time::Duration;

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
                    return Err(iscsi_target::IscsiError::Scsi("Out of bounds".into()));
                }
                Ok(self.data[offset..offset + len].to_vec())
            }

            fn write(&mut self, lba: u64, data: &[u8], block_size: u32) -> ScsiResult<()> {
                let offset = (lba * block_size as u64) as usize;
                if offset + data.len() > self.data.len() {
                    return Err(iscsi_target::IscsiError::Scsi("Out of bounds".into()));
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

        // Start target
        let storage = TestStorage::new(10);
        let target = IscsiTarget::builder()
            .bind_addr("127.0.0.1:13263")
            .target_name("iqn.2025-12.test:missing-param")
            .build(storage)
            .expect("Failed to create target");

        let target = std::sync::Arc::new(target);
        let target_clone = target.clone();

        let target_thread = thread::spawn(move || {
            target_clone.run()
        });

        // Give target time to start
        thread::sleep(Duration::from_millis(500));

        // Manually construct login PDU WITHOUT InitiatorName parameter
        let mut stream = TcpStream::connect("127.0.0.1:13263")
            .expect("Failed to connect");

        // Build login request with only TargetName (missing InitiatorName)
        let params = "TargetName=iqn.2025-12.test:missing-param\0AuthMethod=None\0";
        let padded_params = {
            let mut p = params.to_string();
            while p.len() % 4 != 0 {
                p.push('\0');
            }
            p.into_bytes()
        };

        let isid = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
        let login_pdu = IscsiPdu::login_request(
            isid,
            0, // TSIH
            0, // CID
            0, // CmdSN
            0, // ExpStatSN
            0, // CSG: Security Negotiation
            1, // NSG: Login Operational Negotiation
            true, // Transit
            padded_params,
        );

        // Send PDU
        let pdu_bytes = login_pdu.to_bytes();
        stream.write_all(&pdu_bytes).expect("Failed to write PDU");
        stream.flush().expect("Failed to flush");

        // Read response
        let mut bhs = [0u8; 48];
        stream.read_exact(&mut bhs).expect("Failed to read response BHS");

        // Parse response status from bytes 36-37
        let status_class = bhs[36];
        let status_detail = bhs[37];

        // Should be MISSING_PARAMETER (0x0207)
        assert_eq!(status_class, 0x02, "Status class should be INITIATOR_ERROR (0x02)");
        assert_eq!(status_detail, 0x07, "Status detail should be MISSING_PARAMETER (0x07)");

        // Clean up
        drop(stream);
        target.stop();
        target_thread.join().ok();
    }

    /// Test that CHAP mismatch returns AUTH_FAILURE (0x0201)
    #[test]
    fn test_server_returns_auth_failure() {
        let _ = env_logger::builder().is_test(true).try_init();
        use iscsi_target::{IscsiTarget, ScsiBlockDevice, ScsiResult, AuthConfig, ChapCredentials};
        use std::thread;
        use std::time::Duration;

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
                    return Err(iscsi_target::IscsiError::Scsi("Out of bounds".into()));
                }
                Ok(self.data[offset..offset + len].to_vec())
            }

            fn write(&mut self, lba: u64, data: &[u8], block_size: u32) -> ScsiResult<()> {
                let offset = (lba * block_size as u64) as usize;
                if offset + data.len() > self.data.len() {
                    return Err(iscsi_target::IscsiError::Scsi("Out of bounds".into()));
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

        // Start target with CHAP authentication required
        let storage = TestStorage::new(10);
        let auth_config = AuthConfig::Chap {
            credentials: ChapCredentials::new("testuser", "testpass"),
        };

        let target = IscsiTarget::builder()
            .bind_addr("127.0.0.1:13262")
            .target_name("iqn.2025-12.test:chap")
            .with_auth(auth_config)
            .build(storage)
            .expect("Failed to create target");

        let target = std::sync::Arc::new(target);
        let target_clone = target.clone();

        let target_thread = thread::spawn(move || {
            target_clone.run()
        });

        // Give target time to start
        thread::sleep(Duration::from_millis(500));

        // Try to login without CHAP (client only supports AuthMethod=None)
        let mut client = IscsiClient::connect("127.0.0.1:13262")
            .expect("Failed to connect");

        let result = client.login(
            "iqn.test:initiator",
            "iqn.2025-12.test:chap",
        );

        // Should fail with AUTH_FAILURE because server requires CHAP but client offers None
        assert!(result.is_err(), "Login without CHAP should fail when server requires it");
        let err = result.unwrap_err().to_string();

        // Should mention "Authentication failed" or status code 0x0201
        assert!(err.contains("Authentication") || err.contains("0201"),
            "Error should indicate authentication failure: {}", err);

        // Clean up
        target.stop();
        target_thread.join().ok();
    }

    /// Test that graceful shutdown returns SERVICE_UNAVAILABLE (0x0301)
    #[test]
    fn test_server_returns_service_unavailable_on_shutdown() {
        let _ = env_logger::builder().is_test(true).try_init();
        use iscsi_target::{IscsiTarget, ScsiBlockDevice, ScsiResult};
        use std::thread;
        use std::time::Duration;

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
                    return Err(iscsi_target::IscsiError::Scsi("Out of bounds".into()));
                }
                Ok(self.data[offset..offset + len].to_vec())
            }

            fn write(&mut self, lba: u64, data: &[u8], block_size: u32) -> ScsiResult<()> {
                let offset = (lba * block_size as u64) as usize;
                if offset + data.len() > self.data.len() {
                    return Err(iscsi_target::IscsiError::Scsi("Out of bounds".into()));
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

        // Start target in background thread
        let storage = TestStorage::new(10);
        let target = IscsiTarget::builder()
            .bind_addr("127.0.0.1:13260")
            .target_name("iqn.2025-12.test:shutdown")
            .build(storage)
            .expect("Failed to create target");

        let target = std::sync::Arc::new(target);
        let target_clone = target.clone();

        let target_thread = thread::spawn(move || {
            target_clone.run()
        });

        // Give target time to start
        thread::sleep(Duration::from_millis(500));

        // First login should succeed
        let mut client1 = IscsiClient::connect("127.0.0.1:13260")
            .expect("Failed to connect");

        client1.login("iqn.test:initiator", "iqn.2025-12.test:shutdown")
            .expect("First login should succeed before shutdown");

        // Initiate graceful shutdown
        target.shutdown_gracefully();
        thread::sleep(Duration::from_millis(100));

        // Second login should be rejected with SERVICE_UNAVAILABLE
        let mut client2 = IscsiClient::connect("127.0.0.1:13260")
            .expect("Failed to connect");

        let result = client2.login("iqn.test:initiator2", "iqn.2025-12.test:shutdown");

        assert!(result.is_err(), "Login during shutdown should fail");
        let err = result.unwrap_err().to_string();

        // Should contain "Service unavailable" or 0x0301
        assert!(err.contains("unavailable") || err.contains("0301"),
            "Error should indicate service unavailable: {}", err);

        // Clean up
        client1.logout().ok();
        target.stop();
        target_thread.join().ok();
    }

    /// Test that invalid SessionType returns SESSION_TYPE_NOT_SUPPORTED (0x0209)
    #[test]
    fn test_server_returns_session_type_not_supported() {
        let _ = env_logger::builder().is_test(true).try_init();
        use iscsi_target::{IscsiTarget, ScsiBlockDevice, ScsiResult};
        use iscsi_target::pdu::IscsiPdu;
        use std::io::{Read as IoRead, Write as IoWrite};
        use std::net::TcpStream;
        use std::thread;
        use std::time::Duration;

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
                    return Err(iscsi_target::IscsiError::Scsi("Out of bounds".into()));
                }
                Ok(self.data[offset..offset + len].to_vec())
            }

            fn write(&mut self, lba: u64, data: &[u8], block_size: u32) -> ScsiResult<()> {
                let offset = (lba * block_size as u64) as usize;
                if offset + data.len() > self.data.len() {
                    return Err(iscsi_target::IscsiError::Scsi("Out of bounds".into()));
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

        // Start target
        let storage = TestStorage::new(10);
        let target = IscsiTarget::builder()
            .bind_addr("127.0.0.1:13271")
            .target_name("iqn.2025-12.test:session-type")
            .build(storage)
            .expect("Failed to create target");

        let target = std::sync::Arc::new(target);
        let target_clone = target.clone();

        let target_thread = thread::spawn(move || {
            target_clone.run()
        });

        // Give target time to start
        thread::sleep(Duration::from_millis(500));

        // Manually construct login PDU with invalid SessionType
        let mut stream = TcpStream::connect("127.0.0.1:13271")
            .expect("Failed to connect");

        // Build login request with invalid SessionType="InvalidType"
        let params = "InitiatorName=iqn.2025-12.test:initiator\0TargetName=iqn.2025-12.test:session-type\0SessionType=InvalidType\0AuthMethod=None\0";
        let padded_params = {
            let mut p = params.to_string();
            while p.len() % 4 != 0 {
                p.push('\0');
            }
            p.into_bytes()
        };

        let isid = [0x01, 0x02, 0x03, 0x04, 0x05, 0x09];
        let login_pdu = IscsiPdu::login_request(
            isid,
            0, // TSIH
            0, // CID
            0, // CmdSN
            0, // ExpStatSN
            0, // CSG: Security Negotiation
            1, // NSG: Login Operational Negotiation
            true, // Transit
            padded_params,
        );

        // Send PDU
        let pdu_bytes = login_pdu.to_bytes();
        stream.write_all(&pdu_bytes).expect("Failed to write PDU");
        stream.flush().expect("Failed to flush");

        // Read response
        let mut bhs = [0u8; 48];
        stream.read_exact(&mut bhs).expect("Failed to read response BHS");

        // Parse response status from bytes 36-37
        let status_class = bhs[36];
        let status_detail = bhs[37];

        // Should be SESSION_TYPE_NOT_SUPPORTED (0x0209)
        assert_eq!(status_class, 0x02, "Status class should be INITIATOR_ERROR (0x02)");
        assert_eq!(status_detail, 0x09, "Status detail should be SESSION_TYPE_NOT_SUPPORTED (0x09)");

        // Clean up
        drop(stream);
        target.stop();
        target_thread.join().ok();
    }

    /// Test that exceeding connection limit returns TOO_MANY_CONNECTIONS (0x0206)
    #[test]
    fn test_server_returns_too_many_connections() {
        let _ = env_logger::builder().is_test(true).try_init();
        use iscsi_target::{IscsiTarget, ScsiBlockDevice, ScsiResult};
        use std::thread;
        use std::time::Duration;

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
                    return Err(iscsi_target::IscsiError::Scsi("Out of bounds".into()));
                }
                Ok(self.data[offset..offset + len].to_vec())
            }

            fn write(&mut self, lba: u64, data: &[u8], block_size: u32) -> ScsiResult<()> {
                let offset = (lba * block_size as u64) as usize;
                if offset + data.len() > self.data.len() {
                    return Err(iscsi_target::IscsiError::Scsi("Out of bounds".into()));
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

        // Start target with low connection limit for testing
        let storage = TestStorage::new(10);
        let target = IscsiTarget::builder()
            .bind_addr("127.0.0.1:13272")
            .target_name("iqn.2025-12.test:conn-limit")
            .max_connections(2) // Allow only 2 concurrent connections
            .build(storage)
            .expect("Failed to create target");

        let target = std::sync::Arc::new(target);
        let target_clone = target.clone();

        let target_thread = thread::spawn(move || {
            target_clone.run()
        });

        // Give target time to start
        thread::sleep(Duration::from_millis(500));

        // First two connections should succeed
        let mut client1 = IscsiClient::connect("127.0.0.1:13272")
            .expect("Failed to connect client 1");
        client1.login("iqn.test:initiator1", "iqn.2025-12.test:conn-limit")
            .expect("First connection should succeed");

        let mut client2 = IscsiClient::connect("127.0.0.1:13272")
            .expect("Failed to connect client 2");
        client2.login("iqn.test:initiator2", "iqn.2025-12.test:conn-limit")
            .expect("Second connection should succeed");

        // Verify active connection count
        assert_eq!(target.active_connection_count(), 2, "Should have 2 active connections");

        // Third connection should be rejected with TOO_MANY_CONNECTIONS
        let mut client3 = IscsiClient::connect("127.0.0.1:13272")
            .expect("Failed to connect client 3");

        let result = client3.login("iqn.test:initiator3", "iqn.2025-12.test:conn-limit");

        assert!(result.is_err(), "Third connection should fail due to connection limit");
        let err = result.unwrap_err().to_string();

        // Should contain "Too many connections" or status code 0x0206
        assert!(err.contains("Too many") || err.contains("0206") || err.contains("connection limit"),
            "Error should indicate too many connections: {}", err);

        // Clean up - close first two connections
        client1.logout().ok();
        drop(client1); // Explicitly drop to close TCP connection
        client2.logout().ok();
        drop(client2); // Explicitly drop to close TCP connection

        // Retry logic: wait for server to clean up connections and free slots
        // Server threads need time to process logout and exit
        let mut retry_count = 0;
        let max_retries = 20; // Wait up to 10 seconds (20 * 500ms)
        let mut client4_result = None;

        while retry_count < max_retries {
            thread::sleep(Duration::from_millis(500));

            match IscsiClient::connect("127.0.0.1:13272") {
                Ok(mut client4) => {
                    match client4.login("iqn.test:initiator4", "iqn.2025-12.test:conn-limit") {
                        Ok(_) => {
                            client4_result = Some(client4);
                            break;
                        }
                        Err(e) if e.to_string().contains("0206") || e.to_string().contains("Too many") => {
                            // Still at connection limit, retry
                            log::debug!("Still at connection limit, retrying... ({}/{})", retry_count + 1, max_retries);
                            retry_count += 1;
                        }
                        Err(e) => {
                            panic!("Unexpected error during retry: {}", e);
                        }
                    }
                }
                Err(e) => {
                    panic!("Failed to connect during retry: {}", e);
                }
            }
        }

        assert!(client4_result.is_some(),
            "Connection should succeed after others closed (waited {} retries)", retry_count);

        let mut client4 = client4_result.unwrap();

        // Verify we can now have connections again (connection slots freed up)
        assert!(target.active_connection_count() >= 1,
            "Should have at least one active connection after client4 login");

        // Final cleanup
        client4.logout().ok();
        target.stop();
        target_thread.join().ok();
    }

    /// Test that sending invalid PDU during login returns INVALID_REQUEST_DURING_LOGIN (0x020B)
    #[test]
    fn test_server_returns_invalid_request_during_login() {
        let _ = env_logger::builder().is_test(true).try_init();
        use iscsi_target::{IscsiTarget, ScsiBlockDevice, ScsiResult};
        use iscsi_target::pdu::{IscsiPdu, opcode};
        use std::io::{Read as IoRead, Write as IoWrite};
        use std::net::TcpStream;
        use std::thread;
        use std::time::Duration;

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
                    return Err(iscsi_target::IscsiError::Scsi("Out of bounds".into()));
                }
                Ok(self.data[offset..offset + len].to_vec())
            }

            fn write(&mut self, lba: u64, data: &[u8], block_size: u32) -> ScsiResult<()> {
                let offset = (lba * block_size as u64) as usize;
                if offset + data.len() > self.data.len() {
                    return Err(iscsi_target::IscsiError::Scsi("Out of bounds".into()));
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

        // Start target
        let storage = TestStorage::new(10);
        let target = IscsiTarget::builder()
            .bind_addr("127.0.0.1:13273")
            .target_name("iqn.2025-12.test:invalid-pdu")
            .build(storage)
            .expect("Failed to create target");

        let target = std::sync::Arc::new(target);
        let target_clone = target.clone();

        let target_thread = thread::spawn(move || {
            target_clone.run()
        });

        // Give target time to start
        thread::sleep(Duration::from_millis(500));

        // Connect but send a SCSI command PDU instead of Login Request
        let mut stream = TcpStream::connect("127.0.0.1:13273")
            .expect("Failed to connect");

        // Create a SCSI Command PDU (opcode 0x01) instead of Login Request (0x03)
        // This should be rejected with INVALID_REQUEST_DURING_LOGIN
        let mut pdu = IscsiPdu::new();
        pdu.opcode = opcode::SCSI_COMMAND;
        pdu.immediate = true;
        pdu.itt = 0x12345678;

        // Send the invalid PDU
        let pdu_bytes = pdu.to_bytes();
        stream.write_all(&pdu_bytes).expect("Failed to write PDU");
        stream.flush().expect("Failed to flush");

        // Read response
        let mut bhs = [0u8; 48];
        stream.read_exact(&mut bhs).expect("Failed to read response BHS");

        // Parse response - should be a Login Response with reject status
        let response_opcode = bhs[0] & 0x3F;

        // Response should be Login Response (0x23) with reject status
        if response_opcode == 0x23 {
            // Login Response - parse status from bytes 36-37
            let status_class = bhs[36];
            let status_detail = bhs[37];

            // Should be INVALID_REQUEST_DURING_LOGIN (0x020B)
            assert_eq!(status_class, 0x02, "Status class should be INITIATOR_ERROR (0x02)");
            assert_eq!(status_detail, 0x0B, "Status detail should be INVALID_REQUEST_DURING_LOGIN (0x0B)");
        } else {
            panic!("Expected Login Response (0x23), got opcode 0x{:02x}", response_opcode);
        }

        // Clean up
        drop(stream);
        target.stop();
        target_thread.join().ok();
    }

    /// Test that unsupported version returns UNSUPPORTED_VERSION (0x0205)
    #[test]
    fn test_server_returns_unsupported_version() {
        let _ = env_logger::builder().is_test(true).try_init();
        use iscsi_target::{IscsiTarget, ScsiBlockDevice, ScsiResult};
        use iscsi_target::pdu::IscsiPdu;
        use std::io::{Read as IoRead, Write as IoWrite};
        use std::net::TcpStream;
        use std::thread;
        use std::time::Duration;

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
                    return Err(iscsi_target::IscsiError::Scsi("Out of bounds".into()));
                }
                Ok(self.data[offset..offset + len].to_vec())
            }

            fn write(&mut self, lba: u64, data: &[u8], block_size: u32) -> ScsiResult<()> {
                let offset = (lba * block_size as u64) as usize;
                if offset + data.len() > self.data.len() {
                    return Err(iscsi_target::IscsiError::Scsi("Out of bounds".into()));
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

        // Start target
        let storage = TestStorage::new(10);
        let target = IscsiTarget::builder()
            .bind_addr("127.0.0.1:13274")
            .target_name("iqn.2025-12.test:version-test")
            .build(storage)
            .expect("Failed to create target");

        let target = std::sync::Arc::new(target);
        let target_clone = target.clone();

        let target_thread = thread::spawn(move || {
            target_clone.run()
        });

        // Give target time to start
        thread::sleep(Duration::from_millis(500));

        // Connect and send login with unsupported version
        let mut stream = TcpStream::connect("127.0.0.1:13274")
            .expect("Failed to connect");

        // Build login request with version_min=0x01, version_max=0x02
        // Target only supports 0x00, so this should be rejected
        let params = "InitiatorName=iqn.2025-12.test:initiator\0TargetName=iqn.2025-12.test:version-test\0SessionType=Normal\0AuthMethod=None\0";
        let padded_params = {
            let mut p = params.to_string();
            while p.len() % 4 != 0 {
                p.push('\0');
            }
            p.into_bytes()
        };

        let isid = [0x01, 0x02, 0x03, 0x04, 0x05, 0x0A];
        let mut login_pdu = IscsiPdu::login_request(
            isid,
            0, // TSIH
            0, // CID
            0, // CmdSN
            0, // ExpStatSN
            0, // CSG: Security Negotiation
            1, // NSG: Login Operational Negotiation
            true, // Transit
            padded_params,
        );

        // Set incompatible version: version_max=0x02, version_min=0x01
        // Target supports 0x00, which is outside [0x01, 0x02]
        login_pdu.version_or_reserved = 0x0201; // version_max=0x02, version_min=0x01

        // Send PDU
        let pdu_bytes = login_pdu.to_bytes();
        stream.write_all(&pdu_bytes).expect("Failed to write PDU");
        stream.flush().expect("Failed to flush");

        // Read response
        let mut bhs = [0u8; 48];
        stream.read_exact(&mut bhs).expect("Failed to read response BHS");

        // Parse response status from bytes 36-37
        let status_class = bhs[36];
        let status_detail = bhs[37];

        // Should be UNSUPPORTED_VERSION (0x0205)
        assert_eq!(status_class, 0x02, "Status class should be INITIATOR_ERROR (0x02)");
        assert_eq!(status_detail, 0x05, "Status detail should be UNSUPPORTED_VERSION (0x05)");

        // Clean up
        drop(stream);
        target.stop();
        target_thread.join().ok();
    }

    /// Test that exceeding session limit returns OUT_OF_RESOURCES (0x0302)
    #[test]
    fn test_server_returns_out_of_resources() {
        let _ = env_logger::builder().is_test(true).try_init();
        use iscsi_target::{IscsiTarget, ScsiBlockDevice, ScsiResult};
        use std::thread;
        use std::time::Duration;

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
                    return Err(iscsi_target::IscsiError::Scsi("Out of bounds".into()));
                }
                Ok(self.data[offset..offset + len].to_vec())
            }

            fn write(&mut self, lba: u64, data: &[u8], block_size: u32) -> ScsiResult<()> {
                let offset = (lba * block_size as u64) as usize;
                if offset + data.len() > self.data.len() {
                    return Err(iscsi_target::IscsiError::Scsi("Out of bounds".into()));
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

        // Start target with low session limit for testing
        let storage = TestStorage::new(10);
        let target = IscsiTarget::builder()
            .bind_addr("127.0.0.1:13275")
            .target_name("iqn.2025-12.test:resource-limit")
            .max_sessions(1) // Allow only 1 session
            .build(storage)
            .expect("Failed to create target");

        let target = std::sync::Arc::new(target);
        let target_clone = target.clone();

        let target_thread = thread::spawn(move || {
            target_clone.run()
        });

        // Give target time to start
        thread::sleep(Duration::from_millis(500));

        // First session should succeed
        let mut client1 = IscsiClient::connect("127.0.0.1:13275")
            .expect("Failed to connect client 1");
        client1.login("iqn.test:initiator1", "iqn.2025-12.test:resource-limit")
            .expect("First session should succeed");

        // Give server time to transition to FullFeaturePhase and increment session count
        thread::sleep(Duration::from_millis(200));

        // Second session should be rejected with OUT_OF_RESOURCES (session limit is 1)
        let mut client2 = IscsiClient::connect("127.0.0.1:13275")
            .expect("Failed to connect client 2");

        let result = client2.login("iqn.test:initiator2", "iqn.2025-12.test:resource-limit");

        assert!(result.is_err(), "Second session should fail due to resource limit");
        let err = result.unwrap_err().to_string();

        // Should contain "out of resources" or status code 0x0302
        assert!(err.contains("out of resources") || err.contains("0302") || err.contains("Out of resources"),
            "Error should indicate out of resources: {}", err);

        // Clean up first session
        client1.logout().ok();
        drop(client1);

        // Wait for cleanup
        thread::sleep(Duration::from_millis(500));

        // Now a new session should succeed
        let mut client3 = IscsiClient::connect("127.0.0.1:13275")
            .expect("Failed to connect client 3");
        client3.login("iqn.test:initiator3", "iqn.2025-12.test:resource-limit")
            .expect("Session should succeed after first session closed");

        // Final cleanup
        client3.logout().ok();
        target.stop();
        target_thread.join().ok();
    }

    /// Test that ACL enforcement returns AUTHORIZATION_FAILURE (0x0202)
    #[test]
    fn test_server_returns_authorization_failure() {
        let _ = env_logger::builder().is_test(true).try_init();
        use iscsi_target::{IscsiTarget, ScsiBlockDevice, ScsiResult};
        use std::thread;
        use std::time::Duration;

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
                    return Err(iscsi_target::IscsiError::Scsi("Out of bounds".into()));
                }
                Ok(self.data[offset..offset + len].to_vec())
            }

            fn write(&mut self, lba: u64, data: &[u8], block_size: u32) -> ScsiResult<()> {
                let offset = (lba * block_size as u64) as usize;
                if offset + data.len() > self.data.len() {
                    return Err(iscsi_target::IscsiError::Scsi("Out of bounds".into()));
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

        // Start target with ACL restricting access
        let storage = TestStorage::new(10);
        let target = IscsiTarget::builder()
            .bind_addr("127.0.0.1:13276")
            .target_name("iqn.2025-12.test:acl-test")
            .allowed_initiators(vec![
                "iqn.test:allowed-initiator".to_string(),
            ])
            .build(storage)
            .expect("Failed to create target");

        let target = std::sync::Arc::new(target);
        let target_clone = target.clone();

        let target_thread = thread::spawn(move || {
            target_clone.run()
        });

        // Give target time to start
        thread::sleep(Duration::from_millis(500));

        // Login with allowed initiator should succeed
        let mut client_allowed = IscsiClient::connect("127.0.0.1:13276")
            .expect("Failed to connect allowed client");
        client_allowed.login("iqn.test:allowed-initiator", "iqn.2025-12.test:acl-test")
            .expect("Login with allowed initiator should succeed");
        client_allowed.logout().ok();

        // Give server time to cleanup
        thread::sleep(Duration::from_millis(200));

        // Login with non-allowed initiator should fail with AUTHORIZATION_FAILURE
        let mut client_denied = IscsiClient::connect("127.0.0.1:13276")
            .expect("Failed to connect denied client");
        let result = client_denied.login("iqn.test:denied-initiator", "iqn.2025-12.test:acl-test");

        match result {
            Err(iscsi_target::IscsiError::Protocol(ref msg)) => {
                assert!(
                    msg.contains("AUTHORIZATION_FAILURE") || msg.contains("Authorization failure"),
                    "Expected AUTHORIZATION_FAILURE error, got: {}",
                    msg
                );
            }
            Ok(_) => panic!("Login should have failed with AUTHORIZATION_FAILURE"),
            Err(e) => panic!("Expected Protocol error with AUTHORIZATION_FAILURE, got: {:?}", e),
        }

        // Cleanup
        target.stop();
        target_thread.join().ok();
    }
}
