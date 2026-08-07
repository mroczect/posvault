# posvault_handler – Full API Reference

**Complete documentation for the `posvault_handler` crate**  
Core data types, errors, traits, macros, and constants for the PosVault ecosystem.

---

## Overview

The `posvault_handler` crate serves as the **foundation** for the entire PosVault project. It defines:

- Immutable system‑wide **constants**.
- **Enumerations** for algorithms and synchronization modes.
- A comprehensive **error type** (`PosVaultError`) and a convenience `Result` alias.
- **Macros** for early returns on validation failures.
- **Traits** for event storage, snapshots, journaling, signing, conflict resolution, transport, and codecs.
- Core **data types** (`Event`, `Snapshot`, `JournalEntry`, `Identity`, `CommitHash`, etc.) that are used across all other crates.
- The **`Validate`** trait, which is implemented by several types for self‑validation.

All public items are re‑exported from the crate root via `pub use`, so you can import everything directly:

```rust
use posvault_handler::*;
```

---

## Modules & Re‑exports

The crate consists of the following modules, each re‑exported through `lib.rs`:

| Module       | Contents                                          |
| ------------ | ------------------------------------------------- |
| `constants`  | Compile‑time constants.                           |
| `enums`      | Algorithm and sync mode enumerations.             |
| `errors`     | `PosVaultError` enum and `Result<T>` type.        |
| `macros`     | `ensure!` and `bail!` helper macros.              |
| `traits`     | Core abstractions (`EventStore`, `Signer`, etc.). |
| `types`      | All domain data structures.                       |
| `validation` | The `Validate` trait.                             |

Because of the re‑export, all constants, enums, errors, macros, traits, types, and the `Validate` trait are directly available as `posvault_handler::*`.

---

## Constants (`constants`)

The following constants are defined at compile time and are always accessible.

| Constant                       | Type    | Value              | Description                                                                 |
| ------------------------------ | ------- | ------------------ | --------------------------------------------------------------------------- |
| `DEFAULT_RECIPIENTS_COUNT`     | `usize` | `2`                | Default number of age recipients used when encrypting an event.             |
| `SIGNATURE_ALGORITHM`          | `&str`  | `"ed25519"`        | Name of the signature algorithm used throughout the system.                 |
| `JOURNAL_COMPACTION_THRESHOLD` | `u64`   | `100_000`          | Number of unarchived journal entries that triggers an automatic compaction. |
| `SNAPSHOT_INTERVAL`            | `u64`   | `10_000`           | Number of events after which a new snapshot should be generated.            |
| `MAX_PAYLOAD_SIZE`             | `usize` | `10 * 1024 * 1024` | Maximum size (in bytes) of an event’s plaintext payload.                    |

All constants except `SIGNATURE_ALGORITHM` are verified at compile time to be **greater than zero** using a `const _: () = { assert!(...); };` block. The signature algorithm is checked in a unit test.

---

## Enumerations (`enums`)

### `HashAlgo`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HashAlgo {
    Sha256,
    Sha512,
}
```

Represents the hash algorithm used for commit hashes and other integrity checks.  
Currently supports SHA‑256 and SHA‑512.

### `EncryptionAlgo`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncryptionAlgo {
    AgeX25519,
    AgePassphrase,
    AgeSsh,
}
```

Identifies the age‑based encryption scheme.

- `AgeX25519` – native X25519 identity.
- `AgePassphrase` – passphrase‑based encryption.
- `AgeSsh` – SSH key‑based encryption.

