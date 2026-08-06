use crate::posvault::PosVault;
use libvctrl::codec::{BinaryEncoder, Encoder};
use libvctrl::command::Command;
use libvctrl::domain::object::Object;
use libvctrl::domain::tree::{EntryKind, TreeEntry};
use libvctrl::hashing::{Hasher, Sha512Hasher};
use libvctrl::storage::file_store::FileStore;
use libvctrl::storage::traits::{ObjectStore, ObjectStoreExt, RefStore};
use posvault_handler::errors::Result;
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

        let event_bytes = serde_json::to_vec(&event)
            .map_err(|e| posvault_handler::errors::PosVaultError::Serialization(e.to_string()))?;
        let blob = libvctrl::domain::blob::Blob::new(event_bytes);
        let blob_hash = hasher.hash_blob(blob.as_bytes());
        self.vault
            .store
            .put(&blob_hash, &Object::Blob(blob))
            .map_err(|e| posvault_handler::errors::PosVaultError::Storage(e.to_string()))?;

        let head_commit_hash = self.vault.get_head_commit_hash()?;
        let head_commit = self
            .vault
            .store
            .get_commit(&head_commit_hash)
            .map_err(|e| posvault_handler::errors::PosVaultError::Storage(e.to_string()))?;
        let old_tree = self
            .vault
            .store
            .get_tree(&head_commit.tree)
            .map_err(|e| posvault_handler::errors::PosVaultError::Storage(e.to_string()))?;

        let entry = TreeEntry::new(
            format!("events/{}", event.id.as_str()),
            EntryKind::Blob,
            blob_hash,
        )
        .map_err(|e| posvault_handler::errors::PosVaultError::Storage(e.to_string()))?;

        let mut entries: Vec<TreeEntry> = old_tree.entries().to_vec();
        entries.push(entry);
        let new_tree = libvctrl::domain::tree::Tree::new(entries)
            .map_err(|e| posvault_handler::errors::PosVaultError::Storage(e.to_string()))?;
        let mut buf = Vec::new();
        encoder
            .encode_tree(&new_tree, &mut buf)
            .map_err(|e| posvault_handler::errors::PosVaultError::Storage(e.to_string()))?;
        let new_tree_hash = hasher.hash_tree_encoded(&buf);
        self.vault
            .store
            .put(&new_tree_hash, &Object::Tree(new_tree))
            .map_err(|e| posvault_handler::errors::PosVaultError::Storage(e.to_string()))?;

        let author = libvctrl::domain::user::UserID::new(
            event.author.fingerprint.as_str().to_string(),
            String::new(),
        )
        .map_err(|e| posvault_handler::errors::PosVaultError::Storage(e.to_string()))?;

        let commit_cmd = libvctrl::command::create_commit::CreateCommit {
            tree_hash: new_tree_hash,
            parents: vec![head_commit_hash],
            author: author.clone(),
            committer: author,
            message: format!("append event {}", event.id.as_str()),
            encoder: Box::new(BinaryEncoder),
            hasher: Box::new(Sha512Hasher),
        };

        let store_ptr = &mut self.vault.store as *mut FileStore;
        let store_ref = unsafe { &mut *store_ptr as &mut dyn ObjectStore };
        let refs_ref = unsafe { &mut *store_ptr as &mut dyn RefStore };
        let new_commit_hash = commit_cmd
            .execute(store_ref, refs_ref)
            .map_err(|e| posvault_handler::errors::PosVaultError::Storage(e.to_string()))?;
        self.vault
            .store
            .set_ref("refs/heads/main", &new_commit_hash)
            .map_err(|e| posvault_handler::errors::PosVaultError::Storage(e.to_string()))?;
        Ok(())
    }

    fn get_events_since(&self, _checkpoint: u64) -> Result<Vec<Event>> {
        let head_commit_hash = self.vault.get_head_commit_hash()?;
        let head_commit = self
            .vault
            .store
            .get_commit(&head_commit_hash)
            .map_err(|e| posvault_handler::errors::PosVaultError::Storage(e.to_string()))?;
        let tree = self
            .vault
            .store
            .get_tree(&head_commit.tree)
            .map_err(|e| posvault_handler::errors::PosVaultError::Storage(e.to_string()))?;
        let mut events = Vec::new();
        for entry in tree.entries() {
            if entry.name.starts_with("events/") && entry.kind == EntryKind::Blob {
                let blob =
                    self.vault.store.get_blob(&entry.hash).map_err(|e| {
                        posvault_handler::errors::PosVaultError::Storage(e.to_string())
                    })?;
                let event: Event = serde_json::from_slice(&blob).map_err(|e| {
                    posvault_handler::errors::PosVaultError::Serialization(e.to_string())
                })?;
                events.push(event);
            }
        }
        Ok(events)
    }

    fn latest_checkpoint(&self) -> Result<u64> {
        let events = self.get_events_since(0)?;
        Ok(events.len() as u64)
    }
}
