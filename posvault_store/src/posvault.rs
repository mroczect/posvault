use libvctrl::*;
use posvault_handler::errors::{PosVaultError, Result};
use std::fmt;
use std::path::Path;

pub struct PosVault {
    pub(crate) store: FileStore,
}

impl fmt::Debug for PosVault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PosVault")
            .field("store", &"FileStore { .. }")
            .finish()
    }
}

impl PosVault {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let store = FileStore::open(path.join("store.vctrl"))
            .map_err(|e| PosVaultError::Storage(e.to_string()))?;
        let mut vault = PosVault { store };
        if vault.store.head_ref_name()?.is_none() {
            vault.init()?;
        }
        Ok(vault)
    }

    fn init(&mut self) -> Result<()> {
        let encoder = BinaryEncoder;
        let hasher = Sha512Hasher;

        let empty_tree = Tree::new(vec![])?;
        let mut buf = Vec::new();
        encoder.encode_tree(&empty_tree, &mut buf)?;
        let tree_hash = hasher.hash_tree_encoded(&buf);
        self.store.put(&tree_hash, &Object::Tree(empty_tree))?;

        let author = UserID::new("system".into(), "posvault".into())?;
        let commit = Commit::new(
            tree_hash,
            vec![],
            author.clone(),
            author,
            "initial commit".into(),
            None,
        );
        let mut buf = Vec::new();
        encoder.encode_commit(&commit, &mut buf)?;
        let commit_hash = hasher.hash_commit_encoded(&buf);
        self.store
            .put(&commit_hash, &Object::Commit(Box::new(commit)))?;

        self.store.set_ref("refs/heads/main", &commit_hash)?;
        self.store.set_head("refs/heads/main")?;
        Ok(())
    }

    pub fn get_head_commit_hash(&self) -> Result<Hash> {
        self.store
            .head()?
            .ok_or(PosVaultError::NotFound("HEAD not found".into()))
    }
}
