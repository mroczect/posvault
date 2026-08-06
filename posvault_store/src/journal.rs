use crate::posvault::PosVault;
use libvctrl::codec::{BinaryEncoder, Encoder};
use libvctrl::command::Command;
use libvctrl::domain::object::Object;
use libvctrl::domain::tree::{EntryKind, TreeEntry};
use libvctrl::hashing::{Hasher, Sha512Hasher};
use libvctrl::storage::file_store::FileStore;
use libvctrl::storage::traits::{ObjectStore, ObjectStoreExt, RefStore};
use posvault_handler::errors::Result;
use posvault_handler::traits::Journal;
use posvault_handler::types::JournalEntry;

#[derive(Debug)]
pub struct VctrlJournal {
    vault: PosVault,
}

impl VctrlJournal {
    pub fn new(vault: PosVault) -> Self {
        Self { vault }
    }

    fn ensure_journal_branch(&mut self) -> Result<libvctrl::domain::hash::Hash> {
        let journal_ref = "refs/journal";
        if let Some(hash) = self
            .vault
            .store
            .get_ref(journal_ref)
            .map_err(|e| posvault_handler::errors::PosVaultError::Storage(e.to_string()))?
        {
            return Ok(hash);
        }
        let encoder = BinaryEncoder;
        let hasher = Sha512Hasher;

        let empty_tree = libvctrl::domain::tree::Tree::new(vec![])
            .map_err(|e| posvault_handler::errors::PosVaultError::Storage(e.to_string()))?;
        let mut buf = Vec::new();
        encoder
            .encode_tree(&empty_tree, &mut buf)
            .map_err(|e| posvault_handler::errors::PosVaultError::Storage(e.to_string()))?;
        let tree_hash = hasher.hash_tree_encoded(&buf);
        self.vault
            .store
            .put(&tree_hash, &Object::Tree(empty_tree))
            .map_err(|e| posvault_handler::errors::PosVaultError::Storage(e.to_string()))?;
        let user = libvctrl::domain::user::UserID::new("system".into(), "journal".into())
            .map_err(|e| posvault_handler::errors::PosVaultError::Storage(e.to_string()))?;
        let commit_cmd = libvctrl::command::create_commit::CreateCommit {
            tree_hash,
            parents: vec![],
            author: user.clone(),
            committer: user,
            message: "initialize journal".into(),
            encoder: Box::new(BinaryEncoder),
            hasher: Box::new(Sha512Hasher),
        };

        let store_ptr = &mut self.vault.store as *mut FileStore;
        let store_ref = unsafe { &mut *store_ptr as &mut dyn ObjectStore };
        let refs_ref = unsafe { &mut *store_ptr as &mut dyn RefStore };
        let commit_hash = commit_cmd
            .execute(store_ref, refs_ref)
            .map_err(|e| posvault_handler::errors::PosVaultError::Storage(e.to_string()))?;
        self.vault
            .store
            .set_ref(journal_ref, &commit_hash)
            .map_err(|e| posvault_handler::errors::PosVaultError::Storage(e.to_string()))?;
        Ok(commit_hash)
    }
}

impl Journal for VctrlJournal {
    fn record(&mut self, entry: JournalEntry) -> Result<()> {
        let encoder = BinaryEncoder;
        let hasher = Sha512Hasher;

        let current_commit = self.ensure_journal_branch()?;
        let commit = self
            .vault
            .store
            .get_commit(&current_commit)
            .map_err(|e| posvault_handler::errors::PosVaultError::Storage(e.to_string()))?;
        let old_tree = self
            .vault
            .store
            .get_tree(&commit.tree)
            .map_err(|e| posvault_handler::errors::PosVaultError::Storage(e.to_string()))?;

        let entry_bytes = serde_json::to_vec(&entry)
            .map_err(|e| posvault_handler::errors::PosVaultError::Serialization(e.to_string()))?;
        let blob = libvctrl::domain::blob::Blob::new(entry_bytes);
        let blob_hash = hasher.hash_blob(blob.as_bytes());
        self.vault
            .store
            .put(&blob_hash, &Object::Blob(blob))
            .map_err(|e| posvault_handler::errors::PosVaultError::Storage(e.to_string()))?;

        let tree_entry = TreeEntry::new(
            format!("journal/{}", entry.id.as_str()),
            EntryKind::Blob,
            blob_hash,
        )
        .map_err(|e| posvault_handler::errors::PosVaultError::Storage(e.to_string()))?;

        let mut entries: Vec<TreeEntry> = old_tree.entries().to_vec();
        entries.push(tree_entry);
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

        let user = libvctrl::domain::user::UserID::new(
            entry.author.fingerprint.as_str().to_string(),
            String::new(),
        )
        .map_err(|e| posvault_handler::errors::PosVaultError::Storage(e.to_string()))?;
        let commit_cmd = libvctrl::command::create_commit::CreateCommit {
            tree_hash: new_tree_hash,
            parents: vec![current_commit],
            author: user.clone(),
            committer: user,
            message: entry.action.clone(),
            encoder: Box::new(BinaryEncoder),
            hasher: Box::new(Sha512Hasher),
        };

        let store_ptr = &mut self.vault.store as *mut FileStore;
        let store_ref = unsafe { &mut *store_ptr as &mut dyn ObjectStore };
        let refs_ref = unsafe { &mut *store_ptr as &mut dyn RefStore };
        let new_commit = commit_cmd
            .execute(store_ref, refs_ref)
            .map_err(|e| posvault_handler::errors::PosVaultError::Storage(e.to_string()))?;
        self.vault
            .store
            .set_ref("refs/journal", &new_commit)
            .map_err(|e| posvault_handler::errors::PosVaultError::Storage(e.to_string()))?;
        Ok(())
    }

    fn read_all(&self) -> Result<Vec<JournalEntry>> {
        let journal_ref = "refs/journal";
        let commit_hash = match self
            .vault
            .store
            .get_ref(journal_ref)
            .map_err(|e| posvault_handler::errors::PosVaultError::Storage(e.to_string()))?
        {
            Some(h) => h,
            None => return Ok(vec![]),
        };
        let commit = self
            .vault
            .store
            .get_commit(&commit_hash)
            .map_err(|e| posvault_handler::errors::PosVaultError::Storage(e.to_string()))?;
        let tree = self
            .vault
            .store
            .get_tree(&commit.tree)
            .map_err(|e| posvault_handler::errors::PosVaultError::Storage(e.to_string()))?;
        let mut entries = Vec::new();
        for entry in tree.entries() {
            if entry.name.starts_with("journal/") && entry.kind == EntryKind::Blob {
                let blob =
                    self.vault.store.get_blob(&entry.hash).map_err(|e| {
                        posvault_handler::errors::PosVaultError::Storage(e.to_string())
                    })?;
                let je: JournalEntry = serde_json::from_slice(&blob).map_err(|e| {
                    posvault_handler::errors::PosVaultError::Serialization(e.to_string())
                })?;
                entries.push(je);
            }
        }
        Ok(entries)
    }
}
