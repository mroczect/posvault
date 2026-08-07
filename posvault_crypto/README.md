# posvault_crypto — API Reference

**Complete crate documentation for `posvault_crypto`**  
Provides encryption and decryption of event payloads using the [age](https://github.com/str4d/rage) encryption library (`librage`).

---

## Table of Contents

- [Overview](#overview)
- [Public Functions](#public-functions)
  - [`encrypt_event`](#encrypt_event)
  - [`decrypt_event`](#decrypt_event)
- [Related Types](#related-types)
  - [`Event`](#event)
  - [`EncryptedPayload`](#encryptedpayload)
  - [`PosVaultError`](#posvaulterror)
- [Error Handling](#error-handling)
- [Examples](#examples)
  - [Encrypt & Decrypt with a Single Recipient](#encrypt--decrypt-with-a-single-recipient)
  - [Encrypt for Multiple Recipients](#encrypt-for-multiple-recipients)
- [Implementation Details](#implementation-details)
- [Dependencies](#dependencies)
- [Testing](#testing)
- [Security Considerations](#security-considerations)
- [Full Source Reference](#full-source-reference)

---

## Overview

The `posvault_crypto` crate exposes two functions:

- **`encrypt_event`** — encrypts the payload of an [`Event`] using one or more age recipients.
- **`decrypt_event`** — decrypts an event’s payload using an age identity (private key).

These operations are designed to work seamlessly with the `posvault_handler` types (`Event`, `EncryptedPayload`) and use the `librage` crate for the underlying cryptographic operations.

The crate’s public API is intentionally minimal; all logic is contained in the two functions.

---

## Public Functions

### `encrypt_event`

```rust
pub fn encrypt_event(
    event: &mut Event,
    recipients: &[impl AsRef<str>],
) -> Result<()>
```

Encrypts the current payload of the given event for the specified recipients.

- **`event`** – mutable reference to an [`Event`]. The `payload` field is replaced with the encrypted form (`EncryptedPayload`). The `signature` field is **not** modified.
- **`recipients`** – slice of age public keys. Each item must implement `AsRef<str>` (e.g. `&str`, `String`).
  - The slice **must not be empty**; otherwise an `Encryption` error is returned.
  - If more than one recipient is given, the payload is encrypted such that **any one** of the recipients can decrypt it (multi-recipient age encryption).
  - If exactly one recipient is given, single-recipient encryption is used.

**Returns**  
`Result<()>` – `Ok(())` on success, or an [`PosVaultError::Encryption`] on failure.

**Panics**  
Does not panic. All errors are returned.

**Constraints**

- `recipients` cannot be empty.
- Each recipient must be a valid age public key; otherwise the underlying `librage` call fails.
- The event’s existing payload is consumed and replaced; the original plaintext is lost after successful encryption.

---

### `decrypt_event`

```rust
pub fn decrypt_event(
    event: &mut Event,
    identity: &str,
) -> Result<()>
```

Decrypts the payload of an event using the provided age identity.

- **`event`** – mutable reference to an [`Event`] whose `payload` field holds a ciphertext (previously encrypted with `encrypt_event` or an equivalent age encryption).
- **`identity`** – age identity string (e.g., a `AGE-SECRET-KEY-…` value, or a passphrase if age-passphrase encryption was used). The identity must be able to decrypt the payload.

**Returns**  
`Result<()>` – `Ok(())` on success. On failure, returns [`PosVaultError::Encryption`].

**Constraints**

- The event’s payload must be a valid age ciphertext.
- The provided identity must correspond to one of the recipients used during encryption.
- The `signature` is not touched; only `payload` is replaced with a new `EncryptedPayload` containing the plaintext.

**Important**  
The function replaces `event.payload` with a **new** `EncryptedPayload` containing the decrypted bytes. This means the original `EncryptedPayload` is discarded. The new payload is still wrapped in `EncryptedPayload` (as that is the type required by `Event`), but the bytes inside are the plaintext. This design ensures that the event remains in a consistent state for further processing.

---

## Related Types

### `Event`

From `posvault_handler::types`:

```rust
pub struct Event {
    pub id: EventId,
    pub timestamp: i64,
    pub author: Identity,
    pub payload: EncryptedPayload,
    pub signature: Signature,
}
```

- `payload`: holds either plaintext or ciphertext; managed via `EncryptedPayload`.
- `signature`: an `ed25519` signature (64 bytes). This field is **never modified** by `encrypt_event` or `decrypt_event`.

### `EncryptedPayload`

A newtype around `Vec<u8>` with zeroize-on-drop semantics. It enforces that its content is never empty at construction (`EncryptedPayload::new`).  
The `as_bytes()` method returns the raw byte slice.

### `PosVaultError`

From `posvault_handler::errors`:

```rust
pub enum PosVaultError {
    Encryption(String),
    // … other variants
}
```

The `Encryption` variant is the primary error type returned by this crate. The inner `String` describes the failure reason (e.g., “recipients must not be empty”, “unknown librage error”, etc.).

---

## Error Handling

All functions return `posvault_handler::errors::Result<()>`.

Typical error cases:

| Scenario                              | Error Kind   | Example Message                               |
| ------------------------------------- | ------------ | --------------------------------------------- |
| Empty recipients slice                | `Encryption` | `"recipients must not be empty"`              |
| Invalid recipient key                 | `Encryption` | `"100: invalid recipient key"` (from librage) |
| Decryption with wrong identity        | `Encryption` | `"200: decryption failed"`                    |
| Garbled ciphertext                    | `Encryption` | `"300: invalid ciphertext"`                   |
| `librage` internal error (unexpected) | `Encryption` | `"unknown librage error"`                     |

`map_librage_error` (private) converts `librage::ErrorBody` into these messages by concatenating the `code` and `message`.

---

## Examples

### Encrypt & Decrypt with a Single Recipient

```rust
use posvault_crypto::{encrypt_event, decrypt_event};
use posvault_handler::types::{Event, EventId, Fingerprint, Identity, Role, EncryptedPayload, Signature};

// Build a test event
let id = EventId::generate();
let author = Identity::new(Fingerprint::new("a".repeat(64)).unwrap(), Role::Cashier);
let payload = EncryptedPayload::new(b"secret data".to_vec()).unwrap();
let sig = Signature::new(vec![0u8; 64]).unwrap();
let mut event = Event::new(id, 1, author, payload, sig).unwrap();

// Generate an age keypair (for demonstration)
let kp = librage::generate_keypair();
let data = kp.data.unwrap();
let recipient = data.public_key;         // e.g. "age1..."
let identity = data.secret_key.to_string();

// Encrypt
encrypt_event(&mut event, &[&recipient]).unwrap();
// Now event.payload contains ciphertext

// Decrypt
decrypt_event(&mut event, &identity).unwrap();
// event.payload is back to plaintext
assert_eq!(event.payload.as_bytes(), b"secret data");
```

### Encrypt for Multiple Recipients

```rust
let mut event = /* ... */;
let (rec1, id1) = generate_keys();
let (rec2, id2) = generate_keys();

// Encrypt so that either id1 or id2 can decrypt
encrypt_event(&mut event, &[&rec1, &rec2]).unwrap();

// Either identity works
let mut copy = event.clone();
decrypt_event(&mut copy, &id1).unwrap();
assert_eq!(copy.payload.as_bytes(), original_plaintext);

let mut copy2 = event.clone();
decrypt_event(&mut copy2, &id2).unwrap();
assert_eq!(copy2.payload.as_bytes(), original_plaintext);
```

---

## Implementation Details

The `encrypt_event` function:

1. Converts `event.payload` into a `Vec<u8>` (the plaintext).
2. If `recipients.len() > 1`, calls `librage::encrypt_multiple(&plaintext, &keys)`.
3. If exactly one recipient, calls `librage::encrypt(&plaintext, &key)`.
4. Checks the response for success; if failure, maps the error.
5. Creates a new `EncryptedPayload` from the ciphertext and assigns it to `event.payload`.

The `decrypt_event` function:

1. Takes the current payload as ciphertext bytes.
2. Calls `librage::decrypt(&cipherbytes, identity)`.
3. Maps any error.
4. Creates a new `EncryptedPayload` with the plaintext and replaces the event payload.

The signature field of the event is untouched throughout both operations.

Private helper function:

```rust
fn map_librage_error(body: &Option<librage::ErrorBody>) -> PosVaultError
```

Extracts `code` and `message` from `librage`’s error body; if absent, returns `"unknown librage error"`.

---

## Dependencies

- `posvault_handler` – provides `Event`, `EncryptedPayload`, `PosVaultError`, `Signature`, etc.
- `librage` – the age encryption library (FFI through `librage`).

External (implied): `zeroize` is used inside `EncryptedPayload`.

---

## Testing

The crate includes thorough unit tests in `src/lib.rs` and integration tests in `tests/integration_tests.rs`. Coverage includes:

- Single and multiple recipient encryption/decryption round‑trips.
- Error on empty recipients.
- Error on invalid recipient key.
- Error on decryption with wrong key or garbage identity.
- Preservation of event signature after encryption/decryption.
- Small and large payloads (1 byte, 1 MB).

---

## Security Considerations

- **Recipient validation**: Ensure age public keys are obtained from trusted sources.
- **Identity handling**: Age identities are sensitive; this crate does not manage key storage—callers must keep identities secure.
- **Payload zeroisation**: `EncryptedPayload` wraps data in `Zeroizing<Vec<u8>>`, which clears memory on drop. However, after decryption the plaintext is stored inside a new `EncryptedPayload`; it is still zeroized when dropped.
- **No signing performed**: This crate only operates on the payload. Event signing is handled by `posvault_sign` elsewhere.

---

## Full Source Reference

The complete source for this crate’s public API is available in the project’s `src/lib.rs`. Below is a stripped copy for reference:

```rust
use posvault_handler::errors::{PosVaultError, Result};
use posvault_handler::types::{EncryptedPayload, Event};

pub fn encrypt_event(event: &mut Event, recipients: &[impl AsRef<str>]) -> Result<()> {
    if recipients.is_empty() {
        return Err(PosVaultError::Encryption("recipients must not be empty".into()));
    }
    let plaintext = event.payload.as_bytes().to_vec();
    let cipherbytes = if recipients.len() > 1 {
        let keys: Vec<&str> = recipients.iter().map(|s| s.as_ref()).collect();
        let response = librage::encrypt_multiple(&plaintext, &keys);
        if !response.success {
            return Err(map_librage_error(&response.error));
        }
        let data = response.data.ok_or_else(|| PosVaultError::Encryption("response missing ciphertext data".into()))?;
        data.ciphertext.to_vec()
    } else {
        let single_key = recipients.first().expect("recipients not empty");
        let response = librage::encrypt(&plaintext, single_key.as_ref());
        if !response.success {
            return Err(map_librage_error(&response.error));
        }
        let data = response.data.ok_or_else(|| PosVaultError::Encryption("response missing ciphertext data".into()))?;
        data.ciphertext.to_vec()
    };
    let encrypted = EncryptedPayload::new(cipherbytes)?;
    event.payload = encrypted;
    Ok(())
}

pub fn decrypt_event(event: &mut Event, identity: &str) -> Result<()> {
    let cipherbytes = event.payload.as_bytes().to_vec();
    let response = librage::decrypt(&cipherbytes, identity);
    if !response.success {
        return Err(map_librage_error(&response.error));
    }
    let data = response.data.ok_or_else(|| PosVaultError::Encryption("response missing plaintext data".into()))?;
    let plaintext = data.plaintext.to_vec();
    let decrypted = EncryptedPayload::new(plaintext)?;
    event.payload = decrypted;
    Ok(())
}

fn map_librage_error(body: &Option<librage::ErrorBody>) -> PosVaultError {
    body.as_ref()
        .map(|b| PosVaultError::Encryption(format!("{}: {}", b.code, b.message)))
        .unwrap_or_else(|| PosVaultError::Encryption("unknown librage error".into()))
}
```

For any questions or further details, refer to the integration tests or the `posvault_handler` crate documentation.
