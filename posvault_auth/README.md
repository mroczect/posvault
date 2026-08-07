# posvault_auth – Full API Reference

**Complete crate documentation for `posvault_auth`**  
Authentication and session management for PosVault, including multi‑factor login (password + TOTP) and role‑based access control.

---

## Table of Contents

- [Overview](#overview)
- [Public API](#public-api)
  - [Constants](#constants)
  - [Functions](#functions)
    - [`login`](#login)
    - [`require_role`](#require_role)
  - [Structs](#structs)
    - [`Session`](#session)
- [Login Flow (Detailed)](#login-flow-detailed)
- [Role Guard (`require_role`)](#role-guard-require_role)
- [Error Handling](#error-handling)
- [Examples](#examples)
  - [Successful Login](#successful-login)
  - [Login with Invalid Credentials](#login-with-invalid-credentials)
  - [Role‑based Access Control](#rolebased-access-control)
  - [Session Expiry and Refresh](#session-expiry-and-refresh)
- [Dependencies](#dependencies)
- [Integration with AccountBackend](#integration-with-accountbackend)
- [Security Considerations](#security-considerations)
- [Full Source Reference](#full-source-reference)

---

## Overview

The `posvault_auth` crate is responsible for:

- **Authenticating users** through a two‑factor process: a passphrase (to decrypt an age identity’s private key) and a time‑based one‑time password (TOTP).
- **Creating a time‑limited `Session`** that holds the user’s fingerprint and role.
- **Enforcing role‑based access control** via a guard function (`require_role`).
- Providing a **session refresh** mechanism to extend the validity of an active session.

All public items are available directly from the crate root:

```rust
use posvault_auth::{login, require_role, Session, SESSION_DURATION_SECS};
```

The crate uses the `AccountBackend` trait (from `age_credentials`) to look up user identities and encrypted keys, and relies on `posvault_handler` for its error type and core types (`Fingerprint`, `Role`).

---

## Public API

### Constants

#### `SESSION_DURATION_SECS`

```rust
pub const SESSION_DURATION_SECS: u64 = 28800;
```

Default session lifetime in seconds (8 hours). Used by `login` when creating a new session.  
Sessions created via `Session::new()` also use this default (see the private `DEFAULT_SESSION_DURATION` inside `session.rs`, which has the same value).

---

### Functions

#### `login`

```rust
pub fn login(
    backend: &dyn AccountBackend,
    email: &str,
    passphrase: &str,
    otp_code: &str,
    totp_secret_base32: &str,
) -> Result<Session>
```

Authenticates a user against the provided backend and creates a new `Session`.

##### Parameters

| Parameter            | Type                  | Description                                                                                         |
| -------------------- | --------------------- | --------------------------------------------------------------------------------------------------- |
| `backend`            | `&dyn AccountBackend` | The account backend used to look up user identities and encrypted private keys.                     |
| `email`              | `&str`                | The user’s email address (used as the account identifier).                                          |
| `passphrase`         | `&str`                | The passphrase that was used to encrypt the user’s private key (e.g., the age identity).            |
| `otp_code`           | `&str`                | The time‑based one‑time password (TOTP) provided by the user (usually 6 digits).                    |
| `totp_secret_base32` | `&str`                | The base32‑encoded TOTP secret. Must be a valid `Base32String` as defined by `libage_auth_handler`. |

##### Returns

`Result<Session>` – On success, returns a freshly created `Session` with the user’s fingerprint and role. The session will expire after `SESSION_DURATION_SECS` (8 hours).

##### Authentication Process (high‑level)

1. Look up the user’s age fingerprint by email.
2. Retrieve the user’s identity and encrypted private key.
3. Decrypt the private key using the passphrase (to prove possession of the passphrase).
4. Verify that the decrypted private key is non‑empty and at least 32 bytes.
5. Compute the TOTP for the current time window (with a one‑step drift on either side) and compare with `otp_code`.
6. Map the user’s name (from the identity) to a `Role`.
7. Build a `Session` using the fingerprint and role.

##### Errors

All errors are returned as `PosVaultError::Auth` with a descriptive message. Specific failure scenarios include:

- User not found (`user '…' not found`).
- Identity or encrypted private key missing.
- Passphrase decryption failure (`failed to decrypt private key: …`).
- Decrypted private key too short or empty.
- Invalid TOTP secret format.
- TOTP computation error.
- Invalid OTP code (`invalid OTP code`).
- Invalid fingerprint format from the identity.
- System clock error.

##### Side effects

- None beyond the creation of the `Session` object.
- No state is persisted; the session lives in memory.

##### Important Notes

- The passphrase is wrapped in `Zeroizing<String>` for memory safety.
- The TOTP verification uses a **drift window of ±1 time step** (each step is 30 seconds), so the code remains valid for a total of ~90 seconds.

---

#### `require_role`

```rust
pub fn require_role(
    session: &Session,
    allowed: &[Role],
) -> Result<()>
```

Checks that the given session is both **not expired** and that its role is one of the allowed roles.

##### Parameters

- `session` – a reference to the current `Session`.
- `allowed` – a slice of `Role` values that are permitted.

##### Returns

`Result<()>` – `Ok(())` if the session is active and its role is allowed, otherwise an `Auth` error.

##### Errors

| Condition        | Error message                                                               |
| ---------------- | --------------------------------------------------------------------------- |
| Session expired  | `"session expired"`                                                         |
| Role not allowed | `"role Admin is not allowed; required one of [Manager, Cashier]"` (example) |

##### Usage

Call this function at the entry point of protected operations.  
It is typically combined with a `Session` obtained from `login` or stored in application state.

---

### Structs

#### `Session`

```rust
#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub fingerprint: Fingerprint,
    pub role: Role,
    created_at: u64,
    expires_at: u64,
    duration: u64,
}
```

Represents an authenticated user session.

##### Fields

| Field         | Type          | Visibility | Description                                                        |
| ------------- | ------------- | ---------- | ------------------------------------------------------------------ |
| `id`          | `String`      | public     | A unique session identifier (UUID v4).                             |
| `fingerprint` | `Fingerprint` | public     | The user’s fingerprint (from `posvault_handler`).                  |
| `role`        | `Role`        | public     | The user’s role.                                                   |
| `created_at`  | `u64`         | private    | UNIX timestamp (seconds) when the session was created.             |
| `expires_at`  | `u64`         | private    | UNIX timestamp when the session will expire.                       |
| `duration`    | `u64`         | private    | The total lifetime of the session in seconds. Used by `refresh()`. |

##### Constructors

- **`Session::new(fingerprint: Fingerprint, role: Role) -> Self`**  
  Creates a new session with the default duration (8 hours). Equivalent to `with_duration(fingerprint, role, DEFAULT_SESSION_DURATION)`.

- **`Session::with_duration(fingerprint: Fingerprint, role: Role, duration_secs: u64) -> Self`**  
  Creates a session with a custom lifetime. The `expires_at` field is set to `now + duration_secs`.

##### Methods

| Method                      | Return type | Description                                                                                         |
| --------------------------- | ----------- | --------------------------------------------------------------------------------------------------- |
| `is_expired(&self) -> bool` | `bool`      | Returns `true` if the current system time is at or after `expires_at`.                              |
| `refresh(&mut self)`        | `()`        | Resets `created_at` to now and `expires_at` to `now + duration`, effectively extending the session. |
| `created_at(&self) -> u64`  | `u64`       | Returns the UNIX timestamp when the session was originally created.                                 |
| `expires_at(&self) -> u64`  | `u64`       | Returns the UNIX timestamp when the session will expire.                                            |

##### Example

```rust
let session = Session::new(fingerprint, Role::Admin);
assert!(!session.is_expired());

// Later...
if session.is_expired() {
    // force re‑login
}
```

---

## Login Flow (Detailed)

The `login` function performs the following steps:

1. **Find user by email**  
   Calls `backend.find_by_email(email)`. If no user is found, returns `Auth("user '...' not found")`.

2. **Load identity**  
   Calls `backend.load_identity(&fingerprint)`. The identity contains the user’s display name and fingerprint.

3. **Load encrypted private key**  
   Calls `backend.load_encrypted_private_key(&fingerprint)`. This key is the user’s age identity, encrypted with a passphrase.

4. **Decrypt private key**  
   The passphrase is wrapped in `Zeroizing` (for memory safety) and passed to `crypto::decrypt_with_passphrase`. If decryption fails, an `Auth` error is returned.

5. **Validate private key**  
   The decrypted bytes must be non‑empty and at least 32 bytes, otherwise the login is rejected.

6. **TOTP verification**
   - The `totp_secret_base32` is parsed into a `Base32String` and then converted to a secret.
   - The current time (in seconds) is obtained.
   - For each time offset in `[-1, 0, 1]` (drift steps), the TOTP token is computed using `algorithms::compute_totp_at`.
   - If any computed token matches `otp_code`, the OTP is considered valid.
   - If none match, the function returns `Auth("invalid OTP code")`.

7. **Role mapping**  
   The user’s name (from `age_identity.user_id.name`) is mapped to a `Role` using a simple mapping function (`map_name_to_role`). Unknown names become `Role::Custom(name)`.

8. **Fingerprint conversion**  
   The age fingerprint (a hex string) is converted into a `posvault_handler::types::Fingerprint` using `Fingerprint::new`.

9. **Session creation**  
   A new `Session` is returned with the fingerprint, role, and a duration of `SESSION_DURATION_SECS` (8 hours).

---

## Role Guard (`require_role`)

The `require_role` function implements simple RBAC:

- If the session has expired (`session.is_expired()`), it returns an `Auth` error immediately.
- Otherwise, it checks whether `session.role` is present in the `allowed` slice.
- If not, it returns an `Auth` error indicating the required roles.

**Example:**

```rust
require_role(&session, &[Role::Admin, Role::Manager])?;
// Proceed with admin or manager operation...
```

---

## Error Handling

All functions return `Result<(), PosVaultError>` (or `Result<Session, PosVaultError>`). The `PosVaultError::Auth` variant is used for all authentication‑related failures.

Below is a summary of error strings thrown by `login` and `require_role`:

| Error source                      | Message pattern                                                   |
| --------------------------------- | ----------------------------------------------------------------- |
| User not found                    | `"user '<email>' not found"`                                      |
| Identity not found                | `"identity not found"`                                            |
| Encrypted key not found           | `"encrypted private key not found"`                               |
| Decryption failed                 | `"failed to decrypt private key: ..."`                            |
| Private key empty                 | `"decrypted private key is empty"`                                |
| Private key too short             | `"private key too short"`                                         |
| Invalid TOTP secret               | `"invalid TOTP secret: ..."`                                      |
| System clock error                | `"system clock error"`                                            |
| TOTP computation failed           | `"TOTP computation failed: ..."`                                  |
| Invalid OTP                       | `"invalid OTP code"`                                              |
| Fingerprint construction failure  | (wrapped from `Fingerprint::new`)                                 |
| Session expired (`require_role`)  | `"session expired"`                                               |
| Role not allowed (`require_role`) | `"role Admin is not allowed; required one of [Manager, Cashier]"` |

All these errors are returned as `PosVaultError::Auth`.

---

## Examples

### Successful Login

```rust
use posvault_auth::login;
use age_credentials::backend::traits::AccountBackend;
// Assume we have a concrete backend, e.g., a file‑based one
let backend = MyAccountBackend::new();
let session = login(
    &backend,
    "admin@example.com",
    "strong-passphrase",
    "123456",                // valid TOTP code
    "JBSWY3DPEHPK3PXP"        // Base32 secret
).expect("login should succeed");
assert_eq!(session.role, Role::Admin);
```

### Login with Invalid Credentials

```rust
let result = login(
    &backend,
    "admin@example.com",
    "wrong-passphrase",
    "123456",
    "JBSWY3DPEHPK3PXP"
);
assert!(result.is_err());
match result.unwrap_err() {
    PosVaultError::Auth(msg) => println!("Login failed: {}", msg),
    _ => unreachable!(),
}
```

### Role‑based Access Control

```rust
fn delete_user(session: &Session, user_id: &str) -> Result<()> {
    require_role(session, &[Role::Admin])?;
    // perform deletion...
    Ok(())
}
```

### Session Expiry and Refresh

```rust
let mut session = Session::new(fingerprint, Role::Cashier);
// ... some time passes
if session.is_expired() {
    // instead of re‑logging in, we could refresh if allowed by policy
    session.refresh();
    assert!(!session.is_expired());
}
```

---

## Dependencies

This crate depends on:

- `posvault_handler` – for `Fingerprint`, `Role`, `PosVaultError`, `Result`.
- `age_credentials` – provides `AccountBackend` trait and `crypto::decrypt_with_passphrase`.
- `libage_auth_handler` – for `Base32String`, `TimeStep`, `Digits`, `Algo`.
- `libage_otp` – for `algorithms::compute_totp_at`.
- `zeroize` – for `Zeroizing` the passphrase.
- `uuid` – for generating session IDs.

---

## Integration with `AccountBackend`

The `login` function is generic over any type that implements `AccountBackend`. The backend must provide:

- `find_by_email(&self, email: &str) -> AccountResult<Option<Fingerprint>>`
- `load_identity(&self, fingerprint: &Fingerprint) -> AccountResult<Option<Identity>>`
- `load_encrypted_private_key(&self, fingerprint: &Fingerprint) -> AccountResult<Option<Zeroizing<Vec<u8>>>>`

The `Identity` type used here is from `age_credentials`, not `posvault_handler::Identity`. Its fields include `fingerprint` and `user_id` (which has a `name` field used to derive the `Role`).

This design decouples authentication from the storage backend, allowing pluggable implementations (file‑based, database, in‑memory).

---

## Security Considerations

- **Passphrase handling**: The passphrase is wrapped in `Zeroizing` and its memory is cleared after use. The decrypted private key is also held in a `Zeroizing` context within the backend call.
- **TOTP drift**: By allowing a time step drift of ±1, the system tolerates slight clock skew while still being secure.
- **Session expiration**: Default session length is 8 hours. For sensitive operations, `require_role` should be called on every request, and the session should be checked for expiry.
- **Role mapping**: Role derivation from user name is a simple matching; in production, a more robust mapping (e.g., from a database) should be used, but the `AccountBackend` approach already encapsulates user data, so the name‑to‑role logic could be overridden by a custom `map_name_to_role` or by using `Role::Custom`.

---

## Full Source Reference

The complete public API source is contained in `src/` and re‑exported via `lib.rs`. Below is a condensed version for reference:

```rust
// lib.rs
pub mod guard;
pub mod login;
pub mod session;

pub use guard::*;
pub use login::*;
pub use session::*;

// session.rs
use posvault_handler::types::{Fingerprint, Role};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

const DEFAULT_SESSION_DURATION: u64 = 8 * 3600;

#[derive(Debug, Clone)]
pub struct Session { /* ... */ }
impl Session {
    pub fn new(fingerprint: Fingerprint, role: Role) -> Self;
    pub fn with_duration(fingerprint: Fingerprint, role: Role, duration_secs: u64) -> Self;
    pub fn is_expired(&self) -> bool;
    pub fn refresh(&mut self);
    pub fn created_at(&self) -> u64;
    pub fn expires_at(&self) -> u64;
}

// login.rs
pub const SESSION_DURATION_SECS: u64 = 28800;

pub fn login(
    backend: &dyn AccountBackend,
    email: &str,
    passphrase: &str,
    otp_code: &str,
    totp_secret_base32: &str,
) -> Result<Session>;

// guard.rs
pub fn require_role(session: &Session, allowed: &[Role]) -> Result<()>;
```

For more details, consult the source files linked in the crate’s repository.

---

_End of `posvault_auth` API Reference._
