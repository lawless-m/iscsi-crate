# iSCSI Target Test Suite

This directory contains a comprehensive C-based test suite for validating iSCSI target implementations using libiscsi.

## Quick Start

### 1. Install Dependencies

**Debian/Ubuntu:**
```bash
sudo apt-get install libiscsi-dev build-essential
```

**Fedora/RHEL:**
```bash
sudo dnf install libiscsi-devel gcc make
```

### 2. Build the Test Suite

```bash
cd iscsi-test-suite
make
```

### 3. Configure Your Target

Edit `config/test_config.ini` with your target details:
```ini
[target]
portal = 127.0.0.1:3260
iqn = iqn.2024-12.net.example:storage.target01
lun = 0
```

### 4. Run Tests

```bash
# Run all tests
./iscsi-test-suite config/test_config.ini

# Run specific category
./iscsi-test-suite -c io config/test_config.ini

# Verbose mode
./iscsi-test-suite -v config/test_config.ini
```

## Documentation

- **[README.md](iscsi-test-suite/README.md)** - Complete user documentation
- **[TESTING_GUIDE.md](iscsi-test-suite/TESTING_GUIDE.md)** - Guide to interpreting test results
- **[iscsi-test-plan.md](iscsi-test-suite/iscsi-test-plan.md)** - Detailed implementation plan
- **[CONTENTS.md](iscsi-test-suite/CONTENTS.md)** - Package contents and navigation

## What's Implemented

The test suite currently includes:

### ✅ Implemented Test Categories
1. **Discovery Tests** (4 tests) - Target discovery and enumeration
2. **Login/Logout Tests** (6 tests) - Session establishment and parameters
3. **SCSI Command Tests** (9 tests) - Basic SCSI commands (INQUIRY, READ CAPACITY, etc.)
4. **I/O Operation Tests** (14 tests) - Read/write operations with data integrity verification

### 🚧 Planned Test Categories
5. **Authentication Tests** (7 tests) - CHAP and mutual CHAP
6. **Multi-Connection Tests** (6 tests) - Multiple connections per session
7. **Error Handling Tests** (13 tests) - Network failures and error recovery
8. **Edge Cases** (13 tests) - Boundary conditions and stress tests
9. **Data Integrity Tests** (8 tests) - Long-running stability and crash recovery

## Current Status

The test suite framework and core tests are functional. You can:
- Run basic discovery and login tests
- Execute SCSI command tests
- Perform critical I/O data integrity tests
- Generate detailed test reports

**Total tests:** 33 tests implemented (with placeholders for 59 more)
**Focus:** P0 (must-pass) tests for basic functionality and data integrity

## Testing Your Rust iSCSI Target

To test the Rust iSCSI target in this repository:

1. Build and start your Rust target
2. Update `config/test_config.ini` with the portal address and IQN
3. Run the tests:
   ```bash
   ./iscsi-test-suite config/test_config.ini
   ```

## Exit Codes

- `0` - All tests passed
- `1` - One or more tests failed
- `2` - Configuration or setup error

## Support

For detailed information on:
- Using the test suite: See `iscsi-test-suite/README.md`
- Interpreting failures: See `iscsi-test-suite/TESTING_GUIDE.md`
- Implementation details: See `iscsi-test-suite/iscsi-test-plan.md`

## License

[Specify license]
