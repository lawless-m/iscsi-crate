# iSCSI Target Testing Guide

## Understanding Test Results

This guide explains what each test category validates, how to interpret results, and what failures mean for your iSCSI target implementation.

## Test Categories Deep Dive

### 1. Discovery Tests

**What they test:**
- SendTargets discovery mechanism
- Target list formatting and parsing
- Authentication during discovery
- Target redirection handling

**Why it matters:**
Discovery is how initiators find your target. If discovery fails, initiators can't even attempt to connect.

**Common issues:**
- Missing or malformed target records
- Incorrect portal addresses
- Authentication not enforced
- Redirection loops

**Critical failures:**
- No targets returned when targets exist
- Crash on discovery request
- Malformed discovery response

**Minor failures:**
- Missing optional target attributes
- Verbose error messages

**Recommendation:**
All discovery tests should pass. These are well-specified and straightforward.

### 2. Login/Logout Tests

**What they test:**
- Session establishment
- Parameter negotiation (HeaderDigest, DataDigest, MaxRecvDataSegmentLength, etc.)
- Connection setup within sessions
- Clean session teardown
- Multiple login attempts
- Timeout handling

**Why it matters:**
Login establishes the session parameters that govern all subsequent operations. Wrong parameters can cause failures, performance issues, or incompatibility.

**Common issues:**
- Accepting invalid parameter values
- Conflicting parameter combinations
- Not respecting MaxRecvDataSegmentLength
- Memory leaks on repeated login/logout
- Not timing out stale login attempts

**Critical failures:**
- Cannot establish session at all
- Crash during login
- Connection hangs indefinitely
- Accepting values outside RFC-specified ranges
- Session state corruption on multiple logins

**Minor failures:**
- Suboptimal default values
- Overly conservative parameter negotiation
- Missing optional parameters

**Recommendation:**
Parameter validation is crucial. Many implementations accept out-of-range values which later cause problems. Pay special attention to TL-002 and TL-003.

### 3. Authentication Tests

**What they test:**
- CHAP authentication (one-way)
- Mutual CHAP (bidirectional)
- Correct challenge/response handling
- Authentication failures
- Auth required vs optional scenarios

**Why it matters:**
Security. If authentication is broken, unauthorized access is possible.

**Common issues:**
- Accepting wrong passwords
- Challenge reuse vulnerability
- Mutual CHAP not working
- Authentication bypass
- Timing attacks (though not tested here)

**Critical failures:**
- Wrong password accepted (TA-002)
- Auth required but not enforced (TA-006)
- Mutual CHAP broken (TA-003, TA-004)
- Challenge/response protocol violations

**Minor failures:**
- Non-optimal challenge generation
- Missing auth method negotiation

**Recommendation:**
If you implement authentication, all auth tests must pass. Authentication bypass is a security vulnerability.

### 4. SCSI Command Tests

**What they test:**
- Basic SCSI command set: INQUIRY, TEST UNIT READY, READ CAPACITY, etc.
- Proper response formatting
- Error conditions and sense data
- Invalid LUN handling
- Unsupported command handling

**Why it matters:**
SCSI commands provide metadata about the device. Initiators use these to determine device capabilities, size, and state.

**Common issues:**
- Incorrect INQUIRY data (wrong device type, empty vendor/product)
- Wrong capacity reporting
- Missing sense data on errors
- Not rejecting invalid LUNs properly

**Critical failures:**
- INQUIRY returns garbage (TC-001)
- READ CAPACITY wrong (TC-003, TC-004)
- Crash on unsupported command (TC-008)
- Commands to invalid LUNs succeed (TC-009)

**Minor failures:**
- Non-standard but harmless INQUIRY fields
- Verbose or unusual sense data
- Missing optional mode pages

**Recommendation:**
TC-001 (INQUIRY) and TC-003 (READ CAPACITY) must pass. These are fundamental to device initialization.

### 5. I/O Operation Tests

**What they test:**
- Single and multi-block reads and writes
- Sequential and random access
- Large transfers
- Zero-length transfers
- Maximum transfer sizes
- Data pattern integrity (all zeros, all ones, alternating, random)
- Write-then-read verification
- Overwrite behavior

**Why it matters:**
This is the core functionality. If I/O is broken, nothing else matters.

