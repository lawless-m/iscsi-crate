//! iSCSI session management
//!
//! This module handles session state, connection management, and parameter negotiation
//! based on RFC 3720: https://datatracker.ietf.org/doc/html/rfc3720

use crate::error::{IscsiError, ScsiResult};
use crate::pdu::{self, IscsiPdu, LoginRequest, serialize_text_parameters};
use std::collections::HashMap;

/// Session state machine states (RFC 3720 Section 5)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum SessionState {
    /// Initial state, waiting for first login PDU
    #[default]
    Free,
    /// Security negotiation phase (CHAP, etc.)
    SecurityNegotiation,
    /// Login operational parameter negotiation
    LoginOperationalNegotiation,
    /// Full feature phase - ready for SCSI commands
    FullFeaturePhase,
    /// Logout in progress
    Logout,
    /// Session failed/error state
    Failed,
}


/// Session type (RFC 3720 Section 5.2)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum SessionType {
    /// Normal session for SCSI commands
    #[default]
    Normal,
    /// Discovery session for target discovery (SendTargets)
    Discovery,
}


/// Negotiated session parameters (RFC 3720 Section 12)
#[derive(Debug, Clone)]
pub struct SessionParams {
    // Connection parameters
    /// Maximum data segment length target can receive (default: 8192)
    pub max_recv_data_segment_length: u32,
    /// Maximum data segment length initiator can receive
    pub max_xmit_data_segment_length: u32,

    // Session parameters
    /// Maximum burst length for unsolicited data (default: 262144)
    pub max_burst_length: u32,
    /// First burst length for unsolicited data (default: 65536)
    pub first_burst_length: u32,
    /// Default time to wait before reconnecting (seconds)
    pub default_time2wait: u16,
    /// Default time to retain connection (seconds)
    pub default_time2retain: u16,
    /// Maximum outstanding R2T (Ready to Transfer) PDUs
    pub max_outstanding_r2t: u32,
    /// Data PDU in order (within a sequence)
    pub data_pdu_in_order: bool,
    /// Data sequence in order
    pub data_sequence_in_order: bool,
    /// Error recovery level (0-2)
    pub error_recovery_level: u8,
    /// Immediate data allowed
    pub immediate_data: bool,
    /// Initial R2T required
    pub initial_r2t: bool,

    // Digest settings
    /// Header digest (None, CRC32C)
    pub header_digest: DigestType,
    /// Data digest (None, CRC32C)
    pub data_digest: DigestType,

    // Names
    /// Target name (IQN)
    pub target_name: String,
    /// Initiator name (IQN)
    pub initiator_name: String,
    /// Target alias (optional)
    pub target_alias: String,
    /// Initiator alias (optional)
    pub initiator_alias: String,
}

/// Digest type for header/data
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum DigestType {
    #[default]
    None,
    CRC32C,
}


impl Default for SessionParams {
    fn default() -> Self {
        SessionParams {
            max_recv_data_segment_length: 8192,
            max_xmit_data_segment_length: 8192,
            max_burst_length: 262144,
            first_burst_length: 65536,
            default_time2wait: 2,
            default_time2retain: 20,
            max_outstanding_r2t: 1,
            data_pdu_in_order: true,
            data_sequence_in_order: true,
            error_recovery_level: 0,
            immediate_data: true,
            initial_r2t: false,  // Allow immediate data without waiting for R2T
            header_digest: DigestType::None,
            data_digest: DigestType::None,
            target_name: String::new(),
            initiator_name: String::new(),
            target_alias: String::new(),
            initiator_alias: String::new(),
        }
    }
}

/// Pending write command information
#[derive(Debug, Clone)]
pub struct PendingWrite {
    /// Logical Block Address from the WRITE command
    pub lba: u64,
    /// Transfer length in blocks
    pub transfer_length: u32,
    /// Block size
    pub block_size: u32,
    /// Total bytes received so far
    pub bytes_received: u32,
}

