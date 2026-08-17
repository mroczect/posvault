use std::fmt;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use libvctrl::{
    BinaryDecoder, BinaryEncoder, Blob, Commit, Decoder, Encoder, Hash, Hasher, MemoryRefStore,
    MemoryStore, ObjectStore, RefStore, Sha512Hasher, Tree, UserID, VctrlError,
};

use posvault_handler::errors::{PosVaultError, Result};

/// In-memory replacement for `FileStore`.
///
/// Combines object storage and reference storage in one struct,
/// mimicking the API used by the rest of the crate.
pub struct FileStore {
    objects: MemoryStore,
    refs: MemoryRefStore,
    head_ref_name: Option<String>,
}

impl Default for FileStore {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for FileStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileStore")
            .field("objects", &"MemoryStore")
            .field("refs", &"MemoryRefStore")
            .field("head_ref_name", &self.head_ref_name)
            .finish()
    }
}

impl FileStore {
    pub fn new() -> Self {
        Self {
            objects: MemoryStore::new(),
            refs: MemoryRefStore::new(),
            head_ref_name: None,
        }
    }

    pub fn open(_path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self::new())
    }

    pub fn put(&mut self, hash: &Hash, data: &[u8]) -> Result<()> {
        self.objects.put(hash, data)?;
        Ok(())
    }

    pub fn get(&self, hash: &Hash) -> Result<Box<dyn Read + Send + '_>> {
        self.objects.get(hash).map_err(Into::into)
    }

    /// Reads raw bytes directly. Use this for data stored without an
    /// encoder prefix, such as event JSON blobs or checkpoint counters.
    pub fn read_raw(&self, hash: &Hash) -> Result<Vec<u8>> {
        let mut reader = self.get(hash)?;
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf)?;
        Ok(buf)
    }

    pub fn get_ref(&self, name: &str) -> Result<Option<Hash>> {
        match self.refs.get_ref(name) {
            Ok(h) => Ok(Some(h)),
            Err(VctrlError::RefNotFound(_)) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn set_ref(&mut self, name: &str, hash: &Hash) -> Result<()> {
        self.refs.set_ref(name, hash)?;
        Ok(())
    }

    pub fn head_ref_name(&self) -> Result<Option<String>> {
        Ok(self.head_ref_name.clone())
    }

    pub fn set_head(&mut self, ref_name: &str) -> Result<()> {
        self.head_ref_name = Some(ref_name.to_string());
        Ok(())
    }

    pub fn head(&self) -> Result<Option<Hash>> {
        match &self.head_ref_name {
            Some(name) => self.get_ref(name),
            None => Ok(None),
        }
    }

    pub fn get_blob(&self, hash: &Hash) -> Result<Blob> {
        let buf = self.read_raw(hash)?;
        let decoder = BinaryDecoder;
        let blob = decoder.decode_blob(buf.as_slice())?;
        Ok(blob)
    }

    pub fn get_tree(&self, hash: &Hash) -> Result<Tree> {
        let buf = self.read_raw(hash)?;
        let decoder = BinaryDecoder;
        let tree = decoder.decode_tree(buf.as_slice())?;
        Ok(tree)
    }

    pub fn get_commit(&self, hash: &Hash) -> Result<Commit> {
        let buf = self.read_raw(hash)?;
        let decoder = BinaryDecoder;
        let commit = decoder.decode_commit(buf.as_slice())?;
        Ok(commit)
    }
}

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
}

fn init_store(store: &mut FileStore) -> Result<()> {
    let encoder = BinaryEncoder;
    let hasher = Sha512Hasher;

    let empty_tree = Tree::new(vec![])?;
    let mut tree_bytes = Vec::new();
    encoder.encode_tree(&empty_tree, &mut tree_bytes)?;
    let tree_hash = hasher.hash(tree_bytes.as_slice())?;
    store.put(&tree_hash, &tree_bytes)?;

    let author = UserID::new("system".into(), "posvault@internal".into())?;
    let commit = Commit::new(
        tree_hash,
        vec![],
        author.clone(),
        author,
        "initial commit".into(),
    )?;
    let mut commit_bytes = Vec::new();
    encoder.encode_commit(&commit, &mut commit_bytes)?;
    let commit_hash = hasher.hash(commit_bytes.as_slice())?;
    store.put(&commit_hash, &commit_bytes)?;

    store.set_ref("refs/heads/main", &commit_hash)?;
    store.set_head("refs/heads/main")?;
    Ok(())
}