**Common issues:**
- Data corruption (THE MOST SERIOUS)
- Off-by-one errors in block addressing
- Wrong transfer sizes
- Poor handling of large transfers
- Zero-length operation failures
- Beyond-max-transfer handling

**Critical failures:**
- **Data corruption** (TI-001 through TI-014) - ANY data mismatch is critical
- Writes not persisting (TI-002)
- Reads returning wrong data (TI-001, TI-003, TI-005)
- Random data not matching (TI-010)
- Overwrite leaving traces of old data (TI-014)

**Minor failures:**
- Performance issues (slow but correct)
- Suboptimal max transfer size
- Unnecessary operation splitting

**Recommendation:**
Zero tolerance for data corruption. Every single I/O test involving data verification must pass. A single failure here means the target is not production-ready.

### 6. Multi-Connection Tests

**What they test:**
- Multiple connections within a single session
- I/O across multiple connections
- Connection failure while others continue
- Multiple independent sessions
- Concurrent access to same data
- Dynamic connection add/remove

**Why it matters:**
Multi-connection support improves performance and provides redundancy. Many enterprise environments rely on it.

**Common issues:**
- Connections not properly isolated
- Concurrent writes causing corruption
- Poor connection failure handling
- Memory leaks with many connections
- Race conditions under concurrent access

**Critical failures:**
- Data corruption with concurrent writes (TM-005)
- Connection failure affecting other connections (TM-003)
- Crash when handling multiple connections
- Session state corruption

**Minor failures:**
- Suboptimal load balancing
- Conservative MaxConnections limit
- Performance doesn't scale with connections

**Recommendation:**
If you don't support multiple connections per session, ensure tests skip gracefully. If you do support it, TM-005 (concurrent writes) must pass - data integrity is paramount.

### 7. Error Handling Tests

**What they test:**
- Network disconnects during operations
- Timeout handling
- Invalid sequence numbers
- Corrupted PDUs
- CRC errors (if digests enabled)
- Unexpected PDU types
- Resource exhaustion
- Task abort and LUN reset
- Target reset

**Why it matters:**
Real networks aren't perfect. Targets must handle errors gracefully without corrupting data, crashing, or leaking resources.

**Common issues:**
- Crash on malformed input
- Infinite hangs on disconnects
- Memory leaks on error paths
- Not timing out dead connections
- Poor recovery from network issues
- Task abort not working

**Critical failures:**
- Crash on corrupted PDU (TE-004)
- Hang indefinitely on disconnect (TE-001)
- Data corruption after error
- Resource leaks on error paths (TE-007)
- Abort/reset not working (TE-010, TE-011)

**Minor failures:**
- Conservative timeout values
- Verbose error logging
- Session drops when recovery possible

**Recommendation:**
Robustness tests. Your target will see bad input - it must not crash or corrupt data. TE-001 (disconnect during I/O) is especially important.

### 8. Edge Cases and Stress Tests

**What they test:**
- Boundary conditions (min/max LBA, transfer sizes)
- Rapid connect/disconnect
- Command queue depth
- Mixed workload patterns
- Specific data patterns (all zeros, all ones, alternating, random)
- Sustained throughput
- Long-running stability

**Why it matters:**
Edge cases reveal bugs that normal use doesn't expose. Production systems hit these conditions eventually.

**Common issues:**
- Crash or error at LBA boundaries (TX-002, TX-003)
- Resource leaks under stress (TX-004)
- Queue overflow (TX-005)
- Performance degradation over time (TX-011, TX-012)
- Pattern-specific bugs (TX-007, TX-008, TX-009)

**Critical failures:**
- Crash at max LBA (TX-002)
- Accepts beyond-max LBA (TX-003) - data corruption risk
- Memory leak under repeated operations (TX-004)
- Data corruption with any pattern (TX-007 through TX-010)
- Performance collapse (TX-013)

**Minor failures:**
- Conservative max LBA
- Slower performance than expected
- Occasional timeout under extreme stress

**Recommendation:**
TX-010 (random data) is critical - many bugs only appear with non-zero patterns. TX-011/TX-012 (sustained throughput) reveal memory leaks and performance issues.

### 9. Data Integrity Tests

**What they test:**
- Persistence across disconnections
- Recovery from crashes
- Overlapping writes
- Read-after-write consistency
- Deduplication correctness (if applicable)
- Read stability
- Long-running stability
- Power loss simulation

**Why it matters:**
The ultimate test: does data survive across failures and time? This is what users care about most.

