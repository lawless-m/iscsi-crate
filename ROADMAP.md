# iscsi-target Development Roadmap

This document outlines the implementation phases for completing the iscsi-target crate.

## Current Status: Phase 0 - Foundation Complete ✓

The API structure, trait definitions, and project foundation are complete.

- [x] ScsiBlockDevice trait definition
- [x] IscsiTarget builder pattern API
- [x] Error types and result handling
- [x] Example implementation (in-memory storage)
- [x] Documentation and README
- [x] Project builds successfully
- [x] Pushed to GitHub

## Phase 1: Basic PDU Support

Implement the core iSCSI PDU (Protocol Data Unit) parsing and serialization.

**Goal:** Parse and generate basic iSCSI protocol messages

**Files to implement:**
- `src/pdu.rs` - PDU structure and parsing

**Tasks:**
- [ ] Define PDU header structure (48 bytes)
- [ ] Implement BHS (Basic Header Segment) parsing
- [ ] Implement AHS (Additional Header Segment) parsing
- [ ] Add PDU serialization to bytes
- [ ] Add PDU deserialization from bytes
- [ ] Implement PDU validation
- [ ] Add unit tests for PDU parsing

**Key PDU Types to Support:**
- Login Request/Response
- Text Request/Response
- SCSI Command/Response
- SCSI Data-Out/Data-In
- Logout Request/Response
- NOP-Out/NOP-In

**Reference:** RFC 3720 Section 10 (PDU formats)

**Estimated Complexity:** Medium - Straightforward binary protocol parsing

## Phase 2: Session Management

Implement connection and session state management.

**Goal:** Handle iSCSI login, negotiation, and session lifecycle

**Files to implement:**
- `src/session.rs` - Session and connection management

**Tasks:**
- [ ] Define Session structure
- [ ] Define Connection structure
- [ ] Implement login state machine
- [ ] Handle parameter negotiation (MaxRecvDataSegmentLength, etc.)
- [ ] Implement session authentication (none/CHAP)
- [ ] Track command sequence numbers (CmdSN, StatSN)
- [ ] Handle logout and session cleanup
- [ ] Add session state tests

**Key Concepts:**
- Discovery session vs Normal session
- Leading connection vs additional connections
- Text negotiation key=value pairs
- TSIH (Target Session Identifying Handle)

**Reference:** RFC 3720 Sections 5-7 (Session management)

**Estimated Complexity:** Medium-High - State machine logic

## Phase 3: SCSI Command Handling

Implement SCSI command processing and response generation.

**Goal:** Handle SCSI commands and translate to ScsiBlockDevice calls

**Files to implement:**
- Enhance `src/scsi.rs` with command handlers

**Tasks:**
- [ ] Parse SCSI CDB (Command Descriptor Block)
- [ ] Implement INQUIRY command handler
- [ ] Implement READ CAPACITY (10/16) handlers
- [ ] Implement TEST UNIT READY handler
- [ ] Implement READ (10/16) handlers
- [ ] Implement WRITE (10/16) handlers
- [ ] Implement VERIFY command
- [ ] Generate SCSI response PDUs
- [ ] Handle SCSI sense data for errors
- [ ] Add command handler tests

**SCSI Commands Priority:**
1. INQUIRY - Device identification
2. READ CAPACITY - Get device size
3. TEST UNIT READY - Check device ready
4. READ 10/16 - Read data
5. WRITE 10/16 - Write data
6. VERIFY - Verify written data

**Reference:** 
- RFC 3720 Section 10.3 (SCSI Command)
- SCSI Block Commands (SBC-4) specification

**Estimated Complexity:** Medium - Well-defined command formats

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
