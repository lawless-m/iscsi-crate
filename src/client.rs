//! iSCSI Client library for testing and initiator functionality
//!
//! This module provides a low-level iSCSI client implementation focused on
//! enabling test scenarios with arbitrary PDU transmission and reception.
//!
//! # Overview
//!
//! The client module provides:
//! - Raw TCP socket connection to iSCSI targets
//! - PDU transmission and reception
//! - Session state management
//! - Login/logout phases
//! - SCSI command execution
//! - Arbitrary PDU transmission for testing edge cases
//!
//! # Example: Basic Connection and Login
//!
//! ```no_run
//! use iscsi_target::client::IscsiClient;
//!
//! # fn test() -> Result<(), Box<dyn std::error::Error>> {
//! let mut client = IscsiClient::connect("127.0.0.1:3260")?;
//! client.login(
//!     "iqn.2025-12.local:initiator",
//!     "iqn.2025-12.local:storage.disk1",
//! )?;
//! // ... send SCSI commands ...
//! client.logout()?;
//! # Ok(())
//! # }
//! ```
//!
//! # Example: Raw PDU Transmission (for testing)
//!
//! ```no_run
//! use iscsi_target::client::IscsiClient;
//! use iscsi_target::pdu::IscsiPdu;
//!
//! # fn test() -> Result<(), Box<dyn std::error::Error>> {
//! let mut client = IscsiClient::connect("127.0.0.1:3260")?;
//!
//! // Send custom/malformed PDU for testing
//! let mut pdu = IscsiPdu::new();
//! pdu.opcode = 0x99;  // Invalid opcode
//! client.send_raw_pdu(&pdu)?;
//!
//! // Receive response
//! let response = client.recv_pdu()?;
//! # Ok(())
//! # }
//! ```

use crate::error::{IscsiError, ScsiResult};
use crate::pdu::{self, IscsiPdu, opcode, flags, BHS_SIZE};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

/// iSCSI Client for connecting to targets and sending/receiving PDUs
///
/// The client maintains a TCP connection to the target and handles
/// PDU serialization/deserialization.
pub struct IscsiClient {
    stream: TcpStream,
    cmd_sn: u32,
    exp_stat_sn: u32,
    max_cmd_sn: u32,
    stat_sn: u32,
    initialized: bool,
}

impl IscsiClient {
    /// Connect to an iSCSI target at the given address
    ///
    /// # Arguments
    ///
    /// * `addr` - Address and port in format "host:port" (e.g., "127.0.0.1:3260")
    ///
    /// # Errors
    ///
    /// Returns an error if the TCP connection fails
    pub fn connect(addr: &str) -> ScsiResult<Self> {
        let stream = TcpStream::connect(addr)
            .map_err(IscsiError::Io)?;

        // Set blocking mode and timeouts
        stream.set_nonblocking(false)
            .map_err(IscsiError::Io)?;
        stream.set_read_timeout(Some(Duration::from_secs(10)))
            .map_err(IscsiError::Io)?;
        stream.set_write_timeout(Some(Duration::from_secs(10)))
            .map_err(IscsiError::Io)?;

        Ok(IscsiClient {
            stream,
            cmd_sn: 0,
            exp_stat_sn: 0,
            max_cmd_sn: u32::MAX,
            stat_sn: 0,
            initialized: false,
        })
    }

    /// Perform iSCSI login (security negotiation + operational negotiation + full feature phase)
    ///
    /// # Arguments
    ///
    /// * `initiator_name` - IQN of the initiator (e.g., "iqn.2025-12.local:initiator")
    /// * `target_name` - IQN of the target (e.g., "iqn.2025-12.local:storage.disk1")
    ///
    /// # Errors
    ///
    /// Returns an error if login fails at any phase
    pub fn login(&mut self, initiator_name: &str, target_name: &str) -> ScsiResult<()> {
        // Phase 1: Security Negotiation
        self.login_phase(
            initiator_name,
            target_name,
            flags::CSG_SECURITY_NEG,
            flags::NSG_LOGIN_OP_NEG,
            false,
        )?;

        // Phase 2: Operational Negotiation
        self.login_phase(
            initiator_name,
            target_name,
            flags::CSG_LOGIN_OP_NEG,
            flags::NSG_FULL_FEATURE,
            true,
        )?;

        // Phase 3: Full Feature Phase (transition)
        self.login_phase(
            initiator_name,
            target_name,
            flags::CSG_FULL_FEATURE,
            flags::NSG_FULL_FEATURE,
            true,
        )?;

        self.initialized = true;
        Ok(())
    }

