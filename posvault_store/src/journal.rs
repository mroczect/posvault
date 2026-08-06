use crate::posvault::PosVault;
use libvctrl::*;
use posvault_handler::errors::{PosVaultError, Result};
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

    fn ensure_journal_branch(&mut self) -> Result<Hash> {
        let journal_ref = "refs/journal";
        if let Some(hash) = self.vault.store.get_ref(journal_ref)? {
            return Ok(hash);
        }
        let encoder = BinaryEncoder;
        let hasher = Sha512Hasher;

        let empty_tree = Tree::new(vec![])?;
        let mut buf = Vec::new();
        encoder.encode_tree(&empty_tree, &mut buf)?;
        let tree_hash = hasher.hash_tree_encoded(&buf);
        self.vault
            .store
            .put(&tree_hash, &Object::Tree(empty_tree))?;

        let user = UserID::new("system".into(), "journal".into())?;
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
        self.vault
            .store
            .put(&commit_hash, &Object::Commit(Box::new(commit)))?;
        self.vault.store.set_ref(journal_ref, &commit_hash)?;
        Ok(commit_hash)
    }
}

impl Journal for VctrlJournal {
    fn record(&mut self, entry: JournalEntry) -> Result<()> {
        let encoder = BinaryEncoder;
        let hasher = Sha512Hasher;

        let current_commit = self.ensure_journal_branch()?;
        let commit = self.vault.store.get_commit(&current_commit)?;
        let old_tree = self.vault.store.get_tree(&commit.tree)?;

        let entry_bytes =
            serde_json::to_vec(&entry).map_err(|e| PosVaultError::Serialization(e.to_string()))?;
        let blob = Blob::new(entry_bytes);
        let blob_hash = hasher.hash_blob(blob.as_bytes());
        self.vault.store.put(&blob_hash, &Object::Blob(blob))?;

        let tree_entry = TreeEntry::new(
            format!("journal/{}", entry.id.as_str()),
            EntryKind::Blob,
            blob_hash,
        )?;

        let mut entries: Vec<TreeEntry> = old_tree.entries().to_vec();
        entries.push(tree_entry);
        let new_tree = Tree::new(entries)?;
        let mut buf = Vec::new();
        encoder.encode_tree(&new_tree, &mut buf)?;
        let new_tree_hash = hasher.hash_tree_encoded(&buf);
        self.vault
            .store
            .put(&new_tree_hash, &Object::Tree(new_tree))?;

        let user = UserID::new(entry.author.fingerprint.as_str().to_string(), String::new())?;
        let commit = Commit::new(
            new_tree_hash,
            vec![current_commit],
            user.clone(),
            user,
            entry.action.clone(),
            None,
        );
        let mut buf = Vec::new();
        encoder.encode_commit(&commit, &mut buf)?;
        let new_commit = hasher.hash_commit_encoded(&buf);
        self.vault
            .store
            .put(&new_commit, &Object::Commit(Box::new(commit)))?;
        self.vault.store.set_ref("refs/journal", &new_commit)?;
        Ok(())
    }

    fn read_all(&self) -> Result<Vec<JournalEntry>> {
        let journal_ref = "refs/journal";
        let commit_hash = match self.vault.store.get_ref(journal_ref)? {
            Some(h) => h,
            None => return Ok(vec![]),
        };
        let commit = self.vault.store.get_commit(&commit_hash)?;
        let tree = self.vault.store.get_tree(&commit.tree)?;
        let mut entries = Vec::new();
        for entry in tree.entries() {
            if entry.name.starts_with("journal/") && entry.kind == EntryKind::Blob {
                let blob = self.vault.store.get_blob(&entry.hash)?;
                let je: JournalEntry = serde_json::from_slice(&blob)
                    .map_err(|e| PosVaultError::Serialization(e.to_string()))?;
                entries.push(je);
            }
        }
        Ok(entries)
    }
}
