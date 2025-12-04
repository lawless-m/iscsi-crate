//! iSCSI target server implementation
//!
//! This module provides the main server structure, TCP listener, and connection handling.

use crate::error::{IscsiError, ScsiResult};
use crate::pdu::{self, IscsiPdu, BHS_SIZE, opcode, flags, scsi_status, serialize_text_parameters};
use crate::scsi::{ScsiBlockDevice, ScsiHandler};
use crate::session::{IscsiSession, SessionState, SessionType};
use byteorder::{BigEndian, ByteOrder};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, Shutdown};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::Duration;

/// Default iSCSI port
pub const ISCSI_PORT: u16 = 3260;

/// iSCSI target server
pub struct IscsiTarget<D: ScsiBlockDevice> {
    bind_addr: String,
    target_name: String,
    target_alias: String,
    device: Arc<Mutex<D>>,
    running: Arc<AtomicBool>,
}

impl<D: ScsiBlockDevice + Send + 'static> IscsiTarget<D> {
    /// Create a new builder for configuring the target
    pub fn builder() -> IscsiTargetBuilder<D> {
        IscsiTargetBuilder::new()
    }

    /// Run the iSCSI target server
    ///
    /// This blocks the current thread and processes incoming connections.
    pub fn run(self) -> ScsiResult<()> {
        log::info!("iSCSI target starting on {}", self.bind_addr);
        log::info!("Target name: {}", self.target_name);

        let listener = TcpListener::bind(&self.bind_addr)
            .map_err(|e| IscsiError::Io(e))?;

        // Set non-blocking for graceful shutdown checking
        listener.set_nonblocking(true)
            .map_err(|e| IscsiError::Io(e))?;

        self.running.store(true, Ordering::SeqCst);

        log::info!("iSCSI target listening on {}", self.bind_addr);

        while self.running.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, addr)) => {
                    log::info!("New connection from {}", addr);

                    let device = Arc::clone(&self.device);
                    let target_name = self.target_name.clone();
                    let target_alias = self.target_alias.clone();
                    let running = Arc::clone(&self.running);

                    thread::spawn(move || {
                        if let Err(e) = handle_connection(stream, device, &target_name, &target_alias, running) {
                            log::error!("Connection error from {}: {}", addr, e);
                        }
                        log::info!("Connection closed from {}", addr);
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No connection available, sleep briefly and retry
                    thread::sleep(Duration::from_millis(100));
                }
                Err(e) => {
                    log::error!("Accept error: {}", e);
                }
            }
        }

        log::info!("iSCSI target shutting down");
        Ok(())
    }

    /// Signal the server to stop
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Check if the server is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

