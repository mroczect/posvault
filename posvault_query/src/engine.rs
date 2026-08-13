use libvctrl::{Hasher, Sha512Hasher};
use posvault_handler::errors::Result;
use posvault_handler::traits::{EventStore, SnapshotStore};
use posvault_handler::types::{CommitHash, EncryptedPayload, Snapshot};

use crate::cache::SnapshotCache;

/// Materialized view engine.
///
/// `QueryEngine` rebuilds an encrypted snapshot from an event store and
/// caches the result. It supports incremental rebuilds: if a previous
/// snapshot exists, only events after its checkpoint are replayed.
pub struct QueryEngine<S: EventStore + SnapshotStore> {
    store: S,
    cache: SnapshotCache,
}

impl<S: EventStore + SnapshotStore> QueryEngine<S> {
    /// Creates a new query engine backed by `store`.
    pub fn new(store: S) -> Self {
        QueryEngine {
            store,
            cache: SnapshotCache::new(),
        }
    }

    /// Rebuilds the snapshot from persisted events and stores it.
    ///
    /// The engine performs the following steps:
    /// 1. Load the latest snapshot, if any, to determine the start checkpoint.
    /// 2. Replay all events after that checkpoint through `apply_event`.
    /// 3. Encrypt the resulting state with `encrypt`.
    /// 4. Hash the checkpoint and encrypted state to produce a content hash.
    /// 5. Save the snapshot and update the cache.
    ///
    /// If there are no events and no previous snapshot, the snapshot version
    /// is set to 1 to satisfy the validation rules of
    /// [`Snapshot::new`](posvault_handler::types::Snapshot::new).
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
    {
        let last_snapshot = self.store.load_snapshot()?;
        let (start_checkpoint, mut state) = if let Some(snapshot) = last_snapshot {
            let plain = decrypt(snapshot.data.as_bytes())?;
            (snapshot.version, plain)
        } else {
            (0u64, Vec::new())
        };

        let events = self.store.get_events_since(start_checkpoint)?;
        for event in events {
            let plain_payload = decrypt(event.payload.as_bytes())?;
            apply_event(&mut state, &plain_payload)?;
        }

        let latest_checkpoint = self.store.latest_checkpoint()?;

        let encrypted_state = encrypt(&state)?;
        let encrypted_payload = EncryptedPayload::new(encrypted_state)?;

        let hasher = Sha512Hasher;
        let mut data = Vec::new();
        data.extend_from_slice(&latest_checkpoint.to_be_bytes());
        data.extend_from_slice(encrypted_payload.as_bytes());
        let hash_value = hasher.hash(&data)?;

        let hash_bytes: [u8; 64] = {
            let mut arr = [0u8; 64];
            arr.copy_from_slice(hash_value.as_bytes());
            arr
        };

        let hash = CommitHash::from_bytes(hash_bytes);

        // Ensure a valid snapshot version even when there are no events yet.
        let snapshot_version = if latest_checkpoint == 0 {
            1
        } else {
            latest_checkpoint
        };

        let snapshot = Snapshot::new(snapshot_version, encrypted_payload, hash)?;

        self.store.save_snapshot(snapshot.clone())?;
        self.cache.set(snapshot.clone());
        Ok(snapshot)
    }

    /// Returns the cached snapshot, if any.
    pub fn get_cached_snapshot(&self) -> Option<Snapshot> {
        self.cache.get()
    }

    /// Checks whether the cached snapshot is stale.
    ///
    /// Returns `true` if no snapshot is cached or if the latest checkpoint is
    /// greater than the cached snapshot version.
    pub fn needs_rebuild(&self) -> Result<bool> {
        let latest = self.store.latest_checkpoint()?;
        match self.cache.get() {
            Some(snap) => Ok(latest > snap.version),
            None => Ok(true),
        }
    }

    /// Invalidates the cached snapshot.
    pub fn invalidate_cache(&self) {
        self.cache.invalidate();
    }
}
