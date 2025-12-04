//! Simple iSCSI target example with in-memory storage
//!
//! This example demonstrates how to create an iSCSI target backed by
//! a simple in-memory storage device.

use iscsi_target::{IscsiError, IscsiTarget, ScsiBlockDevice, ScsiResult};

/// Simple in-memory storage backend
struct MemoryStorage {
    data: Vec<u8>,
    block_size: u32,
}

impl MemoryStorage {
    fn new(size_mb: usize, block_size: u32) -> Self {
        let size_bytes = size_mb * 1024 * 1024;
        Self {
            data: vec![0u8; size_bytes],
            block_size,
        }
    }
}

impl ScsiBlockDevice for MemoryStorage {
    fn read(&self, lba: u64, blocks: u32, block_size: u32) -> ScsiResult<Vec<u8>> {
        if block_size != self.block_size {
            return Err(IscsiError::Scsi(format!(
                "block size mismatch: expected {}, got {}",
                self.block_size, block_size
            )));
        }

        let offset = (lba * block_size as u64) as usize;
        let len = (blocks * block_size) as usize;

        if offset + len > self.data.len() {
            return Err(IscsiError::Scsi(format!(
                "read beyond device capacity: LBA {}, blocks {}",
                lba, blocks
            )));
        }

        Ok(self.data[offset..offset + len].to_vec())
    }

    fn write(&mut self, lba: u64, data: &[u8], block_size: u32) -> ScsiResult<()> {
        if block_size != self.block_size {
            return Err(IscsiError::Scsi(format!(
                "block size mismatch: expected {}, got {}",
                self.block_size, block_size
            )));
        }

        let offset = (lba * block_size as u64) as usize;

        if offset + data.len() > self.data.len() {
            return Err(IscsiError::Scsi(format!(
                "write beyond device capacity: LBA {}, bytes {}",
                lba,
                data.len()
            )));
        }

        self.data[offset..offset + data.len()].copy_from_slice(data);
        Ok(())
    }

    fn capacity(&self) -> u64 {
        (self.data.len() / self.block_size as usize) as u64
    }

    fn block_size(&self) -> u32 {
        self.block_size
    }

    fn flush(&mut self) -> ScsiResult<()> {
        // No-op for memory storage
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    // Create 100 MB in-memory storage with 512-byte blocks
    let storage = MemoryStorage::new(100, 512);

    println!("Creating iSCSI target with {} MB in-memory storage", 100);
    println!(
        "Capacity: {} blocks of {} bytes",
        storage.capacity(),
        storage.block_size()
    );

    // Build and configure the target
    let target = IscsiTarget::builder()
        .bind_addr("0.0.0.0:3260")
        .target_name("iqn.2025-12.local:storage.memory-disk")
        .build(storage)?;

    println!("\niSCSI target configured:");
    println!("  Target name: iqn.2025-12.local:storage.memory-disk");
    println!("  Listen address: 0.0.0.0:3260");
    println!("\nNOTE: This is a skeleton - iSCSI protocol implementation pending");
    println!("See RFC 3720: https://datatracker.ietf.org/doc/html/rfc3720\n");

    // Run the target (currently returns error as implementation is pending)
    match target.run() {
        Ok(_) => {
            println!("Target stopped");
            Ok(())
        }
        Err(e) => {
            eprintln!("Target error: {}", e);
            println!("\nThis crate provides the API structure for an iSCSI target.");
            println!("The actual protocol implementation is left as future work.");
            Ok(())
        }
    }
}
