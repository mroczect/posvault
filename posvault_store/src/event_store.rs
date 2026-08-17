use std::fmt;
use std::sync::{Arc, Mutex};

use libvctrl::{
    BinaryEncoder, Commit, Encoder, EntryKind, Hasher, Sha512Hasher, Tree, TreeEntry, UserID,
};

use posvault_handler::errors::{PosVaultError, Result};
use posvault_handler::traits::EventStore;
use posvault_handler::types::Event;

use crate::FileStore;

const BUCKET_SIZE: u64 = 1000;

pub struct VctrlEventStore {
    store: Arc<Mutex<FileStore>>,
}

impl fmt::Debug for VctrlEventStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VctrlEventStore")
            .field("store", &"Arc<Mutex<FileStore>>")
            .finish()
    }
}

impl VctrlEventStore {
    pub fn new(store: Arc<Mutex<FileStore>>) -> Self {
        Self { store }
    }

    fn current_checkpoint(store: &FileStore) -> Result<u64> {
        let head_commit_hash = store
            .head()?
            .ok_or(PosVaultError::NotFound("HEAD not found".into()))?;
        let head_commit = store.get_commit(&head_commit_hash)?;
        let root_tree = store.get_tree(head_commit.tree())?;

        if let Some(entry) = root_tree
            .entries()
            .iter()
            .find(|e| e.name() == "checkpoint" && e.kind() == EntryKind::Blob)
        {
            let raw = store.read_raw(entry.hash())?;
            deserialize_counter(&raw)
        } else {
            Ok(0)
        }
    }
}

impl EventStore for VctrlEventStore {
    fn append_event(&mut self, event: Event) -> Result<()> {
        let mut store = self
            .store
            .lock()
            .map_err(|e| PosVaultError::Storage(e.to_string()))?;

        let old_counter = Self::current_checkpoint(&store)?;
        let new_counter = old_counter + 1;
        let bucket = old_counter / BUCKET_SIZE;

        let event_bytes =
            serde_json::to_vec(&event).map_err(|e| PosVaultError::Serialization(e.to_string()))?;
        let event_hash = Sha512Hasher.hash(event_bytes.as_slice())?;
        store.put(&event_hash, &event_bytes)?;

        let head_commit_hash = store
            .head()?
            .ok_or(PosVaultError::NotFound("HEAD not found".into()))?;
        let head_commit = store.get_commit(&head_commit_hash)?;
        let root_tree = store.get_tree(head_commit.tree())?;

        let mut root_entries: Vec<TreeEntry> = root_tree.entries().to_vec();

        let cp_bytes = serialize_counter(new_counter);
        let cp_hash = Sha512Hasher.hash(cp_bytes.as_slice())?;
        store.put(&cp_hash, &cp_bytes)?;
        root_entries.retain(|e| e.name() != "checkpoint");
        root_entries.push(TreeEntry::new(
            "checkpoint".into(),
            EntryKind::Blob,
            cp_hash,
        )?);

        let bucket_name = format!("events-{}", bucket);
        let mut bucket_tree = if let Some(existing) = root_entries
            .iter()
            .find(|e| e.name() == bucket_name && e.kind() == EntryKind::Tree)
        {
            store.get_tree(existing.hash())?
        } else {
            Tree::new(vec![])?
        };

        let index_name = format!("{:016x}", new_counter);
        let mut bucket_entries = bucket_tree.entries().to_vec();
        bucket_entries.push(TreeEntry::new(index_name, EntryKind::Blob, event_hash)?);
        bucket_entries.sort_by(|a, b| a.name().cmp(b.name()));
        bucket_tree = Tree::new(bucket_entries)?;

        let mut bucket_bytes = Vec::new();
        BinaryEncoder.encode_tree(&bucket_tree, &mut bucket_bytes)?;
        let bucket_hash = Sha512Hasher.hash(bucket_bytes.as_slice())?;
        store.put(&bucket_hash, &bucket_bytes)?;

        root_entries.retain(|e| e.name() != bucket_name);
        root_entries.push(TreeEntry::new(bucket_name, EntryKind::Tree, bucket_hash)?);
        root_entries.sort_by(|a, b| a.name().cmp(b.name()));

        let new_root_tree = Tree::new(root_entries)?;
        let mut new_tree_bytes = Vec::new();
        BinaryEncoder.encode_tree(&new_root_tree, &mut new_tree_bytes)?;
        let new_tree_hash = Sha512Hasher.hash(new_tree_bytes.as_slice())?;
        store.put(&new_tree_hash, &new_tree_bytes)?;

        let author = UserID::new(
            format!(
                "{} ({})",
                event.author.fingerprint.as_str(),
                event.author.role.as_str()
            ),
            "posvault@internal".into(),
        )?;
        let commit = Commit::new(
            new_tree_hash,
            vec![head_commit_hash],
            author.clone(),
            author,
            format!("append event #{}", new_counter),
        )?;
        let mut commit_bytes = Vec::new();
        BinaryEncoder.encode_commit(&commit, &mut commit_bytes)?;
        let commit_hash = Sha512Hasher.hash(commit_bytes.as_slice())?;
        store.put(&commit_hash, &commit_bytes)?;
        store.set_ref("refs/heads/main", &commit_hash)?;
        store.set_head("refs/heads/main")?;

        Ok(())
    }

    fn get_events_since(&self, checkpoint: u64) -> Result<Vec<Event>> {
        let store = self
            .store
            .lock()
            .map_err(|e| PosVaultError::Storage(e.to_string()))?;
        let head_commit_hash = store
            .head()?
            .ok_or(PosVaultError::NotFound("HEAD not found".into()))?;
        let head_commit = store.get_commit(&head_commit_hash)?;
        let root_tree = store.get_tree(head_commit.tree())?;

        let mut events_with_index: Vec<(u64, Event)> = Vec::new();

        for entry in root_tree.entries() {
            if entry.name().starts_with("events-") && entry.kind() == EntryKind::Tree {
                let bucket_str = &entry.name()[7..];
                if let Ok(bucket) = bucket_str.parse::<u64>() {
                    if bucket < checkpoint / BUCKET_SIZE {
                        continue;
                    }
                    let bucket_tree = store.get_tree(entry.hash())?;
                    for be in bucket_tree.entries() {
                        if be.kind() == EntryKind::Blob
                            && let Ok(index) = u64::from_str_radix(be.name(), 16)
                            && index > checkpoint
                        {
                            let raw = store.read_raw(be.hash())?;
                            let event: Event = serde_json::from_slice(&raw)
                                .map_err(|e| PosVaultError::Serialization(e.to_string()))?;
                            events_with_index.push((index, event));
                        }
                    }
                }
            }
        }

        events_with_index.sort_by_key(|(i, _)| *i);
        Ok(events_with_index.into_iter().map(|(_, ev)| ev).collect())
    }

    fn latest_checkpoint(&self) -> Result<u64> {
        let store = self
            .store
            .lock()
            .map_err(|e| PosVaultError::Storage(e.to_string()))?;
        Self::current_checkpoint(&store)
    }
}

fn serialize_counter(counter: u64) -> Vec<u8> {
    counter.to_be_bytes().to_vec()
}

fn deserialize_counter(data: &[u8]) -> Result<u64> {
    if data.len() < 8 {
        return Err(PosVaultError::Storage("checkpoint data corrupt".into()));
    }
    let mut arr = [0u8; 8];
    arr.copy_from_slice(&data[..8]);
    Ok(u64::from_be_bytes(arr))
}