### `SyncMode`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncMode {
    Manual,
    Periodic(u64),
    Realtime,
}
```

Controls how a remote store is synchronised.

- `Manual` – explicit push/pull required.
- `Periodic(seconds)` – automatic sync every `seconds` seconds.
- `Realtime` – immediate sync on every change.

---

## Error Handling (`errors`)

### `PosVaultError`

```rust
#[derive(Error, Debug)]
pub enum PosVaultError {
    Storage(String),
    Encryption(String),
    Auth(String),
    Sync(String),
    Journal(String),
    InvalidInput(String),
    NotFound(String),
    Conflict(String),
    Serialization(String),
    Io(#[from] std::io::Error),
    Vctrl(#[from] libvctrl::error::VctrlError),
    External(Box<dyn std::error::Error + Send + Sync>),
    Tree(#[from] TreeError),
}
```

The single error type for the entire project. Each variant holds a descriptive message or a wrapped error.

| Variant         | Display format            | Typical cause                                                                                |
| --------------- | ------------------------- | -------------------------------------------------------------------------------------------- |
| `Storage`       | `Storage error: …`        | File store operation failed (permissions, disk full, corruption).                            |
| `Encryption`    | `Encryption error: …`     | Encryption/decryption failure (invalid key, malformed ciphertext, empty recipients).         |
| `Auth`          | `Authentication error: …` | Invalid credentials, expired session, missing signature, role not allowed.                   |
| `Sync`          | `Sync error: …`           | Push/pull/merge failure, network error, remote store missing.                                |
| `Journal`       | `Journal error: …`        | Journal record/read failure, compaction issues.                                              |
| `InvalidInput`  | `Invalid input: …`        | Validation failure (empty field, bad format, out‑of‑range value).                            |
| `NotFound`      | `Not found: …`            | A required entity was not found (HEAD, snapshot, branch, user).                              |
| `Conflict`      | `Conflict: …`             | Merge conflict or inconsistent state (not yet used in this crate).                           |
| `Serialization` | `Serialization error: …`  | JSON/Bincode serialisation/deserialisation error.                                            |
| `Io`            | `I/O error: …`            | Standard I/O error, automatically converted from `std::io::Error`.                           |
| `Vctrl`         | `Vctrl error: …`          | Error from the `libvctrl` crate, automatically converted from `libvctrl::error::VctrlError`. |
| `External`      | `External error: …`       | Wraps an arbitrary `Send + Sync` error from an external source.                              |
| `Tree`          | `Tree error: …`           | Error from `libvctrl::domain::tree::TreeError`, automatically converted.                     |

The derive macro `#[derive(Error)]` from the `thiserror` crate provides the `Display` implementation.  
Several variants are `#[from]`, enabling automatic conversion with `?`.

### `Result<T>`

```rust
pub type Result<T> = std::result::Result<T, PosVaultError>;
```

A convenience alias used throughout the project. Every fallible function returns this type.

---

## Macros (`macros`)

Two helper macros are exported to simplify early returns on validation failures. They both return `Err(PosVaultError::InvalidInput(...))`.

### `ensure!`

```rust
macro_rules! ensure {
    ($cond:expr, $msg:literal $(,)?) => {
        if !$cond {
            return Err($crate::errors::PosVaultError::InvalidInput($msg.into()));
        }
    };
}
```

If the condition is `false`, immediately returns an `InvalidInput` error with the given message.

**Usage:**

```rust
ensure!(!payload.is_empty(), "payload must not be empty");
// continues if payload is not empty
```

### `bail!`

```rust
macro_rules! bail {
    ($msg:literal $(,)?) => {
        return Err($crate::errors::PosVaultError::InvalidInput($msg.into()));
    };
}
```

Unconditionally returns an `InvalidInput` error with the given message.

**Usage:**

```rust
bail!("unexpected state");
```

---

## Traits (`traits`)

All traits in this crate require `Debug + Send + Sync`. They are designed to be implemented by various backends (file store, in‑memory, etc.) and are **object‑safe**, meaning they can be used as `dyn Trait`.

### `EventStore`

```rust
pub trait EventStore: Debug + Send + Sync {
    fn append_event(&mut self, event: Event) -> Result<()>;
    fn get_events_since(&self, checkpoint: u64) -> Result<Vec<Event>>;
    fn latest_checkpoint(&self) -> Result<u64>;
}
```

Stores and retrieves events.

- **`append_event`** – Persists a new event. On success, the internal checkpoint should increment.
- **`get_events_since`** – Returns all events with an index **greater than** `checkpoint`, ordered by index ascending.
- **`latest_checkpoint`** – Returns the current event counter (the index of the most recently appended event). A new store starts at `0`.

### `SnapshotStore`

```rust
pub trait SnapshotStore: Debug + Send + Sync {
    fn save_snapshot(&mut self, snapshot: Snapshot) -> Result<()>;
    fn load_snapshot(&self) -> Result<Option<Snapshot>>;
}
```

Manages periodic snapshots of the entire event‑sourced state.

- **`save_snapshot`** – Stores a snapshot. The implementation may decide to keep only the latest one.
- **`load_snapshot`** – Returns the most recent snapshot, or `None` if none exists.

### `Journal`

```rust
pub trait Journal: Debug + Send + Sync {
    fn record(&mut self, entry: JournalEntry) -> Result<()>;
    fn read_all(&self) -> Result<Vec<JournalEntry>>;
}
```

An append‑only audit trail.

- **`record`** – Appends a journal entry.
- **`read_all`** – Returns all journal entries sorted chronologically (or by insertion order). The implementation may perform compaction behind the scenes.

### `Signer`

```rust
pub trait Signer: Debug + Send + Sync {
    fn sign(&self, data: &[u8]) -> Result<Vec<u8>>;
    fn verify(&self, data: &[u8], signature: &[u8]) -> bool;
    fn public_key_bytes(&self) -> &[u8];
}
```

Cryptographic signing and verification.

- **`sign`** – Produces a signature for the given data. The returned vector must be 64 bytes (Ed25519) or another agreed‑upon length.
- **`verify`** – Returns `true` if the signature is valid for the data.
- **`public_key_bytes`** – Returns the public key bytes used to verify signatures.

### `ConflictResolver`

```rust
pub trait ConflictResolver: Debug + Send + Sync {
    fn resolve(&self, base: &[u8], ours: &[u8], theirs: &[u8]) -> Result<Vec<u8>>;
}
```

Performs a three‑way merge on byte‑level data.

- **`resolve`** – Given the common ancestor (`base`), our version (`ours`), and the remote version (`theirs`), returns the merged content. This is used for synchronisation conflicts.

### `Transport`

```rust
pub trait Transport: Debug + Send + Sync {
    fn push(&mut self, refs: &[String]) -> Result<()>;
    fn pull(&mut self, refs: &[String]) -> Result<()>;
}
```

Abstracts the transfer of store data between local and remote repositories.

- **`push`** – Uploads the specified references (e.g., `["refs/heads/main"]`) to the remote.
- **`pull`** – Downloads the specified references from the remote.

### `EventCodec`

```rust
pub trait EventCodec: Debug + Send + Sync {
    fn encode(&self, event: &Event) -> Result<Vec<u8>>;
    fn decode(&self, data: &[u8]) -> Result<Event>;
}
```

Custom serialisation for `Event` objects.

- **`encode`** – Converts an `Event` into a byte vector.
- **`decode`** – Parses a byte vector back into an `Event`.

---

## Types (`types`)

All core data structures are defined in `types.rs`. Sensitive types (`SecretData`, `Signature`, `EncryptedPayload`) wrap their byte payload in `Zeroizing<Vec<u8>>` to zero memory on drop. Their `Debug` output hides the actual data.

### `SecretData`

```rust
#[derive(Clone)]
pub struct SecretData(Zeroizing<Vec<u8>>);
```

Wraps a secret byte vector that is zeroised on drop.

#### Construction

- `SecretData::new(data: Vec<u8>) -> Result<Self>`  
  Returns `InvalidInput` if `data` is empty. Otherwise wraps it in `Zeroizing`.

- `SecretData::from_hex(hex_str: &str) -> Result<Self>`  
  Decodes a hex string into bytes, then calls `new`. Returns `InvalidInput` on invalid hex.

#### Methods

- `as_bytes(&self) -> &[u8]` – Returns a reference to the secret bytes.
- `to_hex(&self) -> String` – Returns the secret as a lower‑case hex string.

#### Traits

- `Debug`: Prints `SecretData` (no contents).
- `PartialEq`, `Eq`: Compares inner bytes securely.

---

### `EventId`

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(String);
```

A unique identifier for an event.

#### Construction

- `EventId::new(id: impl Into<String>) -> Result<Self>`  
  Validates that `id` is **1–64 characters** long and contains only **ASCII alphanumerics** or `-`. Returns `InvalidInput` otherwise.

- `EventId::generate() -> Self`  
  Creates a new random UUID v4 (always valid).

#### Methods

- `as_str(&self) -> &str` – Returns the string representation.

---

### `Fingerprint`

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Fingerprint(String);
```

A 64‑character hexadecimal fingerprint (typically a SHA‑256 hash).

#### Construction

- `Fingerprint::new(hex: impl Into<String>) -> Result<Self>`  
  Validates that the string is exactly **64 characters** long and consists entirely of **lower‑case hex digits** (`0-9`, `a-f`). Returns `InvalidInput` otherwise.

#### Methods

- `as_str(&self) -> &str` – Returns the hex string.

---

### `Role`

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    Admin,
    Manager,
    Cashier,
    Auditor,
    Branch,
    Custom(String),
}
```

Represents the authority level of an identity.

#### Methods

- `as_str(&self) -> &str`  
  Returns a human‑readable string: `"admin"`, `"manager"`, `"cashier"`, `"auditor"`, `"branch"`, or the inner string for `Custom`.

---

### `Identity`

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identity {
    pub fingerprint: Fingerprint,
    pub role: Role,
}
```

Links a fingerprint to a role.

#### Construction

- `Identity::new(fingerprint: Fingerprint, role: Role) -> Self` – Simple constructor.

---

### `Signature`

```rust
#[derive(Clone)]
pub struct Signature(Zeroizing<Vec<u8>>);
```

A 64‑byte cryptographic signature (Ed25519). The inner bytes are zeroised on drop.

#### Construction

- `Signature::new(bytes: Vec<u8>) -> Result<Self>`  
  Rejects input that is **not exactly 64 bytes** with `InvalidInput`.

#### Methods

- `as_bytes(&self) -> &[u8]` – Returns the raw signature bytes.

#### Serialisation

`Signature` implements `Serialize` and `Deserialize` by treating the inner bytes as a vector. Deserialisation calls `Signature::new`, which performs length validation.

#### Traits

- `Debug`: Prints `Signature` (no content).
- `PartialEq`, `Eq`: Compares raw bytes.

---

### `EncryptedPayload`

```rust
#[derive(Clone)]
pub struct EncryptedPayload(Zeroizing<Vec<u8>>);
```

Stores encrypted (or sometimes plaintext) data with zeroisation on drop.

#### Construction

- `EncryptedPayload::new(data: Vec<u8>) -> Result<Self>`  
  Rejects empty vectors with `InvalidInput`.

#### Methods

- `as_bytes(&self) -> &[u8]` – Returns the payload bytes.

#### Serialisation

Works like `Signature`: `Serialize` writes the inner bytes, `Deserialize` expects a valid non‑empty vector and validates length.

#### Traits

- `Debug`: Prints `EncryptedPayload` (no content).
- `PartialEq`, `Eq`: Compares bytes.

---

### `Event`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: EventId,
    pub timestamp: i64,
    pub author: Identity,
    pub payload: EncryptedPayload,
    pub signature: Signature,
}
```

The core event structure in the event‑sourced model.

#### Construction

- `Event::new(id, timestamp, author, payload, signature) -> Result<Self>`  
  Constructs an event and then calls `self.validate()`.  
  Validation requirement: `timestamp > 0`. Returns `InvalidInput` otherwise.

#### Validation

Implements `Validate` (see below). The only rule is that `timestamp` must be strictly positive.

---

### `Recipient`

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recipient(String);
```

Represents an age public key (recipient).

#### Construction

- `Recipient::new(key: impl Into<String>) -> Result<Self>`  
  Enforces the following rules:
  - Must start with `"age1"`.
  - Length must be **> 4** and **≤ 512**.
  - All characters after the prefix must be **lower‑case ASCII letters** or **digits**.

