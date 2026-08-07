# posvault

> Encrypted, version‑controlled POS database with RBAC and 2FA

**posvault** is a complete, event‑sourced Point‑of‑Sale backend built in Rust. It combines strong encryption, Git‑like version control, role‑based access control, and two‑factor authentication into a single, embeddable database.

- **Encrypted** – every event payload is encrypted with [age](https://github.com/FiloSottile/age) using one or more recipients.
- **Version‑controlled** – data is stored as a series of signed, immutable events in a content‑addressed Merkle DAG, thanks to [libvctrl](https://github.com/libvctrl/libvctrl). You get branching, snapshots, and history for free.
- **RBAC** – fine‑grained roles (`Admin`, `Manager`, `Cashier`, …) guard every operation.
- **2FA** – login requires a passphrase **and** a time‑based one‑time password (TOTP).

---

## Features

- **Event Sourcing** – all state changes are captured as append‑only events. Current state is derived by replaying and snapshotting.
- **Encrypted Payloads** – events are encrypted with age (X25519 or passphrase) before being committed. No plain‑text ever hits disk.
- **Authenticated & Signed** – every event and journal entry is signed with Ed25519. Optional strict/loose verification.
- **Snapshot & Query** – a built‑in query engine materialises the latest state from snapshots and recent events.
- **Journal (Audit Trail)** – a separate append‑only journal records every action for compliance.
- **Branches & Sync** – create store‑specific branches, switch between them, and push/pull entire repositories to a remote filesystem.
- **Strong Typing** – all core types (`Event`, `Snapshot`, `JournalEntry`, …) are validated on construction.
- **Modular Crates** – use the high‑level `posvault` facade or pick individual crates depending on your needs.

---

## Architecture

The project is split into several Rust crates, each with a clear responsibility:

| Crate              | Description                                                                                                                                |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `posvault`         | **Umbrella facade**. Re‑exports everything and provides a high‑level `PosVault` struct that ties together storage, auth, crypto, and sync. |
| `posvault_handler` | Core types, errors, traits, and macros shared across all crates.                                                                           |
| `posvault_auth`    | 2FA login, sessions, and role‑based guard (`require_role`).                                                                                |
| `posvault_crypto`  | Encrypt/decrypt event payloads with age (single or multiple recipients).                                                                   |
| `posvault_query`   | Event‑sourced query engine with snapshot caching. Includes example stock management.                                                       |
| `posvault_sign`    | Ed25519 signer and decorators that automatically sign events/journal entries.                                                              |
| `posvault_store`   | Concrete storage layer using `libvctrl`'s content‑addressed file store. Implements `EventStore`, `Journal`, `SnapshotStore`.               |
| `posvault_sync`    | Branch management (`create_store_branch`, `checkout_branch`), a CSV union resolver, and a file‑based transport for syncing.                |

All crates are designed to be used together via the `posvault` re‑export, but you can also compose them directly.

---

## Getting Started

### Prerequisites

- Rust 1.70+ (stable)
- A working C compiler (for building `libvctrl` and its dependencies)

### Installation

Add the top‑level crate to your `Cargo.toml`:

```toml
[dependencies]
posvault = "0.1.0"   # use latest version from crates.io
```

Or, if you only need a specific component:

```toml
posvault_handler = "0.1.0"
posvault_store   = "0.1.0"
# etc.
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
    vault.login(&my_backend, "cashier@example.com", "pass", "123456", "JBSWY3DPEHPK3PXP")?;

    // 5. Append an event
    let event = Event::new(
        EventId::generate(),
        1700000000,
        Identity::new(Fingerprint::new("a".repeat(64))?, Role::Cashier),
        EncryptedPayload::new(b"sale: 2x coffee".to_vec())?,
        Signature::new(vec![0u8; 64])?,
    )?;
    vault.transact(event)?;

    // 6. Sync to a remote directory
    vault.sync_to_remote("/backup/remote-vault")?;

    Ok(())
}
```

For a complete walkthrough of the stock inventory example, see the `posvault_query` documentation.

---

## Documentation

Full API reference for every crate is available:

- [`posvault` (umbrella)][posvault-docs]
- [`posvault_handler`][handler-docs]
- [`posvault_auth`][auth-docs]
- [`posvault_crypto`][crypto-docs]
- [`posvault_query`][query-docs]
- [`posvault_sign`][sign-docs]
- [`posvault_store`][store-docs]
- [`posvault_sync`][sync-docs]

Or browse the source in the [repository](https://github.com/your-org/posvault).

---

## Security

- **Passphrases & keys** are zeroised in memory after use.
- **Encrypted payloads** are opaque on disk; plaintext is only visible after successful decryption.
- **Event signatures** prevent tampering and are verified on read (optional strict mode).
- **Two‑factor authentication** protects login even if the passphrase is compromised.
- **Role‑based access** ensures only authorised users can mutate data.

**Important:** This project is under active development. It has **not** been audited by a third party. Use it at your own risk.

---

## Contributing

Pull requests are welcome! Please open an issue first to discuss what you’d like to change.  
Make sure to add tests for new functionality and run `cargo test --workspace`.

---

## License

This project is licensed under the [MIT License](LICENSE).  
See the `LICENSE` file for full details.