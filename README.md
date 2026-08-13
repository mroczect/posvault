# posvault

> Encrypted, version-controlled POS database with RBAC and 2FA

**posvault** is a complete, event-sourced Point-of-Sale backend built in Rust. It combines strong encryption, Git-like version control, role-based access control, and two-factor authentication into a single, embeddable database.

- **Encrypted** – every event payload is encrypted with [age](https://github.com/FiloSottile/age) using one or more recipients.
- **Version-controlled** – data is stored as a series of signed, immutable events in a content-addressed Merkle DAG, thanks to [libvctrl](https://github.com/libvctrl/libvctrl). Branching, snapshots, and history are provided by the underlying storage.
- **RBAC** – fine-grained roles (`Admin`, `Manager`, `Cashier`, `Auditor`, `Branch`, `Custom`) guard every mutation.
- **2FA** – login requires a passphrase **and** a time-based one-time password (TOTP).

---

## Status

The workspace currently compiles cleanly, passes all tests, and satisfies strict Clippy lints. However, the project is under active development and has **not** been audited by a third party. Some components are still placeholders:

- `FileStore` is currently in-memory (no disk persistence yet).
- `pull_and_merge` is not implemented safely and returns a descriptive error.
- Doc-tests are not yet written; public API documentation remains sparse in a few crates.

Use at your own risk in production.

---

## Features

- **Event Sourcing** – all state changes are captured as append-only events. Current state is derived by replaying and snapshotting.
- **Encrypted Payloads** – events are encrypted with age (X25519 or passphrase) before being committed. No plaintext event data is written to storage.
- **Authenticated & Signed** – every event and journal entry can be signed with Ed25519. Strict and loose verification modes are supported.
- **Snapshot & Query** – a built-in query engine materialises the latest state from snapshots and recent events. A stock inventory example is included.
- **Journal (Audit Trail)** – a separate append-only journal records every action for compliance and auditing.
- **Branches & Sync** – create store-specific branches, switch between them, and push/pull entire repositories to a remote filesystem using `FileTransport`.
- **Strong Typing** – all core types (`Event`, `Snapshot`, `JournalEntry`, `Signature`, etc.) are validated on construction.
- **Modular Crates** – use the high-level `posvault` facade or pick individual crates depending on your needs.

---

## Architecture

The project is split into several Rust crates, each with a clear responsibility:

| Crate              | Description                                                                                                                                |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `posvault`         | **Umbrella facade**. Re-exports everything and provides a high-level `PosVault` struct that ties together storage, auth, crypto, and sync. |
| `posvault_handler` | Core types, errors, traits, and macros shared across all crates.                                                                           |
| `posvault_auth`    | 2FA login, sessions, and role-based guard (`require_role`).                                                                                |
| `posvault_crypto`  | Encrypt/decrypt event payloads with age (single or multiple recipients).                                                                   |
| `posvault_query`   | Event-sourced query engine with snapshot caching. Includes example stock management.                                                       |
| `posvault_sign`    | Ed25519 signer and decorators that automatically sign events/journal entries.                                                              |
| `posvault_store`   | Concrete storage layer using `libvctrl`'s content-addressed file store. Implements `EventStore`, `Journal`, `SnapshotStore`.               |
| `posvault_sync`    | Branch management (`create_store_branch`, `checkout_branch`), a CSV union resolver, and a file-based transport for syncing.                |

All crates are designed to be used together via the `posvault` re-export, but you can also compose them directly.

---

## Getting Started

### Prerequisites

- Rust stable (latest tested version: 1.96.0)
- A working C compiler (for building `libvctrl` and its dependencies)

### Installation

Add the top-level crate to your `Cargo.toml`:

```toml
[dependencies]
posvault = "0.2.3"
```

Or, if you only need a specific component:

```toml
posvault_handler = "1.0.0"
posvault_store   = "1.0.0"
posvault_auth    = "0.2.3"
```

### Quick Example

```rust
use posvault::*;

fn main() -> Result<()> {
    // 1. Open or create a vault
    let mut vault = PosVault::open("./my-vault")?;

    // 2. Set encryption recipients (age public keys)
    vault.set_recipients(vec!["age1...".to_string()]);

    // 3. Set a signer
    let (signing_key, _vkey) = generate_keypair();
    vault.set_signer(Ed25519Signer::new(signing_key));

    // 4. Login (requires a backend implementing AccountBackend)
    vault.login(
        &my_backend,
        "cashier@example.com",
        "passphrase",
        "123456",
        "JBSWY3DPEHPK3PXP",
    )?;

    // 5. Append an event
    let event = Event::new(
        EventId::generate(),
        1_700_000_000,
        Identity::new(Fingerprint::new("a".repeat(64))?, Role::Cashier),
        EncryptedPayload::new(b"sale: 2x coffee".to_vec())?,
        Signature::new(vec![0u8; 64])?,
    )?;
    vault.transact(event)?;

    // 6. Record a journal entry
    let entry = JournalEntry::new(
        EventId::generate(),
        1_700_000_000,
        "user.login".to_string(),
        Identity::new(Fingerprint::new("a".repeat(64))?, Role::Admin),
        "details".to_string(),
        Signature::new(vec![0u8; 64])?,
    )?;
    vault.journal(entry)?;

    // 7. Sync to a remote directory
    vault.sync_to_remote("/backup/remote-vault")?;

    Ok(())
}
```

For a complete walkthrough of the stock inventory example, see the `posvault_query` documentation.

---

## Documentation

Full API reference for every crate is available:

- [`posvault` (umbrella)](https://docs.rs/posvault)
- [`posvault_handler`](https://docs.rs/posvault_handler)
- [`posvault_auth`](https://docs.rs/posvault_auth)
- [`posvault_crypto`](https://docs.rs/posvault_crypto)
- [`posvault_query`](https://docs.rs/posvault_query)
- [`posvault_sign`](https://docs.rs/posvault_sign)
- [`posvault_store`](https://docs.rs/posvault_store)
- [`posvault_sync`](https://docs.rs/posvault_sync)

Or browse the source in the [repository](https://github.com/your-org/posvault).

---

## Security

- **Passphrases & keys** are zeroised in memory after use (`zeroize`).
- **Encrypted payloads** are opaque on disk; plaintext is only visible after successful decryption.
- **Event signatures** prevent tampering and are verified on read (optional strict mode).
- **Two-factor authentication** protects login even if the passphrase is compromised.
- **Role-based access** ensures only authorised users can mutate data.

**Important:** This project is under active development. It has **not** been audited by a third party. Use it at your own risk.

---

## Known Limitations

- `FileStore` is backed by in-memory storage (`MemoryStore` + `MemoryRefStore`), so data is lost when the process exits.
- `pull_and_merge` is not implemented; it always returns a `Sync` error.
- Cryptographic verification is not constant-time on all platforms (see `posvault_sign` for details).
- The root `PosVault` facade assumes an `AccountBackend` from `age_credentials` for authentication, which is not included by default.

---

## Contributing

Pull requests are welcome! Please open an issue first to discuss what you’d like to change.  
Make sure to add tests for new functionality and run:

```bash
make fmt
make ci
```

For local CI, the `Makefile` provides:

- `make fmt` – format all code.
- `make clippy` – run strict lints.
- `make test` – run all workspace tests.
- `make ci` – run fmt, clippy, and test.

---

## License

This project is licensed under the [MIT License](LICENSE).  
See the `LICENSE` file for full details.
