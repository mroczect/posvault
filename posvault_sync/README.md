# posvault_sync – Full API Reference

**Complete documentation for the `posvault_sync` crate**  
Provides branch management, a conflict resolver for CSV‑like data, and a file‑based transport for local store synchronisation.

---

## Overview

The `posvault_sync` crate adds synchronisation primitives to the PosVault ecosystem. It does **not** implement full network sync; instead it focuses on:

- **Branch management** – creating, switching, and querying store branches using the `RefStore` trait from `libvctrl`.
- **Conflict resolution** – a simple union‑based resolver (`UnionCsvResolver`) for line‑oriented data, designed for CSV‑like merge conflicts.
- **File transport** – copying a local store to a remote path (or vice versa) using `FileTransport`, which implements the `Transport` trait.
- **Pull & merge stub** – `pull_and_merge` is **not yet implemented** due to limitations in `libvctrl`; it always returns an error.

All public items are available from the crate root:

```rust
use posvault_sync::{
    create_store_branch, checkout_branch, current_branch,
    UnionCsvResolver,
    pull_and_merge,
    FileTransport,
};
```

---

## Modules & Re‑exports

The crate contains four public modules, all re‑exported at the root:

| Module      | Contents                                                   |
| ----------- | ---------------------------------------------------------- |
| `branch`    | `create_store_branch`, `checkout_branch`, `current_branch` |
| `resolver`  | `UnionCsvResolver`                                         |
| `sync`      | `pull_and_merge`                                           |
| `transport` | `FileTransport`                                            |

---

## Branch Management

These functions operate on any type that implements `libvctrl::storage::traits::RefStore` (i.e., the underlying reference store of a `FileStore`). They manage branches in a Git‑like fashion using `refs/heads/` prefixes.

### `create_store_branch`

```rust
pub fn create_store_branch(
    refs: &mut dyn RefStore,
    store_id: &str,
) -> Result<BranchName>
```

Creates a new branch named `store-{store_id}` (i.e., `refs/heads/store-<store_id>`) that points to the current HEAD commit, and then checks it out (sets HEAD to that branch).

#### Parameters

- `refs` – a mutable reference to a `RefStore` implementation (e.g., a `FileStore` behind a mutex).
- `store_id` – a string identifier for the store. The full branch name becomes `refs/heads/store-<store_id>`.

#### Returns

`Result<BranchName>` – a validated `BranchName` representing the new branch (without the `refs/heads/` prefix).

#### Errors

- `PosVaultError::NotFound("HEAD not found")` if the repository does not have a current HEAD.
- `PosVaultError::Storage` if any reference operation fails (e.g., setting refs).
- `BranchName::new` validation errors (e.g., invalid characters).

#### Side effects

- A new reference `refs/heads/store-{store_id}` is created pointing to the current HEAD.
- The repository’s HEAD is updated to point to the new branch.

---

### `checkout_branch`

```rust
pub fn checkout_branch(
    refs: &mut dyn RefStore,
    branch_name: &BranchName,
) -> Result<()>
```

Switches the repository’s HEAD to the given branch.

#### Parameters

- `refs` – mutable reference to the reference store.
- `branch_name` – a valid `BranchName` that will be prefixed with `refs/heads/` to form the full reference name.

#### Returns

`Result<()>` – `Ok(())` if the branch exists and HEAD was updated.

#### Errors

- `PosVaultError::NotFound(format!("branch '{}' not found", branch_name.as_str()))` if the branch does not exist.
- `PosVaultError::Storage` if reference operations fail.

#### Side effects

- The repository HEAD is updated to `refs/heads/{branch_name}`.

---

### `current_branch`

```rust
pub fn current_branch(refs: &dyn RefStore) -> Result<Option<BranchName>>
```

Queries the name of the currently checked‑out branch.

#### Parameters

- `refs` – an immutable reference to the reference store.

#### Returns

`Result<Option<BranchName>>` – `Some(BranchName)` if the HEAD reference name starts with `refs/heads/`, otherwise `None` (e.g., if HEAD is detached or missing). The branch name is stripped of the `refs/heads/` prefix.

