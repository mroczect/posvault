# posvault_store – Full API Reference

**Complete documentation for the `posvault_store` crate**  
Provides concrete `FileStore`-backed implementations of `EventStore`, `Journal`, `SnapshotStore`, and the main `PosVault` repository handle.

---

## Overview

The `posvault_store` crate is the **storage layer** for the PosVault ecosystem. It uses the `libvctrl` file‑backed object store (`FileStore`) to persist events, journal entries, and snapshots in a **content‑addressed, Git‑like** data structure. All implementations are thread‑safe (using `Arc<Mutex<FileStore>>`) and follow the traits defined in `posvault_handler`.

Key features:

- **`PosVault`** – opens (or creates) a repository at a given path and provides a shared reference to the underlying `FileStore`. This reference is then used to construct the specialised stores.
- **`VctrlEventStore`** – stores events in bucketed Merkle trees, with checkpoints that map directly to the event counter.
- **`VctrlJournal`** – an append‑only journal with automatic compaction when the number of unarchived entries exceeds a configurable threshold.
- **`VctrlSnapshotStore`** – saves and loads snapshots, retaining all versions under timestamped references and always keeping a `refs/snapshots/latest` pointer.

All public items are re‑exported from the crate root:

```rust
use posvault_store::{
    PosVault,
    VctrlEventStore,
    VctrlJournal,
    VctrlSnapshotStore,
};
```

---

## Modules & Re‑exports

The crate contains four public modules:

| Module           | Primary type(s)                                   |
| ---------------- | ------------------------------------------------- |
| `posvault`       | `PosVault` – repository handle                    |
| `event_store`    | `VctrlEventStore` – implements `EventStore`       |
| `journal`        | `VctrlJournal` – implements `Journal`             |
| `snapshot_store` | `VctrlSnapshotStore` – implements `SnapshotStore` |

All types are re‑exported at the crate root via `pub use` in `lib.rs`.

---

## Core Types

### `PosVault`

```rust
pub struct PosVault {
    pub(crate) store: Arc<Mutex<FileStore>>,
    pub(crate) path: PathBuf,
}
```

Represents a PosVault repository. It wraps an `Arc<Mutex<FileStore>>` for shared access and the directory path. This struct is the entry point for creating the underlying store and obtaining references to it.

#### Methods

##### `open`

```rust
pub fn open(path: impl AsRef<Path>) -> Result<Self>
```

Opens an existing repository at `path`, or creates a new one if it does not exist.

- If the directory does not exist, it is created (including parent directories) using `std::fs::create_dir_all`.
- A `FileStore` is opened on `path.join("store.vctrl")`.
- If the store is empty (no HEAD ref), an initial commit is created with an empty tree, and the branch `refs/heads/main` is set as HEAD.
- Returns a `PosVault` containing the `Arc<Mutex<FileStore>>` and the canonicalised path.

**Errors**:

- `PosVaultError::Storage` if directory creation fails or `FileStore::open` fails.
- Errors from `init_store` (unlikely; only possible if the empty tree or commit encoding fails).

##### `store_arc`

```rust
pub fn store_arc(&self) -> Arc<Mutex<FileStore>>
```

Returns a new `Arc` clone of the internal `FileStore` handle. Use this to create `VctrlEventStore`, `VctrlJournal`, or `VctrlSnapshotStore`.

##### `store_ref`

```rust
#[doc(hidden)]
pub fn store_ref(&self) -> &Arc<Mutex<FileStore>>
```

Returns a reference to the inner `Arc`. This is marked `#[doc(hidden)]` and intended for internal or test use only; it allows borrowing without cloning.

---

### `VctrlEventStore`

```rust
pub struct VctrlEventStore {
    store: Arc<Mutex<FileStore>>,
}
```

Implements `posvault_handler::traits::EventStore` using a `FileStore` with a Merkle‑tree based indexing scheme.

#### Constructor

- **`VctrlEventStore::new(store: Arc<Mutex<FileStore>>) -> Self`**  
  Creates a new event store over the given `FileStore` handle.

#### Implementation Details

Events are stored in **buckets** of 1000 events (`BUCKET_SIZE`). The root tree contains:

- A `checkpoint` blob storing the current event counter (as 8 big‑endian bytes).
- Bucket trees named `events-{bucket_number}`. Each bucket tree holds entries `{index:016x}` pointing to the serialised event blob.

The **checkpoint** acts as the event counter – after appending one event, the checkpoint increments by one. The `latest_checkpoint` method reads this blob; if none exists, it returns 0.

#### `EventStore` Trait Implementation

##### `append_event`

```rust
fn append_event(&mut self, event: Event) -> Result<()>
```