**Common issues:**
- Data lost on disconnect
- Corruption after crash
- Partial writes visible
- Read instability
- Deduplication breaking correctness
- Corruption in long-running tests

**Critical failures:**
- **Data loss** (DI-001, DI-002)
- **Corruption after crash** (DI-002)
- **Partial writes exposed** (DI-003) - atomicity violation
- **Read instability** (DI-006)
- **Long-term corruption** (DI-007)

**Minor failures:**
- Performance impact of journaling/sync
- Conservative durability guarantees

**Recommendation:**
These are the most important tests. DI-002 (crash recovery) and DI-007 (long-running stability) are particularly critical. Any failure here means data is at risk.

## Test Priority Classification

### P0 - Must Pass (Ship Blockers)
These tests validate fundamental correctness. Failures mean the target is not safe to use.

**Data Integrity:**
- All TI tests (I/O operations with data verification)
- All DI tests (data integrity across failures)
- TM-005 (concurrent write consistency)
- TX-010 (random data integrity)

**Basic Functionality:**
- TL-001 (basic login)
- TC-001 (INQUIRY)
- TC-003 (READ CAPACITY)
- TD-001 (discovery)

**Security (if auth enabled):**
- TA-002 (CHAP failure with wrong password)
- TA-006 (auth enforcement)

### P1 - Should Pass (Quality Issues)
These tests validate robustness and RFC compliance. Failures indicate bugs or non-conformance.

**Protocol Compliance:**
- TL-002 (parameter negotiation)
- TL-003 (invalid parameter rejection)
- All authentication tests (if auth supported)
- All SCSI command tests

**Error Handling:**
- TE-001 (disconnect during I/O)
- TE-004 (corrupted PDU)
- TE-010 (task abort)
- TE-011 (LUN reset)

**Edge Cases:**
- TX-002 (max LBA)
- TX-003 (beyond max LBA)
- TX-004 (rapid connect/disconnect)

### P2 - Nice to Pass (Enhancement Opportunities)
These tests validate advanced features and optimizations. Failures suggest areas for improvement.

**Advanced Features:**
- All TM tests (multi-connection)
- TX-011/TX-012 (sustained throughput)
- TX-013 (mixed workload)

**Optimizations:**
- Parameter negotiation for max values
- Performance under stress

## Interpreting Specific Failures

### "Data mismatch at block X"
**Severity:** CRITICAL  
**Meaning:** Written data doesn't match read data  
**Cause:** Buffer handling bug, addressing error, deduplication bug, race condition  
**Action:** 
1. Check if specific to certain patterns (all zeros, random, etc.)
2. Test with single connection only to isolate concurrency issues
3. Verify block addressing logic
4. Check content-addressing hash collisions (if applicable)

### "Target accepted out-of-range value"
**Severity:** HIGH  
**Meaning:** Parameter validation missing  
**Cause:** No bounds checking on negotiated parameters  
**Action:**
1. Add parameter validation per RFC 7143 Section 13
2. Test with fuzzing for other validation gaps

### "Authentication failed but should have succeeded"
**Severity:** HIGH (if auth required)  
**Meaning:** Auth implementation broken  
**Cause:** CHAP algorithm bug, challenge/response mismatch, MD5 issue  
**Action:**
1. Log CHAP exchange to debug
2. Verify MD5 implementation
3. Check challenge generation and response validation

### "Connection hang on disconnect"
**Severity:** HIGH  
**Meaning:** No timeout or recovery mechanism  
**Cause:** Missing timeout handling, infinite wait on socket  
**Action:**
1. Implement connection timeout
2. Add error recovery level support
3. Test all timeout scenarios

### "Crash on invalid PDU"
**Severity:** CRITICAL  
**Meaning:** No input validation  
**Cause:** Buffer overflow, null pointer, unhandled case  
**Action:**
1. Add PDU validation before processing
2. Fuzz test with random PDUs
3. Use safe parsing with bounds checking

### "Memory leak detected"
**Severity:** MEDIUM to HIGH  
**Meaning:** Resources not freed  
**Cause:** Missing cleanup on error paths, connection leaks  
**Action:**
1. Run with valgrind: `valgrind --leak-check=full <target>`
2. Add cleanup to all error paths
3. Test repeated connect/disconnect cycles