    /// Perform a single login phase
    fn login_phase(
        &mut self,
        initiator_name: &str,
        target_name: &str,
        csg: u8,
        nsg: u8,
        transit: bool,
    ) -> ScsiResult<()> {
        // Build login request parameters
        let mut params = String::new();
        params.push_str(&format!("InitiatorName={}\0", initiator_name));
        params.push_str(&format!("TargetName={}\0", target_name));

        if csg == flags::CSG_SECURITY_NEG {
            params.push_str("AuthMethod=None\0");
        }

        if csg == flags::CSG_LOGIN_OP_NEG {
            params.push_str("HeaderDigest=None\0");
            params.push_str("DataDigest=None\0");
            params.push_str("MaxRecvDataSegmentLength=8192\0");
            params.push_str("MaxBurstLength=262144\0");
            params.push_str("FirstBurstLength=65536\0");
            params.push_str("DefaultTime2Wait=2\0");
            params.push_str("DefaultTime2Retain=20\0");
            params.push_str("MaxOutstandingR2T=1\0");
            params.push_str("ImmediateData=Yes\0");
            params.push_str("InitialR2TIOV=0\0");
            params.push_str("DataPDUInOrder=Yes\0");
            params.push_str("DataSequenceInOrder=Yes\0");
            params.push_str("ErrorRecoveryLevel=0\0");
            params.push_str("SessionType=Normal\0");
        }

        // Pad to 4-byte boundary
        while params.len() % 4 != 0 {
            params.push('\0');
        }

        // Create login request PDU
        let mut pdu = IscsiPdu::new();
        pdu.opcode = opcode::LOGIN_REQUEST;
        pdu.immediate = true;
        pdu.flags = if transit { flags::TRANSIT } else { 0 };
        pdu.flags |= (csg & 0x03) << 2; // Current stage
        pdu.flags |= nsg & 0x03;        // Next stage
        pdu.itt = self.cmd_sn; // Use cmd_sn as itt
        pdu.specific[0] = 0; // Version max
        pdu.specific[1] = 0; // Version active
        pdu.data = params.into_bytes();

        // Send login request
        self.send_pdu(&pdu)?;

        // Receive login response
        let response = self.recv_pdu()?;

        // Verify login response
        if response.opcode != opcode::LOGIN_RESPONSE {
            return Err(IscsiError::InvalidPdu(format!(
                "Expected LOGIN_RESPONSE (0x23), got opcode 0x{:02x}",
                response.opcode
            )));
        }

        // Extract response and status from bytes 2-3 of specific
        let status_class = response.specific[0];
        let status_detail = response.specific[1];

        if status_class != pdu::login_status::SUCCESS {
            return Err(IscsiError::Protocol(format!(
                "Login failed: class=0x{:02x}, detail=0x{:02x}",
                status_class, status_detail
            )));
        }

        // Update sequence numbers from response
        // exp_cmd_sn: specific[4:8]
        // max_cmd_sn: specific[8:12]
        // stat_sn: specific[12:16]
        if response.specific.len() >= 16 {
            self.exp_stat_sn = u32::from_be_bytes([
                response.specific[4],
                response.specific[5],
                response.specific[6],
                response.specific[7],
            ]);
            self.max_cmd_sn = u32::from_be_bytes([
                response.specific[8],
                response.specific[9],
                response.specific[10],
                response.specific[11],
            ]);
        }

        // Increment cmd_sn for next command
        self.cmd_sn = self.cmd_sn.wrapping_add(1);

        Ok(())
    }

