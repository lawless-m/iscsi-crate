//! iSCSI target server implementation
//!
//! This module provides the main server structure and builder pattern.

use crate::error::{IscsiError, ScsiResult};
use crate::scsi::ScsiBlockDevice;
use std::sync::{Arc, Mutex};

/// iSCSI target server
pub struct IscsiTarget<D: ScsiBlockDevice> {
    bind_addr: String,
    target_name: String,
    device: Arc<Mutex<D>>,
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

        // TODO: Implement iSCSI protocol server
        // This is a placeholder that will be filled in during implementation

        Err(IscsiError::Config(
            "iSCSI target implementation pending - see RFC 3720".to_string()
        ))
    }
}

/// Builder for configuring an iSCSI target
pub struct IscsiTargetBuilder<D: ScsiBlockDevice> {
    bind_addr: Option<String>,
    target_name: Option<String>,
    _phantom: std::marker::PhantomData<D>,
}

impl<D: ScsiBlockDevice> IscsiTargetBuilder<D> {
    fn new() -> Self {
        Self {
            bind_addr: None,
            target_name: None,
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

    /// Build the target with the specified storage device
    pub fn build(self, device: D) -> ScsiResult<IscsiTarget<D>> {
        let bind_addr = self.bind_addr.unwrap_or_else(|| "0.0.0.0:3260".to_string());
        let target_name = self.target_name.unwrap_or_else(|| {
            "iqn.2025-12.local:storage.default".to_string()
        });

        // Validate IQN format (basic check)
        if !target_name.starts_with("iqn.") {
            return Err(IscsiError::Config(
                "target_name must be in IQN format (e.g., iqn.2025-12.local:storage.disk1)".to_string()
            ));
        }

        Ok(IscsiTarget {
            bind_addr,
            target_name,
            device: Arc::new(Mutex::new(device)),
        })
    }
}
