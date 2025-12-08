# iSCSI Target Test Suite - Implementation Plan

## Overview
Build a comprehensive C-based test suite using libiscsi to validate iSCSI target implementations. The suite must be completely independent of the target implementation to provide genuine black-box validation.

## Project Goals
1. Validate RFC 7143 (iSCSI) protocol conformance
2. Test error handling and edge cases
3. Verify data integrity under various conditions
4. Test multi-connection and multi-session scenarios
5. Provide clear, actionable test reports
6. Be target-agnostic (works against any iSCSI target)

## Dependencies
- libiscsi (https://github.com/sahlberg/libiscsi)
- Standard C toolchain (gcc/clang)
- POSIX environment (Linux/Unix)

## Project Structure

```
iscsi-test-suite/
├── src/
│   ├── main.c                    # Test runner entry point
│   ├── test_framework.c/h        # Test harness infrastructure
│   ├── test_discovery.c/h        # Discovery and login tests
│   ├── test_auth.c/h            # Authentication tests
│   ├── test_commands.c/h        # SCSI command tests
│   ├── test_io.c/h              # I/O operation tests
│   ├── test_multiconn.c/h       # Multi-connection tests
│   ├── test_error.c/h           # Error handling tests
│   ├── test_edge_cases.c/h      # Edge case and stress tests
│   ├── test_integrity.c/h       # Data integrity tests
│   └── utils.c/h                # Helper functions
├── config/
│   └── test_config.ini          # Test configuration file
├── reports/
│   └── (generated test reports)
├── Makefile
├── README.md
└── TESTING_GUIDE.md
```

## Test Configuration

### config/test_config.ini
```ini
[target]
# Target portal address
portal = 192.168.1.100:3260

# Target IQN (leave empty for discovery)
iqn = 

# LUN to test (default 0)
lun = 0

[authentication]
# Auth method: none, chap, mutual_chap
auth_method = none

# CHAP credentials (if applicable)
username = 
password = 
mutual_username = 
mutual_password = 

[test_parameters]
# Block size for I/O tests
block_size = 512

# Number of blocks for large transfers
large_transfer_blocks = 1024

# Timeout for operations (seconds)
timeout = 30

# Number of iterations for stress tests
stress_iterations = 100

[options]
# Verbosity level: 0=errors only, 1=normal, 2=verbose, 3=debug
verbosity = 1

# Stop on first failure
stop_on_fail = false

# Generate detailed report file
generate_report = true
```

## Test Framework Design

### Core Test Infrastructure

**Test Result Structure:**
```c
typedef enum {
    TEST_PASS,
    TEST_FAIL,
    TEST_SKIP,
    TEST_ERROR
} test_result_t;

typedef struct {
    const char *test_name;
    const char *category;
    test_result_t result;
    char *message;
    double duration_ms;
} test_report_t;
```

**Test Function Signature:**
```c
typedef test_result_t (*test_func_t)(struct iscsi_context *iscsi, test_report_t *report);
```

**Test Registration:**
- Tests self-register into categories
- Framework iterates and executes
- Results collected and reported

## Test Categories and Specific Tests

### 1. Discovery Tests (`test_discovery.c`)

#### TD-001: Basic Discovery
- Send discovery session request
- Verify response contains target list
- Parse and validate target records
- **Expected**: Valid target list with at least one entry

#### TD-002: Discovery With Authentication
- Attempt discovery with CHAP credentials
- Verify authentication sequence
- **Expected**: Successful discovery if auth configured

#### TD-003: Discovery Without Credentials
- Attempt discovery without auth when required
- **Expected**: Authentication failure

#### TD-004: Target Redirection
- Test if target sends redirect
- Verify redirect handling
- **Expected**: Proper redirect response or none

### 2. Login/Logout Tests (`test_discovery.c`)

#### TL-001: Basic Login
- Connect to target
- Complete login phase
- Verify session establishment
- **Expected**: Successful login, session ID assigned

#### TL-002: Parameter Negotiation
- Test all standard key=value pairs:
  - HeaderDigest (None, CRC32C)
  - DataDigest (None, CRC32C)
  - MaxRecvDataSegmentLength
  - MaxBurstLength
  - FirstBurstLength
  - DefaultTime2Wait
  - DefaultTime2Retain
  - InitialR2T
  - ImmediateData
  - MaxConnections
  - MaxOutstandingR2T
- **Expected**: Negotiated values within spec limits

#### TL-003: Invalid Parameter Values
- Send out-of-range parameter values
- Send conflicting parameters
- **Expected**: Target rejects or negotiates to valid values

#### TL-004: Multiple Login Attempts
- Successful login
- Logout
- Login again
- **Expected**: Clean reconnection

#### TL-005: Login Timeout
- Initiate login, pause mid-sequence
- **Expected**: Target times out gracefully

#### TL-006: Simultaneous Logins
- Multiple connections to same target
- **Expected**: Either multiple sessions or proper rejection

### 3. Authentication Tests (`test_auth.c`)

#### TA-001: CHAP Authentication Success
- Login with correct CHAP credentials
- **Expected**: Successful authentication

#### TA-002: CHAP Authentication Failure
- Login with incorrect password
- **Expected**: Authentication failure, no session

#### TA-003: Mutual CHAP Success
- Both initiator and target authenticate
- **Expected**: Successful mutual auth

#### TA-004: Mutual CHAP Partial Failure
- Target authenticates, initiator provides wrong target credentials
- **Expected**: Authentication failure

#### TA-005: CHAP Challenge Reuse
- Attempt to reuse same challenge
- **Expected**: Proper challenge/response sequence

#### TA-006: No Auth When Required
- Target requires auth, initiator doesn't provide
- **Expected**: Login failure

#### TA-007: Auth When Not Required
- Target doesn't require auth, initiator provides anyway
- **Expected**: Successful login (auth ignored or accepted)

### 4. SCSI Command Tests (`test_commands.c`)

#### TC-001: INQUIRY Command
- Send INQUIRY to LUN 0
- Parse response
- Verify standard fields (device type, vendor, product)
- **Expected**: Valid INQUIRY response

#### TC-002: TEST UNIT READY
- Send TEST UNIT READY
- **Expected**: Success or appropriate sense data

#### TC-003: READ CAPACITY (10)
- Request capacity of LUN
- Verify block size and LUN size
- **Expected**: Valid capacity information

#### TC-004: READ CAPACITY (16)
- Request capacity with 16-byte command
- **Expected**: Valid capacity for large LUNs

#### TC-005: MODE SENSE
- Request various mode pages
- **Expected**: Valid mode page data

#### TC-006: REQUEST SENSE
- Generate error condition
- Request sense data
- **Expected**: Appropriate sense key/ASC/ASCQ

#### TC-007: REPORT LUNS
- Request list of LUNs
- **Expected**: Valid LUN list

#### TC-008: Invalid Command
- Send unsupported SCSI command
- **Expected**: Check condition with ILLEGAL REQUEST

#### TC-009: Command to Invalid LUN
- Send command to non-existent LUN
- **Expected**: Appropriate error response

### 5. I/O Operation Tests (`test_io.c`)

#### TI-001: Single Block Read
- Write known pattern
- Read single block
- Verify data matches
- **Expected**: Data integrity maintained

#### TI-002: Single Block Write
- Write single block with pattern
- Read back and verify
- **Expected**: Write successful, data readable

#### TI-003: Multi-Block Sequential Read
- Write sequential pattern across multiple blocks
- Read in single operation
- Verify all data
- **Expected**: Correct sequential data

#### TI-004: Multi-Block Sequential Write
- Write multiple blocks in single operation
- Read back and verify
- **Expected**: All blocks written correctly

#### TI-005: Random Access Reads
- Write blocks at various LBAs
- Read in random order
- Verify each block
- **Expected**: All reads return correct data

#### TI-006: Random Access Writes
- Write to random LBAs
- Verify each write
- **Expected**: All writes successful

#### TI-007: Large Transfer Read
- Read large number of blocks (exceed typical buffer sizes)
- **Expected**: Successful transfer, correct data

#### TI-008: Large Transfer Write
- Write large amount of data
- Verify write completed
- **Expected**: Successful large write

#### TI-009: Zero-Length Transfer
- Attempt read/write of 0 blocks
- **Expected**: Operation completes without error (or appropriate error)

#### TI-010: Maximum Transfer Size
- Determine max transfer from negotiation
- Write at max size
- Read at max size
- **Expected**: Successful at max negotiated size

#### TI-011: Beyond Maximum Transfer
- Attempt transfer larger than negotiated max
- **Expected**: Split into multiple operations or error

#### TI-012: Unaligned Access
- Read/write at non-block-aligned offsets (if supported)
- **Expected**: Appropriate behavior per target capability

#### TI-013: Write-Read-Verify Pattern
- Write known patterns (0x00, 0xFF, 0xAA, 0x55, random)
- Read back
- Verify byte-for-byte
- **Expected**: Perfect data integrity

#### TI-014: Overwrite Test
- Write pattern A
- Write pattern B to same location
- Read back
- **Expected**: Pattern B present, A completely overwritten

### 6. Multi-Connection Tests (`test_multiconn.c`)

#### TM-001: Multiple Connections Single Session
- Login with MaxConnections > 1
- Establish additional connections
- Verify all connections active
- **Expected**: Multiple connections within session

#### TM-002: I/O Across Multiple Connections
- Issue I/O on different connections
- Verify all complete successfully
- **Expected**: Concurrent I/O works correctly

#### TM-003: Connection Failure Recovery
- Establish multiple connections
- Drop one connection
- Continue I/O on remaining connections
- **Expected**: Graceful handling, other connections unaffected

#### TM-004: Multiple Independent Sessions
- Create separate sessions from different initiators
- **Expected**: Sessions isolated correctly

#### TM-005: Concurrent Writes Same LBA
- Multiple connections write to same block simultaneously
- Verify one wins, data is consistent (not corrupted)
- **Expected**: Serialization or proper locking

#### TM-006: Connection Add/Remove
- Start with one connection
- Add connections dynamically
- Remove connections
- **Expected**: Dynamic connection management works

### 7. Error Handling Tests (`test_error.c`)

#### TE-001: Network Disconnect During I/O
- Start large I/O operation
- Forcibly close connection mid-transfer
- Reconnect and verify state
- **Expected**: Target handles disconnect, session recoverable

#### TE-002: Timeout Handling
- Issue command and don't respond to R2T
- **Expected**: Target times out gracefully

#### TE-003: Invalid Sequence Number
- Send PDU with wrong sequence number
- **Expected**: Target rejects or handles per error recovery level

#### TE-004: Corrupted PDU Header
- Send malformed PDU header
- **Expected**: Connection drops or error returned

#### TE-005: CRC Error (if enabled)
- Send PDU with wrong header/data digest
- **Expected**: PDU rejected, error indicated

#### TE-006: Unexpected PDU Type
- Send PDU of wrong type for current phase
- **Expected**: Protocol error, connection drop

#### TE-007: Resource Exhaustion
- Open maximum connections
- Attempt to open one more
- **Expected**: Graceful rejection

#### TE-008: Invalid R2T Response
- Respond to R2T with wrong data length
- **Expected**: Error handling, possible connection drop

#### TE-009: Duplicate ITT
- Send commands with same Initiator Task Tag
- **Expected**: Proper rejection or handling

#### TE-010: ABORT TASK
- Issue long-running command
- Send ABORT TASK for that command
- **Expected**: Command aborted, appropriate response

#### TE-011: LUN RESET
- Issue LUN RESET task management
- Verify all pending commands on LUN abort
- **Expected**: LUN reset successful, clean state

#### TE-012: TARGET WARM RESET
- Issue target warm reset (if supported)
- **Expected**: All sessions drop, target remains available

#### TE-013: TARGET COLD RESET
- Issue target cold reset (if supported)
- **Expected**: Target resets completely

### 8. Edge Cases and Stress Tests (`test_edge_cases.c`)

#### TX-001: Minimum Transfer Size
- Write/read 1 byte (if LUN supports < block size)
- **Expected**: Appropriate behavior

#### TX-002: Maximum LBA
- Write to highest valid LBA
- Read back
- **Expected**: Successful operation at boundary

#### TX-003: Beyond Maximum LBA
- Attempt access beyond LUN capacity
- **Expected**: Error response (ILLEGAL REQUEST)

#### TX-004: Rapid Connect/Disconnect
- Login and logout repeatedly
- **Expected**: Target handles gracefully, no resource leaks

#### TX-005: Command Queue Depth
- Issue maximum queued commands
- Verify all complete
- **Expected**: Queue management works correctly

#### TX-006: Mixed Read/Write Workload
- Simultaneous reads and writes
- **Expected**: Correct operation, no data corruption

#### TX-007: All Zeros Pattern
- Write blocks of all zeros
- Verify deduplication if content-addressed
- Read back
- **Expected**: Correct data, efficient storage

#### TX-008: All Ones Pattern
- Write blocks of all 0xFF
- Read back
- **Expected**: Correct data

#### TX-009: Alternating Pattern
- Write 0xAA55AA55... pattern
- Read back
- **Expected**: Exact pattern preserved

#### TX-010: Random Data Pattern
- Write cryptographically random data
- Read back
- **Expected**: Exact match

#### TX-011: Sustained Write Throughput
- Write continuously for extended period
- Measure throughput
- **Expected**: Consistent performance, no degradation

#### TX-012: Sustained Read Throughput
- Read continuously for extended period
- Measure throughput
- **Expected**: Consistent performance

#### TX-013: Mixed Workload Performance
- Random mix of reads/writes
- Various sizes
- **Expected**: Stable performance under mixed load

### 9. Data Integrity Tests (`test_integrity.c`)

#### DI-001: Write-Disconnect-Reconnect-Verify
- Write data
- Disconnect cleanly
- Reconnect
- Read and verify
- **Expected**: Data persists across connection

#### DI-002: Write-Crash-Recover-Verify
- Write data
- Kill connection without logout
- Reconnect
- Read and verify
- **Expected**: Data committed or operation atomic

#### DI-003: Overlapping Write Test
- Multiple writes to overlapping regions
- Verify final state consistent
- **Expected**: No partial writes visible, atomicity

#### DI-004: Read After Write Consistency
- Write block
- Immediately read same block
- **Expected**: Read returns written data

#### DI-005: Deduplication Verification (if applicable)
- Write identical blocks to different LBAs
- Verify both read back correctly
- Check storage efficiency if possible
- **Expected**: Correct data, dedup works transparently

#### DI-006: Compare Multiple Reads
- Write block
- Read multiple times
- Verify all reads identical
- **Expected**: Read stability, no corruption

#### DI-007: Long-Running Stability
- Continuous I/O for extended period (hours)
- Verify no data corruption
- **Expected**: Perfect data integrity over time

#### DI-008: Power Loss Simulation (if possible)
- Heavy write workload
- Abrupt connection termination
- Reconnect and verify filesystem/data
- **Expected**: No corruption, atomic operations

## Test Execution Flow

### 1. Initialization Phase
- Parse configuration file
- Validate target parameters
- Initialize libiscsi context
- Check target reachability

### 2. Discovery Phase (if IQN not specified)
- Perform discovery
- List available targets
- Select target for testing

### 3. Test Execution
- Run tests in order by category
- For each test:
  - Setup: establish connection if needed
  - Execute: run test logic
  - Verify: check results
  - Teardown: cleanup resources
  - Record: log results

### 4. Reporting Phase
- Summarize results by category
- Generate detailed report
- Output to console and file
- Return exit code (0 = all pass, 1 = failures)

## Output Format

### Console Output (Normal Verbosity)
```
iSCSI Target Test Suite v1.0
===============================
Target: 192.168.1.100:3260
IQN: iqn.2024-12.net.example:storage.target01
LUN: 0

[Discovery Tests]
  TD-001: Basic Discovery                    [PASS]  (0.125s)
  TD-002: Discovery With Authentication      [SKIP]  (no auth configured)
  
[Login/Logout Tests]
  TL-001: Basic Login                        [PASS]  (0.342s)
  TL-002: Parameter Negotiation              [PASS]  (0.287s)
  TL-003: Invalid Parameter Values           [FAIL]  (0.156s)
    └─ Target accepted out-of-range MaxRecvDataSegmentLength
  
[I/O Operation Tests]
  TI-001: Single Block Read                  [PASS]  (0.012s)
  TI-002: Single Block Write                 [PASS]  (0.015s)
  ...

===============================
Results: 87 passed, 2 failed, 3 skipped
Duration: 45.3 seconds
```

### Detailed Report File (reports/test_report_YYYYMMDD_HHMMSS.txt)
```
iSCSI Target Test Suite - Detailed Report
==========================================
Date: 2024-12-08 15:30:45
Target: 192.168.1.100:3260
IQN: iqn.2024-12.net.example:storage.target01
Configuration: config/test_config.ini

Test Results:
-------------

[TL-003: Invalid Parameter Values] - FAIL
Duration: 0.156s
Category: Login/Logout Tests
Expected: Target should reject MaxRecvDataSegmentLength > 16777215
Actual: Target accepted value of 99999999
Details:
  - Sent login PDU with MaxRecvDataSegmentLength=99999999
  - Target response: LoginAccept with value=99999999
  - RFC 7143 Section 13.12: values > 16777215 reserved
  - Recommendation: Implement proper parameter validation

...

Summary by Category:
--------------------
Discovery Tests:       2 passed, 0 failed, 1 skipped
Login/Logout Tests:    5 passed, 1 failed, 0 skipped
Authentication Tests:  0 passed, 0 failed, 7 skipped (no auth configured)
SCSI Command Tests:    9 passed, 0 failed, 0 skipped
I/O Operation Tests:   14 passed, 1 failed, 0 skipped
...

Overall: 87 passed, 2 failed, 3 skipped
```

## Implementation Guidelines

### Error Handling
- All libiscsi calls must check return values
- Network errors should be caught and reported clearly
- Timeouts should be configurable
- Failed tests should not crash the entire suite

### Memory Management
- Clean up all allocated memory
- Free libiscsi contexts properly
- Use valgrind to verify no leaks

### Logging
- Multiple verbosity levels
- Debug mode shows all PDU exchanges
- Normal mode shows test progress
- Quiet mode shows only failures

### Test Isolation
- Each test should be independent
- Reset state between tests where possible
- Don't assume previous test passed
- Each test creates its own connection unless testing multi-connection

### Performance Considerations
- Tests should complete in reasonable time (<5 minutes for full suite)
- Large transfers should be configurable size
- Stress tests should have iteration counts in config
- Consider parallel test execution for independent tests (future)

## Building and Running

### Makefile
```make
CC = gcc
CFLAGS = -Wall -Wextra -O2 -g
LDFLAGS = -liscsi -lpthread

SRCS = $(wildcard src/*.c)
OBJS = $(SRCS:.c=.o)
TARGET = iscsi-test-suite

.PHONY: all clean test

all: $(TARGET)

$(TARGET): $(OBJS)
	$(CC) $(CFLAGS) -o $@ $^ $(LDFLAGS)

%.o: %.c
	$(CC) $(CFLAGS) -c -o $@ $<

clean:
	rm -f $(OBJS) $(TARGET)
	rm -f reports/*.txt

test: $(TARGET)
	./$(TARGET) config/test_config.ini
```

### Usage
```bash
# Run all tests
./iscsi-test-suite config/test_config.ini

# Run with verbose output
./iscsi-test-suite -v config/test_config.ini

# Run specific category
./iscsi-test-suite -c io config/test_config.ini

# Generate report only (no console output)
./iscsi-test-suite -q config/test_config.ini

# Stop on first failure
./iscsi-test-suite -f config/test_config.ini
```

## Future Enhancements

1. **Parallel Test Execution**: Run independent tests simultaneously
2. **Performance Benchmarking**: Integrated fio-like benchmarks
3. **Continuous Testing**: Watch mode for development
4. **JSON Output**: Machine-readable results for CI/CD
5. **Target Comparison**: Test multiple targets and compare
6. **Protocol Fuzzing**: Randomized PDU generation
7. **RFC Coverage Report**: Map tests to RFC sections
8. **Pluggable Backends**: Test different libiscsi versions
9. **Network Simulation**: Inject latency, packet loss
10. **Remote Execution**: Run tests from different machines

## Success Criteria

A target implementation passes if:
1. All mandatory tests pass (PASS)
2. No FAIL results in critical categories
3. SKIP results are due to configuration (not target limitations)
4. Data integrity tests show zero corruption
5. Error handling tests demonstrate graceful degradation
6. Performance tests meet reasonable benchmarks

## Delivery

Provide to Claude Code:
1. This plan document
2. Request implementation of:
   - Basic framework and test runner
   - Discovery and login tests
   - I/O operation tests
   - Data integrity tests
   - Initial error handling tests
3. Configuration file template
4. README with usage instructions
5. TESTING_GUIDE explaining how to interpret results
