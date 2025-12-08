# iSCSI Test Suite Package Contents

This package contains the complete specification and documentation for building a comprehensive iSCSI target test suite using libiscsi.

## Files in This Package

### 1. iscsi-test-plan.md (START HERE)
**Purpose:** Complete implementation plan for Claude Code  
**What it contains:**
- Detailed project architecture
- All test categories with specific test cases (92 individual tests)
- Test execution flow and framework design
- Output format specifications
- Configuration file structure
- Build system design
- Success criteria and future enhancements

**How to use:**
This is the master document. Give this to Claude Code to implement the test suite. It contains everything needed to build the complete testing framework.

### 2. README.md
**Purpose:** User documentation for the finished test suite  
**What it contains:**
- Installation instructions
- Configuration guide
- Usage examples
- Troubleshooting common issues
- CI/CD integration examples
- Command-line options reference

**How to use:**
This becomes the README.md file in the final project. It's what users will read to understand how to run the tests.

### 3. TESTING_GUIDE.md
**Purpose:** Deep dive into test interpretation and validation  
**What it contains:**
- Detailed explanation of each test category
- How to interpret different failure types
- Priority classification (P0/P1/P2)
- Common bug patterns found by tests
- Production readiness checklist
- Testing strategy recommendations

**How to use:**
Read this to understand what tests actually validate and how to interpret results. Essential for knowing whether your target is production-ready.

### 4. CONTENTS.md (this file)
**Purpose:** Navigation guide for this package  
**What it contains:**
- File descriptions
- Getting started instructions
- Development workflow

## Quick Start Guide

### For Implementation (Using Claude Code)

1. **Provide the plan to Claude Code:**
   ```
   I need you to implement an iSCSI target test suite based on this plan.
   [Attach iscsi-test-plan.md]
   ```

2. **Start with basic framework:**
   - Ask Claude Code to begin with the test framework and runner
   - Then add discovery and login tests
   - Then I/O and data integrity tests
   - Finally error handling and edge cases

3. **Iterative development:**
   - Test each category as it's implemented
   - Point at a known-good target (like LIO) first
   - Then test against your Rust iSCSI target

### For Testing Your Target

Once the test suite is built:

1. **Configure your target:**
   - Edit `config/test_config.ini`
   - Set portal address, IQN, credentials

2. **Run basic tests first:**
   ```bash
   ./iscsi-test-suite -c discovery,login config/test_config.ini
   ```

3. **Run data integrity tests:**
   ```bash
   ./iscsi-test-suite -c io,integrity config/test_config.ini
   ```

4. **Run full suite:**
   ```bash
   ./iscsi-test-suite config/test_config.ini
   ```

5. **Review results:**
   - Check console output for failures
   - Read detailed report in `reports/` directory
   - Consult TESTING_GUIDE.md for interpretation

## Project Structure (After Implementation)

```
iscsi-test-suite/
├── README.md                    # User documentation (from this package)
├── TESTING_GUIDE.md            # Test interpretation guide (from this package)
├── Makefile                    # Build system
├── config/
│   └── test_config.ini        # Test configuration
├── src/
│   ├── main.c                 # Entry point
│   ├── test_framework.c/h     # Core test infrastructure
│   ├── test_discovery.c/h     # Discovery and login tests
│   ├── test_auth.c/h         # Authentication tests
│   ├── test_commands.c/h     # SCSI command tests
│   ├── test_io.c/h           # I/O operation tests
│   ├── test_multiconn.c/h    # Multi-connection tests
│   ├── test_error.c/h        # Error handling tests
│   ├── test_edge_cases.c/h   # Edge cases and stress tests
│   ├── test_integrity.c/h    # Data integrity tests
│   └── utils.c/h             # Helper functions
└── reports/
    └── (generated test reports)
```

## Development Workflow

### Phase 1: Basic Framework (Day 1)
- Implement test framework and runner
- Configuration file parsing
- Basic discovery and login tests
- Get first tests running against any target

### Phase 2: Core Testing (Days 2-3)
- SCSI command tests
- Basic I/O operations (read/write/verify)
- Data integrity tests
- Authentication tests

### Phase 3: Advanced Testing (Days 4-5)
- Multi-connection tests
- Error handling tests
- Edge cases and stress tests
- Long-running stability tests

### Phase 4: Polish (Day 6)
- Detailed reporting
- Performance measurement
- Documentation
- Bug fixes from testing

## Test Coverage Summary

