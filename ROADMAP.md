# iscsi-target Development Roadmap

This document outlines the implementation phases for completing the iscsi-target crate.

## Current Status: Phase 3 - SCSI Command Handling Complete ✓

The API structure, trait definitions, project foundation, PDU layer, session management, and SCSI command handling are complete.

### Phase 0 - Foundation ✓
- [x] ScsiBlockDevice trait definition
- [x] IscsiTarget builder pattern API
- [x] Error types and result handling
- [x] Example implementation (in-memory storage)
- [x] Documentation and README
- [x] Project builds successfully
- [x] Pushed to GitHub

### Phase 1 - PDU Support ✓
- [x] Define PDU header structure (48 bytes BHS)
- [x] Implement BHS (Basic Header Segment) parsing
- [x] Add PDU serialization to bytes
- [x] Add PDU deserialization from bytes
- [x] Implement PDU validation
- [x] Add unit tests for PDU parsing (14 tests passing)
- [x] Login Request/Response PDUs
- [x] Text Request/Response PDUs
- [x] SCSI Command/Response PDUs
- [x] SCSI Data-Out/Data-In PDUs
- [x] Logout Request/Response PDUs
- [x] NOP-Out/NOP-In PDUs

**Reference:** RFC 3720 Section 10 (PDU formats)

### Phase 2 - Session Management ✓
- [x] Define Session structure (IscsiSession)
- [x] Define Connection structure (IscsiConnection)
- [x] Implement login state machine (SessionState enum)
- [x] Handle parameter negotiation (all RFC 3720 parameters)
- [x] Implement session authentication (None, ready for CHAP)
- [x] Track command sequence numbers (CmdSN, StatSN, ExpCmdSN)
- [x] Handle logout and session cleanup
- [x] Add session state tests (14 tests passing, 28 total)
- [x] Discovery session support (SendTargets)
- [x] Digest type negotiation (None/CRC32C)

**Reference:** RFC 3720 Sections 5-7 (Session management)

### Phase 3 - SCSI Command Handling ✓
- [x] Parse SCSI CDB (Command Descriptor Block)
- [x] Implement INQUIRY command handler (standard + VPD pages)
- [x] Implement READ CAPACITY (10/16) handlers
- [x] Implement TEST UNIT READY handler
- [x] Implement READ (10/16) handlers
- [x] Implement WRITE (10/16) handlers
- [x] Implement VERIFY command
- [x] Implement MODE SENSE (6/10) handlers
- [x] Implement REQUEST SENSE handler
- [x] Implement REPORT LUNS handler
- [x] Implement SYNCHRONIZE CACHE handler
- [x] Implement START STOP UNIT handler
- [x] Generate SCSI response with proper status codes
- [x] Handle SCSI sense data for errors
- [x] Add command handler tests (18 tests, 46 total)

**Reference:** SCSI Block Commands (SBC-4) specification

## Phase 4: Target Server Implementation

Wire everything together into a working TCP server.

**Goal:** Complete end-to-end iSCSI target server

**Files to implement:**
- Complete `src/target.rs` implementation

**Tasks:**
- [ ] Implement TCP listener on port 3260
- [ ] Handle incoming connections
- [ ] Implement login phase
- [ ] Implement full feature phase
- [ ] Process commands in sequence
- [ ] Handle multiple concurrent connections
- [ ] Implement proper shutdown
- [ ] Add integration tests
- [ ] Test with real iSCSI initiators (Linux, Windows)

**Architecture:**
```
TcpListener (0.0.0.0:3260)
    ↓
Accept connection
    ↓
Login Phase (Session creation)
    ↓
Full Feature Phase (Command processing)
    ↓
    ├─→ Read PDU
    ├─→ Parse PDU
    ├─→ Handle SCSI command
    ├─→ Call ScsiBlockDevice
    ├─→ Generate response PDU
    └─→ Send response
```

**Reference:** RFC 3720 Section 8 (State transitions)

**Estimated Complexity:** High - Integration and concurrency

## Phase 5: Testing and Hardening

Test with real-world initiators and fix issues.