  Returns `InvalidInput` on violation.

#### Methods

- `as_str(&self) -> &str` – Returns the full key string.

---

### `BranchName`

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchName(String);
```

A branch name for a store.

#### Construction

- `BranchName::new(name: impl Into<String>) -> Result<Self>`  
  Validation:
  - Length between **1** and **255** characters.
  - Only characters: ASCII alphanumeric, `-`, `_`, `/`.

  Returns `InvalidInput` otherwise.

#### Methods

- `as_str(&self) -> &str` – Returns the branch name.

---

### `CommitHash`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CommitHash([u8; 64]);
```

A 64‑byte hash (e.g., SHA‑512). This type is `Copy`.

#### Construction

- `CommitHash::from_bytes(bytes: [u8; 64]) -> Self` – Wraps a fixed‑size array.
- `CommitHash::from_hex(hex_str: &str) -> Result<Self>` – Decodes a hex string, expects **exactly 128 hex characters** (i.e., 64 bytes). Returns `InvalidInput` on bad format or length.

#### Methods

- `as_bytes(&self) -> &[u8; 64]` – Returns a reference to the hash bytes.
- `to_hex(&self) -> String` – Encodes the hash as a lower‑case hex string.
- `is_zero(&self) -> bool` – Returns `true` if all bytes are zero.

#### Serialisation

`CommitHash` serializes as a hex string using `to_hex()` and deserializes by calling `from_hex`.

---

### `Snapshot`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub version: u64,
    pub data: EncryptedPayload,
    pub hash: CommitHash,
}
```

Represents a complete state snapshot.

#### Construction

- `Snapshot::new(version: u64, data: EncryptedPayload, hash: CommitHash) -> Result<Self>`  
  Calls `validate()` after construction.

#### Validation (via `Validate`)

- `version` **must be > 0**.
- `hash` **must not be zero** (i.e., `!hash.is_zero()`).

Returns `InvalidInput` if either rule is broken.

---

### `JournalEntry`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    pub id: EventId,
    pub timestamp: i64,
    pub action: String,
    pub author: Identity,
    pub details: String,
    pub signature: Signature,
}
```

