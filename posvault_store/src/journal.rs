use std::fmt;
use std::sync::{Arc, Mutex};

use libvctrl::Hash;
use libvctrl::Object;
use libvctrl::codec::BinaryEncoder;
use libvctrl::codec::Encoder;
use libvctrl::domain::blob::Blob;
use libvctrl::domain::commit::Commit;
use libvctrl::domain::tree::{EntryKind, Tree, TreeEntry};
use libvctrl::domain::user::UserID;
use libvctrl::hashing::Hasher;
use libvctrl::hashing::Sha512Hasher;
use libvctrl::storage::file_store::FileStore;
use libvctrl::storage::traits::ObjectStore;
use libvctrl::storage::traits::{ObjectStoreExt, RefStore};

use posvault_handler::constants::JOURNAL_COMPACTION_THRESHOLD;
use posvault_handler::errors::{PosVaultError, Result};
use posvault_handler::traits::Journal;
use posvault_handler::types::JournalEntry;

pub struct VctrlJournal {
    store: Arc<Mutex<FileStore>>,
    compaction_threshold: u64,
}

impl fmt::Debug for VctrlJournal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VctrlJournal")
            .field("store", &"Arc<Mutex<FileStore>>")
            .field("compaction_threshold", &self.compaction_threshold)
            .finish()
    }
}

impl VctrlJournal {
    pub fn new(store: Arc<Mutex<FileStore>>) -> Self {
        Self {
            store,
            compaction_threshold: JOURNAL_COMPACTION_THRESHOLD,
        }
    }

    pub fn with_threshold(store: Arc<Mutex<FileStore>>, threshold: u64) -> Self {
        Self {
            store,
            compaction_threshold: threshold,
        }
    }

    fn ensure_journal_branch(store: &mut FileStore) -> Result<Hash> {
        let journal_ref = "refs/journal";
        if let Some(h) = store.get_ref(journal_ref)? {
            return Ok(h);
        }
        let encoder = BinaryEncoder;
        let hasher = Sha512Hasher;

        let empty_tree = Tree::new(vec![])?;
        let mut buf = Vec::new();
        encoder.encode_tree(&empty_tree, &mut buf)?;
        let tree_hash = hasher.hash_tree_encoded(&buf);
        store.put(&tree_hash, &Object::Tree(empty_tree))?;

        let user = UserID::new("system".into(), "journal@internal".into())?;
        let commit = Commit::new(
            tree_hash,
            vec![],
            user.clone(),
            user,
            "initialize journal".into(),
            None,
        );
        let mut buf = Vec::new();
        encoder.encode_commit(&commit, &mut buf)?;
        let commit_hash = hasher.hash_commit_encoded(&buf);
        store.put(&commit_hash, &Object::Commit(Box::new(commit)))?;
        store.set_ref(journal_ref, &commit_hash)?;
        Ok(commit_hash)
    }

    fn compact(store: &mut FileStore) -> Result<()> {
        let journal_ref = "refs/journal";
        let head_hash = store
            .get_ref(journal_ref)?
            .ok_or_else(|| PosVaultError::NotFound("journal ref not found".into()))?;
        let head_commit = store.get_commit(&head_hash)?;
        let tree = store.get_tree(&head_commit.tree)?;

        let mut unarchived: Vec<JournalEntry> = Vec::new();
        let mut archive_entries: Vec<TreeEntry> = Vec::new();

        for entry in tree.entries() {
            if entry.name.starts_with("archive-") {
                archive_entries.push(entry.clone());
            } else if entry.name.starts_with("journal-") && entry.kind == EntryKind::Blob {
                let blob = store.get_blob(&entry.hash)?;
                let je: JournalEntry = serde_json::from_slice(&blob)
                    .map_err(|e| PosVaultError::Serialization(e.to_string()))?;
                unarchived.push(je);
            }
        }

        if unarchived.is_empty() {
            return Ok(());
        }

        unarchived.sort_by(|a, b| {
            a.timestamp
                .cmp(&b.timestamp)
                .then_with(|| a.id.as_str().cmp(b.id.as_str()))
        });

        let next_seq = archive_entries.len() as u64;
        let archive_blob = Blob::new(
            serde_json::to_vec(&unarchived)
                .map_err(|e| PosVaultError::Serialization(e.to_string()))?,
        );
        let archive_hash = Sha512Hasher.hash_blob(archive_blob.as_bytes());
        store.put(&archive_hash, &Object::Blob(archive_blob))?;
        archive_entries.push(TreeEntry::new(
            format!("archive-{}", next_seq),
            EntryKind::Blob,
            archive_hash,
        )?);

        let new_tree = Tree::new(archive_entries)?;
        let mut buf = Vec::new();
        BinaryEncoder.encode_tree(&new_tree, &mut buf)?;
        let new_tree_hash = Sha512Hasher.hash_tree_encoded(&buf);
        store.put(&new_tree_hash, &Object::Tree(new_tree))?;

        let user = UserID::new("system".into(), "compaction@internal".into())?;
        let commit = Commit::new(
            new_tree_hash,
            vec![head_hash],
            user.clone(),
            user,
            "journal compaction".into(),
            None,
        );
        let mut buf = Vec::new();
        BinaryEncoder.encode_commit(&commit, &mut buf)?;
        let commit_hash = Sha512Hasher.hash_commit_encoded(&buf);
        store.put(&commit_hash, &Object::Commit(Box::new(commit)))?;
        store.set_ref(journal_ref, &commit_hash)?;
        Ok(())
    }