### "Performance degradation over time"
**Severity:** MEDIUM  
**Meaning:** Resource leak or inefficient algorithm  
**Cause:** Memory leak, cache pollution, inefficient data structures  
**Action:**
1. Profile with perf or similar
2. Check for resource leaks
3. Verify O(1) lookup for content-addressed blocks

### "Deduplication broke data integrity"
**Severity:** CRITICAL  
**Meaning:** Content-addressing has bug  
**Cause:** Hash collision, refcount error, premature deletion  
**Action:**
1. Verify hash function correctness (BLAKE3 should be collision-resistant)
2. Check reference counting logic
3. Ensure block isn't deleted while referenced
4. Test with blocks that hash to similar values

## What "Pass" Really Means

A passing test means:
1. Operation completed as expected
2. No errors returned
3. Data integrity verified (where applicable)
4. Timing within reasonable bounds
5. No resource leaks detected
6. No crashes or hangs

A passing test does **not** mean:
- Optimal performance
- No room for improvement
- Compliance with every nuance of RFC 7143
- Security hardening complete

## Production Readiness Checklist

Your iSCSI target is production-ready when:

- [ ] **Zero P0 failures** - All data integrity tests pass
- [ ] **Minimal P1 failures** - Protocol compliance good
- [ ] **Security validated** - Auth tests pass (if auth used)
- [ ] **Stress tested** - Long-running tests complete without issues
- [ ] **Multi-initiator safe** - Concurrent access doesn't corrupt
- [ ] **Recovers gracefully** - Error handling tests pass
- [ ] **Performance acceptable** - Meets your throughput requirements
- [ ] **Monitored in production** - You have observability

## Common Implementation Bugs Found by Tests

1. **Buffer overruns** - Found by: TE-004 (corrupted PDU), TX-007 (large transfer)
2. **Off-by-one errors** - Found by: TX-002/TX-003 (boundary LBAs), TI-003 (multi-block)
3. **Race conditions** - Found by: TM-005 (concurrent writes), TX-013 (mixed workload)
4. **Memory leaks** - Found by: TX-004 (rapid connect/disconnect), DI-007 (long-running)
5. **Hash collisions** - Found by: DI-005 (deduplication), TX-010 (random data)
6. **Incomplete cleanup** - Found by: TE-001 (disconnect), TE-007 (resource exhaustion)
7. **State corruption** - Found by: TL-004 (multiple logins), TM-003 (connection failure)
8. **Parameter validation gaps** - Found by: TL-003 (invalid values)

## Testing Strategy Recommendations

### During Development
1. Run basic tests frequently: `./iscsi-test-suite -c io,commands`
2. Focus on data integrity first
3. Add stress tests as features stabilize
4. Use verbose mode to understand failures: `-v`

### Before Release
1. Run full test suite: `./iscsi-test-suite config/test_config.ini`
2. Run long-running tests (DI-007) for at least 1 hour
3. Test with multiple configurations (auth/no-auth, digests/no-digests)
4. Test with different initiators (Linux, Windows)
5. Run with valgrind to check for leaks

### After Changes
1. Run full test suite to catch regressions
2. Pay extra attention to data integrity tests
3. Test specific area of change with `-c` flag
4. Run stress tests if concurrency changed

### Continuous Integration
1. Run on every commit
2. Gate releases on all P0 tests passing
3. Track flaky tests and investigate
4. Monitor test duration for performance regressions

## Getting Help

If tests fail and you're not sure why:

1. Run specific test with verbose output: `-v`
2. Check detailed report in `reports/`
3. Compare with known-good target (LIO, TGT)
4. Consult RFC 7143 for protocol details
5. Check libiscsi examples for correct usage
6. Review your code with focus on error path from failure

If you believe a test is incorrect:
1. Document why you think it's wrong
2. Reference RFC section that supports your view
3. Test against multiple known-good implementations
4. File an issue with detailed explanation

## Summary

The test suite is comprehensive but not exhaustive. Passing all tests gives high confidence in:
- Basic correctness
- Protocol compliance
- Data integrity
- Error handling
- Common edge cases

It does not guarantee:
- Performance under extreme load
- Security against sophisticated attacks
- Correct behavior in all possible scenarios
- Bug-free implementation

Use the test suite as a quality gate, but also:
- Test with real workloads
- Monitor in production
- Have a rollback plan
- Keep improving based on real-world issues

Remember: **Data integrity is paramount**. Any test that involves data verification must pass before considering the target production-ready.
