# posvault – Full API Reference

**Complete documentation for the `posvault` crate**  
Umbrella crate that re‑exports the entire PosVault ecosystem and provides a high‑level, integrated `PosVault` façade for common operations.

---

## Table of Contents

- [Overview](#overview)
- [Crate Re‑exports](#crate-re-exports)
- [Core Type: `PosVault`](#core-type-posvault)
  - [`PosVault::open`](#posvaultopen)
  - [`PosVault::set_recipients`](#posvaultset_recipients)
  - [`PosVault::set_signer`](#posvaultset_signer)
  - [`PosVault::login`](#posvaultlogin)
  - [`PosVault::session`](#posvaultsession)
  - [`PosVault::transact`](#posvaulttransact)
  - [`PosVault::journal`](#posvaultjournal)
  - [`PosVault::query_engine`](#posvaultquery_engine)
  - [`PosVault::decrypt_payload`](#posvaultdecrypt_payload)
  - [`PosVault::sync_to_remote`](#posvaultsync_to_remote)
- [CombinedStore](#combinedstore)
  - [Struct Definition](#combinedstore-definition)
  - [Trait Implementations](#trait-implementations-for-combinedstore)
- [Error Handling](#error-handling)
- [Examples](#examples)
  - [Opening a Vault and Performing Operations](#opening-a-vault-and-performing-operations)
  - [Using the Re‑exported Items Directly](#using-the-re-exported-items-directly)
- [Dependencies](#dependencies)
- [Full Source Reference](#full-source-reference)

---

## Overview

The `posvault` crate is the **top‑level entry point** for the PosVault event‑sourced application framework. It brings together all underlying crates (`posvault_handler`, `posvault_auth`, `posvault_crypto`, `posvault_query`, `posvault_sign`, `posvault_store`, `posvault_sync`) and adds a **unified façade** – the `PosVault` struct – that manages:

- A local repository (`posvault_store::PosVault` as `Store`)
- A user session (via `posvault_auth`)
- Age encryption recipients for automatic payload encryption
- An Ed25519 signer for automatic event/journal signing

The crate also defines a `CombinedStore` that combines `VctrlEventStore` and `VctrlSnapshotStore` into a single type that implements both `EventStore` and `SnapshotStore`, which is required by the `QueryEngine`.

All types, traits, functions, and constants from the sub‑crates are **re‑exported** so that you can write `use posvault::*;` and have everything you need in scope.

---

## Crate Re‑exports

The `lib.rs` file re‑exports the following public items from each sub‑crate:

| Re‑exported Item(s)                                                                                               | Source Crate       | Description                                                           |
| ----------------------------------------------------------------------------------------------------------------- | ------------------ | --------------------------------------------------------------------- |
| `Session`, `login`, `require_role`                                                                                | `posvault_auth`    | Authentication and role‑based access                                  |
| `decrypt_event`, `encrypt_event`                                                                                  | `posvault_crypto`  | Age encryption/decryption of event payloads                           |
| `constants`, `enums`, `errors::PosVaultError`, `errors::Result`, `traits`, `types`                                | `posvault_handler` | Core types, errors, traits, and constants                             |
| `QueryEngine`, `query_examples`                                                                                   | `posvault_query`   | Event‑sourced query engine and example stock functions                |
| `Ed25519Signer`, `generate_keypair`, `SignedJournal`, `SignedEventStore`                                          | `posvault_sign`    | Signing wrappers for journals and event stores                        |
| `VctrlEventStore`, `VctrlJournal`, `VctrlSnapshotStore`                                                           | `posvault_store`   | Concrete file‑store implementations                                   |
| `FileTransport`, `UnionCsvResolver`, `checkout_branch`, `create_store_branch`, `current_branch`, `pull_and_merge` | `posvault_sync`    | Branch management, conflict resolution, file transport, and pull stub |
| `PosVault` (façade)                                                                                               | _this crate_       | High‑level integrated vault                                           |

Thus, importing `posvault::*` gives you access to every public symbol in the entire PosVault system.

---

## Core Type: `PosVault`

```rust
pub struct PosVault {
    pub store: Store,                  // the underlying file‑store (posvault_store::PosVault)
    pub path: PathBuf,                 // repository path
    session: Option<Session>,          // current authenticated session
    recipients: Vec<String>,           // age public keys for encryption
    signer: Option<Ed25519Signer>,    // optional signing key
}
```

The central façade that holds a repository, an optional authenticated session, a set of encryption recipients, and an optional signer. It provides methods for login, event transactions, journal recording, querying, and syncing.

All methods except `open` return `Result` (from `posvault_handler::errors`).

---

### `PosVault::open`

```rust
pub fn open(path: impl AsRef<Path>) -> Result<Self>
```

Creates or opens a PosVault repository at the given path. Delegates to `posvault_store::PosVault::open`. The returned `PosVault` starts with no session, an empty recipient list, and no signer.

- **Parameters**:
  - `path` – directory where the repository lives (will be created if missing).
- **Returns**: a new `PosVault` instance with `session: None`, `recipients: vec![]`, `signer: None`.
- **Errors**: `Storage` if directory creation or store opening fails.

---

### `PosVault::set_recipients`

```rust
pub fn set_recipients(&mut self, recipients: Vec<String>)
```

Sets the list of age public keys that will be used to encrypt event payloads during [`transact`](#posvaulttransact). If empty, encryption is **skipped**.

- **Parameters**:
  - `recipients` – vector of age public key strings. Should be valid age recipients (validated later during encryption).

---

### `PosVault::set_signer`

```rust
pub fn set_signer(&mut self, signer: Ed25519Signer)
```

Sets the Ed25519 signer that will be used to sign events and journal entries during [`transact`](#posvaulttransact) and [`journal`](#posvaultjournal). If `None`, events and entries are **not signed**.

- **Parameters**:
  - `signer` – an `Ed25519Signer` that contains a signing key.

---

### `PosVault::login`

```rust
pub fn login(
    &mut self,
    backend: &dyn age_credentials::backend::traits::AccountBackend,
    email: &str,
    passphrase: &str,
    otp_code: &str,
    totp_secret_base32: &str,
) -> Result<&Session>
```

Authenticates a user using the provided `AccountBackend` and stores the resulting session inside the vault. This session is then required for subsequent `transact`, `journal`, and `session` calls.

- **Parameters**: identical to [`posvault_auth::login`](#posvault_authlogin) – see that reference for details.
- **Returns**: a reference to the stored `Session` (valid as long as the `PosVault` is not dropped).
- **Side effects**: sets `self.session = Some(session)`.
- **Errors**: all errors from `posvault_auth::login` (user not found, invalid passphrase, invalid OTP, etc.).

---

### `PosVault::session`

```rust
pub fn session(&self) -> Result<&Session>
```

Returns a reference to the current authenticated session. Fails if no session exists (i.e., `login` was not called or the vault was just opened).

- **Errors**: `PosVaultError::Auth("not logged in")` if session is `None`.

---

### `PosVault::transact`

```rust
pub fn transact(&mut self, mut event: Event) -> Result<()>
```

The primary method for appending a new event to the store. It performs the following steps in order:

1. **Authentication**: Ensures a valid session exists (calls `self.session()?`).
2. **Encryption** (optional): If `self.recipients` is not empty, encrypts the event’s payload using `posvault_crypto::encrypt_event` with those recipients.
3. **Signing** (optional): If a signer is set, wraps the underlying `VctrlEventStore` in a `SignedEventStore` and appends the event (which will be signed automatically). Otherwise, appends directly to a plain `VctrlEventStore`.

**Important**: The event passed in must have its signature field set to the default zero signature (64 zero bytes). The signer will overwrite it.

- **Parameters**:
  - `event` – the event to append. Will be modified in‑place (encryption and signing).
- **Errors**:
  - `Auth` if not logged in.
  - `Encryption` if recipient keys are invalid or encryption fails.
  - `Serialization` / `Storage` from the underlying store.
  - `Auth` if the event already had a non‑zero signature (from `SignedEventStore`).

---

### `PosVault::journal`

```rust
pub fn journal(&mut self, entry: JournalEntry) -> Result<()>
```

Records a journal entry. Like `transact`, it requires an active session and will optionally sign the entry if a signer is configured.

1. Checks session (logged in).
2. If `signer` is set, wraps the `VctrlJournal` in a `SignedJournal` and records the entry (which signs it). Otherwise, uses a plain `VctrlJournal`.

- **Parameters**:
  - `entry` – the journal entry to record. Must have a zero signature initially.
- **Errors**:
  - `Auth` if not logged in or signature already present.
  - `Serialization` / `Storage` from the journal store.

---

### `PosVault::query_engine`

```rust
pub fn query_engine(&self) -> Result<QueryEngine<CombinedStore>>
```

Creates a `QueryEngine` backed by the vault’s store. It combines a `VctrlEventStore` and `VctrlSnapshotStore` into a `CombinedStore` that satisfies both `EventStore` and `SnapshotStore`.

- **Returns**: a new `QueryEngine` that can be used for snapshot rebuilds and queries.
- **Usage**: You typically call this, then use `posvault_query::get_stock` or custom rebuild logic.

- **Note**: This does **not** require an active session. It is a read‑only operation.

---

### `PosVault::decrypt_payload`

```rust
pub fn decrypt_payload(&self, event: &mut Event, identity: &str) -> Result<()>
```

Convenience wrapper around `posvault_crypto::decrypt_event`. Decrypts the payload of a given event using an age identity.

- **Parameters**:
  - `event` – mutable reference to the event whose payload will be decrypted in‑place.
  - `identity` – age identity string (e.g., `AGE-SECRET-KEY-…`).
- **Errors**: `Encryption` if decryption fails or identity is invalid.

This method does **not** require a session; decryption is independent.

---

### `PosVault::sync_to_remote`

```rust
pub fn sync_to_remote(&self, remote_path: impl AsRef<Path>) -> Result<()>
```

Performs a simple file‑based push of the entire local repository to a remote directory using `posvault_sync::FileTransport::push`. This copies the whole store directory (including the `store.vctrl` database) to the remote path.

- **Parameters**:
  - `remote_path` – directory where the remote copy should be placed.
- **Errors**: `NotFound` if local store directory missing, `Sync` if copy fails.

---

## CombinedStore

A helper struct that bundles `VctrlEventStore` and `VctrlSnapshotStore` into a single type. This is necessary because `QueryEngine` requires a single generic parameter that implements **both** `EventStore` and `SnapshotStore`.

### CombinedStore Definition

```rust
#[derive(Debug)]
pub struct CombinedStore {
    pub event_store: VctrlEventStore,
    pub snapshot_store: VctrlSnapshotStore,
}
```

### Trait Implementations for `CombinedStore`

#### `EventStore` Implementation

```rust
impl EventStore for CombinedStore {
    fn append_event(&mut self, event: Event) -> Result<()> {
        self.event_store.append_event(event)
    }
    fn get_events_since(&self, checkpoint: u64) -> Result<Vec<Event>> {
        self.event_store.get_events_since(checkpoint)
    }
    fn latest_checkpoint(&self) -> Result<u64> {
        self.event_store.latest_checkpoint()
    }
}
```

Delegates directly to the inner `VctrlEventStore`.

#### `SnapshotStore` Implementation

```rust
impl SnapshotStore for CombinedStore {
    fn save_snapshot(&mut self, snapshot: Snapshot) -> Result<()> {
        self.snapshot_store.save_snapshot(snapshot)
    }
    fn load_snapshot(&self) -> Result<Option<Snapshot>> {
        self.snapshot_store.load_snapshot()
    }
}
```

Delegates directly to the inner `VctrlSnapshotStore`.

`CombinedStore` is typically not instantiated manually; use `PosVault::query_engine()` to obtain a `QueryEngine<CombinedStore>`.

---

## Error Handling

Every public method in `PosVault` returns `Result<_, PosVaultError>` (the error type re‑exported from `posvault_handler`). The possible error variants are:

| Variant         | Typical scenario in `PosVault`                                              |
| --------------- | --------------------------------------------------------------------------- |
| `Auth`          | Missing session, login failure, session expired (when using guard outside). |
| `Encryption`    | Encryption/decryption failure during `transact` or `decrypt_payload`.       |
| `Storage`       | Underlying file store errors, lock poisoning.                               |
| `Serialization` | JSON/bincode failures inside store or during journal recording.             |
| `NotFound`      | Repository not found, missing HEAD, etc.                                    |
| `Sync`          | Copy failure in `sync_to_remote`.                                           |

For full details on each error, see the `posvault_handler` API reference.

---

## Examples

### Opening a Vault and Performing Operations

```rust
use posvault::*;

fn main() -> Result<()> {
    // 1. Open or create a vault
    let mut vault = PosVault::open("./my-vault")?;

    // 2. Configure recipients (age public keys)
    vault.set_recipients(vec!["age1...".to_string()]);

    // 3. Set signer
    let (signing_key, _verifying_key) = generate_keypair();
    vault.set_signer(Ed25519Signer::new(signing_key));

    // 4. Login (assume backend implements AccountBackend)
    let backend = MyAccountBackend::new();
    vault.login(&backend, "admin@example.com", "pass", "123456", "JBSWY3DPEHPK3PXP")?;

    // 5. Create and append an event
    let event = Event::new(
        EventId::generate(),
        1700000000,
        Identity::new(Fingerprint::new("a".repeat(64))?, Role::Admin),
        EncryptedPayload::new(b"my payload".to_vec())?,
        Signature::new(vec![0u8; 64])?,
    )?;
    vault.transact(event)?;

    // 6. Record a journal entry
    let entry = JournalEntry::new(
        EventId::generate(),
        1700000001,
        "user.login".into(),
        Identity::new(Fingerprint::new("a".repeat(64))?, Role::Admin),
        "logged in".into(),
        Signature::new(vec![0u8; 64])?,
    )?;
    vault.journal(entry)?;

    // 7. Query the store
    let mut engine = vault.query_engine()?;
    // (use engine with posvault_query functions)

    // 8. Sync to remote
    vault.sync_to_remote("/backup/remote-vault")?;

    Ok(())
}
```

### Using the Re‑exported Items Directly

Since all sub‑crate items are re‑exported, you can also use them without the `PosVault` façade if you prefer direct access:

```rust
use posvault::{
    Session, login, require_role,
    encrypt_event, decrypt_event,
    PosVaultError, Result,
    EventStore, SnapshotStore,
    Event, Snapshot, // etc.
    QueryEngine, get_stock,
    Ed25519Signer, SignedEventStore,
    VctrlEventStore, PosVault as Store,
    FileTransport, create_store_branch,
};
```

---

## Dependencies

This crate depends on all the PosVault sub‑crates and external libraries they require:

- `posvault_auth`, `posvault_crypto`, `posvault_handler`, `posvault_query`, `posvault_sign`, `posvault_store`, `posvault_sync`
- `age_credentials` (for the `AccountBackend` trait, used in `login`)
- `libvctrl` (indirectly via `posvault_store`)

No additional configuration is needed. The `PosVault` façade simply ties the components together.

---

## Full Source Reference

The entire public API is contained in:

- `lib.rs` – re‑exports everything from sub‑crates.
- `vault.rs` – defines `PosVault` (façade) and `CombinedStore`.

All sub‑crate items are documented in their respective API references:

- [`posvault_handler`](../posvault_handler/README.md)
- [`posvault_auth`](../posvault_auth/README.md)
- [`posvault_crypto`](../posvault_crypto/README.md)
- [`posvault_query`](../posvault_query/README.md)
- [`posvault_sign`](../posvault_sign/README.md)
- [`posvault_store`](../posvault_store/README.md)
- [`posvault_sync`](../posvault_sync/README.md)

Use this top‑level crate as the single dependency for application code to keep imports clean.

---

_End of `posvault` API Reference._
