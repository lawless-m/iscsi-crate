# Next Steps for iSCSI Target Development

This document provides a quick-start guide for resuming work on the iscsi-target crate.

## Current State

**Status:** Phase 2 (Session Management) complete, ready for SCSI Command Handling

**What's Done:**
- ✓ Project structure and Cargo.toml
- ✓ ScsiBlockDevice trait API
- ✓ IscsiTarget builder pattern
- ✓ Error types
- ✓ Example implementation
- ✓ Documentation
- ✓ Pushed to GitHub: https://github.com/lawless-m/iscsi-crate
- ✓ **PDU parsing and serialization (Phase 1 complete)**
- ✓ All PDU types implemented (Login, Text, SCSI, Data-In/Out, NOP, Logout)
- ✓ **Session Management (Phase 2 complete)**
- ✓ IscsiSession with login state machine
- ✓ Parameter negotiation (all RFC 3720 parameters)
- ✓ Sequence number tracking (CmdSN, StatSN)
- ✓ 28 unit tests passing

**What's Next:**
Implement SCSI Command Handling (Phase 3) to process SCSI commands.

## Where to Start

### Quick Start: Phase 3 - SCSI Command Handling

Start with `src/scsi.rs` - enhance with command handlers.

**Goal:** Handle SCSI commands and translate to ScsiBlockDevice calls

**File:** `src/scsi.rs`

**Steps:**

1. Implement INQUIRY command handler:
```rust
pub fn handle_inquiry(cdb: &[u8], device: &dyn ScsiBlockDevice) -> ScsiResult<Vec<u8>> {
    // Return standard INQUIRY response (36+ bytes)
    // Device type, vendor ID, product ID, etc.
}
```

2. Implement READ CAPACITY 10/16:
```rust
pub fn handle_read_capacity_10(device: &dyn ScsiBlockDevice) -> ScsiResult<Vec<u8>> {
    // Return 8 bytes: last LBA (4 bytes) + block size (4 bytes)
}
```

3. Implement READ 10/16 and WRITE 10/16:
```rust
pub fn handle_read_10(cdb: &[u8], device: &dyn ScsiBlockDevice) -> ScsiResult<Vec<u8>> {
    // Parse LBA and transfer length from CDB
    // Call device.read() and return data
}
```

4. Generate SCSI sense data for errors

**Reference:** SCSI Block Commands (SBC-4) specification

### After SCSI Commands Work

Continue in this order:

1. **Target Server** (`src/target.rs`)
   - TCP listener implementation
   - Connection handling
   - Wire everything together

## Testing Approach

### During Development

Create a test initiator in `examples/test_initiator.rs`:

```rust
// Simple client that sends login request
// Validates server responses
// Tests basic SCSI commands
```

### After Basic Implementation

Test with real initiators:

**Linux:**
```bash
# From your Linux system (10.0.1.7)
sudo iscsiadm -m discovery -t sendtargets -p 127.0.0.1
sudo iscsiadm -m node --login
```

**Windows:**
```powershell
# From your Windows VM
iscsicli AddTargetPortal 127.0.0.1 3260
iscsicli LoginTarget iqn.2025-12.local:storage.disk1
```

### Use Wireshark

```bash
# Capture iSCSI traffic
sudo tcpdump -i lo -w iscsi.pcap port 3260
wireshark iscsi.pcap
```

Compare your implementation against TGT (working reference).

## Development Environment

### After Moving Directories

The project is in: `/home/matt/Git/iscsi-crate`

```bash
cd /home/matt/Git/iscsi-crate

# Build
cargo build

# Run example
cargo run --example simple_target

# Run tests
cargo test

# Check for issues
cargo clippy
```

### Recommended Setup

Open these files side-by-side:
- `IMPLEMENTATION.md` - Technical reference
- `src/pdu.rs` - Your current work
- RFC 3720 in browser - Specification

## Implementation Order

Follow this sequence to minimize dependencies:

### Phase 1: PDU Layer ✓ COMPLETE
- [x] `src/pdu.rs` - Basic PDU structure (48-byte BHS)
- [x] Parse LOGIN_REQUEST
- [x] Serialize LOGIN_RESPONSE
- [x] Parse SCSI_COMMAND
- [x] Serialize SCSI_RESPONSE
- [x] Parse/Serialize TEXT, NOP, LOGOUT, DATA-IN/OUT PDUs
- [x] Unit tests (14 tests passing)

