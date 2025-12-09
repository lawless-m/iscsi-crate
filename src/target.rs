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
    stream.set_read_timeout(Some(Duration::from_secs(300))).map_err(IscsiError::Io)?;
    stream.set_write_timeout(Some(Duration::from_secs(30))).map_err(IscsiError::Io)?;

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

    log::info!(
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

            // Check if immediate data is present in this PDU
            if !pdu.data.is_empty() {
                log::debug!(
                    "WRITE command with immediate data: ITT=0x{:08x}, LBA={}, {} bytes (expected {})",
                    cmd.itt, lba, pdu.data.len(), expected_data_len
                );

                // Write the immediate data
                log::debug!(
                    "Writing immediate data: LBA={}, {} bytes (blocks {}-{})",
                    lba, pdu.data.len(), lba, lba + (pdu.data.len() as u64 / block_size as u64) - 1
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

                // If this is the final PDU and all data has been received, send success response
                if cmd.final_flag && pdu.data.len() == expected_data_len {
                    log::debug!(
                        "Write complete: ITT=0x{:08x}, {} bytes written",
                        cmd.itt, pdu.data.len()
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
                } else {
                    // Store pending write for additional Data-Out PDUs
                    // Start tracking from the end of the immediate data
                    let bytes_received = pdu.data.len() as u32;
                    session.pending_writes.insert(cmd.itt, PendingWrite {
                        lba,
                        transfer_length,
                        block_size,
                        bytes_received,
                    });
                    log::debug!(
                        "Stored pending write for continuation: ITT=0x{:08x}, {} of {} bytes received",
                        cmd.itt, bytes_received, expected_data_len
                    );
                }
            } else {
                // No immediate data, expect Data-Out PDUs
                session.pending_writes.insert(cmd.itt, PendingWrite {
                    lba,
                    transfer_length,
                    block_size,
                    bytes_received: 0,
                });

                log::debug!(
                    "Stored pending write: ITT=0x{:08x}, LBA={}, blocks={}, block_size={}",
                    cmd.itt, lba, transfer_length, block_size
                );
            }
        }

        // For write commands, we've handled immediate data or stored pending writes
        // Return empty - responses will be sent when data is complete
        return Ok(vec![]);
    }

    // Handle non-write commands (reads, inquiries, etc.)
    let response = if is_sync_cache {
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

        ScsiHandler::handle_command(&cmd.cdb, &*device_guard, None)?
    };

    // Build response PDU(s)
    let mut responses = Vec::new();

    if cmd.read && !response.data.is_empty() {
        // Send data with Data-In PDU(s)
        let max_data_seg = session.params.max_xmit_data_segment_length as usize;
        let mut offset = 0u32;
        let mut data_sn = 0u32;

        // StatSN should only be valid for the final PDU (with F and S bits set)
        // For non-final PDUs, StatSN is reserved
        let stat_sn = session.next_stat_sn();

        while offset < response.data.len() as u32 {
            let remaining = response.data.len() - offset as usize;
            let chunk_size = remaining.min(max_data_seg);
            let is_final = offset as usize + chunk_size >= response.data.len();

            let chunk = response.data[offset as usize..offset as usize + chunk_size].to_vec();

            // StatSN is only valid for final PDU; for non-final PDUs use 0
            let pdu_stat_sn = if is_final { stat_sn } else { 0 };

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

    // Look up the pending write command
    let pending_write = session.pending_writes.get_mut(&data_out.itt);

    if pending_write.is_none() {
        log::warn!("Received Data-Out for unknown ITT=0x{:08x}", data_out.itt);
        return Ok(vec![]);
    }

    let pending = pending_write.unwrap();
    let block_size = pending.block_size;
    let transfer_length = pending.transfer_length;

    // Calculate the LBA for this chunk based on buffer_offset
    // buffer_offset is the offset from the start of the transfer
    let lba = pending.lba + (data_out.buffer_offset / block_size as u32) as u64;

    log::debug!(
        "Writing Data-Out: ITT=0x{:08x}, buffer_offset={}, LBA={}, {} bytes",
        data_out.itt, data_out.buffer_offset, lba, data_out.data.len()
    );

    // Write the data
    let mut device_guard = device.lock().map_err(|_| {
        IscsiError::Scsi("Device lock poisoned".to_string())
    })?;

    let write_result = device_guard.write(lba, &data_out.data, block_size);
    drop(device_guard);

    // Update bytes received - should match buffer_offset + current data length
    let expected_bytes = data_out.buffer_offset + data_out.data.len() as u32;
    pending.bytes_received = expected_bytes;

    let total_expected = transfer_length * block_size;

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

    // If this is the final data PDU, send a response and clean up
    if data_out.final_flag {
        // Verify all data was received
        if pending.bytes_received != total_expected {
            log::warn!(
                "Incomplete write: received {} bytes, expected {}",
                pending.bytes_received, total_expected
            );
        }

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
