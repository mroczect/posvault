use libvctrl::codec::{BinaryEncoder, Encoder};
use libvctrl::command::Command;
use libvctrl::domain::object::Object;
use libvctrl::domain::tree::Tree;
use libvctrl::hashing::{Hasher, Sha512Hasher};
use libvctrl::storage::file_store::FileStore;
use libvctrl::storage::traits::{ObjectStore, RefStore};
use posvault_handler::errors::{PosVaultError, Result};
use std::fmt;
use std::path::Path;

pub struct PosVault {
    pub store: FileStore,
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
        if vault
            .store
            .head_ref_name()
            .map_err(|e| PosVaultError::Storage(e.to_string()))?
            .is_none()
        {
            vault.init()?;
        }
        Ok(vault)
    }

    fn init(&mut self) -> Result<()> {
        let encoder = BinaryEncoder;
        let hasher = Sha512Hasher;

        let empty_tree = Tree::new(vec![]).map_err(|e| PosVaultError::Storage(e.to_string()))?;
        let mut buf = Vec::new();
        encoder
            .encode_tree(&empty_tree, &mut buf)
            .map_err(|e| PosVaultError::Storage(e.to_string()))?;
        let tree_hash = hasher.hash_tree_encoded(&buf);
        self.store
            .put(&tree_hash, &Object::Tree(empty_tree))
            .map_err(|e| PosVaultError::Storage(e.to_string()))?;

        let author = libvctrl::domain::user::UserID::new("system".into(), "posvault".into())
            .map_err(|e| PosVaultError::Storage(e.to_string()))?;
        let commit_cmd = libvctrl::command::create_commit::CreateCommit {
            tree_hash,
            parents: vec![],
            author: author.clone(),
            committer: author,
            message: "initial commit".into(),
            encoder: Box::new(BinaryEncoder),
            hasher: Box::new(Sha512Hasher),
        };

        let store_ptr = &mut self.store as *mut FileStore;
        let store_ref = unsafe { &mut *store_ptr as &mut dyn ObjectStore };
        let refs_ref = unsafe { &mut *store_ptr as &mut dyn RefStore };
        let commit_hash = commit_cmd
            .execute(store_ref, refs_ref)
            .map_err(|e| PosVaultError::Storage(e.to_string()))?;

        self.store
            .set_ref("refs/heads/main", &commit_hash)
            .map_err(|e| PosVaultError::Storage(e.to_string()))?;
        self.store
            .set_head("refs/heads/main")
            .map_err(|e| PosVaultError::Storage(e.to_string()))?;
        Ok(())
    }

    pub fn get_head_commit_hash(&self) -> Result<libvctrl::domain::hash::Hash> {
        self.store
            .head()
            .map_err(|e| PosVaultError::Storage(e.to_string()))?
            .ok_or(PosVaultError::NotFound("HEAD not found".into()))
    }
}