**Goal:** Reliable, production-ready implementation

**Tasks:**
- [ ] Test with Linux open-iscsi initiator
- [ ] Test with Windows iSCSI initiator
- [ ] Test with ESXi iSCSI initiator
- [ ] Handle error conditions gracefully
- [ ] Add comprehensive unit tests
- [ ] Add integration tests
- [ ] Performance testing and optimization
- [ ] Documentation review
- [ ] Example applications

**Test Scenarios:**
- Basic read/write operations
- Large sequential reads/writes
- Random I/O patterns
- Connection drops and recovery
- Multiple initiators (if supported)
- Concurrent operations

**Estimated Complexity:** High - Real-world compatibility

## Phase 6: Advanced Features (Optional)

Optional features for enhanced functionality.

**Tasks:**
- [ ] CHAP authentication
- [ ] Multiple LUNs per target
- [ ] Discovery sessions (SendTargets)
- [ ] Error recovery levels
- [ ] Persistent reservations
- [ ] Thin provisioning support
- [ ] TRIM/UNMAP support
- [ ] Async event notifications

**Estimated Complexity:** Varies by feature

## Phase 7: Publication

Prepare for crates.io release.

**Tasks:**
- [ ] Review all documentation
- [ ] Add comprehensive examples
- [ ] Create migration guide
- [ ] Set version to 0.1.0 (or 1.0.0 if fully tested)
- [ ] `cargo publish --dry-run`
- [ ] `cargo publish`
- [ ] Announce on Reddit, Discord, etc.

## Implementation Notes

### Recommended Approach

**Incremental Development:**
1. Start with Phase 1 (PDU parsing)
2. Write unit tests for each PDU type
3. Move to Phase 2 (sessions) once PDU parsing is solid
4. Test each phase independently before moving on

**Testing Strategy:**
- Unit tests for each module
- Integration tests using mock initiators
- Real-world tests with actual iSCSI initiators
- Use Wireshark to debug protocol issues

### Time Estimates

These are rough estimates for someone familiar with Rust:

- Phase 1: 2-3 days (PDU parsing is tedious but straightforward)
- Phase 2: 3-4 days (state machine complexity)
- Phase 3: 2-3 days (command handling is well-defined)
- Phase 4: 3-5 days (integration and debugging)
- Phase 5: 5-7 days (testing and fixing issues)
- Phase 6: Varies (1-2 days per feature)

**Total: 15-25 days of focused development**

### Resources

**Specifications:**
- [RFC 3720: iSCSI](https://datatracker.ietf.org/doc/html/rfc3720)
- [RFC 3721: iSCSI Naming](https://datatracker.ietf.org/doc/html/rfc3721)
- [RFC 3722: iSCSI String Profile](https://datatracker.ietf.org/doc/html/rfc3722)
- [SCSI Block Commands (SBC-4)](https://www.t10.org/drafts.htm)

**Tools:**
- Wireshark (iSCSI protocol analysis)
- Linux open-iscsi (testing initiator)
- Windows iSCSI initiator (testing)

**Similar Projects:**
- [rust-iscsi](https://github.com/cholcombe973/iscsi) - iSCSI initiator in Rust
- [tgt](http://stgt.sourceforge.net/) - Linux SCSI target (C reference)
- [vblade](https://github.com/OpenAoE/vblade) - AoE target (simpler protocol)

## Success Criteria

The implementation is complete when:

1. ✓ Code builds without warnings
2. ✓ All unit tests pass
3. ✓ Linux initiator can connect and mount
4. ✓ Windows initiator can connect and format
5. ✓ Can read/write files successfully
6. ✓ No data corruption under normal operations
7. ✓ Documentation is comprehensive
8. ✓ Examples work as documented

## Non-Goals (Out of Scope)

These features are explicitly NOT planned for initial release:

- iSCSI initiator functionality (client side)
- FC (Fibre Channel) support
- iSER (iSCSI Extensions for RDMA)
- High availability / clustering
- Built-in RAID functionality
- GUI or management interface

These can be added later or as separate crates.