/// Handle a single iSCSI connection
fn handle_connection<D: ScsiBlockDevice>(
    mut stream: TcpStream,
    device: Arc<Mutex<D>>,
    target_name: &str,
    target_alias: &str,
    running: Arc<AtomicBool>,
) -> ScsiResult<()> {
    // Set blocking mode and timeouts for the connection
    stream.set_nonblocking(false).map_err(|e| IscsiError::Io(e))?;
    stream.set_read_timeout(Some(Duration::from_secs(300))).map_err(|e| IscsiError::Io(e))?;
    stream.set_write_timeout(Some(Duration::from_secs(30))).map_err(|e| IscsiError::Io(e))?;

    let mut session = IscsiSession::new();
    session.params.target_name = target_name.to_string();
    session.params.target_alias = target_alias.to_string();

    // Main connection loop
    while running.load(Ordering::SeqCst) {
        // Read PDU from stream
        let pdu = match read_pdu(&mut stream) {
            Ok(pdu) => pdu,
            Err(IscsiError::Io(ref e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                log::debug!("Connection closed by initiator");
                break;
            }
            Err(IscsiError::Io(ref e)) if e.kind() == std::io::ErrorKind::WouldBlock => {
                continue;
            }
            Err(IscsiError::Io(ref e)) if e.kind() == std::io::ErrorKind::TimedOut => {
                log::debug!("Connection timeout, closing");
                break;
            }
            Err(e) => {
                log::error!("Error reading PDU: {}", e);
                break;
            }
        };

        log::debug!("Received PDU: {} (opcode 0x{:02x})", pdu.opcode_name(), pdu.opcode);

        // Process PDU based on session state
        let response = match session.state {
            SessionState::Free | SessionState::SecurityNegotiation | SessionState::LoginOperationalNegotiation => {
                handle_login_phase(&mut session, &pdu, target_name)?
            }
            SessionState::FullFeaturePhase => {
                handle_full_feature_phase(&mut session, &pdu, &device, target_name)?
            }
            SessionState::Logout => {
                log::info!("Session logout complete");
                break;
            }
            SessionState::Failed => {
                log::error!("Session in failed state");
                break;
            }
        };

        // Send response(s)
        for resp_pdu in response {
            log::debug!("Sending PDU: {} (opcode 0x{:02x})", resp_pdu.opcode_name(), resp_pdu.opcode);
            write_pdu(&mut stream, &resp_pdu)?;
        }
    }

    // Clean shutdown
    let _ = stream.shutdown(Shutdown::Both);
    Ok(())
}

/// Read a PDU from the TCP stream
fn read_pdu(stream: &mut TcpStream) -> ScsiResult<IscsiPdu> {
    // Read 48-byte BHS
    let mut bhs = [0u8; BHS_SIZE];
    stream.read_exact(&mut bhs).map_err(|e| IscsiError::Io(e))?;

    // Parse AHS length and data segment length from BHS
    let ahs_length = bhs[4] as usize * 4;
    let data_length = ((bhs[5] as u32) << 16) | ((bhs[6] as u32) << 8) | (bhs[7] as u32);
    let padded_data_len = ((data_length as usize + 3) / 4) * 4;

    // Read remaining data (AHS + data segment + padding)
    let total_len = BHS_SIZE + ahs_length + padded_data_len;
    let mut full_pdu = vec![0u8; total_len];
    full_pdu[..BHS_SIZE].copy_from_slice(&bhs);

    if total_len > BHS_SIZE {
        stream.read_exact(&mut full_pdu[BHS_SIZE..]).map_err(|e| IscsiError::Io(e))?;
    }

    IscsiPdu::from_bytes(&full_pdu)
}

/// Write a PDU to the TCP stream
fn write_pdu(stream: &mut TcpStream, pdu: &IscsiPdu) -> ScsiResult<()> {
    let bytes = pdu.to_bytes();
    stream.write_all(&bytes).map_err(|e| IscsiError::Io(e))?;
    stream.flush().map_err(|e| IscsiError::Io(e))?;
    Ok(())
}

/// Handle PDUs during login phase
fn handle_login_phase(
    session: &mut IscsiSession,
    pdu: &IscsiPdu,
    target_name: &str,
) -> ScsiResult<Vec<IscsiPdu>> {
    match pdu.opcode {
        opcode::LOGIN_REQUEST => {
            let response = session.process_login(pdu, target_name)?;
            Ok(vec![response])
        }
        opcode::TEXT_REQUEST => {
            // Text request during login (e.g., SendTargets for discovery)
            handle_text_request(session, pdu, target_name)
        }
        _ => {
            log::warn!("Unexpected opcode 0x{:02x} during login phase", pdu.opcode);
            // Could send a reject PDU here
            Ok(vec![])
        }
    }
}

/// Handle PDUs during full feature phase
fn handle_full_feature_phase<D: ScsiBlockDevice>(
    session: &mut IscsiSession,
    pdu: &IscsiPdu,
    device: &Arc<Mutex<D>>,
    target_name: &str,
) -> ScsiResult<Vec<IscsiPdu>> {
    match pdu.opcode {
        opcode::SCSI_COMMAND => {
            handle_scsi_command(session, pdu, device)
        }
        opcode::SCSI_DATA_OUT => {
            handle_scsi_data_out(session, pdu, device)
        }
        opcode::NOP_OUT => {
            let response = session.process_nop_out(pdu)?;
            Ok(vec![response])
        }
        opcode::LOGOUT_REQUEST => {
            let response = session.process_logout(pdu)?;
            Ok(vec![response])
        }
        opcode::TEXT_REQUEST => {
            handle_text_request(session, pdu, target_name)
        }
        opcode::TASK_MANAGEMENT_REQUEST => {
            handle_task_management(session, pdu)
        }
        _ => {
            log::warn!("Unsupported opcode 0x{:02x} in full feature phase", pdu.opcode);
            Ok(vec![])
        }
    }
}

/// Handle SCSI Command PDU
fn handle_scsi_command<D: ScsiBlockDevice>(
    session: &mut IscsiSession,
    pdu: &IscsiPdu,
    device: &Arc<Mutex<D>>,
) -> ScsiResult<Vec<IscsiPdu>> {
    let cmd = pdu.parse_scsi_command()?;

    log::debug!(
        "SCSI Command: CDB[0]=0x{:02x}, LUN={}, ITT=0x{:08x}, ExpLen={}",
        cmd.cdb[0], cmd.lun, cmd.itt, cmd.expected_data_length
    );

    // Validate command sequence number
    let cmd_sn = BigEndian::read_u32(&pdu.specific[4..8]);
    if !session.validate_cmd_sn(cmd_sn) {
        log::warn!("Invalid CmdSN: {}, expected: {}", cmd_sn, session.exp_cmd_sn);
    }

    let device_guard = device.lock().map_err(|_| {
        IscsiError::Scsi("Device lock poisoned".to_string())
    })?;

    // Handle the SCSI command
    let response = ScsiHandler::handle_command(&cmd.cdb, &*device_guard, None)?;

    drop(device_guard);

    // Build response PDU(s)
    let mut responses = Vec::new();

    if cmd.read && !response.data.is_empty() {
        // Send data with Data-In PDU(s)
        let max_data_seg = session.params.max_xmit_data_segment_length as usize;
        let mut offset = 0u32;
        let mut data_sn = 0u32;

        while offset < response.data.len() as u32 {
            let remaining = response.data.len() - offset as usize;
            let chunk_size = remaining.min(max_data_seg);
            let is_final = offset as usize + chunk_size >= response.data.len();

            let chunk = response.data[offset as usize..offset as usize + chunk_size].to_vec();

            let data_in = IscsiPdu::scsi_data_in(
                cmd.itt,
                0xFFFF_FFFF, // TTT
                session.next_stat_sn(),
                session.exp_cmd_sn,
                session.max_cmd_sn,
                data_sn,
                offset,
                chunk,
                is_final,
                if is_final { Some(response.status) } else { None },
            );

            responses.push(data_in);
            offset += chunk_size as u32;
            data_sn += 1;
        }
    } else {
        // No data or write command - send SCSI Response
        let sense_data = response.sense.as_ref().map(|s| s.to_bytes());
        let scsi_resp = IscsiPdu::scsi_response(
            cmd.itt,
            session.next_stat_sn(),
            session.exp_cmd_sn,
            session.max_cmd_sn,
            response.status,
            0, // iSCSI response code: completed
            0, // residual count
            sense_data.as_deref(),
        );
        responses.push(scsi_resp);
    }

    Ok(responses)
}

/// Handle SCSI Data-Out PDU (write data from initiator)
fn handle_scsi_data_out<D: ScsiBlockDevice>(
    session: &mut IscsiSession,
    pdu: &IscsiPdu,
    device: &Arc<Mutex<D>>,
) -> ScsiResult<Vec<IscsiPdu>> {
    let data_out = pdu.parse_scsi_data_out()?;

    log::debug!(
        "SCSI Data-Out: ITT=0x{:08x}, DataSN={}, Offset={}, Len={}",
        data_out.itt, data_out.data_sn, data_out.buffer_offset, data_out.data.len()
    );

    // For simplicity, we'll handle immediate/unsolicited writes here
    // In a full implementation, we'd track the original command and accumulate data

    // If this is the final data PDU, we need to send a response
    if data_out.final_flag {
        // Get device access for write
        let mut device_guard = device.lock().map_err(|_| {
            IscsiError::Scsi("Device lock poisoned".to_string())
        })?;

        // Parse LBA from the stored command (simplified - would need command tracking)
        // For now, we'll use offset / block_size as a simple approximation
        let block_size = device_guard.block_size();
        let lba = (data_out.buffer_offset / block_size) as u64;

        // Write the data
        let write_result = device_guard.write(lba, &data_out.data, block_size);

        drop(device_guard);

        let (status, sense) = match write_result {
            Ok(()) => (scsi_status::GOOD, None),
            Err(_) => {
                let sense = crate::scsi::SenseData::medium_error();
                (pdu::scsi_status::CHECK_CONDITION, Some(sense.to_bytes()))
            }
        };

        let response = IscsiPdu::scsi_response(
            data_out.itt,
            session.next_stat_sn(),
            session.exp_cmd_sn,
            session.max_cmd_sn,
            status,
            0,
            0,
            sense.as_deref(),
        );

        Ok(vec![response])
    } else {
        // More data expected, no response yet
        Ok(vec![])
    }
}

/// Handle Text Request (e.g., SendTargets for discovery)
fn handle_text_request(
    session: &mut IscsiSession,
    pdu: &IscsiPdu,
    target_name: &str,
) -> ScsiResult<Vec<IscsiPdu>> {
    let text_req = pdu.parse_text_request()?;

    log::debug!("Text Request: ITT=0x{:08x}, params: {:?}", text_req.itt, text_req.parameters);

    // Check for SendTargets request (discovery)
    let is_send_targets = text_req.parameters.iter()
        .any(|(k, v)| k == "SendTargets" && (v == "All" || v == ""));

    let response_params = if is_send_targets && session.session_type == SessionType::Discovery {
        // Return target list
        session.handle_send_targets(target_name, &format!("0.0.0.0:{}", ISCSI_PORT))
    } else {
        // Echo back or handle other text parameters
        vec![]
    };

    let response_data = serialize_text_parameters(&response_params);

    let response = IscsiPdu::text_response(
        text_req.itt,
        0xFFFF_FFFF, // TTT
        session.next_stat_sn(),
        session.exp_cmd_sn,
        session.max_cmd_sn,
        true, // final
        response_data,
    );

    Ok(vec![response])
}

/// Handle Task Management Request
fn handle_task_management(
    session: &mut IscsiSession,
    pdu: &IscsiPdu,
) -> ScsiResult<Vec<IscsiPdu>> {
    // For now, just acknowledge task management requests
    // A full implementation would handle ABORT TASK, LUN RESET, etc.

    let function = pdu.flags & 0x7F;
    log::debug!("Task Management: function={}", function);

    // Build response
    let mut response = IscsiPdu::new();
    response.opcode = opcode::TASK_MANAGEMENT_RESPONSE;
    response.flags = flags::FINAL;
    response.itt = pdu.itt;

    // Response code: function complete
    response.specific[0] = 0x00;
    // StatSN
    response.specific[4..8].copy_from_slice(&session.next_stat_sn().to_be_bytes());
    // ExpCmdSN
    response.specific[8..12].copy_from_slice(&session.exp_cmd_sn.to_be_bytes());
    // MaxCmdSN
    response.specific[12..16].copy_from_slice(&session.max_cmd_sn.to_be_bytes());

    Ok(vec![response])
}

/// Builder for configuring an iSCSI target
pub struct IscsiTargetBuilder<D: ScsiBlockDevice> {
    bind_addr: Option<String>,
    target_name: Option<String>,
    target_alias: Option<String>,
    _phantom: std::marker::PhantomData<D>,
}

impl<D: ScsiBlockDevice> IscsiTargetBuilder<D> {
    fn new() -> Self {
        Self {
            bind_addr: None,
            target_name: None,
            target_alias: None,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Set the bind address (default: 0.0.0.0:3260)
    pub fn bind_addr(mut self, addr: &str) -> Self {
        self.bind_addr = Some(addr.to_string());
        self
    }

    /// Set the iSCSI target name (IQN format)
    ///
    /// Example: iqn.2025-12.local:storage.disk1
    pub fn target_name(mut self, name: &str) -> Self {
        self.target_name = Some(name.to_string());
        self
    }

    /// Set the target alias (human-readable name)
    pub fn target_alias(mut self, alias: &str) -> Self {
        self.target_alias = Some(alias.to_string());
        self
    }

    /// Build the target with the specified storage device
    pub fn build(self, device: D) -> ScsiResult<IscsiTarget<D>> {
        let bind_addr = self.bind_addr.unwrap_or_else(|| format!("0.0.0.0:{}", ISCSI_PORT));
        let target_name = self.target_name.unwrap_or_else(|| {
            "iqn.2025-12.local:storage.default".to_string()
        });
        let target_alias = self.target_alias.unwrap_or_else(|| "iSCSI Target".to_string());

        // Validate IQN format (basic check)
        if !target_name.starts_with("iqn.") && !target_name.starts_with("eui.") && !target_name.starts_with("naa.") {
            return Err(IscsiError::Config(
                "target_name must be in IQN, EUI, or NAA format (e.g., iqn.2025-12.local:storage.disk1)".to_string()
            ));
        }

        Ok(IscsiTarget {
            bind_addr,
            target_name,
            target_alias,
            device: Arc::new(Mutex::new(device)),
            running: Arc::new(AtomicBool::new(false)),
        })
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock device for testing
    struct MockDevice {
        capacity: u64,
        block_size: u32,
        data: Vec<u8>,
    }

    impl MockDevice {
        fn new(capacity: u64, block_size: u32) -> Self {
            let size = (capacity * block_size as u64) as usize;
            MockDevice {
                capacity,
                block_size,
                data: vec![0u8; size],
            }
        }
    }

    impl ScsiBlockDevice for MockDevice {
        fn read(&self, lba: u64, blocks: u32, block_size: u32) -> ScsiResult<Vec<u8>> {
            let offset = (lba * block_size as u64) as usize;
            let len = (blocks * block_size) as usize;
            if offset + len > self.data.len() {
                return Err(IscsiError::Scsi("Read out of bounds".into()));
            }
            Ok(self.data[offset..offset + len].to_vec())
        }

        fn write(&mut self, lba: u64, data: &[u8], block_size: u32) -> ScsiResult<()> {
            let offset = (lba * block_size as u64) as usize;
            if offset + data.len() > self.data.len() {
                return Err(IscsiError::Scsi("Write out of bounds".into()));
            }
            self.data[offset..offset + data.len()].copy_from_slice(data);
            Ok(())
        }

        fn capacity(&self) -> u64 {
            self.capacity
        }

        fn block_size(&self) -> u32 {
            self.block_size
        }
    }

    #[test]
    fn test_builder_default() {
        let device = MockDevice::new(1000, 512);
        let target = IscsiTarget::builder()
            .build(device)
            .unwrap();

        assert_eq!(target.bind_addr, "0.0.0.0:3260");
        assert!(target.target_name.starts_with("iqn."));
    }

    #[test]
    fn test_builder_custom() {
        let device = MockDevice::new(1000, 512);
        let target = IscsiTarget::builder()
            .bind_addr("127.0.0.1:3260")
            .target_name("iqn.2025-12.test:disk1")
            .target_alias("Test Disk")
            .build(device)
            .unwrap();

        assert_eq!(target.bind_addr, "127.0.0.1:3260");
        assert_eq!(target.target_name, "iqn.2025-12.test:disk1");
        assert_eq!(target.target_alias, "Test Disk");
    }

    #[test]
    fn test_builder_invalid_iqn() {
        let device = MockDevice::new(1000, 512);
        let result = IscsiTarget::builder()
            .target_name("invalid-name")
            .build(device);

        assert!(result.is_err());
    }

    #[test]
    fn test_running_flag() {
        let device = MockDevice::new(1000, 512);
        let target = IscsiTarget::builder()
            .build(device)
            .unwrap();

        assert!(!target.is_running());
        target.running.store(true, Ordering::SeqCst);
        assert!(target.is_running());
        target.stop();
        assert!(!target.is_running());
    }

    #[test]
    fn test_pdu_roundtrip() {
        // Test that PDU serialization/deserialization works correctly
        let mut pdu = IscsiPdu::new();
        pdu.opcode = opcode::NOP_IN;
        pdu.flags = flags::FINAL;
        pdu.itt = 0x12345678;

        let bytes = pdu.to_bytes();
        let parsed = IscsiPdu::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.opcode, opcode::NOP_IN);
        assert_eq!(parsed.flags, flags::FINAL);
        assert_eq!(parsed.itt, 0x12345678);
    }
}
