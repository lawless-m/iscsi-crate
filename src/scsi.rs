//! SCSI block device trait and command handling
//!
//! This module defines the interface that storage backends must implement.

use crate::error::ScsiResult;

/// SCSI block device trait
///
/// Implement this trait to provide storage backend for the iSCSI target.
/// The trait is designed to be simple and focused on block-level operations.
pub trait ScsiBlockDevice: Send + Sync {
    /// Read blocks from the device
    ///
    /// # Arguments
    /// * `lba` - Logical block address to start reading from
    /// * `blocks` - Number of blocks to read
    /// * `block_size` - Size of each block in bytes
    ///
    /// # Returns
    /// Vector containing the requested data (length = blocks * block_size)
    fn read(&self, lba: u64, blocks: u32, block_size: u32) -> ScsiResult<Vec<u8>>;

    /// Write blocks to the device
    ///
    /// # Arguments
    /// * `lba` - Logical block address to start writing to
    /// * `data` - Data to write (length must be multiple of block_size)
    /// * `block_size` - Size of each block in bytes
    fn write(&mut self, lba: u64, data: &[u8], block_size: u32) -> ScsiResult<()>;

    /// Get total capacity in logical blocks
    fn capacity(&self) -> u64;

    /// Get block size in bytes (typically 512 or 4096)
    fn block_size(&self) -> u32;

    /// Flush any pending writes to stable storage
    fn flush(&mut self) -> ScsiResult<()> {
        // Default implementation: no-op
        Ok(())
    }
}

/// SCSI command opcodes (subset needed for basic block storage)
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScsiCommand {
    TestUnitReady = 0x00,
    Inquiry = 0x12,
    ReadCapacity10 = 0x25,
    Read10 = 0x28,
    Write10 = 0x2A,
    ReadCapacity16 = 0x9E,
    Read16 = 0x88,
    Write16 = 0x8A,
}

impl ScsiCommand {
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0x00 => Some(ScsiCommand::TestUnitReady),
            0x12 => Some(ScsiCommand::Inquiry),
            0x25 => Some(ScsiCommand::ReadCapacity10),
            0x28 => Some(ScsiCommand::Read10),
            0x2A => Some(ScsiCommand::Write10),
            0x9E => Some(ScsiCommand::ReadCapacity16),
            0x88 => Some(ScsiCommand::Read16),
            0x8A => Some(ScsiCommand::Write16),
            _ => None,
        }
    }
}