/// iSCSI Session
///
/// Represents an active iSCSI session between an initiator and target.
#[derive(Debug, Clone)]
pub struct IscsiSession {
    /// Initiator Session ID (6 bytes)
    pub isid: [u8; 6],
    /// Target Session Identifying Handle (assigned by target)
    pub tsih: u16,
    /// Connection ID
    pub cid: u16,
    /// Session type
    pub session_type: SessionType,
    /// Current session state
    pub state: SessionState,
    /// Negotiated parameters
    pub params: SessionParams,

    // Sequence numbers
    /// Expected command sequence number from initiator
    pub exp_cmd_sn: u32,
    /// Maximum command sequence number initiator can use
    pub max_cmd_sn: u32,
    /// Status sequence number (target → initiator)
    pub stat_sn: u32,

    // Login tracking
    /// Current login stage
    current_stage: u8,
    /// Next login stage
    next_stage: u8,

    // Command tracking
    /// Pending write commands indexed by ITT (Initiator Task Tag)
    pub pending_writes: HashMap<u32, PendingWrite>,
}

impl Default for IscsiSession {
    fn default() -> Self {
        Self::new()
    }
}

impl IscsiSession {
    /// Create a new session
    pub fn new() -> Self {
        IscsiSession {
            isid: [0u8; 6],
            tsih: 0,
            cid: 0,
            session_type: SessionType::Normal,
            state: SessionState::Free,
            params: SessionParams::default(),
            exp_cmd_sn: 1,
            max_cmd_sn: 1,
            stat_sn: 0,
            current_stage: 0,
            next_stage: 0,
            pending_writes: HashMap::new(),
        }
    }

    /// Create session from login request
    pub fn from_login_request(login: &LoginRequest, target_name: &str) -> Self {
        let mut session = IscsiSession::new();
        session.isid = login.isid;
        session.cid = login.cid;
        session.exp_cmd_sn = login.cmd_sn;
        session.max_cmd_sn = login.cmd_sn + 1;
        session.current_stage = login.csg;
        session.next_stage = login.nsg;
        session.params.target_name = target_name.to_string();

        // Parse initiator parameters
        for (key, value) in &login.parameters {
            session.apply_initiator_param(key, value);
        }

        // Set initial state based on CSG
        session.state = match login.csg {
            0 => SessionState::SecurityNegotiation,
            1 => SessionState::LoginOperationalNegotiation,
            3 => SessionState::FullFeaturePhase,
            _ => SessionState::SecurityNegotiation,
        };

        session
    }