An entry in the audit journal.

#### Construction

- `JournalEntry::new(id, timestamp, action, author, details, signature) -> Result<Self>`  
  Constructs the struct and calls `validate()`.

#### Validation (via `Validate`)

- `timestamp` **> 0**.
- `action` must be **non‑empty** and **≤ 256 characters**.

Returns `InvalidInput` on failure.

---

## Validation (`validation`)

### `Validate` trait

```rust
pub trait Validate {
    fn validate(&self) -> Result<()>;
}
```

Types that can check their own invariants. The following types implement `Validate`:

- **`Event`** – ensures `timestamp > 0`.
- **`Snapshot`** – ensures `version > 0` and `hash` is non‑zero.
- **`JournalEntry`** – ensures `timestamp > 0` and `action` length between 1 and 256.

You can implement `Validate` for your own types to integrate with the same error‑handling patterns.

---

## Examples

### Creating a Valid Event

```rust
use posvault_handler::*;

fn create_event() -> Result<Event> {
    let id = EventId::generate();
    let fingerprint = Fingerprint::new("a".repeat(64))?;
    let author = Identity::new(fingerprint, Role::Admin);
    let payload = EncryptedPayload::new(b"hello".to_vec())?;
    let sig = Signature::new(vec![0u8; 64])?;
    Event::new(id, 1, author, payload, sig)
}
```

### Using `ensure!` for Early Validation

```rust
use posvault_handler::*;

fn process(data: &[u8]) -> Result<()> {
    ensure!(!data.is_empty(), "data must not be empty");
    // ... process
    Ok(())
}
```

### Loading a Snapshot

```rust
use posvault_handler::*;

fn get_latest_snapshot(store: &dyn SnapshotStore) -> Result<Snapshot> {
    store.load_snapshot()?
         .ok_or_else(|| PosVaultError::NotFound("no snapshot available".into()))
}
```

---

## Crate Structure (for reference)

```
src/
├── constants.rs
├── enums.rs
├── errors.rs
├── lib.rs          (pub use all modules)
├── macros.rs
├── traits.rs
├── types.rs
└── validation.rs
```

All public items are available via `use posvault_handler::*;`.  
The crate depends on `serde`, `thiserror`, `hex`, `uuid`, `zeroize`, and `libvctrl`.

---

_End of `posvault_handler` API Reference._
