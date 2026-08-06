use crate::posvault::PosVault;
use libvctrl::*;
use posvault_handler::errors::{PosVaultError, Result};
use posvault_handler::traits::EventStore;
use posvault_handler::types::Event;

#[derive(Debug)]
pub struct VctrlEventStore {
    vault: PosVault,
}

impl VctrlEventStore {
    pub fn new(vault: PosVault) -> Self {
        Self { vault }
    }
}

impl EventStore for VctrlEventStore {
    fn append_event(&mut self, event: Event) -> Result<()> {
        let encoder = BinaryEncoder;
        let hasher = Sha512Hasher;

        let event_bytes =
            serde_json::to_vec(&event).map_err(|e| PosVaultError::Serialization(e.to_string()))?;
        let blob = Blob::new(event_bytes);
        let blob_hash = hasher.hash_blob(blob.as_bytes());
        self.vault.store.put(&blob_hash, &Object::Blob(blob))?;

        let head_commit_hash = self.vault.get_head_commit_hash()?;
        let head_commit = self.vault.store.get_commit(&head_commit_hash)?;
        let old_tree = self.vault.store.get_tree(&head_commit.tree)?;

        let entry = TreeEntry::new(
            format!("events-{}", event.id.as_str()),
            EntryKind::Blob,
            blob_hash,
        )?;

        let mut entries: Vec<TreeEntry> = old_tree.entries().to_vec();
        entries.push(entry);
        let new_tree = Tree::new(entries)?;
        let mut buf = Vec::new();
        encoder.encode_tree(&new_tree, &mut buf)?;
        let new_tree_hash = hasher.hash_tree_encoded(&buf);
        self.vault
            .store
            .put(&new_tree_hash, &Object::Tree(new_tree))?;

        let author = UserID::new(event.author.fingerprint.as_str().to_string(), String::new())?;

        let commit = Commit::new(
            new_tree_hash,
            vec![head_commit_hash],
            author.clone(),
            author,
            format!("append event {}", event.id.as_str()),
            None,
        );
        let mut buf = Vec::new();
        encoder.encode_commit(&commit, &mut buf)?;
        let new_commit_hash = hasher.hash_commit_encoded(&buf);
        self.vault
            .store
            .put(&new_commit_hash, &Object::Commit(Box::new(commit)))?;
        self.vault
            .store
            .set_ref("refs/heads/main", &new_commit_hash)?;
        Ok(())
    }

    fn get_events_since(&self, _checkpoint: u64) -> Result<Vec<Event>> {
        Err(PosVaultError::InvalidInput(
            "get_events_since not yet implemented".into(),
        ))
    }

    fn latest_checkpoint(&self) -> Result<u64> {
        let head_commit_hash = self.vault.get_head_commit_hash()?;
        let head_commit = self.vault.store.get_commit(&head_commit_hash)?;
        let tree = self.vault.store.get_tree(&head_commit.tree)?;
        let mut count: u64 = 0;
        for entry in tree.entries() {
            if entry.name.starts_with("events/") && entry.kind == EntryKind::Blob {
                count += 1;
            }
        }
        Ok(count)
    }
}
