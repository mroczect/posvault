use posvault_handler::errors::Result;
use posvault_handler::traits::{EventStore, SnapshotStore};
use posvault_handler::types::Snapshot;

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

    pub fn rebuild_snapshot<F>(&mut self, apply_event: F) -> Result<Snapshot>
    where
        F: Fn(&mut Vec<u8>, &[u8]) -> Result<()>,
    {
        let last_snapshot = self.store.load_snapshot()?;
        let (start_checkpoint, mut state) = if let Some(snapshot) = last_snapshot {
            let state = snapshot.data.as_bytes().to_vec();
            (snapshot.version, state)
        } else {
            (0u64, Vec::new())
        };

        let events = self.store.get_events_since(start_checkpoint)?;
        for event in events {
            apply_event(&mut state, event.payload.as_bytes())?;
        }

        let latest_checkpoint = self.store.latest_checkpoint()?;
        let snapshot = Snapshot::new(
            latest_checkpoint,
            posvault_handler::types::EncryptedPayload::new(state.clone())?,
            posvault_handler::types::CommitHash::from_bytes([1u8; 64]),
        )?;

        self.store.save_snapshot(snapshot.clone())?;
        self.cache.set(snapshot.clone());
        Ok(snapshot)
    }

    pub fn invalidate_cache(&self) {
        self.cache.invalidate();
    }
}
