# Error Message Improvements - Client & Server

## Current State vs Improved State

### CLIENT-SIDE IMPROVEMENTS (Implemented)

#### Before: Silent Failures
```rust
if client.login(initiator_iqn(), target_iqn()).is_ok() {
    // Test runs
}
// Test passes even if login failed!
```

#### After: Actionable Errors
```rust
login_to_target(&mut client);
// On failure:
// Failed to login to target
// Error: Protocol error: Login failed: class=0x02, detail=0x03
//
// Configuration:
// - Portal: 127.0.0.1:3260
// - Target IQN: iqn.2025-12.local:storage.memory-disk
// - Initiator IQN: iqn.2025-12.local:initiator
//
// Troubleshooting:
// 1. Verify target IQN is correct - run discovery:
//    cargo run --example discover_targets -- 127.0.0.1:3260
//
// 2. Check target accepts this initiator IQN
//
// 3. For TGTD, check ACL settings:
//    sudo tgtadm --mode target --op show
```

### SERVER-SIDE IMPROVEMENTS (Needed)

#### Current: Generic Errors
```rust
// Server returns:
login_status::INITIATOR_ERROR, 0x03  // "Target not found"
```

Client sees:
```
Error: Protocol error: Login failed: class=0x02, detail=0x03
```

#### Proposed: Specific Errors with Context

The iSCSI protocol provides detailed status codes:
- AUTH_FAILURE (0x0201) - authentication failed
- AUTHORIZATION_FAILURE (0x0202) - ACL/permissions issue
- TARGET_NOT_FOUND (0x0203) - IQN doesn't exist
- MISSING_PARAMETER (0x0207) - required parameter missing
- SESSION_TYPE_NOT_SUPPORTED (0x0209) - discovery/normal session type issue

**Server should return appropriate code:**
```rust
// Wrong IQN
return login_error(login_status::TARGET_NOT_FOUND);

// ACL denies this initiator
return login_error(login_status::AUTHORIZATION_FAILURE);

// Missing InitiatorName parameter
return login_error(login_status::MISSING_PARAMETER);
```

**Client should decode these:**
```rust
match status_detail {
    login_status::TARGET_NOT_FOUND => {
        "Target IQN not found on this portal.\n\
         \n\
         The target doesn't have the IQN you specified.\n\
         \n\
         Fix:\n\
         1. Run discovery to see available targets:\n\
            cargo run --example discover_targets -- {portal}\n\
         \n\
         2. Update test-config.toml with correct IQN"
    }
    login_status::AUTHORIZATION_FAILURE => {
        "Initiator not authorized to access this target.\n\
         \n\
         The target has ACL restrictions.\n\
         \n\
         Fix:\n\
         1. For TGTD, add initiator to ACL:\n\
            sudo tgtadm --lld iscsi --op bind --mode target \\\n\
              --tid 1 -I {initiator_iqn}\n\
         \n\
         2. For Rust target, check auth_config settings"
    }
    // etc.
}
```

## Examples of Helpful Error Messages

### 1. Connection Failure
**Bad:**
```
Error: Connection refused (os error 111)
```

**Good:**
```
Failed to connect to iSCSI target at 127.0.0.1:3260
Error: Connection refused (os error 111)

Troubleshooting:
1. Check if target is running:
   lsof -i:3260 (Linux)
   netstat -an | grep 3260 (Windows)

2. For Rust target, start it with:
   cargo run --example simple_target -- 0.0.0.0:3260

3. For TGTD, check with:
   sudo tgtadm --mode target --op show

4. Verify test-config.toml has correct portal address
```

### 2. Discovery Mismatch
**Bad:**
```
Assertion failed: !targets.is_empty()
```

**Good:**
```
Expected target 'iqn.2025-12.local:storage.memory-disk' not found in discovery results

Discovered targets:
  - iqn.2025-12.lan.home:debian-live at 127.0.0.1:3260

This means:
1. test-config.toml 'iqn' doesn't match actual target IQN
2. Target is advertising different IQN than configured

Fix by updating test-config.toml with correct IQN from discovery
```

### 3. SCSI Command Failure
**Bad:**
```
Error: I/O error
```

**Good:**
```
INQUIRY command failed
Error: I/O error: Resource temporarily unavailable (os error 11)
CDB: [12, 00, 00, 00, ff, 00]

Troubleshooting:
1. Check if logged in to target
2. Verify target supports this SCSI command
3. Check LUN is valid (current: 0)
4. Review target logs for detailed error
```

## Implementation Checklist

### Client-Side (Partially Done)
- [x] Helper functions with actionable errors
- [x] Connection error with troubleshooting steps
- [x] Login error with config display and troubleshooting
- [x] SCSI command errors with CDB display
- [x] Discovery errors with manual command suggestions
- [ ] Decode all iSCSI login status codes
- [ ] Decode SCSI sense codes with meaning
- [ ] Add "Common Causes" section for each error type

### Server-Side (To Do)
- [ ] Return specific login status codes (not just TARGET_NOT_FOUND)
- [ ] Add detailed error logging with context
- [ ] Validate all required login parameters
- [ ] Check ACLs and return AUTHORIZATION_FAILURE when appropriate
- [ ] Detect authentication issues and return AUTH_FAILURE
- [ ] Log WHY login failed (for admin debugging)

### Protocol-Level
- [ ] Map RFC 3720 status codes to human-readable messages
- [ ] Map SCSI sense codes to explanations
- [ ] Provide "Expected vs Actual" comparisons
- [ ] Suggest concrete fix commands when possible