1. Locks the store mutex.
2. Reads the current checkpoint and increments it.
3. Serialises the event to JSON and stores it as a blob.
4. Updates the checkpoint blob.
5. Creates or updates the corresponding bucket tree by adding a new entry with the hex‑encoded index.
6. Builds a new root tree containing the updated bucket and checkpoint.
7. Creates a new commit with message `append event #{new_counter}` and updates the `refs/heads/main` reference and HEAD.

The commit author is built from the event’s author fingerprint and role, with email `posvault@internal`.

##### `get_events_since`

```rust
fn get_events_since(&self, checkpoint: u64) -> Result<Vec<Event>>
```

1. Locks the store and reads the HEAD commit and root tree.
2. Iterates over all root entries whose name starts with `"events-"`.
3. Parses the bucket number; skips buckets where `bucket < checkpoint / BUCKET_SIZE`.
4. For each remaining bucket tree, iterates over blob entries whose hex‑decoded index is `> checkpoint`.
5. Deserialises each blob into an `Event` and collects them with their index.
6. Sorts the result by index and returns a vector of events.

Events are returned in index order (chronological insertion order).

##### `latest_checkpoint`

```rust
fn latest_checkpoint(&self) -> Result<u64>
```

Locks the store and reads the checkpoint blob from the HEAD tree. Returns `0` if no checkpoint exists yet.

#### Private Helpers

- `serialize_counter(counter: u64) -> Vec<u8>` – big‑endian bytes.
- `deserialize_counter(data: &[u8]) -> Result<u64>` – parses big‑endian bytes.

---

### `VctrlJournal`

```rust
pub struct VctrlJournal {
    store: Arc<Mutex<FileStore>>,
    compaction_threshold: u64,
}
```

Implements `posvault_handler::traits::Journal`. It keeps a separate branch `refs/journal` whose tree contains individual `journal-{id}` blobs and optionally `archive-{seq}` blobs of compacted entries.

#### Constructors

- **`VctrlJournal::new(store: Arc<Mutex<FileStore>>) -> Self`**  
  Creates a journal with the default compaction threshold (`JOURNAL_COMPACTION_THRESHOLD`, 100,000).

- **`VctrlJournal::with_threshold(store: Arc<Mutex<FileStore>>, threshold: u64) -> Self`**  
  Creates a journal with a custom threshold. Used for testing with smaller values.

#### `Journal` Trait Implementation

##### `record`

```rust
fn record(&mut self, entry: JournalEntry) -> Result<()>
```

1. Locks the store and ensures the journal branch exists (creates it if missing, with an initial empty tree commit).
2. Reads the current journal tree.
3. Serialises the entry to JSON and stores it as a blob named `journal-{entry.id}`.
4. Creates a new tree with the added entry.
5. Builds a new commit (author from the entry’s fingerprint, action as the commit message) and updates `refs/journal`.
6. After recording, calls `maybe_compact` to check if the number of unarchived entries exceeds the threshold.

##### `read_all`

```rust
fn read_all(&self) -> Result<Vec<JournalEntry>>
```

1. Locks the store and reads the journal branch HEAD.
2. Reads the tree and collects:
   - Blobs with names starting with `"archive-"` → deserialised as `Vec<JournalEntry>` (compacted batches).
   - Blobs with names starting with `"journal-"` → individual `JournalEntry`.
3. Merges all entries, sorting by timestamp and then by ID.
4. Returns the sorted list. If the journal branch does not exist, returns an empty vector.

#### Compaction

- **`compact(store: &mut FileStore) -> Result<()>`**  
  Collects all unarchived entries (those not in `archive-` blobs), sorts them, and serialises them into a single `archive-{seq}` blob. The original individual blobs are removed from the tree (by creating a new tree without them) and a compaction commit is recorded.

- **`maybe_compact(store: &mut FileStore, threshold: u64) -> Result<()>`**  
  Counts unarchived entries; if the count > threshold, calls `compact`.

Compaction ensures the journal tree does not grow indefinitely with many small blobs.

---

### `VctrlSnapshotStore`

```rust
pub struct VctrlSnapshotStore {
    store: Arc<Mutex<FileStore>>,
}
```

Implements `posvault_handler::traits::SnapshotStore`. Snapshots are stored as blobs referenced by timestamped references under `refs/snapshots/`.

#### Constructor

- **`VctrlSnapshotStore::new(store: Arc<Mutex<FileStore>>) -> Self`**  
  Wraps the given `FileStore` handle.

#### `SnapshotStore` Trait Implementation

##### `save_snapshot`

```rust
fn save_snapshot(&mut self, snapshot: Snapshot) -> Result<()>
```

1. Locks the store.
2. Serialises the `Snapshot` to JSON and creates a blob.
3. Stores the blob in the object store.
4. Creates two references:
   - `refs/snapshots/{timestamp}` → the blob hash (timestamp is current Unix seconds).
   - `refs/snapshots/latest` → the blob hash (always points to the most recent snapshot).
5. Does **not** delete older snapshots; all versions are retained.

