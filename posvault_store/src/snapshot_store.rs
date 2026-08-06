use crate::posvault::PosVault;
use libvctrl::domain::object::Object;
use libvctrl::hashing::{Hasher, Sha512Hasher};
use libvctrl::storage::traits::{ObjectStore, ObjectStoreExt, RefStore};
use posvault_handler::errors::Result;
use posvault_handler::traits::SnapshotStore;
use posvault_handler::types::Snapshot;

#[derive(Debug)]
pub struct VctrlSnapshotStore {
    vault: PosVault,
}

impl VctrlSnapshotStore {
    pub fn new(vault: PosVault) -> Self {
        Self { vault }
    }
}

impl SnapshotStore for VctrlSnapshotStore {
    fn save_snapshot(&mut self, snapshot: Snapshot) -> Result<()> {
        let hasher = Sha512Hasher;
        let bytes = serde_json::to_vec(&snapshot)
            .map_err(|e| posvault_handler::errors::PosVaultError::Serialization(e.to_string()))?;
        let blob = libvctrl::domain::blob::Blob::new(bytes);
        let hash = hasher.hash_blob(blob.as_bytes());
        self.vault
            .store
            .put(&hash, &Object::Blob(blob))
            .map_err(|e| posvault_handler::errors::PosVaultError::Storage(e.to_string()))?;
        self.vault
            .store
            .set_ref("refs/snapshots/latest", &hash)
            .map_err(|e| posvault_handler::errors::PosVaultError::Storage(e.to_string()))?;
        Ok(())
    }

    fn load_snapshot(&self) -> Result<Option<Snapshot>> {
        let hash = match self
            .vault
            .store
            .get_ref("refs/snapshots/latest")
            .map_err(|e| posvault_handler::errors::PosVaultError::Storage(e.to_string()))?
        {
            Some(h) => h,
            None => return Ok(None),
        };
        let blob = self
            .vault
            .store
            .get_blob(&hash)
            .map_err(|e| posvault_handler::errors::PosVaultError::Storage(e.to_string()))?;
        let snapshot: Snapshot = serde_json::from_slice(&blob)
            .map_err(|e| posvault_handler::errors::PosVaultError::Serialization(e.to_string()))?;
        Ok(Some(snapshot))
    }
}