### Phase 2: Session Layer ✓ COMPLETE
- [x] `src/session.rs` - Session structure (IscsiSession)
- [x] Login state machine (SessionState enum)
- [x] Parameter negotiation (all RFC 3720 params)
- [x] Sequence number tracking (CmdSN, StatSN, ExpCmdSN)
- [x] Session tests (14 tests, 28 total)
- [x] Discovery session support

### Phase 3: SCSI Layer (NEXT)
- [ ] `src/scsi.rs` - SCSI command handlers
- [ ] INQUIRY command
- [ ] READ CAPACITY 10
- [ ] READ 10
- [ ] WRITE 10
- [ ] SCSI response generation
- [ ] Command tests

### Phase 4: Server Layer (3-5 days)
- [ ] `src/target.rs` - Complete implementation
- [ ] TCP listener
- [ ] Connection handler
- [ ] Login phase handling
- [ ] Full feature phase handling
- [ ] Integration tests

### Phase 5: Testing (5-7 days)
- [ ] Test with Linux initiator
- [ ] Test with Windows initiator
- [ ] Fix compatibility issues
- [ ] Performance testing
- [ ] Documentation updates

### Phase 6: Polish (2-3 days)
- [ ] Remove dead_code warnings
- [ ] Add comprehensive examples
- [ ] Final documentation review
- [ ] Prepare for publication

## Key Resources

### Files in This Project

- `ROADMAP.md` - High-level development phases
- `IMPLEMENTATION.md` - Detailed technical guide (READ THIS!)
- `README.md` - User-facing documentation
- `src/lib.rs` - Public API
- `src/scsi.rs` - Trait and SCSI commands
- `examples/simple_target.rs` - Example usage

### External References

**Must Read:**
- [RFC 3720](https://datatracker.ietf.org/doc/html/rfc3720) - iSCSI specification
  - Section 10: PDU formats (start here!)
  - Section 5-7: Session management
  - Section 8: State transitions

**Reference Implementations:**
- TGT (C) - Working iSCSI target on your Linux box
- open-iscsi (C) - Linux initiator source code
- Wireshark dissector - Shows how to parse PDUs

**Testing Tools:**
- Wireshark - Protocol analysis
- `iscsiadm` (Linux) - Initiator tool
- `iscsicli` (Windows) - Initiator tool

## Quick Commands Reference

```bash
# Development
cd /home/matt/Git/iscsi-crate
cargo build
cargo test
cargo run --example simple_target

# Check for issues
cargo clippy
cargo fmt

# Git operations
git status
git add -A
git commit -m "Implement PDU parsing"
git push

# Testing with Linux initiator
sudo iscsiadm -m discovery -t sendtargets -p 127.0.0.1
sudo iscsiadm -m node --login
sudo iscsiadm -m node --logout

# Debugging
sudo tcpdump -i lo -w /tmp/iscsi.pcap port 3260
wireshark /tmp/iscsi.pcap
```

## Tips for Success

### Start Small
- Get LOGIN working first
- Then INQUIRY
- Then READ CAPACITY
- Then READ/WRITE
- Build incrementally

### Use Existing Tools
- Compare your PDUs against TGT's PDUs in Wireshark
- Copy exact byte patterns from working implementations
- Test each PDU type individually

### Debug with Logging
```rust
log::debug!("Received PDU: opcode={:02x}, flags={:02x}", pdu.opcode, pdu.flags);
log::debug!("Data: {:02x?}", &pdu.data[..16]);
```

### Don't Optimize Early
- Get it working first
- Clean it up later
- Performance doesn't matter until it works

### When Stuck
1. Check Wireshark - what does TGT send?
2. Check RFC 3720 - what should you send?
3. Check your bytes - what are you actually sending?
4. Compare all three

## Expected Timeline

**Focused development:** 15-25 days

**Part-time development:** 4-8 weeks

**No rush timeline:** When it's ready

The "tedious but not difficult" nature means it's straightforward but time-consuming. Each phase builds on the previous one, so don't skip ahead.

## When It's Done

The implementation is complete when:

1. Builds without warnings
2. Linux initiator can connect and mount
3. Windows initiator can connect and format
4. Files can be read/written successfully
5. No data corruption
6. All tests pass

Then it's ready to publish to crates.io!

## Publishing Checklist

When ready to publish (don't do this until fully implemented):

- [ ] Update version in Cargo.toml to 1.0.0
- [ ] Remove "Early Development" warning from README
- [ ] Add comprehensive examples
- [ ] All tests passing
- [ ] Documentation complete
- [ ] `cargo publish --dry-run`
- [ ] `cargo publish`

---

**Next Command:** `cd /home/matt/Git/iscsi-crate && cargo build`

Good luck! The foundation is solid, now it's just implementing the RFC.
