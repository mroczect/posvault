use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use libvctrl::Object;
use libvctrl::codec::BinaryEncoder;
use libvctrl::codec::Encoder;
use libvctrl::domain::commit::Commit;
use libvctrl::domain::tree::Tree;
use libvctrl::domain::user::UserID;
use libvctrl::hashing::Hasher;
use libvctrl::hashing::Sha512Hasher;
use libvctrl::storage::file_store::FileStore;
use libvctrl::storage::traits::ObjectStore;
use libvctrl::storage::traits::RefStore;

use posvault_handler::errors::{PosVaultError, Result};

pub struct PosVault {
    pub(crate) store: Arc<Mutex<FileStore>>,
    pub(crate) path: PathBuf,
}

impl fmt::Debug for PosVault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PosVault")
            .field("store", &"Arc<Mutex<FileStore>>")
            .field("path", &self.path)
            .finish()
    }
}

impl PosVault {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        std::fs::create_dir_all(&path).map_err(|e| PosVaultError::Storage(e.to_string()))?;
        let file_store = FileStore::open(path.join("store.vctrl"))
            .map_err(|e| PosVaultError::Storage(e.to_string()))?;
        let store = Arc::new(Mutex::new(file_store));

        {
            let mut s = store
                .lock()
                .map_err(|e| PosVaultError::Storage(e.to_string()))?;
            if s.head_ref_name()?.is_none() {
                init_store(&mut s)?;
            }
        }

        Ok(PosVault { store, path })
    }

    pub fn store_arc(&self) -> Arc<Mutex<FileStore>> {
        Arc::clone(&self.store)
    }
    #[doc(hidden)]
    pub fn store_ref(&self) -> &Arc<Mutex<FileStore>> {
        &self.store
    }
}

fn init_store(store: &mut FileStore) -> Result<()> {
    let encoder = BinaryEncoder;
    let hasher = Sha512Hasher;

    let empty_tree = Tree::new(vec![])?;
    let mut buf = Vec::new();
    encoder.encode_tree(&empty_tree, &mut buf)?;
    let tree_hash = hasher.hash_tree_encoded(&buf);
    store.put(&tree_hash, &Object::Tree(empty_tree))?;

    let author = UserID::new("system".into(), "posvault@internal".into())?;
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
    store.put(&commit_hash, &Object::Commit(Box::new(commit)))?;

    store.set_ref("refs/heads/main", &commit_hash)?;
    store.set_head("refs/heads/main")?;
    Ok(())
}