##### `load_snapshot`

```rust
fn load_snapshot(&self) -> Result<Option<Snapshot>>
```

1. Locks the store.
2. Looks up `refs/snapshots/latest`. If it does not exist, returns `None`.
3. Retrieves the blob and deserialises it into a `Snapshot`.
4. Returns `Some(snapshot)`.

No snapshot exists initially; the first call to `save_snapshot` creates the `latest` reference.

---

## Error Handling

All methods return `posvault_handler::errors::Result<T>`. Errors are drawn from `PosVaultError` variants:

| Variant         | Common causes in this crate                                                             |
| --------------- | --------------------------------------------------------------------------------------- |
| `Storage`       | Mutex lock poisoning, file I/O errors, missing objects (corruption).                    |
| `NotFound`      | Missing HEAD commit, journal branch, snapshot reference, or checkpoint blob.            |
| `Serialization` | JSON serialisation/deserialisation failures (malformed event/journal data).             |
| `InvalidInput`  | (rare; not directly produced by store, but can propagate from constructor validations). |

Specific error messages include:

- `"HEAD not found"` – when the main branch is missing.
- `"checkpoint data corrupt"` – when the checkpoint blob is less than 8 bytes.
- `"journal ref not found"` – when the journal branch is missing (before initialisation).
- Errors from `serde_json` are wrapped in `Serialization`.

---

## Examples

### Opening a Repository and Creating Event/Journal/Snapshot Stores

```rust
use posvault_store::{
    PosVault, VctrlEventStore, VctrlJournal, VctrlSnapshotStore,
};
use std::sync::Arc;

let vault = PosVault::open("./my_vault").expect("Failed to open vault");
let store_handle = vault.store_arc(); // Arc<Mutex<FileStore>>

let mut event_store = VctrlEventStore::new(Arc::clone(&store_handle));
let mut journal = VctrlJournal::new(Arc::clone(&store_handle));
let mut snapshot_store = VctrlSnapshotStore::new(Arc::clone(&store_handle));
```

### Appending an Event and Reading Events Since a Checkpoint

```rust
use posvault_handler::types::*;

let event = Event::new(
    EventId::generate(),
    100,
    Identity::new(Fingerprint::new("a".repeat(64)).unwrap(), Role::Cashier),
    EncryptedPayload::new(b"payload".to_vec()).unwrap(),
    Signature::new(vec![0u8; 64]).unwrap(),
).unwrap();

event_store.append_event(event.clone()).unwrap();
assert_eq!(event_store.latest_checkpoint().unwrap(), 1);

let recent = event_store.get_events_since(0).unwrap();
assert_eq!(recent.len(), 1);
```

### Recording and Reading Journal Entries

```rust
let entry = JournalEntry::new(
    EventId::generate(),
    200,
    "user.login".into(),
    Identity::new(Fingerprint::new("b".repeat(64)).unwrap(), Role::Admin),
    "success".into(),
    Signature::new(vec![0u8; 64]).unwrap(),
).unwrap();

journal.record(entry).unwrap();
let all_entries = journal.read_all().unwrap();
assert!(!all_entries.is_empty());
```

### Saving and Loading a Snapshot

```rust
let snapshot = Snapshot::new(
    1,
    EncryptedPayload::new(b"snapshot_data".to_vec()).unwrap(),
    CommitHash::from_bytes([1u8; 64]),
).unwrap();

snapshot_store.save_snapshot(snapshot.clone()).unwrap();
let loaded = snapshot_store.load_snapshot().unwrap();
assert_eq!(loaded, Some(snapshot));
```

---

## Dependencies

- `posvault_handler` – traits and types (`Event`, `Snapshot`, `JournalEntry`, errors).
- `libvctrl` – `FileStore`, `Blob`, `Commit`, `Tree`, `BinaryEncoder`, hashing, storage traits.
- `serde_json` – serialisation of events, journal entries, and snapshots.
- `std::sync` – `Arc`, `Mutex` for thread‑safe store sharing.

All stores operate on the same underlying `FileStore` (via `Arc`), enabling consistent cross‑store operations (e.g., event store can be read while snapshot store is updated) with proper synchronisation.

---

## Full Source Reference

The public API is contained in the four modules listed above. All implementations are generic over the inner store (`FileStore`) and are designed to be composable with higher‑level wrappers (e.g., `SignedEventStore` from `posvault_sign`).

For exact method signatures and internal logic, refer to the source files:

- `src/posvault.rs` – `PosVault`
- `src/event_store.rs` – `VctrlEventStore` and `EventStore` impl
- `src/journal.rs` – `VctrlJournal` and `Journal` impl
- `src/snapshot_store.rs` – `VctrlSnapshotStore` and `SnapshotStore` impl

All crate items are exported via `lib.rs`.

---

_End of `posvault_store` API Reference._
