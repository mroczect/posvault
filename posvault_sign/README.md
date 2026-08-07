# posvault_sign – Full API Reference

**Complete crate documentation for `posvault_sign`**  
Provides Ed25519 signing and verification, plus decorators that automatically sign events and journal entries before storage.

---

## Table of Contents

- [Overview](#overview)
- [Public API](#public-api)
  - [Modules & Re‑exports](#modules--re-exports)
  - [Ed25519Signer](#ed25519signer)
    - [Constructor](#ed25519signernew)
    - [Accessing the Verifying Key](#verifying_key)
    - [Signer Trait Implementation](#signer-trait-implementation-for-ed25519signer)
  - [generate_keypair](#generate_keypair)
  - [SignedJournal](#signedjournalj-g)
    - [Constructors](#signedjournal-new-and-new_loose)
    - [Journal Trait Implementation](#journal-trait-implementation-for-signedjournal)
  - [SignedEventStore](#signedeventstores-g)
    - [Constructors](#signedeventstore-new-and-new_loose)
    - [EventStore Trait Implementation](#eventstore-trait-implementation-for-signedeventstore)
- [Verification Modes (Strict vs. Loose)](#verification-modes-strict-vs-loose)
- [Error Handling](#error-handling)
- [Examples](#examples)
  - [Signing and Appending an Event](#signing-and-appending-an-event)
  - [Recording a Signed Journal Entry](#recording-a-signed-journal-entry)
  - [Verifying a Signature Directly](#verifying-a-signature-directly)
  - [Using Loose Verification](#using-loose-verification)
- [Dependencies](#dependencies)
- [Full Source Reference](#full-source-reference)

---

## Overview

The `posvault_sign` crate supplies cryptographic signing for the PosVault event‑store and journal. It does **not** implement encryption; that is handled by `posvault_crypto`. Instead, this crate:

- Implements the `Signer` trait from `posvault_handler` using **Ed25519** (via the `ed25519-dalek` crate).
- Provides a free function `generate_keypair` to create new signing/verifying keys.
- Offers **decorator wrappers** `SignedEventStore` and `SignedJournal` that automatically sign events and journal entries on insertion, and verify signatures on retrieval.
- Supports **strict** and **loose** verification modes, allowing flexible handling of signature failures.

All public items are available from the crate root:

```rust
use posvault_sign::{Ed25519Signer, generate_keypair, SignedEventStore, SignedJournal};
```

---

## Public API

### Modules & Re‑exports

The crate defines three modules and re‑exports them all:

| Module           | Contents                                                        |
| ---------------- | --------------------------------------------------------------- |
| `ed25519`        | `Ed25519Signer` struct, `generate_keypair` function             |
| `signed_journal` | `SignedJournal<J, G>` decorator that implements `Journal`       |
| `signed_store`   | `SignedEventStore<S, G>` decorator that implements `EventStore` |

Everything is accessible directly from `posvault_sign::*`.

---

### `Ed25519Signer`

```rust
#[derive(Clone)]
pub struct Ed25519Signer {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
}
```

A concrete implementation of `posvault_handler::traits::Signer` using the Ed25519 signature scheme. The struct hides its keys; `Debug` prints only the struct name.

#### `Ed25519Signer::new`

```rust
pub fn new(signing_key: SigningKey) -> Self
```

Constructs an `Ed25519Signer` from a `SigningKey` (from `ed25519_dalek`). The corresponding `VerifyingKey` is extracted automatically.

#### `verifying_key`

```rust
pub fn verifying_key(&self) -> VerifyingKey
```

Returns a copy of the `VerifyingKey`. This can be used to share the public key without exposing the secret.

#### `Signer` Trait Implementation for `Ed25519Signer`

`Ed25519Signer` implements the `Signer` trait:

- **`sign(&self, data: &[u8]) -> Result<Vec<u8>>`**  
  Signs `data` using the Ed25519 signing key and returns the signature as a 64‑byte vector. This method never fails; the `Result` is always `Ok`.

- **`verify(&self, data: &[u8], signature: &[u8]) -> bool`**  
  Returns `true` if `signature` (must be 64 bytes) is a valid Ed25519 signature for `data` under the signer’s verifying key. If the byte slice cannot be converted to a 64‑byte array, it returns `false`.

- **`public_key_bytes(&self) -> &[u8]`**  
  Returns the raw bytes of the verifying key (public key), typically 32 bytes.

---

### `generate_keypair`

```rust
pub fn generate_keypair() -> (SigningKey, VerifyingKey)
```

Uses a cryptographically secure random number generator (`OsRng`) to create a fresh Ed25519 keypair.

- **Returns** a tuple `(signing_key, verifying_key)`.
- The `SigningKey` can be used to construct an `Ed25519Signer` via `Ed25519Signer::new`.
- The `VerifyingKey` can be distributed to verify signatures.

**Important:** This function does **not** persist the key; it is the caller’s responsibility to store the signing key securely.

---

### `SignedJournal<J, G>`

```rust
pub struct SignedJournal<J: Journal, G: Signer> {
    inner: J,
    signer: G,
    strict_verification: bool,
}
```

A decorator that wraps any `Journal` implementation and transparently signs every `JournalEntry` before recording it, and verifies the signatures when reading entries.

#### `SignedJournal` – `new` and `new_loose`

- **`SignedJournal::new(inner: J, signer: G) -> Self`**  
  Creates a new `SignedJournal` with **strict** verification enabled. In strict mode, if any signature fails during `read_all()`, the entire operation returns an error.

- **`SignedJournal::new_loose(inner: J, signer: G) -> Self`**  
  Creates a `SignedJournal` with **loose** verification. In loose mode, `read_all()` silently skips entries whose signatures are invalid and only returns the valid ones.

#### `Journal` Trait Implementation for `SignedJournal`

`SignedJournal` implements `Journal`, so it can be used wherever a `Journal` is expected.

##### `record`

```rust
fn record(&mut self, mut entry: JournalEntry) -> Result<()>
```

1. Checks that the entry does **not** already have a signature (all bytes must be zero). If it already has a signature, returns `Auth("entry already has a signature")`.
2. Builds a `SignableJournalEntry` containing the fields `id`, `timestamp`, `action`, `author`, and `details` (the original `signature` is excluded).
3. Serialises it using `bincode` and signs the result with the internal `signer`.
4. Replaces the entry’s signature with the newly created `Signature`.
5. Delegates to `inner.record(entry)`.

##### `read_all`

```rust
fn read_all(&self) -> Result<Vec<JournalEntry>>
```

1. Reads all entries from the inner journal.
2. For each entry, reconstructs the `SignableJournalEntry` and verifies its signature.
3. If verification succeeds, the entry is included in the result.
4. If verification fails:
   - A warning is logged via `log::warn!`.
   - In **strict** mode, an `Auth` error is returned immediately.
   - In **loose** mode, the entry is simply ignored and processing continues.

The final `Vec<JournalEntry>` contains only verified entries (and in strict mode, all entries must be valid).

---

### `SignedEventStore<S, G>`

```rust
pub struct SignedEventStore<S: EventStore, G: Signer> {
    inner: S,
    signer: G,
    strict_verification: bool,
}
```

A decorator that wraps any `EventStore` implementation and automatically signs every `Event` before appending it, and verifies signatures when retrieving events.

#### `SignedEventStore` – `new` and `new_loose`

- **`SignedEventStore::new(inner: S, signer: G) -> Self`**  
  Strict verification. On signature failure, `get_events_since` returns an error.

- **`SignedEventStore::new_loose(inner: S, signer: G) -> Self`**  
  Loose verification. Invalid events are silently dropped during retrieval.

#### `EventStore` Trait Implementation for `SignedEventStore`

`SignedEventStore` implements `EventStore`, so it can be used everywhere an `EventStore` is needed.

##### `append_event`

```rust
fn append_event(&mut self, mut event: Event) -> Result<()>
```

1. Checks that the event’s signature is still the default (64 zero bytes). If already signed, returns `Auth("event already has a signature")`.
2. Constructs a `SignableEvent` containing `id`, `timestamp`, `author`, and `payload` (the original signature is omitted).
3. Serialises with `bincode` and signs.
4. Updates `event.signature` with the new `Signature`.
5. Calls `self.inner.append_event(event)`.

##### `get_events_since`

```rust
fn get_events_since(&self, checkpoint: u64) -> Result<Vec<Event>>
```

1. Retrieves events from the inner store since the given checkpoint.
2. For each event, builds the `SignableEvent` and verifies its signature.
3. Valid events are kept; invalid ones are either:
   - Logged and dropped (loose mode), or
   - Cause an `Auth` error to be returned (strict mode).
4. Returns a vector of verified events, preserving order.

##### `latest_checkpoint`

```rust
fn latest_checkpoint(&self) -> Result<u64>
```

Directly delegates to `self.inner.latest_checkpoint()`. Signatures do not affect checkpoints.

---

## Verification Modes (Strict vs. Loose)

Both `SignedEventStore` and `SignedJournal` offer two constructors:

- **Strict** (`new`) – any signature verification failure when reading data immediately halts the operation with `PosVaultError::Auth`. This ensures that every stored entry has a valid signature from the trusted signer.
- **Loose** (`new_loose`) – invalid entries are skipped (with a warning log). The returned collection contains only entries that passed verification. This is useful when you want to read data without being blocked by a few corrupted or foreign entries.

The mode only affects _retrieval_ methods (`get_events_since`, `read_all`). Insertion always signs the entry.

---

## Error Handling

All public methods return `posvault_handler::errors::Result<()>` (or `Result<Vec<T>>`). The following error scenarios can occur:

| Scenario                                      | Error Variant   | Example Message                                                        |
| --------------------------------------------- | --------------- | ---------------------------------------------------------------------- |
| Attempt to sign an already signed entry/event | `Auth`          | `"entry already has a signature"` or `"event already has a signature"` |
| Serialisation failure (bincode)               | `Serialization` | (from bincode)                                                         |
| Signature verification failure (strict mode)  | `Auth`          | `"Signature verification failed for journal entry ..."`                |
| Inner store errors                            | Propagated      | (any `PosVaultError` variant)                                          |

Note that `sign()` itself never fails; it always returns `Ok(signature_bytes)`.

The `generate_keypair` function does not return a `Result` – it panics only if the OS randomness source fails (extremely unlikely).

---

## Examples

### Signing and Appending an Event

```rust
use posvault_sign::{Ed25519Signer, SignedEventStore, generate_keypair};
use posvault_handler::traits::EventStore;
use posvault_handler::types::{Event, EventId, Identity, Role, EncryptedPayload, Signature, Fingerprint};

// Assume DummyStore implements EventStore (e.g., in-memory)
struct DummyStore;
impl EventStore for DummyStore { /* ... */ }

// Generate a keypair
let (signing_key, verifying_key) = generate_keypair();
let signer = Ed25519Signer::new(signing_key);
let store = DummyStore;
let mut signed_store = SignedEventStore::new(store, signer);

// Create an unsigned event (signature all zero)
let event = Event::new(
    EventId::generate(),
    1,
    Identity::new(Fingerprint::new("a".repeat(64)).unwrap(), Role::Cashier),
    EncryptedPayload::new(b"data".to_vec()).unwrap(),
    Signature::new(vec![0u8; 64]).unwrap(),
).unwrap();

// Append – the event is automatically signed
signed_store.append_event(event).unwrap();
```

### Recording a Signed Journal Entry

```rust
use posvault_sign::{Ed25519Signer, SignedJournal, generate_keypair};
use posvault_handler::traits::Journal;
use posvault_handler::types::{JournalEntry, EventId, Identity, Fingerprint, Role, Signature};

struct DummyJournal;
impl Journal for DummyJournal { /* ... */ }

let (signing_key, _) = generate_keypair();
let signer = Ed25519Signer::new(signing_key);
let journal = DummyJournal;
let mut signed_journal = SignedJournal::new(journal, signer);

let entry = JournalEntry::new(
    EventId::generate(),
    2,
    "user.login".to_string(),
    Identity::new(Fingerprint::new("a".repeat(64)).unwrap(), Role::Admin),
    "details".to_string(),
    Signature::new(vec![0u8; 64]).unwrap(),
).unwrap();

signed_journal.record(entry).unwrap();
```

### Verifying a Signature Directly

```rust
let (signing_key, _) = generate_keypair();
let signer = Ed25519Signer::new(signing_key);
let data = b"hello";
let sig = signer.sign(data).unwrap();

// Verification requires a 64-byte array
let mut sig_bytes = [0u8; 64];
sig_bytes.copy_from_slice(&sig);
assert!(signer.verify(data, &sig_bytes));
assert!(!signer.verify(b"wrong", &sig_bytes));
```

### Using Loose Verification

```rust
let (signing_key, _) = generate_keypair();
let signer = Ed25519Signer::new(signing_key);
let store = DummyStore;
// Loose mode – will skip invalid events instead of erroring out
let mut loose_store = SignedEventStore::new_loose(store, signer);

// Append valid event...
// Later, if some events were corrupted, retrieval will just ignore them.
let valid_events = loose_store.get_events_since(0).unwrap();
```

---

## Dependencies

- `posvault_handler` – provides `Signer`, `EventStore`, `Journal`, and all data types (`Event`, `JournalEntry`, `Signature`, etc.).
- `ed25519_dalek` – Ed25519 implementation.
- `rand_core` – for `OsRng`.
- `bincode` – for deterministic serialisation of signable data.
- `serde` – `SignableEvent` and `SignableJournalEntry` derive `Serialize`.
- `log` – used for warnings on verification failures.

No additional configuration is required; the crate is ready to plug into any backend that implements the `posvault_handler` traits.

---

## Full Source Reference

The complete public API is defined across three files, re‑exported in `lib.rs`:

**`ed25519.rs`**

- `pub struct Ed25519Signer { ... }`
- `pub fn generate_keypair() -> (SigningKey, VerifyingKey)`

**`signed_journal.rs`**

- `pub struct SignedJournal<J: Journal, G: Signer> { ... }`
- Constructors: `new(inner: J, signer: G) -> Self`, `new_loose(...)`
- Implements `Journal` for `SignedJournal<J, G>`.

**`signed_store.rs`**

- `pub struct SignedEventStore<S: EventStore, G: Signer> { ... }`
- Constructors: `new(inner: S, signer: G) -> Self`, `new_loose(...)`
- Implements `EventStore` for `SignedEventStore<S, G>`.

All implementations are generic, allowing any combination of inner store and signer that satisfy the trait bounds.

---

_End of `posvault_sign` API Reference._