    /// Discover available targets at the connected portal
    ///
    /// Performs SendTargets discovery to get a list of available iSCSI targets.
    ///
    /// # Arguments
    ///
    /// * `initiator_name` - IQN of the initiator
    ///
    /// # Returns
    ///
    /// A vector of tuples containing (target_iqn, target_address)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use iscsi_target::client::IscsiClient;
    ///
    /// # fn test() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut client = IscsiClient::connect("127.0.0.1:3260")?;
    /// let targets = client.discover("iqn.2025-12.local:initiator")?;
    /// for (iqn, addr) in targets {
    ///     println!("Target: {} at {}", iqn, addr);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn discover(&mut self, initiator_name: &str) -> ScsiResult<Vec<(String, String)>> {
        // Perform discovery login (SessionType=Discovery)
        self.discovery_login(initiator_name)?;

        // Send SendTargets Text Request
        let mut params = String::new();
        params.push_str("SendTargets=All\0");
        while params.len() % 4 != 0 {
            params.push('\0');
        }

        let mut pdu = IscsiPdu::new();
        pdu.opcode = opcode::TEXT_REQUEST;
        pdu.flags = flags::FINAL;
        pdu.itt = self.cmd_sn;
        // TTT = 0xFFFFFFFF for new request
        pdu.specific[0..4].copy_from_slice(&0xFFFF_FFFFu32.to_be_bytes());
        // CmdSN
        pdu.specific[4..8].copy_from_slice(&self.cmd_sn.to_be_bytes());
        // ExpStatSN
        pdu.specific[8..12].copy_from_slice(&self.exp_stat_sn.to_be_bytes());
        pdu.data = params.into_bytes();

        // Send text request
        self.send_pdu(&pdu)?;

        // Receive text response
        let response = self.recv_pdu()?;

        if response.opcode != opcode::TEXT_RESPONSE {
            return Err(IscsiError::InvalidPdu(format!(
                "Expected TEXT_RESPONSE (0x24), got opcode 0x{:02x}",
                response.opcode
            )));
        }

        // Parse response parameters
        let params = pdu::parse_text_parameters(&response.data)?;

        // Extract target information
        let mut targets = Vec::new();
        let mut current_target: Option<String> = None;

        for (key, value) in params {
            match key.as_str() {
                "TargetName" => {
                    current_target = Some(value);
                }
                "TargetAddress" => {
                    if let Some(iqn) = current_target.take() {
                        // TargetAddress format is "host:port,portal-group-tag"
                        // We just need the host:port part
                        let addr = value.split(',').next().unwrap_or(&value).to_string();
                        targets.push((iqn, addr));
                    }
                }
                _ => {}
            }
        }

        // For discovery sessions, we can just close the connection
        // (logout not needed - connection will be closed when client is dropped)