The test suite includes **92 individual tests** across 9 categories:

1. **Discovery Tests** (4 tests)
   - Basic discovery, authentication, redirection

2. **Login/Logout Tests** (6 tests)
   - Session establishment, parameter negotiation, timeouts

3. **Authentication Tests** (7 tests)
   - CHAP, mutual CHAP, failure cases

4. **SCSI Command Tests** (9 tests)
   - INQUIRY, capacity, mode sense, error conditions

5. **I/O Operation Tests** (14 tests)
   - Single/multi-block, sequential/random, patterns, large transfers

6. **Multi-Connection Tests** (6 tests)
   - Multiple connections per session, concurrent I/O

7. **Error Handling Tests** (13 tests)
   - Disconnects, timeouts, corrupted PDUs, task management

8. **Edge Cases** (13 tests)
   - Boundaries, stress tests, patterns, sustained workload

9. **Data Integrity Tests** (8 tests)
   - Persistence, crash recovery, long-running stability

## Priority Tests to Implement First

If time is limited, focus on these critical tests first:

**Must Have (P0):**
1. TL-001: Basic Login
2. TC-001: INQUIRY
3. TC-003: READ CAPACITY
4. TI-001: Single Block Read
5. TI-002: Single Block Write
6. TI-013: Write-Read-Verify Pattern
7. DI-001: Write-Disconnect-Reconnect-Verify
8. DI-006: Compare Multiple Reads

**Should Have (P1):**
1. TL-002: Parameter Negotiation
2. TE-001: Network Disconnect During I/O
3. TX-002/TX-003: Boundary LBA Tests
4. DI-007: Long-Running Stability

## What Makes This Test Suite Different

### Compared to Manual Testing:
- Automated and repeatable
- Comprehensive coverage (92 tests vs ad-hoc)
- Catches edge cases humans miss
- Consistent interpretation of RFC

### Compared to Microsoft HLK:
- Much simpler to setup (no Windows Server infrastructure)
- Faster to run (minutes vs hours)
- Open source and modifiable
- Target-agnostic
- Focused on iSCSI, not Windows certification

### Compared to Basic fio Testing:
- Protocol-level validation (not just I/O)
- Tests discovery, login, authentication
- Error handling and edge cases
- Data integrity across failures
- RFC compliance verification

## Important Notes

### About the Rust iSCSI Target Being Tested:
- **You didn't write or read the implementation** - This test suite provides the validation you need
- **Content-addressed storage** - Some tests (DI-005) specifically validate deduplication correctness
- **Black-box approach** - Tests don't assume anything about internal implementation

### About Test Independence:
- Tests are written in C using libiscsi
- Completely independent of Rust target implementation
- Tests work against any iSCSI target (LIO, TGT, custom)
- No coupling between test code and target code

### About Confidence Building:
Once all P0 tests pass:
- You have high confidence in data integrity
- Basic protocol compliance verified
- Safe to use for your Linux boot use case
- Can release Rust crate with validation evidence

## Next Steps

1. **Immediate:** Give `iscsi-test-plan.md` to Claude Code for implementation

2. **As tests are built:** Run against known-good target (LIO) to validate tests themselves

3. **Once working:** Run against your Rust iSCSI target

4. **Fix failures:** Iterate with Claude Code on the Rust implementation

5. **Document results:** Include test results in your crate's documentation

6. **Optional:** Submit to HLK for official Microsoft certification (separate process)

## Support During Development

### If Implementation Issues Arise:
- Refer back to `iscsi-test-plan.md` for architecture details
- Check libiscsi documentation and examples
- Start simple (framework + discovery) before adding complexity

### If Tests Fail:
- Consult `TESTING_GUIDE.md` for interpretation
- Use verbose mode (`-v`) to see detailed protocol traces
- Compare behavior with known-good target
- Focus on P0 failures first (data integrity)

### If You're Unsure:
- Test against LIO first to validate test suite itself
- Run subset of tests with `-c` flag
- Start with non-destructive tests (discovery, INQUIRY)
- Add I/O tests once confidence builds

## Conclusion

This package provides everything needed to build a professional-grade iSCSI test suite. The implementation plan is detailed enough for Claude Code to execute, and the supporting documentation explains how to interpret results and validate your target.

**Your goal:** Validate your Rust iSCSI target is production-ready without having to manually test it or read the RFC in detail.

**This package delivers:** Automated, comprehensive testing that gives you confidence to release your crate with evidence of correctness.

Good luck with the implementation and testing!
