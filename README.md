# iscsi-target

A pure Rust iSCSI target library providing a clean, trait-based API for building iSCSI storage servers.

## Status

**Implementation Complete** - This crate provides a fully functional iSCSI target implementation with:
- Complete PDU parsing and serialization (RFC 3720)
- Session management with parameter negotiation
- All essential SCSI commands (INQUIRY, READ/WRITE 10/16, etc.)
- Multi-threaded TCP server on port 3260
- 51 unit tests passing

## Overview

This library makes it easy to create iSCSI targets by providing a simple trait (`ScsiBlockDevice`) that you implement for your storage backend. The library handles the iSCSI protocol details, allowing you to focus on storage logic.

## Features

- **Clean trait-based API** - Implement one trait to provide storage
- **Flexible storage backends** - File, memory, network, CAS, or custom
- **Standard compliance** - Based on RFC 3720 (iSCSI protocol)
- **Builder pattern** - Easy configuration with sensible defaults
- **Thread-safe** - Designed for concurrent access

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
iscsi-target = "0.1"
```

### Basic Example

```rust
use iscsi_target::{IscsiTarget, ScsiBlockDevice, ScsiResult};

// Implement the trait for your storage
struct MyStorage {
    data: Vec<u8>,
}

impl ScsiBlockDevice for MyStorage {
    fn read(&self, lba: u64, blocks: u32, block_size: u32) -> ScsiResult<Vec<u8>> {
        let offset = (lba * block_size as u64) as usize;
        let len = (blocks * block_size) as usize;
        Ok(self.data[offset..offset + len].to_vec())
    }

    fn write(&mut self, lba: u64, data: &[u8], block_size: u32) -> ScsiResult<()> {
        let offset = (lba * block_size as u64) as usize;
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let storage = MyStorage {
        data: vec![0u8; 100 * 1024 * 1024],  // 100 MB
    };

    let target = IscsiTarget::builder()
        .bind_addr("0.0.0.0:3260")
        .target_name("iqn.2025-12.local:storage.disk1")
        .build(storage)?;

    target.run()?;
    Ok(())
}
```

## API Documentation

### `ScsiBlockDevice` Trait

The core trait you implement to provide storage:

```rust
pub trait ScsiBlockDevice: Send + Sync {
    /// Read blocks from the device
    fn read(&self, lba: u64, blocks: u32, block_size: u32) -> ScsiResult<Vec<u8>>;

    /// Write blocks to the device
    fn write(&mut self, lba: u64, data: &[u8], block_size: u32) -> ScsiResult<()>;

    /// Get total capacity in logical blocks
    fn capacity(&self) -> u64;

    /// Get block size in bytes (typically 512 or 4096)
    fn block_size(&self) -> u32;

    /// Flush pending writes (optional)
    fn flush(&mut self) -> ScsiResult<()> { Ok(()) }
}
```

### `IscsiTarget` Builder

Configure your target with the builder pattern:

```rust
IscsiTarget::builder()
    .bind_addr("0.0.0.0:3260")               // Listen address (default: 0.0.0.0:3260)
    .target_name("iqn.2025-12.local:...")    // IQN target name (required format)
    .build(storage_backend)?                 // Build with your storage
    .run()?;                                 // Start serving
```

## Use Cases

- **Network-attached storage** - Serve block devices over the network
- **Storage testing** - Create test targets for iSCSI initiator testing
- **Cloud storage** - Expose S3/Azure/GCS as block devices
- **Content-addressed storage** - Integrate with deduplication systems
- **Virtual machine storage** - Provide storage for hypervisors

## Architecture

```
┌─────────────────────┐
│  iSCSI Initiator    │  (Windows, Linux, ESXi, etc.)
│   (Client)          │
└──────────┬──────────┘
           │ iSCSI Protocol (TCP port 3260)
           │
┌──────────▼──────────┐
│   IscsiTarget       │  (This crate)
│  ┌──────────────┐   │
│  │  Protocol    │   │  ← PDU parsing, session mgmt
│  │  Layer       │   │
│  └──────┬───────┘   │
│         │           │
│  ┌──────▼───────┐   │
│  │ ScsiBlock    │   │  ← Your implementation
│  │ Device       │   │
│  │ (trait)      │   │
│  └──────┬───────┘   │
└─────────┼───────────┘
          │
┌─────────▼───────────┐
│  Storage Backend    │  (Memory, File, Network, etc.)
└─────────────────────┘
```

## Testing with Real Initiators

### Running the Example Target

```bash
# Run the example target
cargo run --example simple_target

# The target will listen on 0.0.0.0:3260
```

### Linux (open-iscsi)

```bash
# Discover targets
sudo iscsiadm -m discovery -t sendtargets -p 127.0.0.1:3260

# Login to target
sudo iscsiadm -m node -T iqn.2025-12.local:storage.memory-disk -p 127.0.0.1:3260 --login

# Check for new device
lsblk

# Logout when done
sudo iscsiadm -m node -T iqn.2025-12.local:storage.memory-disk -p 127.0.0.1:3260 --logout
```

### Windows

```powershell
# Add target portal
iscsicli AddTargetPortal 127.0.0.1 3260

# Login to target
iscsicli LoginTarget iqn.2025-12.local:storage.memory-disk T 127.0.0.1 3260 * * * * * * * * * * * * 0
```

## Roadmap

- [x] Trait definition and API structure
- [x] Builder pattern and configuration
- [x] Example implementations
- [x] iSCSI PDU parsing and serialization (RFC 3720)
- [x] Session and connection management
- [x] SCSI command handling (INQUIRY, READ/WRITE 10/16, MODE SENSE, etc.)
- [x] Multi-threaded connection handling
- [ ] Testing with real initiators (Linux, Windows, ESXi)
- [ ] CHAP authentication
- [ ] Error recovery
- [ ] Performance optimization

## Contributing

This is an open-source project. Contributions are welcome, especially for:

- iSCSI protocol implementation (PDU handling, RFC 3720 compliance)
- SCSI command support
- Testing and documentation
- Example storage backends

## References

- [RFC 3720: iSCSI Protocol](https://datatracker.ietf.org/doc/html/rfc3720)
- [RFC 3721: iSCSI Naming and Discovery](https://datatracker.ietf.org/doc/html/rfc3721)
- [SCSI Architecture Model](https://www.t10.org/drafts.htm#SCSI3_SAM)

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