    /// Apply an initiator parameter during negotiation
    fn apply_initiator_param(&mut self, key: &str, value: &str) {
        match key {
            "InitiatorName" => {
                self.params.initiator_name = value.to_string();
            }
            "InitiatorAlias" => {
                self.params.initiator_alias = value.to_string();
            }
            "TargetName" => {
                // Initiator requests specific target
                self.params.target_name = value.to_string();
            }
            "SessionType" => {
                self.session_type = if value == "Discovery" {
                    SessionType::Discovery
                } else {
                    SessionType::Normal
                };
            }
            "MaxRecvDataSegmentLength" => {
                if let Ok(v) = value.parse::<u32>() {
                    // This is initiator's max recv, which is our max xmit
                    self.params.max_xmit_data_segment_length = v;
                }
            }
            "MaxBurstLength" => {
                if let Ok(v) = value.parse::<u32>() {
                    self.params.max_burst_length = v.min(self.params.max_burst_length);
                }
            }
            "FirstBurstLength" => {
                if let Ok(v) = value.parse::<u32>() {
                    self.params.first_burst_length = v.min(self.params.first_burst_length);
                }
            }
            "DefaultTime2Wait" => {
                if let Ok(v) = value.parse::<u16>() {
                    self.params.default_time2wait = v.max(self.params.default_time2wait);
                }
            }
            "DefaultTime2Retain" => {
                if let Ok(v) = value.parse::<u16>() {
                    self.params.default_time2retain = v.min(self.params.default_time2retain);
                }
            }
            "MaxOutstandingR2T" => {
                if let Ok(v) = value.parse::<u32>() {
                    self.params.max_outstanding_r2t = v.min(self.params.max_outstanding_r2t);
                }
            }
            "DataPDUInOrder" => {
                self.params.data_pdu_in_order = value == "Yes";
            }
            "DataSequenceInOrder" => {
                self.params.data_sequence_in_order = value == "Yes";
            }
            "ErrorRecoveryLevel" => {
                if let Ok(v) = value.parse::<u8>() {
                    self.params.error_recovery_level = v.min(self.params.error_recovery_level);
                }
            }
            "ImmediateData" => {
                // AND operation: only true if both want it
                self.params.immediate_data = self.params.immediate_data && (value == "Yes");
            }
            "InitialR2T" => {
                // OR operation: true if either wants it
                self.params.initial_r2t = self.params.initial_r2t || (value == "Yes");
            }
            "HeaderDigest" => {
                self.params.header_digest = if value.contains("CRC32C") {
                    DigestType::CRC32C
                } else {
                    DigestType::None
                };
            }
            "DataDigest" => {
                self.params.data_digest = if value.contains("CRC32C") {
                    DigestType::CRC32C
                } else {
                    DigestType::None
                };
            }
            _ => {
                // Unknown parameter - ignore
                log::debug!("Ignoring unknown parameter: {}={}", key, value);
            }
        }
    }

    /// Generate target response parameters for login
    pub fn generate_response_params(&self) -> Vec<(String, String)> {
        let mut params = Vec::new();

        // Note: SessionType and TargetName are declarative (initiator-only) and should NOT be echoed back
        // Only send TargetAlias if configured
        if self.session_type == SessionType::Normal {
            if !self.params.target_alias.is_empty() {
                params.push(("TargetAlias".to_string(), self.params.target_alias.clone()));
            }
        }

        // Negotiated parameters
        params.push((
            "MaxRecvDataSegmentLength".to_string(),
            self.params.max_recv_data_segment_length.to_string(),
        ));
        params.push((
            "MaxBurstLength".to_string(),
            self.params.max_burst_length.to_string(),
        ));
        params.push((
            "FirstBurstLength".to_string(),
            self.params.first_burst_length.to_string(),
        ));
        params.push((
            "DefaultTime2Wait".to_string(),
            self.params.default_time2wait.to_string(),
        ));
        params.push((
            "DefaultTime2Retain".to_string(),
            self.params.default_time2retain.to_string(),
        ));
        params.push((
            "MaxOutstandingR2T".to_string(),
            self.params.max_outstanding_r2t.to_string(),
        ));
        params.push((
            "DataPDUInOrder".to_string(),
            if self.params.data_pdu_in_order { "Yes" } else { "No" }.to_string(),
        ));
        params.push((
            "DataSequenceInOrder".to_string(),
            if self.params.data_sequence_in_order { "Yes" } else { "No" }.to_string(),
        ));
        params.push((
            "ErrorRecoveryLevel".to_string(),
            self.params.error_recovery_level.to_string(),
        ));
        params.push((
            "ImmediateData".to_string(),
            if self.params.immediate_data { "Yes" } else { "No" }.to_string(),
        ));
        params.push((
            "InitialR2T".to_string(),
            if self.params.initial_r2t { "Yes" } else { "No" }.to_string(),
        ));
        params.push((
            "HeaderDigest".to_string(),
            match self.params.header_digest {
                DigestType::None => "None",
                DigestType::CRC32C => "CRC32C",
            }.to_string(),
        ));
        params.push((
            "DataDigest".to_string(),
            match self.params.data_digest {
                DigestType::None => "None",
                DigestType::CRC32C => "CRC32C",
            }.to_string(),
        ));

        params
    }

