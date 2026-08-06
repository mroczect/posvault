use crate::posvault::PosVault;
use libvctrl::*;
use posvault_handler::*;
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
        self.vault.store.put(&hash, &Object::Blob(blob))?;
        self.vault.store.set_ref("refs/snapshots/latest", &hash)?;
        Ok(())
    }

    fn load_snapshot(&self) -> Result<Option<Snapshot>> {
        let hash = match self.vault.store.get_ref("refs/snapshots/latest")? {
            Some(h) => h,
            None => return Ok(None),
        };
        let blob = self.vault.store.get_blob(&hash)?;
        let snapshot: Snapshot = serde_json::from_slice(&blob)
            .map_err(|e| posvault_handler::errors::PosVaultError::Serialization(e.to_string()))?;
        Ok(Some(snapshot))
    }
}