#### Errors

- `PosVaultError::Storage` if querying the HEAD ref name fails.

---

## Conflict Resolution

### `UnionCsvResolver`

```rust
#[derive(Debug)]
pub struct UnionCsvResolver;
```

Implements the `ConflictResolver` trait from `posvault_handler`. It performs a **union merge** on line‑oriented data (e.g., CSV files) by combining lines from both versions while avoiding duplicates.

#### `ConflictResolver` Implementation

```rust
impl ConflictResolver for UnionCsvResolver {
    fn resolve(&self, base: &[u8], ours: &[u8], theirs: &[u8]) -> Result<Vec<u8>>
}
```

The resolve logic works as follows:

1. Split all three inputs (`base`, `ours`, `theirs`) into lines (using `std::str::from_utf8` and `.lines()`). Invalid UTF‑8 sequences are treated as empty strings (lossy).
2. **Fast‑forward detection**:
   - If `ours` is identical to `base`, the result is `theirs` unchanged.
   - If `theirs` is identical to `base`, the result is `ours` unchanged.
3. **Conflict resolution**:
   - Start with the lines from `ours`.
   - For each line in `theirs` that is **not already present** in `ours`, append it.
   - Join the merged lines with `\n` and return the byte vector.

This is a simple, conflict‑free union: it assumes that duplicate lines are equivalent and that order is not critical. It is primarily intended for CSV‑like data where each line is an independent record.

#### Parameters

- `base` – the common ancestor data.
- `ours` – our version.
- `theirs` – their version.

#### Returns

`Result<Vec<u8>>` – the resolved byte vector. This method never fails; it always returns `Ok(...)`.

#### Notes

- No validation or schema awareness is performed; it operates purely on lines.
- The resolver never returns `Err`; the `Result` return type is for compatibility with the trait.

---

## Synchronisation

### `pull_and_merge`

```rust
pub fn pull_and_merge(
    _local_store_path: &Path,
    _remote_store_path: &Path,
    _author: UserID,
) -> Result<()>
```

**Currently unimplemented.** Always returns:

```
Err(PosVaultError::Sync("pull_and_merge is not yet safe due to limitations in libvctrl".into()))
```

This function is a placeholder for a future full synchronisation routine that would merge remote changes into a local store with conflict resolution. As of now, use `FileTransport` for simple push/pull of entire store directories.

---

## Transport

### `FileTransport`

```rust
#[derive(Debug)]
pub struct FileTransport {
    local_store_path: PathBuf,
    remote_store_path: PathBuf,
}
```

Implements the `Transport` trait by recursively copying the entire store directory from the local path to the remote path (push) or vice versa (pull). This is the simplest possible synchronisation mechanism – it mirrors the whole `store.vctrl` directory and its contents.

#### Constructor

- **`FileTransport::new(local: impl AsRef<Path>, remote: impl AsRef<Path>) -> Self`**  
  Creates a new `FileTransport` with the given local and remote directory paths. The paths are stored as `PathBuf`s.

#### `Transport` Trait Implementation

##### `push`

```rust
fn push(&mut self, _refs: &[String]) -> Result<()>
```

Copies the **entire local store directory** (recursively) to the remote path.

- If the local path does not exist, returns `PosVaultError::NotFound("local store not found")`.
- On copy failure, wraps the I/O error in `PosVaultError::Sync("push failed: …")`.

**Note:** The `refs` parameter is currently ignored; the entire directory is always copied.

##### `pull`

```rust
fn pull(&mut self, _refs: &[String]) -> Result<()>
```

Copies the **entire remote store directory** to the local path.

- If the remote path does not exist, returns `PosVaultError::NotFound("remote store not found")`.
- On copy failure, wraps the error in `PosVaultError::Sync("pull failed: …")`.

Again, the `refs` parameter is ignored; everything is synced.

#### Internal Helper (private)

`copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()>` performs the actual copy. It:

- Creates destination directories as needed.
- Copies files using `std::fs::copy`.
- Recurses into subdirectories.
- If the source is a single file (not a directory), the destination parent directory is created before copying.