    /// Process a login request and generate response
    pub fn process_login(&mut self, pdu: &IscsiPdu, target_name: &str) -> ScsiResult<IscsiPdu> {
        let login = pdu.parse_login_request()?;

        // First login - initialize session
        if self.state == SessionState::Free {
            self.isid = login.isid;
            self.cid = login.cid;
            self.exp_cmd_sn = login.cmd_sn;
            self.max_cmd_sn = login.cmd_sn + 1;
            self.params.target_name = target_name.to_string();
        }

        // Apply parameters from this login PDU
        log::debug!("Received {} login parameters: {:?}", login.parameters.len(), login.parameters);
        for (key, value) in &login.parameters {
            self.apply_initiator_param(key, value);
        }

        // Validate target name for normal sessions
        if self.session_type == SessionType::Normal {
            let requested_target = login.parameters.iter()
                .find(|(k, _)| k == "TargetName")
                .map(|(_, v)| v.as_str());

            if let Some(req_name) = requested_target {
                if req_name != target_name {
                    return self.create_login_reject(
                        pdu.itt,
                        pdu::login_status::INITIATOR_ERROR,
                        0x03, // Target not found
                    );
                }
            }
        }

        // Update stages
        self.current_stage = login.csg;
        self.next_stage = login.nsg;

        log::debug!("Login: CSG={}, NSG={}, Transit={}", login.csg, login.nsg, login.transit);

        // Determine response
        let transit = login.transit;
        let (response_csg, response_nsg, response_transit) = if transit {
            // Initiator wants to transition
            match (login.csg, login.nsg) {
                (0, 1) => {
                    // Security → Login Op Neg
                    self.state = SessionState::LoginOperationalNegotiation;
                    (login.csg, login.nsg, true) // Echo back the transition
                }
                (0, 3) => {
                    // Security → Full Feature Phase
                    self.state = SessionState::FullFeaturePhase;
                    // Only assign TSIH for Normal sessions, not Discovery
                    if self.session_type == SessionType::Normal {
                        self.tsih = self.generate_tsih();
                    }
                    (login.csg, login.nsg, true) // Echo back the transition
                }
                (1, 3) => {
                    // Login Op Neg → Full Feature Phase
                    self.state = SessionState::FullFeaturePhase;
                    // Only assign TSIH for Normal sessions, not Discovery
                    if self.session_type == SessionType::Normal {
                        self.tsih = self.generate_tsih();
                    }
                    (login.csg, login.nsg, true) // Echo back the transition
                }
                _ => {
                    // Stay in current stage
                    (login.csg, login.nsg, false)
                }
            }
        } else {
            // Initiator not ready to transition
            (login.csg, login.nsg, false)
        };

        log::debug!("Response: CSG={}, NSG={}, Transit={}", response_csg, response_nsg, response_transit);

        // Generate response parameters
        let response_params = if response_transit && response_nsg == 3 {
            // Final login response
            if self.session_type == SessionType::Discovery {
                // Discovery sessions - only echo back operational parameters
                let mut params = vec![];

                // Only include AuthMethod if we're in the Security Negotiation stage
                if response_csg == 0 {
                    params.push(("AuthMethod".to_string(), "None".to_string()));
                }

                // Include operational parameters that were negotiated
                for (key, _value) in &login.parameters {
                    match key.as_str() {
                        "MaxRecvDataSegmentLength" => {
                            params.push(("MaxRecvDataSegmentLength".to_string(),
                                        self.params.max_recv_data_segment_length.to_string()));
                        }
                        "HeaderDigest" => {
                            params.push(("HeaderDigest".to_string(), "None".to_string()));
                        }
                        "DataDigest" => {
                            params.push(("DataDigest".to_string(), "None".to_string()));
                        }
                        _ => {}
                    }
                }
                params
            } else {
                // Normal sessions get full parameter negotiation
                self.generate_response_params()
            }
        } else {
            // Intermediate response - only send AuthMethod during security negotiation
            if response_csg == 0 {
                vec![("AuthMethod".to_string(), "None".to_string())]
            } else {
                vec![]
            }
        };

        let response_data = serialize_text_parameters(&response_params);

        log::debug!("Sending {} response parameters: {:?}", response_params.len(), response_params);
        log::debug!("Response data ({} bytes): {:?}", response_data.len(), String::from_utf8_lossy(&response_data));

        // Increment stat_sn for this response
        self.stat_sn = self.stat_sn.wrapping_add(1);

        Ok(IscsiPdu::login_response(
            self.isid,
            self.tsih,
            self.stat_sn,
            self.exp_cmd_sn,
            self.max_cmd_sn,
            pdu::login_status::SUCCESS,
            0, // status detail
            response_csg,
            response_nsg,
            response_transit,
            pdu.itt,
            response_data,
        ))
    }