    fn maybe_compact(store: &mut FileStore, threshold: u64) -> Result<()> {
        let journal_ref = "refs/journal";
        let head = match store.get_ref(journal_ref)? {
            Some(h) => h,
            None => return Ok(()),
        };
        let commit = store.get_commit(&head)?;
        let tree = store.get_tree(&commit.tree)?;
        let unarchived_count = tree
            .entries()
            .iter()
            .filter(|e| e.name.starts_with("journal-") && e.kind == EntryKind::Blob)
            .count();

        if unarchived_count as u64 > threshold {
            Self::compact(store)?;
        }
        Ok(())
    }
}

impl Journal for VctrlJournal {
    fn record(&mut self, entry: JournalEntry) -> Result<()> {
        let mut store = self
            .store
            .lock()
            .map_err(|e| PosVaultError::Storage(e.to_string()))?;

        let current_commit = Self::ensure_journal_branch(&mut store)?;
        let commit = store.get_commit(&current_commit)?;
        let old_tree = store.get_tree(&commit.tree)?;

        let entry_bytes =
            serde_json::to_vec(&entry).map_err(|e| PosVaultError::Serialization(e.to_string()))?;
        let blob = Blob::new(entry_bytes);
        let blob_hash = Sha512Hasher.hash_blob(blob.as_bytes());
        store.put(&blob_hash, &Object::Blob(blob))?;

        let tree_entry = TreeEntry::new(
            format!("journal-{}", entry.id.as_str()),
            EntryKind::Blob,
            blob_hash,
        )?;
        let mut new_entries: Vec<TreeEntry> = old_tree.entries().to_vec();
        new_entries.push(tree_entry);
        let new_tree = Tree::new(new_entries)?;

        let mut buf = Vec::new();
        BinaryEncoder.encode_tree(&new_tree, &mut buf)?;
        let new_tree_hash = Sha512Hasher.hash_tree_encoded(&buf);
        store.put(&new_tree_hash, &Object::Tree(new_tree))?;

        let user = UserID::new(
            entry.author.fingerprint.as_str().to_string(),
            "journal@internal".into(),
        )?;
        let commit = Commit::new(
            new_tree_hash,
            vec![current_commit],
            user.clone(),
            user,
            entry.action.clone(),
            None,
        );
        let mut buf = Vec::new();
        BinaryEncoder.encode_commit(&commit, &mut buf)?;
        let new_commit_hash = Sha512Hasher.hash_commit_encoded(&buf);
        store.put(&new_commit_hash, &Object::Commit(Box::new(commit)))?;
        store.set_ref("refs/journal", &new_commit_hash)?;

        let threshold = self.compaction_threshold;
        Self::maybe_compact(&mut store, threshold)?;

        Ok(())
    }

    fn read_all(&self) -> Result<Vec<JournalEntry>> {
        let store = self
            .store
            .lock()
            .map_err(|e| PosVaultError::Storage(e.to_string()))?;
        let journal_ref = "refs/journal";
        let commit_hash = match store.get_ref(journal_ref)? {
            Some(h) => h,
            None => return Ok(vec![]),
        };

        let commit = store.get_commit(&commit_hash)?;
        let tree = store.get_tree(&commit.tree)?;

        let mut entries = Vec::new();
        for entry in tree.entries() {
            if entry.name.starts_with("archive-") {
                let blob = store.get_blob(&entry.hash)?;
                let archived: Vec<JournalEntry> = serde_json::from_slice(&blob)
                    .map_err(|e| PosVaultError::Serialization(e.to_string()))?;
                entries.extend(archived);
            } else if entry.name.starts_with("journal-") && entry.kind == EntryKind::Blob {
                let blob = store.get_blob(&entry.hash)?;
                let je: JournalEntry = serde_json::from_slice(&blob)
                    .map_err(|e| PosVaultError::Serialization(e.to_string()))?;
                entries.push(je);
            }
        }
        entries.sort_by(|a, b| {
            a.timestamp
                .cmp(&b.timestamp)
                .then_with(|| a.id.as_str().cmp(b.id.as_str()))
        });
        Ok(entries)
    }
}
