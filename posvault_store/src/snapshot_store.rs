use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use libvctrl::Object;
use libvctrl::domain::blob::Blob;
use libvctrl::hashing::Hasher;
use libvctrl::hashing::Sha512Hasher;
use libvctrl::storage::file_store::FileStore;
use libvctrl::storage::traits::ObjectStore;
use libvctrl::storage::traits::{ObjectStoreExt, RefStore};

use posvault_handler::errors::{PosVaultError, Result};
use posvault_handler::traits::SnapshotStore;
use posvault_handler::types::Snapshot;

pub struct VctrlSnapshotStore {
    store: Arc<Mutex<FileStore>>,
}

impl fmt::Debug for VctrlSnapshotStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VctrlSnapshotStore")
            .field("store", &"Arc<Mutex<FileStore>>")
            .finish()
    }
}

impl VctrlSnapshotStore {
    pub fn new(store: Arc<Mutex<FileStore>>) -> Self {
        Self { store }
    }

    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

impl SnapshotStore for VctrlSnapshotStore {
    fn save_snapshot(&mut self, snapshot: Snapshot) -> Result<()> {
        let mut store = self
            .store
            .lock()
            .map_err(|e| PosVaultError::Storage(e.to_string()))?;

        let bytes = serde_json::to_vec(&snapshot)
            .map_err(|e| PosVaultError::Serialization(e.to_string()))?;
        let blob = Blob::new(bytes);
        let hash = Sha512Hasher.hash_blob(blob.as_bytes());
        store.put(&hash, &Object::Blob(blob))?;

        let ts = Self::current_timestamp();
        let ref_name = format!("refs/snapshots/{}", ts);
        store.set_ref(&ref_name, &hash)?;
        store.set_ref("refs/snapshots/latest", &hash)?;

        Ok(())
    }

    fn load_snapshot(&self) -> Result<Option<Snapshot>> {
        let store = self
            .store
            .lock()
            .map_err(|e| PosVaultError::Storage(e.to_string()))?;
        let hash = match store.get_ref("refs/snapshots/latest")? {
            Some(h) => h,
            None => return Ok(None),
        };
        let blob = store.get_blob(&hash)?;
        let snapshot: Snapshot = serde_json::from_slice(&blob)
            .map_err(|e| PosVaultError::Serialization(e.to_string()))?;
        Ok(Some(snapshot))
    }
}