    /// Create a login reject response
    fn create_login_reject(&self, itt: u32, status_class: u8, status_detail: u8) -> ScsiResult<IscsiPdu> {
        Ok(IscsiPdu::login_response(
            self.isid,
            0, // No TSIH for reject
            self.stat_sn,
            self.exp_cmd_sn,
            self.max_cmd_sn,
            status_class,
            status_detail,
            self.current_stage,
            self.next_stage,
            false, // No transit on error
            itt,
            Vec::new(),
        ))
    }

    /// Generate a unique TSIH
    fn generate_tsih(&self) -> u16 {
        // Simple TSIH generation - in production, would be globally unique
        use std::time::{SystemTime, UNIX_EPOCH};
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        ((duration.as_millis() & 0xFFFF) as u16).max(1)
    }

    /// Check if session is in full feature phase
    pub fn is_full_feature(&self) -> bool {
        self.state == SessionState::FullFeaturePhase
    }

    /// Check if this is a discovery session
    pub fn is_discovery(&self) -> bool {
        self.session_type == SessionType::Discovery
    }

    /// Get next StatSN and increment
    pub fn next_stat_sn(&mut self) -> u32 {
        let sn = self.stat_sn;
        self.stat_sn = self.stat_sn.wrapping_add(1);
        sn
    }

    /// Validate and update CmdSN from incoming PDU
    pub fn validate_cmd_sn(&mut self, cmd_sn: u32) -> bool {
        // Check if CmdSN is within window
        let in_window = Self::sn_in_window(cmd_sn, self.exp_cmd_sn, self.max_cmd_sn);

        if in_window && cmd_sn == self.exp_cmd_sn {
            // Expected command - advance window
            self.exp_cmd_sn = self.exp_cmd_sn.wrapping_add(1);
            self.max_cmd_sn = self.max_cmd_sn.wrapping_add(1);
        }

        in_window
    }

    /// Check if a sequence number is within the command window
    fn sn_in_window(sn: u32, exp_sn: u32, max_sn: u32) -> bool {
        // Handle wraparound using signed comparison
        let diff_exp = sn.wrapping_sub(exp_sn) as i32;
        let diff_max = max_sn.wrapping_sub(sn) as i32;
        diff_exp >= 0 && diff_max >= 0
    }

    /// Process logout request
    pub fn process_logout(&mut self, pdu: &IscsiPdu) -> ScsiResult<IscsiPdu> {
        let logout = pdu.parse_logout_request()?;

        self.state = SessionState::Logout;

        Ok(IscsiPdu::logout_response(
            logout.itt,
            self.next_stat_sn(),
            self.exp_cmd_sn,
            self.max_cmd_sn,
            pdu::logout_response::SUCCESS,
            self.params.default_time2wait,
            self.params.default_time2retain,
        ))
    }

