//! iSCSI target server implementation
//!
//! This module provides the main server structure, TCP listener, and connection handling.

use crate::error::{IscsiError, ScsiResult};
use crate::pdu::{self, IscsiPdu, BHS_SIZE, opcode, flags, scsi_status, serialize_text_parameters};
use crate::scsi::{ScsiBlockDevice, ScsiHandler, ScsiResponse};
use crate::session::{IscsiSession, PendingWrite, SessionState};
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
    auth_config: crate::auth::AuthConfig,
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
            .map_err(IscsiError::Io)?;

        // Set non-blocking for graceful shutdown checking
        listener.set_nonblocking(true)
            .map_err(IscsiError::Io)?;

        self.running.store(true, Ordering::SeqCst);

        log::info!("iSCSI target listening on {}", self.bind_addr);

        while self.running.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, addr)) => {
                    log::info!("New connection from {}", addr);

                    let device = Arc::clone(&self.device);
                    let target_name = self.target_name.clone();
                    let target_alias = self.target_alias.clone();
                    let auth_config = self.auth_config.clone();
                    let running = Arc::clone(&self.running);

                    thread::spawn(move || {
                        if let Err(e) = handle_connection(stream, device, &target_name, &target_alias, auth_config, running) {
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
    auth_config: crate::auth::AuthConfig,
    running: Arc<AtomicBool>,
) -> ScsiResult<()> {
    // Get the local address that the client connected to
    let local_addr = stream.local_addr().map_err(IscsiError::Io)?;
    // Set blocking mode and timeouts for the connection
    stream.set_nonblocking(false).map_err(IscsiError::Io)?;
    // During login phase, use a shorter timeout to detect stalled logins quickly
    // This prevents resource leaks from clients that initiate login but never complete it
    stream.set_read_timeout(Some(Duration::from_secs(5))).map_err(IscsiError::Io)?;
    stream.set_write_timeout(Some(Duration::from_secs(5))).map_err(IscsiError::Io)?;

    let mut session = IscsiSession::new();
    session.params.target_name = target_name.to_string();
    session.params.target_alias = target_alias.to_string();
    session.set_auth_config(auth_config);

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
        let target_address = local_addr.to_string();
        let prev_state = session.state.clone();
        let response = match session.state {
            SessionState::Free | SessionState::SecurityNegotiation | SessionState::LoginOperationalNegotiation => {
                handle_login_phase(&mut session, &pdu, target_name, &target_address)?
            }
            SessionState::FullFeaturePhase => {
                handle_full_feature_phase(&mut session, &pdu, &device, target_name, &target_address)?
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

        // Adjust timeout when transitioning to FullFeaturePhase
        if prev_state != SessionState::FullFeaturePhase && session.state == SessionState::FullFeaturePhase {
            log::info!("Session entered FullFeaturePhase, increasing timeout");
            stream.set_read_timeout(Some(Duration::from_secs(300))).ok();
            stream.set_write_timeout(Some(Duration::from_secs(30))).ok();
        }

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
    stream.read_exact(&mut bhs).map_err(IscsiError::Io)?;

    // Parse AHS length and data segment length from BHS
    let ahs_length = bhs[4] as usize * 4;
    let data_length = ((bhs[5] as u32) << 16) | ((bhs[6] as u32) << 8) | (bhs[7] as u32);
    let padded_data_len = (data_length as usize).div_ceil(4) * 4;

    // Read remaining data (AHS + data segment + padding)
    let total_len = BHS_SIZE + ahs_length + padded_data_len;
    let mut full_pdu = vec![0u8; total_len];
    full_pdu[..BHS_SIZE].copy_from_slice(&bhs);

    if total_len > BHS_SIZE {
        stream.read_exact(&mut full_pdu[BHS_SIZE..]).map_err(IscsiError::Io)?;
    }

    let pdu = IscsiPdu::from_bytes(&full_pdu)?;

    // Log received PDU header details
    if full_pdu.len() >= 48 {
        log::debug!("Received PDU header hex: {}", full_pdu[0..48].iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" "));
        log::debug!("  [0] Opcode: 0x{:02x}", full_pdu[0]);
        log::debug!("  [1] Flags: 0x{:02x}", full_pdu[1]);
        log::debug!("  [5-7] DataSegmentLength: {} bytes", (full_pdu[5] as u32) << 16 | (full_pdu[6] as u32) << 8 | full_pdu[7] as u32);
    }

    Ok(pdu)
}

/// Write a PDU to the TCP stream
fn write_pdu(stream: &mut TcpStream, pdu: &IscsiPdu) -> ScsiResult<()> {
    let bytes = pdu.to_bytes();

    // Log PDU header in detail
    if bytes.len() >= 48 {
        log::debug!("PDU Header hex: {}", bytes[0..48].iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" "));
        log::debug!("  [0] Opcode: 0x{:02x}", bytes[0]);
        log::debug!("  [1] Flags: 0x{:02x}", bytes[1]);
        log::debug!("  [5-7] DataSegmentLength: {} bytes", (bytes[5] as u32) << 16 | (bytes[6] as u32) << 8 | bytes[7] as u32);
        log::debug!("  Data segment ({} bytes): {:?}", bytes.len() - 48, String::from_utf8_lossy(&bytes[48..]));
    }

    stream.write_all(&bytes).map_err(IscsiError::Io)?;
    stream.flush().map_err(IscsiError::Io)?;
    Ok(())
}

/// Handle PDUs during login phase
fn handle_login_phase(
    session: &mut IscsiSession,
    pdu: &IscsiPdu,
    target_name: &str,
    target_address: &str,
) -> ScsiResult<Vec<IscsiPdu>> {
    match pdu.opcode {
        opcode::LOGIN_REQUEST => {
            let response = session.process_login(pdu, target_name)?;
            Ok(vec![response])
        }
        opcode::TEXT_REQUEST => {
            // Text request during login (e.g., SendTargets for discovery)
            handle_text_request(session, pdu, target_name, target_address)
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
    target_address: &str,
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
            handle_text_request(session, pdu, target_name, target_address)
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

    log::warn!(
        "SCSI Command: CDB[0]=0x{:02x}, LUN=0x{:016x}, ITT=0x{:08x}, ExpLen={}, read={}, write={}, final={}, data_len={}",
        cmd.cdb[0], cmd.lun, cmd.itt, cmd.expected_data_length, cmd.read, cmd.write, cmd.final_flag, pdu.data.len()
    );

    // Validate LUN - only LUN 0 is supported
    // iSCSI LUNs are encoded per RFC 3720 section 3.4.6.1
    // For simplicity, we check if the raw LUN value is 0
    // LUN 0 is always encoded as 0x0000000000000000 regardless of addressing method
    if cmd.lun != 0 {
        log::warn!("Command 0x{:02x} to invalid LUN: 0x{:016x}", cmd.cdb[0], cmd.lun);
        let sense = crate::scsi::SenseData::new(
            crate::scsi::sense_key::ILLEGAL_REQUEST,
            crate::scsi::asc::LOGICAL_UNIT_NOT_SUPPORTED,
            0,
        );
        return Ok(vec![IscsiPdu::scsi_response(
            cmd.itt,
            session.next_stat_sn(),
            session.exp_cmd_sn,
            session.max_cmd_sn,
            pdu::scsi_status::CHECK_CONDITION,
            0,
            0,
            Some(&sense.to_bytes()),
        )]);
    }

    // Validate command sequence number
    let cmd_sn = BigEndian::read_u32(&pdu.specific[4..8]);
    if !session.validate_cmd_sn(cmd_sn) {
        log::warn!("Invalid CmdSN: {}, expected: {}", cmd_sn, session.exp_cmd_sn);
    }

    // Check command type
    let opcode = cmd.cdb[0];
    log::debug!("Processing SCSI opcode 0x{:02x}", opcode);
    let is_sync_cache = opcode == 0x35 || opcode == 0x91;
    let is_write_cmd = matches!(opcode, 0x0a | 0x2a | 0x8a);

    // Handle WRITE commands separately (they use immediate data or Data-Out PDUs)
    if is_write_cmd {
        // Extract LBA and transfer length from CDB
        let (lba, transfer_length) = match opcode {
            0x0a | 0x2a => {
                // WRITE(6) or WRITE(10)
                if opcode == 0x0a && cmd.cdb.len() >= 6 {
                    // WRITE(6): LBA is 21 bits in bytes 1-3
                    let lba_21 = ((cmd.cdb[1] as u32 & 0x1F) << 16)
                               | ((cmd.cdb[2] as u32) << 8)
                               | (cmd.cdb[3] as u32);
                    let length = cmd.cdb[4] as u32;
                    (lba_21 as u64, length)
                } else if opcode == 0x2a && cmd.cdb.len() >= 10 {
                    // WRITE(10): LBA is 32 bits in bytes 2-5
                    let lba = BigEndian::read_u32(&cmd.cdb[2..6]) as u64;
                    let length = BigEndian::read_u16(&cmd.cdb[7..9]) as u32;
                    (lba, length)
                } else {
                    (0, 0)
                }
            }
            0x8a => {
                // WRITE(16): LBA is 64 bits in bytes 2-9
                if cmd.cdb.len() >= 16 {
                    let lba = BigEndian::read_u64(&cmd.cdb[2..10]);
                    let length = BigEndian::read_u32(&cmd.cdb[10..14]);
                    (lba, length)
                } else {
                    (0, 0)
                }
            }
            _ => (0, 0),
        };

        if transfer_length > 0 {
            let device_guard = device.lock().map_err(|_| {
                IscsiError::Scsi("Device lock poisoned".to_string())
            })?;
            let block_size = device_guard.block_size();
            drop(device_guard);

            let expected_data_len = transfer_length as usize * block_size as usize;
            let bytes_received = pdu.data.len() as u32;

            // Write immediate data if present
            if !pdu.data.is_empty() {
                log::debug!(
                    "WRITE command with immediate data: ITT=0x{:08x}, LBA={}, {} bytes (expected {})",
                    cmd.itt, lba, pdu.data.len(), expected_data_len
                );

                let mut device_guard = device.lock().map_err(|_| {
                    IscsiError::Scsi("Device lock poisoned".to_string())
                })?;

                let write_result = device_guard.write(lba, &pdu.data, block_size);
                drop(device_guard);

                if let Err(e) = write_result {
                    log::error!("Write failed: {}", e);
                    let sense = crate::scsi::SenseData::medium_error();
                    return Ok(vec![IscsiPdu::scsi_response(
                        cmd.itt,
                        session.next_stat_sn(),
                        session.exp_cmd_sn,
                        session.max_cmd_sn,
                        pdu::scsi_status::CHECK_CONDITION,
                        0,
                        0,
                        Some(&sense.to_bytes()),
                    )]);
                }
            }

            // If all data has been received, send success response
            if bytes_received as usize == expected_data_len {
                log::debug!(
                    "Write complete: ITT=0x{:08x}, {} bytes written",
                    cmd.itt, bytes_received
                );
                return Ok(vec![IscsiPdu::scsi_response(
                    cmd.itt,
                    session.next_stat_sn(),
                    session.exp_cmd_sn,
                    session.max_cmd_sn,
                    pdu::scsi_status::GOOD,
                    0,
                    0,
                    None,
                )]);
            }

            // Need more data - generate TTT and store pending write
            let ttt = session.next_target_transfer_tag();
            let remaining_bytes = expected_data_len as u32 - bytes_received;

            log::debug!(
                "WRITE needs R2T: ITT=0x{:08x}, TTT=0x{:08x}, received={}, remaining={}, total={}",
                cmd.itt, ttt, bytes_received, remaining_bytes, expected_data_len
            );

            // Store pending write
            session.pending_writes.insert(cmd.itt, PendingWrite {
                lba,
                transfer_length,
                block_size,
                bytes_received,
                ttt,
                r2t_sn: 0,
                lun: cmd.lun,
            });

            // Send R2T to request the remaining data
            // RFC 3720: R2T requests data starting at buffer_offset (bytes already received)
            // with desired_data_transfer_length being the remaining bytes needed
            // We may need to send multiple R2Ts if remaining data > MaxBurstLength
            let max_burst = session.params.max_burst_length;
            let mut responses = Vec::new();
            let mut offset = bytes_received;
            let mut r2t_sn = 0u32;

            while offset < expected_data_len as u32 {
                let remaining = expected_data_len as u32 - offset;
                let request_len = remaining.min(max_burst);

                log::debug!(
                    "Sending R2T: ITT=0x{:08x}, TTT=0x{:08x}, R2TSN={}, offset={}, len={}",
                    cmd.itt, ttt, r2t_sn, offset, request_len
                );

                let r2t = IscsiPdu::r2t(
                    cmd.lun,
                    cmd.itt,
                    ttt,
                    session.stat_sn, // StatSN is not incremented for R2T
                    session.exp_cmd_sn,
                    session.max_cmd_sn,
                    r2t_sn,
                    offset,
                    request_len,
                );
                responses.push(r2t);

                offset += request_len;
                r2t_sn += 1;
            }

            // Update pending write with next R2T sequence number
            if let Some(pending) = session.pending_writes.get_mut(&cmd.itt) {
                pending.r2t_sn = r2t_sn;
            }

            return Ok(responses);
        }

        // For write commands with no transfer, send immediate success
        return Ok(vec![IscsiPdu::scsi_response(
            cmd.itt,
            session.next_stat_sn(),
            session.exp_cmd_sn,
            session.max_cmd_sn,
            pdu::scsi_status::GOOD,
            0,
            0,
            None,
        )]);
    }

    // Handle non-write commands (reads, inquiries, etc.)
    let response = if opcode == 0x03 {
        // REQUEST SENSE (0x03) - return stored sense data instead of calling handler
        log::info!("REQUEST SENSE called - returning stored sense data");
        if cmd.cdb.len() < 6 {
            ScsiResponse::check_condition(crate::scsi::SenseData::invalid_command())
        } else {
            let alloc_len = cmd.cdb[4] as usize;

            // Return the stored sense data, or NO_SENSE if none is stored
            let mut data = match &session.last_sense_data {
                Some(sense_bytes) => {
                    log::info!("Returning stored sense data: {:02x?}", sense_bytes);
                    sense_bytes.clone()
                }
                None => {
                    log::warn!("No stored sense data - returning NO_SENSE");
                    // No stored sense data - return NO_SENSE
                    let sense = crate::scsi::SenseData::new(
                        crate::scsi::sense_key::NO_SENSE,
                        crate::scsi::asc::NO_ADDITIONAL_SENSE,
                        0,
                    );
                    sense.to_bytes()
                }
            };

            data.truncate(alloc_len.min(data.len()));
            ScsiResponse::good(data)
        }
    } else if is_sync_cache {
        // SYNCHRONIZE CACHE needs mutable access to call flush()
        let mut device_guard = device.lock().map_err(|_| {
            IscsiError::Scsi("Device lock poisoned".to_string())
        })?;

        log::debug!("Calling flush() for SYNCHRONIZE CACHE command");
        device_guard.flush()?;

        ScsiResponse::good_no_data()
    } else {
        // Other commands use immutable access
        let device_guard = device.lock().map_err(|_| {
            IscsiError::Scsi("Device lock poisoned".to_string())
        })?;

        let resp = ScsiHandler::handle_command(&cmd.cdb, &*device_guard, None)?;

        if !resp.data.is_empty() {
            log::debug!("SCSI command returned {} bytes, first 16: {:02x?}",
                        resp.data.len(), &resp.data[..resp.data.len().min(16)]);
        }

        resp
    };

    // Build response PDU(s)
    let mut responses = Vec::new();

    if cmd.read && !response.data.is_empty() {
        // Send data with Data-In PDU(s)
        let max_data_seg = session.params.max_xmit_data_segment_length as usize;
        let mut offset = 0u32;
        let mut data_sn = 0u32;

        log::debug!("Large read: total_data={} bytes, max_data_seg={} bytes, will send {} PDUs",
                    response.data.len(), max_data_seg, (response.data.len() + max_data_seg - 1) / max_data_seg);

        while offset < response.data.len() as u32 {
            let remaining = response.data.len() - offset as usize;
            let chunk_size = remaining.min(max_data_seg);
            let is_final = offset as usize + chunk_size >= response.data.len();

            let chunk = response.data[offset as usize..offset as usize + chunk_size].to_vec();

            log::debug!("Sending Data-In PDU: offset={}, chunk_size={}, is_final={}, data_sn={}, first 16 bytes: {:02x?}",
                        offset, chunk_size, is_final, data_sn, &chunk[..chunk.len().min(16)]);

            // StatSN should only be incremented for the final PDU (with F and S bits set)
            // For non-final PDUs, StatSN is reserved and set to 0
            let pdu_stat_sn = if is_final { session.next_stat_sn() } else { 0 };

            let data_in = IscsiPdu::scsi_data_in(
                cmd.itt,
                0xFFFF_FFFF, // TTT
                pdu_stat_sn,
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

        if response.status == pdu::scsi_status::CHECK_CONDITION {
            if let Some(ref sd) = response.sense {
                let sense_bytes = sd.to_bytes();
                log::info!(
                    "Sending CHECK CONDITION with sense data: sense_key=0x{:02x}, asc=0x{:02x}, ascq=0x{:02x}",
                    sd.sense_key, sd.asc, sd.ascq
                );
                log::debug!("Sense data bytes: {:02x?}", sense_bytes);
                // Store the FULL sense data (including response code) for REQUEST SENSE
                session.last_sense_data = Some(sense_bytes);
            } else {
                log::warn!("CHECK CONDITION status but no sense data available!");
            }
        } else {
            // Clear sense data when status is GOOD
            session.last_sense_data = None;
        }

        // RFC 3720: Response field indicates whether the target successfully processed the command
        // Use 0x00 (Command Completed at Target) for all successful SCSI status values, including CHECK_CONDITION
        // libiscsi parses sense data from the SCSI Response PDU when Response=0x00
        let response_code = 0; // Command Completed at Target

        // Include sense data in the response PDU per RFC 3720 Section 10.4.7.
        // We also store it for REQUEST SENSE retrieval, as libiscsi will call REQUEST SENSE
        // to retrieve the actual sense data from the task structure.
        let pdu_sense_data = sense_data.as_deref();

        let scsi_resp = IscsiPdu::scsi_response(
            cmd.itt,
            session.next_stat_sn(),
            session.exp_cmd_sn,
            session.max_cmd_sn,
            response.status,
            response_code,
            0, // residual count
            pdu_sense_data,
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
        "SCSI Data-Out: ITT=0x{:08x}, TTT=0x{:08x}, DataSN={}, Offset={}, Len={}, Final={}",
        data_out.itt, data_out.ttt, data_out.data_sn, data_out.buffer_offset, data_out.data.len(), data_out.final_flag
    );

    // Look up the pending write command
    let pending_write = session.pending_writes.get_mut(&data_out.itt);

    if pending_write.is_none() {
        log::warn!("Received Data-Out for unknown ITT=0x{:08x}", data_out.itt);
        return Ok(vec![]);
    }

    let pending = pending_write.unwrap();
    let block_size = pending.block_size;
    let transfer_length = pending.transfer_length;
    let base_lba = pending.lba;
    let total_expected = transfer_length * block_size;

    // Calculate the LBA for this chunk based on buffer_offset
    // buffer_offset is the byte offset from the start of the transfer
    let lba = base_lba + (data_out.buffer_offset as u64 / block_size as u64);

    log::debug!(
        "Writing Data-Out: ITT=0x{:08x}, buffer_offset={}, LBA={}, {} bytes (base_lba={})",
        data_out.itt, data_out.buffer_offset, lba, data_out.data.len(), base_lba
    );

    // Write the data
    let mut device_guard = device.lock().map_err(|_| {
        IscsiError::Scsi("Device lock poisoned".to_string())
    })?;

    let write_result = device_guard.write(lba, &data_out.data, block_size);
    drop(device_guard);

    // Update bytes received - track the highest offset written
    // This handles out-of-order Data-Out PDUs correctly
    let end_offset = data_out.buffer_offset + data_out.data.len() as u32;
    if end_offset > pending.bytes_received {
        pending.bytes_received = end_offset;
    }

    log::debug!(
        "Updated bytes received: {}/{} bytes",
        pending.bytes_received,
        total_expected
    );

    let (status, sense) = match write_result {
        Ok(()) => (scsi_status::GOOD, None),
        Err(e) => {
            log::error!("Write failed: {}", e);
            let sense = crate::scsi::SenseData::medium_error();
            (pdu::scsi_status::CHECK_CONDITION, Some(sense.to_bytes()))
        }
    };

    // Check if all data has been received
    // The final flag indicates the last PDU for this R2T sequence
    // We complete when all expected bytes are received
    if pending.bytes_received >= total_expected {
        log::debug!(
            "Write complete: ITT=0x{:08x}, {} bytes total",
            data_out.itt, pending.bytes_received
        );

        // Remove the pending write
        session.pending_writes.remove(&data_out.itt);

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
    } else if status != scsi_status::GOOD {
        // Error occurred - remove pending write and send error response
        session.pending_writes.remove(&data_out.itt);

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
    target_address: &str,
) -> ScsiResult<Vec<IscsiPdu>> {
    let text_req = pdu.parse_text_request()?;

    log::debug!("Text Request: ITT=0x{:08x}, params: {:?}", text_req.itt, text_req.parameters);

    // Check for SendTargets request (discovery)
    let is_send_targets = text_req.parameters.iter()
        .any(|(k, v)| k == "SendTargets" && (v == "All" || v.is_empty()));

    let response_params = if is_send_targets {
        // Return target list for any SendTargets request
        // (RFC 3720: Discovery works even if SessionType isn't explicitly set)
        session.handle_send_targets(target_name, target_address)
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
    auth_config: crate::auth::AuthConfig,
    _phantom: std::marker::PhantomData<D>,
}

impl<D: ScsiBlockDevice> IscsiTargetBuilder<D> {
    fn new() -> Self {
        Self {
            bind_addr: None,
            target_name: None,
            target_alias: None,
            auth_config: crate::auth::AuthConfig::None,
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

    /// Set the authentication configuration
    pub fn with_auth(mut self, auth_config: crate::auth::AuthConfig) -> Self {
        self.auth_config = auth_config;
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
            auth_config: self.auth_config,
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