        Ok(targets)
    }

    /// Perform discovery login (SessionType=Discovery)
    fn discovery_login(&mut self, initiator_name: &str) -> ScsiResult<()> {
        // Phase 1: Security Negotiation
        self.discovery_login_phase(
            initiator_name,
            flags::CSG_SECURITY_NEG,
            flags::NSG_LOGIN_OP_NEG,
            false,
        )?;

        // Phase 2: Operational Negotiation with SessionType=Discovery
        self.discovery_login_phase(
            initiator_name,
            flags::CSG_LOGIN_OP_NEG,
            flags::NSG_FULL_FEATURE,
            true,
        )?;

        self.initialized = true;
        Ok(())
    }

    /// Perform a single discovery login phase
    fn discovery_login_phase(
        &mut self,
        initiator_name: &str,
        csg: u8,
        nsg: u8,
        transit: bool,
    ) -> ScsiResult<()> {
        // Build login request parameters
        let mut params = String::new();
        params.push_str(&format!("InitiatorName={}\0", initiator_name));

        if csg == flags::CSG_SECURITY_NEG {
            params.push_str("AuthMethod=None\0");
            params.push_str("SessionType=Discovery\0");
        }

        if csg == flags::CSG_LOGIN_OP_NEG {
            params.push_str("HeaderDigest=None\0");
            params.push_str("DataDigest=None\0");
            params.push_str("MaxRecvDataSegmentLength=8192\0");
            params.push_str("DefaultTime2Wait=2\0");
            params.push_str("DefaultTime2Retain=20\0");
            params.push_str("ErrorRecoveryLevel=0\0");
        }

        // Pad to 4-byte boundary
        while params.len() % 4 != 0 {
            params.push('\0');
        }

        // Create login request PDU
        let mut pdu = IscsiPdu::new();
        pdu.opcode = opcode::LOGIN_REQUEST;
        pdu.immediate = true;
        pdu.flags = if transit { flags::TRANSIT } else { 0 };
        pdu.flags |= (csg & 0x03) << 2; // Current stage
        pdu.flags |= nsg & 0x03;        // Next stage
        pdu.itt = self.cmd_sn; // Use cmd_sn as itt
        pdu.specific[0] = 0; // Version max
        pdu.specific[1] = 0; // Version active
        pdu.data = params.into_bytes();

        // Send login request
        self.send_pdu(&pdu)?;

        // Receive login response
        let response = self.recv_pdu()?;

        // Verify login response
        if response.opcode != opcode::LOGIN_RESPONSE {
            return Err(IscsiError::InvalidPdu(format!(
                "Expected LOGIN_RESPONSE (0x23), got opcode 0x{:02x}",
                response.opcode
            )));
        }

        // Extract status from response
        let status_class = response.specific[0];
        let status_detail = response.specific[1];

        if status_class != pdu::login_status::SUCCESS {
            return Err(IscsiError::Protocol(format!(
                "Discovery login failed: class=0x{:02x}, detail=0x{:02x}",
                status_class, status_detail
            )));
        }

        // Update sequence numbers from response
        if response.specific.len() >= 16 {
            self.exp_stat_sn = u32::from_be_bytes([
                response.specific[4],
                response.specific[5],
                response.specific[6],
                response.specific[7],
            ]);
            self.max_cmd_sn = u32::from_be_bytes([
                response.specific[8],
                response.specific[9],
                response.specific[10],
                response.specific[11],
            ]);
        }

        // Increment cmd_sn for next command
        self.cmd_sn = self.cmd_sn.wrapping_add(1);

        Ok(())
    }

    /// Send a PDU to the target
    ///
    /// Serializes the PDU to bytes and writes it to the TCP stream.
    pub fn send_pdu(&mut self, pdu: &IscsiPdu) -> ScsiResult<()> {
        let bytes = pdu.to_bytes();
        self.stream.write_all(&bytes)
            .map_err(IscsiError::Io)?;
        Ok(())
    }

    /// Send a raw PDU to the target for testing purposes
    ///
    /// This allows sending arbitrary/malformed PDUs for edge case testing.
    pub fn send_raw_pdu(&mut self, pdu: &IscsiPdu) -> ScsiResult<()> {
        self.send_pdu(pdu)
    }

    /// Receive a PDU from the target
    ///
    /// Reads the 48-byte BHS and any data segment from the TCP stream.
    pub fn recv_pdu(&mut self) -> ScsiResult<IscsiPdu> {
        let mut buf = vec![0u8; BHS_SIZE];
        self.stream.read_exact(&mut buf)
            .map_err(IscsiError::Io)?;

        // Parse BHS to get data length
        if buf.len() < BHS_SIZE {
            return Err(IscsiError::InvalidPdu(format!(
                "BHS too short: {} bytes",
                buf.len()
            )));
        }

        // Extract data segment length from bytes 5-7
        let data_len = ((buf[5] as u32) << 16)
            | ((buf[6] as u32) << 8)
            | (buf[7] as u32);

        // Calculate padded length (rounded up to 4-byte boundary)
        let padded_len = ((data_len + 3) / 4) * 4;

        if padded_len > 0 {
            let mut data_buf = vec![0u8; padded_len as usize];
            self.stream.read_exact(&mut data_buf)
                .map_err(IscsiError::Io)?;
            buf.extend_from_slice(&data_buf);
        }

        // Parse complete PDU
        IscsiPdu::from_bytes(&buf)
    }

    /// Send a SCSI command and receive the response
    ///
    /// # Arguments
    ///
    /// * `cdb` - SCSI Command Descriptor Block
    /// * `data_out` - Optional data to send with command (for WRITE operations)
    pub fn send_scsi_command(&mut self, cdb: &[u8], data_out: Option<&[u8]>) -> ScsiResult<IscsiPdu> {
        if !self.initialized {
            return Err(IscsiError::Session(
                "Not logged in. Call login() first.".to_string(),
            ));
        }

        // Create SCSI command PDU
        let mut pdu = IscsiPdu::new();
        pdu.opcode = opcode::SCSI_COMMAND;
        pdu.flags = flags::FINAL | flags::WRITE; // Mark as final
        pdu.itt = self.cmd_sn;
        pdu.lun = 0; // LUN 0

        // CDB length in bytes 28-31 (high byte), command in 32-47
        pdu.specific[0] = (cdb.len() as u8) & 0x0F;
        pdu.specific[1] = 0; // Attribute
        pdu.specific[4] = (cdb.len() as u8) & 0xFF; // CDB length

        // Copy CDB into specific field
        let cdb_start = 16;
        if cdb.len() <= 16 {
            pdu.specific[cdb_start..cdb_start + cdb.len()].copy_from_slice(cdb);
        } else {
            return Err(IscsiError::InvalidPdu(format!(
                "CDB too long: {} bytes (max 16)",
                cdb.len()
            )));
        }

        // Add data segment if needed
        if let Some(data) = data_out {
            pdu.data = data.to_vec();
            pdu.flags |= flags::WRITE;
        }

        // Set sequence numbers
        // CmdSN: specific[20:24]
        // ExpStatSN: specific[24:28]
        pdu.specific[20..24].copy_from_slice(&self.cmd_sn.to_be_bytes());
        pdu.specific[24..28].copy_from_slice(&self.exp_stat_sn.to_be_bytes());

        // Send command
        self.send_pdu(&pdu)?;
        self.cmd_sn = self.cmd_sn.wrapping_add(1);

        // For simplicity, receive one response
        // In real implementation, might need to handle multiple responses
        self.recv_pdu()
    }

    /// Perform iSCSI logout
    pub fn logout(&mut self) -> ScsiResult<()> {
        let mut pdu = IscsiPdu::new();
        pdu.opcode = opcode::LOGOUT_REQUEST;
        pdu.immediate = true;
        pdu.flags = flags::FINAL;
        pdu.itt = self.cmd_sn;

        // Set sequence numbers
        pdu.specific[20..24].copy_from_slice(&self.cmd_sn.to_be_bytes());
        pdu.specific[24..28].copy_from_slice(&self.exp_stat_sn.to_be_bytes());

        self.send_pdu(&pdu)?;
        let _response = self.recv_pdu()?;

        self.initialized = false;
        Ok(())
    }

    /// Get the current command sequence number
    pub fn cmd_sn(&self) -> u32 {
        self.cmd_sn
    }

    /// Get the current expected status sequence number
    pub fn exp_stat_sn(&self) -> u32 {
        self.exp_stat_sn
    }

    /// Get the maximum command sequence number from target
    pub fn max_cmd_sn(&self) -> u32 {
        self.max_cmd_sn
    }

    /// Check if client is logged in (in full feature phase)
    pub fn is_logged_in(&self) -> bool {
        self.initialized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        // This test requires a running target
        // Commented out for now - can be integrated test
        // let client = IscsiClient::connect("127.0.0.1:3260");
        // assert!(client.is_ok());
    }
}