This function is not exposed publicly, but it is the core of `FileTransport`'s behaviour.

---

## Error Handling

All public functions return `posvault_handler::errors::Result<T>`. The following error variants are commonly used:

| Variant        | Scenarios                                                     |
| -------------- | ------------------------------------------------------------- |
| `NotFound`     | Missing HEAD, branch, local/remote store directory.           |
| `Storage`      | Reference store errors (e.g., `set_ref`, `get_ref` failures). |
| `Sync`         | Transport copy failures, `pull_and_merge` not implemented.    |
| `InvalidInput` | Propagated from `BranchName::new` if branch name is invalid.  |

`UnionCsvResolver::resolve` never returns an error; it always returns `Ok(...)`.

---

## Examples

### Branch Management

```rust
use posvault_sync::{create_store_branch, checkout_branch, current_branch};
use posvault_handler::types::BranchName;
use libvctrl::storage::traits::RefStore;

// Assume `store` is a &mut dyn RefStore (e.g., from a FileStore mutex guard)
let branch = create_store_branch(store, "tokomainan")?;
assert_eq!(branch.as_str(), "store-tokomainan");
assert_eq!(current_branch(store)?.as_deref(), Some("store-tokomainan"));

// Switch to another branch
let branch2 = create_store_branch(store, "cabang1")?;
checkout_branch(store, &branch)?;
assert_eq!(current_branch(store)?.as_deref(), Some("store-tokomainan"));
```

### Conflict Resolution with UnionCsvResolver

```rust
use posvault_sync::UnionCsvResolver;
use posvault_handler::traits::ConflictResolver;

let resolver = UnionCsvResolver;

let base = b"apple\nbanana";
let ours = b"apple\nbanana\ncherry";
let theirs = b"apple\nbanana\ndurian";

let merged = resolver.resolve(base, ours, theirs).unwrap();
let merged_str = String::from_utf8(merged).unwrap();
// Contains all lines: "apple", "banana", "cherry", "durian"
assert!(merged_str.contains("cherry"));
assert!(merged_str.contains("durian"));
```

### File Transport (Push/Pull)

```rust
use posvault_sync::FileTransport;
use posvault_handler::traits::Transport;
use std::fs;
use tempfile::TempDir;

let local_dir = TempDir::new().unwrap();
let remote_dir = TempDir::new().unwrap();

// Create a test file in the local directory
fs::write(local_dir.path().join("store.vctrl"), b"local data").unwrap();

let mut transport = FileTransport::new(local_dir.path(), remote_dir.path());
transport.push(&[]).unwrap();

// Now the remote directory contains the file
let remote_file = remote_dir.path().join("store.vctrl");
assert!(remote_file.exists());
assert_eq!(fs::read_to_string(&remote_file).unwrap(), "local data");
```

### pull_and_merge (currently always fails)

```rust
use posvault_sync::pull_and_merge;
use std::path::Path;
use libvctrl::domain::user::UserID;

let author = UserID::new("tester".into(), "test@posvault.internal".into()).unwrap();
let result = pull_and_merge(Path::new("/tmp/local"), Path::new("/tmp/remote"), author);
assert!(result.is_err());
```

---

## Dependencies

- `posvault_handler` – for traits (`ConflictResolver`, `Transport`), types (`BranchName`), and errors.
- `libvctrl` – for `RefStore`, `UserID` (used in `pull_and_merge` and branch functions).
- `std::fs`, `std::path` – for file copying in `FileTransport`.

No additional configuration is needed. The crate is designed to work with any `RefStore` implementation, typically the `FileStore` from `libvctrl`.

---

## Full Source Reference

The public API is defined across four files:

- `branch.rs` – `create_store_branch`, `checkout_branch`, `current_branch`
- `resolver.rs` – `UnionCsvResolver`
- `sync.rs` – `pull_and_merge`
- `transport.rs` – `FileTransport`

All items are re‑exported in `lib.rs`.

For exact signatures and implementation details, refer to the source snippets above.

---

_End of `posvault_sync` API Reference._