    /// Process NOP-Out (ping) request
    pub fn process_nop_out(&mut self, pdu: &IscsiPdu) -> ScsiResult<IscsiPdu> {
        let nop = pdu.parse_nop_out()?;

        // Only respond if ITT is not 0xFFFFFFFF (unsolicited NOP-In)
        if nop.itt == 0xFFFF_FFFF {
            // Target-initiated ping, initiator responding - no response needed
            return Err(IscsiError::Protocol("Unsolicited NOP-Out response".to_string()));
        }

        Ok(IscsiPdu::nop_in(
            nop.itt,
            0xFFFF_FFFF, // TTT for response
            self.next_stat_sn(),
            self.exp_cmd_sn,
            self.max_cmd_sn,
            nop.lun,
        ))
    }

    /// Handle SendTargets discovery request
    pub fn handle_send_targets(&self, target_name: &str, target_address: &str) -> Vec<(String, String)> {
        vec![
            ("TargetName".to_string(), target_name.to_string()),
            ("TargetAddress".to_string(), format!("{},1", target_address)),
        ]
    }
}

/// Connection state for a single TCP connection within a session
#[derive(Debug, Clone)]
pub struct IscsiConnection {
    /// Connection ID
    pub cid: u16,
    /// Connection state
    pub state: ConnectionState,
    /// Associated session
    pub session_id: Option<u16>, // TSIH
}

/// Connection states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum ConnectionState {
    /// Initial state
    #[default]
    Free,
    /// In login phase
    InLogin,
    /// Logged in, full feature phase
    LoggedIn,
    /// In logout phase
    InLogout,
    /// Cleanup state
    Cleanup,
}


