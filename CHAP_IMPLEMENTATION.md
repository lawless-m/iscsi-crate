# CHAP Authentication Implementation

## Overview
Implementing CHAP (Challenge-Handshake Authentication Protocol) for Microsoft Windows certification compliance and production security requirements.

## Status: IN PROGRESS

### Completed ✅

#### 1. Auth Module (`src/auth.rs`)
- **ChapAlgorithm**: MD5 algorithm support (identifier 5)
- **ChapCredentials**: Username/secret storage
- **AuthConfig**: Three modes:
  - `None`: No authentication (current default)
  - `Chap`: One-way authentication (initiator → target)
  - `MutualChap`: Two-way authentication (both directions)
- **ChapAuthState**: Challenge generation and response validation
  - Random identifier generation
  - Random challenge generation (16 bytes)
  - MD5 response calculation: `MD5(identifier + secret + challenge)`
  - Constant-time comparison (timing attack prevention)
- **Helper functions**: Hex encoding/decoding
- **Tests**: Full test coverage

#### 2. Dependencies
```toml
md5 = "0.7"      # CHAP response calculation
rand = "0.8"     # Challenge generation
hex = "0.4"      # Hex encoding/decoding
```

#### 3. Error Handling
- Added `IscsiError::Auth(String)` variant

### In Progress 🔄

#### 3. Session Integration (`src/session.rs`)
Need to add:
- `auth_config: AuthConfig` field to `IscsiSession`
- `chap_state: Option<ChapAuthState>` field
- CHAP parameter negotiation in login phase

### Remaining Tasks 📋

#### 4. Session CHAP Logic
**Security Negotiation Phase (CSG=0):**

1. **Initiator sends** (in login request):
   ```
   AuthMethod=CHAP
   ```

2. **Target responds** with:
   ```
   AuthMethod=CHAP
   CHAP_A=5              # MD5 algorithm
   CHAP_I=<identifier>   # Random byte (as string)
   CHAP_C=<challenge>    # Random bytes (hex encoded)
   ```

3. **Initiator responds** with:
   ```
   CHAP_N=<username>     # Username
   CHAP_R=<response>     # MD5(id + secret + challenge) in hex
   ```

4. **Target validates**:
   - Lookup username credentials
   - Calculate expected response
   - Compare with provided response
   - If valid: proceed to operational negotiation
   - If invalid: send login reject with `AUTH_FAILURE` (0x0201)

**Mutual CHAP (Optional):**

5. **Initiator sends** (during security negotiation):
   ```
   CHAP_I=<initiator_id>
   CHAP_C=<initiator_challenge>
   ```

6. **Target responds** with:
   ```
   CHAP_N=<target_username>
   CHAP_R=<target_response>
   ```

7. **Initiator validates** target response

#### 5. Target Integration (`src/target.rs`)

```rust
// IscsiTargetBuilder needs to accept AuthConfig
pub fn with_auth(mut self, auth: AuthConfig) -> Self {
    self.auth_config = Some(auth);
    self
}

// Pass to session during connection handling
```

#### 6. Example Updates

**Create `examples/chap_target.rs`:**
```rust
use iscsi_target::{AuthConfig, ChapCredentials, IscsiTarget};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let auth = AuthConfig::Chap {
        credentials: ChapCredentials::new("iscsi-user", "secretpassword"),
    };

    let target = IscsiTarget::builder()
        .bind_addr("0.0.0.0:3260")
        .target_name("iqn.2025-12.local:storage.secure-disk")
        .with_auth(auth)
        .build(storage)?;

    target.run()?;
    Ok(())
}
```

**Mutual CHAP example:**
```rust
let auth = AuthConfig::MutualChap {
    target_credentials: ChapCredentials::new("target-user", "target-secret"),
    initiator_credentials: ChapCredentials::new("initiator-user", "initiator-secret"),
};
```

#### 7. Testing Plan

**Linux (open-iscsi):**
```bash
# Configure CHAP in /etc/iscsi/iscsid.conf
node.session.auth.authmethod = CHAP
node.session.auth.username = iscsi-user
node.session.auth.password = secretpassword

# Discover and login
sudo iscsiadm -m discovery -t sendtargets -p 127.0.0.1:3260
sudo iscsiadm -m node -T iqn.2025-12.local:storage.secure-disk -p 127.0.0.1:3260 --login
```

**Windows (Microsoft iSCSI Initiator):**
1. Open iSCSI Initiator Control Panel
2. Discovery tab → Add target portal
3. Targets tab → Select target → Connect
4. Advanced Settings → Enable CHAP login
5. Enter username and password
6. Verify connection succeeds

**Mutual CHAP (Windows):**
- Advanced Settings → Enable mutual CHAP
- Enter target secret for reverse authentication

#### 8. Security Considerations

**Implemented:**
- ✅ Constant-time comparison (prevents timing attacks)
- ✅ Random challenge generation
- ✅ Per-session identifiers

**Additional recommendations:**
- Use strong secrets (>12 characters)
- Consider IPsec for encryption (CHAP only authenticates, doesn't encrypt)
- Regular credential rotation
- Audit logging for failed authentication attempts

#### 9. Microsoft Certification Requirements

**Required for Windows Server/Hyper-V certification:**
- ✅ CHAP authentication support
- ✅ MD5 algorithm (CHAP_A=5)
- ⏳ Mutual CHAP (recommended but not required)
- ⏳ Integration testing with Windows Initiator
- ⏳ Multiple concurrent authenticated sessions

**RFC Compliance:**
- RFC 3720: iSCSI Protocol
- RFC 1994: PPP Challenge Handshake Authentication Protocol (CHAP)
- RFC 2865: RADIUS (for MD5 algorithm reference)

## Implementation Files

### Modified Files
- `src/lib.rs`: Export auth module
- `src/error.rs`: Add `Auth` error variant
- `Cargo.toml`: Add dependencies

### New Files
- `src/auth.rs`: Complete CHAP implementation
- `examples/chap_target.rs`: (pending) CHAP example
- `CHAP_IMPLEMENTATION.md`: This file

### Files to Modify
- `src/session.rs`: Add CHAP negotiation logic
- `src/target.rs`: Pass AuthConfig to sessions
- `examples/simple_target.rs`: Update with optional auth

## Next Steps

1. Add `AuthConfig` field to `IscsiSession`
2. Implement CHAP parameter exchange in `handle_login`
3. Add authentication validation before Full Feature Phase
4. Update `IscsiTargetBuilder` with `with_auth()` method
5. Create CHAP examples
6. Test with Linux and Windows initiators
7. Document configuration in README.md

## Notes

- Current implementation defaults to `AuthMethod=None` for backward compatibility
- CHAP is required for Windows/Hyper-V production deployments
- Mutual CHAP provides bidirectional authentication
- Consider adding IPsec support for encryption (CHAP only authenticates)
