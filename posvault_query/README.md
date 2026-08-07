# posvault_query – Full API Reference

**Complete crate documentation for `posvault_query`**  
Event‑sourced query engine with snapshot caching and example application logic for stock management.

---

## Table of Contents

- [Overview](#overview)
- [Public API](#public-api)
  - [Modules & Re‑exports](#modules--re-exports)
  - [Structs](#structs)
    - [`SnapshotCache`](#snapshotcache)
    - [`QueryEngine<S>`](#queryengine-s)
  - [Example Functions & Types (`examples` module)](#example-functions--types-examples-module)
    - [`StockState`](#stockstate)
    - [`apply_stock_event`](#apply_stock_event)
    - [`get_stock`](#get_stock)
    - [`daily_sales`](#daily_sales)
- [How It Works](#how-it-works)
  - [Snapshot Rebuilding](#snapshot-rebuilding)
  - [Cache Management](#cache-management)
  - [Stock Management Example](#stock-management-example)
- [Error Handling](#error-handling)
- [Examples](#examples)
  - [Basic Query Engine Setup](#basic-query-engine-setup)
  - [Using the Stock Functions](#using-the-stock-functions)
- [Dependencies](#dependencies)
- [Full Source Reference](#full-source-reference)

---

## Overview

The `posvault_query` crate provides a generic query engine for event‑sourced applications. It builds and maintains materialised views (snapshots) by applying events to a state accumulator, using the `EventStore` and `SnapshotStore` traits from `posvault_handler`.

Key features:

- **Snapshot Caching** – a thread‑safe cache (`SnapshotCache`) avoids rebuilding the latest snapshot on every query.
- **Lazy Rebuild** – the `QueryEngine` only rebuilds a snapshot when the event store has new events since the last snapshot.
- **Pluggable Crypto** – encryption/decryption closures allow the engine to work with encrypted payloads.
- **Example Application** – the `examples` module shows how to use the engine for a simple stock inventory with functions like `apply_stock_event` and `get_stock`.

All public items are available from the crate root:

```rust
use posvault_query::{
    SnapshotCache, QueryEngine,
    apply_stock_event, get_stock, daily_sales, StockState,
};
```

---

## Public API

### Modules & Re‑exports

The crate defines three modules, all re‑exported at the root:

| Module     | Contents                                                              |
| ---------- | --------------------------------------------------------------------- |
| `cache`    | `SnapshotCache` – thread‑safe cache for the latest snapshot.          |
| `engine`   | `QueryEngine<S>` – generic engine that rebuilds and serves snapshots. |
| `examples` | Example stock management functions and the `StockState` type alias.   |

Because of the re‑export, you can use everything directly via `posvault_query::*`.

---

### Structs

#### `SnapshotCache`

```rust
pub struct SnapshotCache {
    inner: Mutex<Option<Snapshot>>,
}
```

A simple thread‑safe cache that holds at most one `Snapshot`. It is used internally by `QueryEngine` to avoid unnecessary rebuilds.

##### Methods

| Method                           | Description                                                              |
| -------------------------------- | ------------------------------------------------------------------------ |
| `SnapshotCache::new() -> Self`   | Creates an empty cache.                                                  |
| `get(&self) -> Option<Snapshot>` | Returns a clone of the cached snapshot, or `None` if the cache is empty. |
| `set(&self, snapshot: Snapshot)` | Stores a snapshot in the cache, replacing any previous entry.            |
| `invalidate(&self)`              | Clears the cache, setting it to `None`.                                  |

`SnapshotCache` also implements `Default` (via `Default::default()`, which returns `SnapshotCache::new()`).

---

#### `QueryEngine<S>`

```rust
pub struct QueryEngine<S: EventStore + SnapshotStore> {
    store: S,
    cache: SnapshotCache,
}
```

A generic event‑sourced query engine parameterised by a store that implements both `EventStore` and `SnapshotStore`. It uses a `SnapshotCache` to hold the most recent snapshot and can rebuild it on demand.

##### Constructor

- **`QueryEngine::new(store: S) -> Self`**  
  Creates a new engine with an empty cache and the given combined event/snapshot store.

##### Methods

##### `rebuild_snapshot`

```rust
pub fn rebuild_snapshot<F, D, E>(
    &mut self,
    decrypt: D,
    encrypt: E,
    apply_event: F,
) -> Result<Snapshot>
where
    F: Fn(&mut Vec<u8>, &[u8]) -> Result<()>,
    D: Fn(&[u8]) -> Result<Vec<u8>>,
    E: Fn(&[u8]) -> Result<Vec<u8>>,
```

Rebuilds the latest snapshot from the event store by:

1. Loading the most recent snapshot (if any) from `self.store.load_snapshot()`.
2. Decrypting its payload using `decrypt` to obtain the initial state bytes.
3. Fetching all events since the snapshot’s version (or from the beginning if no snapshot exists) via `self.store.get_events_since(start_checkpoint)`.
4. For each event, decrypting its payload and applying it to the state using the `apply_event` closure.
5. Encrypting the final state with `encrypt`.
6. Computing a `CommitHash` over the latest checkpoint and the encrypted state.
7. Storing the new snapshot in the store and in the cache.
8. Returning the new snapshot.

If there are **no events** at all (`latest_checkpoint == 0`), a dummy snapshot with a hard‑coded hash (`CommitHash::from_bytes([1u8; 64])`) and version `0` is returned, to avoid panics in the caller.

**Parameters:**

| Closure       | Signature                             | Role                                                                                                                                                                                   |
| ------------- | ------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `decrypt`     | `(&[u8]) -> Result<Vec<u8>>`          | Decrypts an `EncryptedPayload`’s bytes (ciphertext → plaintext).                                                                                                                       |
| `encrypt`     | `(&[u8]) -> Result<Vec<u8>>`          | Encrypts the final state bytes (plaintext → ciphertext) before storing the snapshot.                                                                                                   |
| `apply_event` | `(&mut Vec<u8>, &[u8]) -> Result<()>` | Applies a decrypted event payload to the in‑memory state (`&mut Vec<u8>`). The state starts as the decrypted snapshot (or empty). The second parameter is the decrypted event payload. |

**Returns** `Result<Snapshot>` – the newly built snapshot, which is also cached and persisted.

##### `get_cached_snapshot`

```rust
pub fn get_cached_snapshot(&self) -> Option<Snapshot>
```

Returns a clone of the currently cached snapshot, if any.

##### `needs_rebuild`

```rust
pub fn needs_rebuild(&self) -> Result<bool>
```

Checks whether the cache is outdated. Returns `true` if:

- The cache is empty, or
- The event store’s `latest_checkpoint()` is greater than the cached snapshot’s `version` (i.e., there are new events).

This is used to decide whether to call `rebuild_snapshot` before serving a query.

##### `invalidate_cache`

```rust
pub fn invalidate_cache(&self)
```

Clears the snapshot cache. The next call to `needs_rebuild` will return `true`, causing a full rebuild on the next `rebuild_snapshot`.

---

### Example Functions & Types (`examples` module)

The `examples` module demonstrates how to use `QueryEngine` for a simple stock inventory. It is not a separate binary; its functions are public and intended for reuse in application code.

#### `StockState`

```rust
pub type StockState = HashMap<String, u64>;
```

A type alias representing the entire stock state: a mapping from item names to their quantities.

#### `apply_stock_event`

```rust
pub fn apply_stock_event(state: &mut Vec<u8>, payload: &[u8]) -> Result<()>
```

An `apply_event` closure suitable for `rebuild_snapshot`. It:

1. Deserialises the current state from `state` (if non‑empty) into a `StockState` (a `HashMap<String, u64>`).
2. Deserialises the event `payload` as a `(String, i64)` tuple representing an item name and a delta (`+` for add, `-` for subtract).
3. Updates the stock map accordingly: adds the delta if positive; subtracts and checks for underflow if negative.
4. Reserialises the updated `StockState` back into `state`.

Returns `PosVaultError::InvalidInput("insufficient stock")` if a subtraction would make the quantity negative.

#### `get_stock`

```rust
pub fn get_stock<S: EventStore + SnapshotStore>(
    engine: &mut QueryEngine<S>,
    decrypt: &dyn Fn(&[u8]) -> Result<Vec<u8>>,
    encrypt: &dyn Fn(&[u8]) -> Result<Vec<u8>>,
    item: &str,
) -> Result<u64>
```

A convenience function to query the current stock of a specific item. It:

1. Checks if the engine needs a rebuild (`engine.needs_rebuild()?`); if so, calls `engine.rebuild_snapshot(decrypt, encrypt, apply_stock_event)`.
2. Retrieves the cached snapshot.
3. Decrypts its payload to obtain the serialised `StockState`.
4. Deserialises it and returns the quantity for `item`, defaulting to `0` if the item is not present.

**Parameters:**

- `engine` – a mutable reference to a `QueryEngine<S>`.
- `decrypt` – same as in `rebuild_snapshot`.
- `encrypt` – same as in `rebuild_snapshot`.
- `item` – the item name to look up.

**Returns** the current stock quantity.

#### `daily_sales`

```rust
pub fn daily_sales<S: EventStore + SnapshotStore>(
    _engine: &mut QueryEngine<S>,
    _decrypt: &dyn Fn(&[u8]) -> Result<Vec<u8>>,
    _encrypt: &dyn Fn(&[u8]) -> Result<Vec<u8>>,
    _date: &str,
) -> Result<u64>
```

A placeholder for a function that would compute daily sales. Currently unimplemented – always panics with “not implemented yet”.

---

## How It Works

### Snapshot Rebuilding

The `QueryEngine` implements a classic event‑sourcing pattern:

1. Load the last saved snapshot (if any) from the snapshot store.
2. Load all events that occurred after the snapshot’s version.
3. Apply each event to the snapshot’s decrypted state, producing a new state.
4. Encrypt the new state, compute a hash, and save it as a new snapshot.
5. Cache the new snapshot for immediate reuse.

This approach avoids replaying the entire event history every time a query is made.

### Cache Management

The `SnapshotCache` is a thin wrapper around a `Mutex<Option<Snapshot>>`. It is:

- **Set** after a successful rebuild.
- **Retrieved** via `get_cached_snapshot()` for quick access.
- **Invalidated** when the application knows the underlying store has changed externally, or when a new event has been appended (though normally calling `rebuild_snapshot` or `needs_rebuild` will detect new events automatically).
- **Checked** by `needs_rebuild()` to determine if the cached version is behind the event store’s latest checkpoint.

### Stock Management Example

The `examples` module shows a concrete use case:

- Each event payload is a JSON tuple `("item_name", delta)`, e.g., `["apple", 10]` adds 10 apples, `["apple", -2]` removes 2 apples.
- The state is a JSON object `{"apple": 8, "banana": 5}`.
- `apply_stock_event` parses the event, updates the map, and returns the updated state.
- `get_stock` checks the cache, triggers a rebuild if necessary, and looks up the item.

This pattern can be adapted to any domain by providing different `apply_event`, `decrypt`, and `encrypt` closures.

---

## Error Handling

All public functions return `posvault_handler::errors::Result<T>`. The crate uses errors from the `PosVaultError` enum, primarily:

- `Serialization` – when JSON serialisation/deserialisation fails inside `apply_stock_event` or `get_stock`.
- `InvalidInput` – when stock would become negative (`"insufficient stock"`).
- `NotFound` – when `get_cached_snapshot` is called but the cache is empty (in `get_stock`).
- `Storage` / `Encryption` – propagated from the underlying store or the encrypt/decrypt closures.

The `QueryEngine` methods can also return errors from `EventStore` and `SnapshotStore`.

---

## Examples

### Basic Query Engine Setup

```rust
use posvault_handler::traits::{EventStore, SnapshotStore};
use posvault_query::QueryEngine;

// Assume MyStore implements EventStore + SnapshotStore
let store = MyStore::new();
let mut engine = QueryEngine::new(store);
```

### Using the Stock Functions

```rust
use posvault_query::{QueryEngine, get_stock, apply_stock_event};
use posvault_handler::errors::Result;

fn query_apple_stock(engine: &mut QueryEngine<impl EventStore + SnapshotStore>) -> Result<u64> {
    let decrypt = |data: &[u8]| -> Result<Vec<u8>> { Ok(data.to_vec()) }; // no‑op for plaintext
    let encrypt = |data: &[u8]| -> Result<Vec<u8>> { Ok(data.to_vec()) }; // no‑op
    get_stock(engine, &decrypt, &encrypt, "apple")
}
```

If the event store is already populated with events, `get_stock` will automatically rebuild the snapshot on the first call and return the correct quantity.

For encrypted stores, replace `decrypt`/`encrypt` with calls to `posvault_crypto::decrypt_event`/`encrypt_event` or similar functions.

---

## Dependencies

- `posvault_handler` – provides `EventStore`, `SnapshotStore`, `Snapshot`, `Event`, `CommitHash`, `EncryptedPayload`, `PosVaultError`, `Result`.
- `libvctrl` – for `Hasher`, `Sha512Hasher` used in hash computation.
- `serde_json` – for serialisation in the example stock functions.
- Standard library: `std::sync::Mutex`, `std::collections::HashMap`.

---

## Full Source Reference

The public API is defined in the following files:

- `cache.rs` → `SnapshotCache`
- `engine.rs` → `QueryEngine<S>`
- `examples.rs` → `StockState`, `apply_stock_event`, `get_stock`, `daily_sales`
- `lib.rs` → re‑exports all of the above

For exact signatures, see the source snippets above.  
The entire crate is designed to be minimal and composable, allowing any `EventStore + SnapshotStore` combination to be used with the query engine.

---

_End of `posvault_query` API Reference._
