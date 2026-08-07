use libvctrl::hashing::{Hasher, Sha512Hasher};
use posvault_handler::errors::Result;
use posvault_handler::traits::{EventStore, SnapshotStore};
use posvault_handler::types::{CommitHash, EncryptedPayload, Snapshot};

use crate::cache::SnapshotCache;

pub struct QueryEngine<S: EventStore + SnapshotStore> {
    store: S,
    cache: SnapshotCache,
}

impl<S: EventStore + SnapshotStore> QueryEngine<S> {
    pub fn new(store: S) -> Self {
        QueryEngine {
            store,
            cache: SnapshotCache::new(),
        }
    }

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

        if latest_checkpoint == 0 {
            let snapshot = Snapshot::new(
                0,
                EncryptedPayload::new(encrypt(&state)?)?,
                CommitHash::from_bytes([1u8; 64]),
            )?;
            return Ok(snapshot);
        }

        let encrypted_state = encrypt(&state)?;
        let encrypted_payload = EncryptedPayload::new(encrypted_state)?;

        let hasher = Sha512Hasher;
        let mut data = Vec::new();
        data.extend_from_slice(&latest_checkpoint.to_be_bytes());
        data.extend_from_slice(encrypted_payload.as_bytes());
        let hash_value = hasher.hash_blob(&data);

        let hash_bytes: [u8; 64] = {
            let mut arr = [0u8; 64];
            arr.copy_from_slice(hash_value.as_bytes());
            arr
        };

        let hash = CommitHash::from_bytes(hash_bytes);

        let snapshot = Snapshot::new(latest_checkpoint, encrypted_payload, hash)?;

        self.store.save_snapshot(snapshot.clone())?;
        self.cache.set(snapshot.clone());
        Ok(snapshot)
    }

    pub fn get_cached_snapshot(&self) -> Option<Snapshot> {
        self.cache.get()
    }

    pub fn needs_rebuild(&self) -> Result<bool> {
        let latest = self.store.latest_checkpoint()?;
        match self.cache.get() {
            Some(snap) => Ok(latest > snap.version),
            None => Ok(true),
        }
    }

    pub fn invalidate_cache(&self) {
        self.cache.invalidate();
    }
}