impl IscsiConnection {
    /// Create a new connection
    pub fn new(cid: u16) -> Self {
        IscsiConnection {
            cid,
            state: ConnectionState::Free,
            session_id: None,
        }
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_new() {
        let session = IscsiSession::new();
        assert_eq!(session.state, SessionState::Free);
        assert_eq!(session.tsih, 0);
        assert_eq!(session.exp_cmd_sn, 1);
    }

    #[test]
    fn test_session_params_default() {
        let params = SessionParams::default();
        assert_eq!(params.max_recv_data_segment_length, 8192);
        assert_eq!(params.max_burst_length, 262144);
        assert_eq!(params.first_burst_length, 65536);
        assert_eq!(params.error_recovery_level, 0);
        assert!(params.data_pdu_in_order);
        assert!(params.data_sequence_in_order);
    }

    #[test]
    fn test_session_type() {
        let mut session = IscsiSession::new();
        assert_eq!(session.session_type, SessionType::Normal);

        session.apply_initiator_param("SessionType", "Discovery");
        assert_eq!(session.session_type, SessionType::Discovery);

        session.apply_initiator_param("SessionType", "Normal");
        assert_eq!(session.session_type, SessionType::Normal);
    }

    #[test]
    fn test_parameter_negotiation() {
        let mut session = IscsiSession::new();

        // Test MaxBurstLength - should take minimum
        session.params.max_burst_length = 262144;
        session.apply_initiator_param("MaxBurstLength", "131072");
        assert_eq!(session.params.max_burst_length, 131072);

        // Test InitiatorName
        session.apply_initiator_param("InitiatorName", "iqn.2025-12.test:initiator");
        assert_eq!(session.params.initiator_name, "iqn.2025-12.test:initiator");

        // Test ErrorRecoveryLevel - should take minimum
        session.params.error_recovery_level = 2;
        session.apply_initiator_param("ErrorRecoveryLevel", "1");
        assert_eq!(session.params.error_recovery_level, 1);
    }

    #[test]
    fn test_immediate_data_negotiation() {
        let mut session = IscsiSession::new();

        // Both want it - should be true
        session.params.immediate_data = true;
        session.apply_initiator_param("ImmediateData", "Yes");
        assert!(session.params.immediate_data);

        // Target wants it, initiator doesn't - should be false
        session.params.immediate_data = true;
        session.apply_initiator_param("ImmediateData", "No");
        assert!(!session.params.immediate_data);
    }

    #[test]
    fn test_initial_r2t_negotiation() {
        let mut session = IscsiSession::new();

        // Both don't want it - should be false
        session.params.initial_r2t = false;
        session.apply_initiator_param("InitialR2T", "No");
        assert!(!session.params.initial_r2t);

        // One wants it - should be true (OR operation)
        session.params.initial_r2t = false;
        session.apply_initiator_param("InitialR2T", "Yes");
        assert!(session.params.initial_r2t);
    }

    #[test]
    fn test_sequence_number_validation() {
        let mut session = IscsiSession::new();
        session.exp_cmd_sn = 100;
        session.max_cmd_sn = 110;

        // In window - valid
        assert!(session.validate_cmd_sn(100));
        assert_eq!(session.exp_cmd_sn, 101); // Should advance

        // Also in window
        assert!(session.validate_cmd_sn(105));

        // Out of window - invalid
        assert!(!session.validate_cmd_sn(50));
        assert!(!session.validate_cmd_sn(200));
    }

    #[test]
    fn test_stat_sn_increment() {
        let mut session = IscsiSession::new();
        session.stat_sn = 0;

        assert_eq!(session.next_stat_sn(), 0);
        assert_eq!(session.stat_sn, 1);

        assert_eq!(session.next_stat_sn(), 1);
        assert_eq!(session.stat_sn, 2);
    }

    #[test]
    fn test_generate_response_params() {
        let mut session = IscsiSession::new();
        session.params.target_name = "iqn.2025-12.local:storage".to_string();
        session.params.max_recv_data_segment_length = 8192;

        let params = session.generate_response_params();

        // Check that required params are present
        assert!(params.iter().any(|(k, v)| k == "SessionType" && v == "Normal"));
        assert!(params.iter().any(|(k, _)| k == "MaxRecvDataSegmentLength"));
        assert!(params.iter().any(|(k, _)| k == "MaxBurstLength"));
    }

    #[test]
    fn test_session_states() {
        let mut session = IscsiSession::new();

        assert!(!session.is_full_feature());
        assert!(!session.is_discovery());

        session.state = SessionState::FullFeaturePhase;
        assert!(session.is_full_feature());

        session.session_type = SessionType::Discovery;
        assert!(session.is_discovery());
    }

    #[test]
    fn test_connection_new() {
        let conn = IscsiConnection::new(1);
        assert_eq!(conn.cid, 1);
        assert_eq!(conn.state, ConnectionState::Free);
        assert!(conn.session_id.is_none());
    }

    #[test]
    fn test_digest_type_default() {
        assert_eq!(DigestType::default(), DigestType::None);
    }

    #[test]
    fn test_send_targets() {
        let session = IscsiSession::new();
        let targets = session.handle_send_targets(
            "iqn.2025-12.local:storage",
            "192.168.1.100:3260"
        );

        assert_eq!(targets.len(), 2);
        assert!(targets.iter().any(|(k, v)| k == "TargetName" && v == "iqn.2025-12.local:storage"));
        assert!(targets.iter().any(|(k, v)| k == "TargetAddress" && v == "192.168.1.100:3260,1"));
    }

    #[test]
    fn test_header_digest_negotiation() {
        let mut session = IscsiSession::new();

        session.apply_initiator_param("HeaderDigest", "None");
        assert_eq!(session.params.header_digest, DigestType::None);

        session.apply_initiator_param("HeaderDigest", "CRC32C");
        assert_eq!(session.params.header_digest, DigestType::CRC32C);

        session.apply_initiator_param("HeaderDigest", "None,CRC32C");
        assert_eq!(session.params.header_digest, DigestType::CRC32C);
    }
}
