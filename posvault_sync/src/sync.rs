use crate::branch::current_branch;
use crate::resolver::UnionCsvResolver;
use libvctrl::codec::BinaryEncoder;
use libvctrl::command::Command;
use libvctrl::command::merge_branch::MergeBranch;
use libvctrl::domain::user::UserID;
use libvctrl::hashing::Sha512Hasher;
use libvctrl::merge::{ConflictResolver as VctrlConflictResolver, ThreeWayMerger};
use libvctrl::storage::file_store::FileStore;
use libvctrl::storage::traits::{ObjectStore, RefStore};
use posvault_handler::errors::{PosVaultError, Result};
use posvault_handler::traits::ConflictResolver;
use std::fs;
use std::path::Path;

pub fn pull_and_merge(
    local_store_path: &Path,
    remote_store_path: &Path,
    author: UserID,
) -> Result<()> {
    let mut local_store = FileStore::open(local_store_path.join("store.vctrl"))
        .map_err(|e| PosVaultError::Storage(e.to_string()))?;
    let local_branch = current_branch(&local_store)?
        .ok_or_else(|| PosVaultError::Sync("no current branch".into()))?;

    let tmp_dir = tempfile::tempdir().map_err(|e| PosVaultError::Sync(e.to_string()))?;
    let tmp_store_path = tmp_dir.path().join("store.vctrl");
    let remote_file = remote_store_path.join("store.vctrl");
    if remote_file.exists() {
        fs::copy(&remote_file, &tmp_store_path).map_err(|e| PosVaultError::Sync(e.to_string()))?;
    } else {
        fs::copy(remote_store_path, &tmp_store_path)
            .map_err(|e| PosVaultError::Sync(e.to_string()))?;
    }

    let _remote_store =
        FileStore::open(&tmp_store_path).map_err(|e| PosVaultError::Sync(e.to_string()))?;

    let remote_branch = format!("refs/heads/{}", local_branch.as_str());
    let _remote_hash = _remote_store
        .get_ref(&remote_branch)
        .map_err(|e| PosVaultError::Sync(e.to_string()))?
        .ok_or_else(|| {
            PosVaultError::NotFound(format!(
                "remote branch '{}' not found",
                local_branch.as_str()
            ))
        })?;

    let merger = Box::new(ThreeWayMerger);
    let resolver = Box::new(VctrlUnionAdapter(UnionCsvResolver));
    let encoder = Box::new(BinaryEncoder);
    let hasher = Box::new(Sha512Hasher);

    let merge_cmd = MergeBranch {
        branch_name: remote_branch.clone(),
        author: author.clone(),
        committer: author,
        merger,
        resolver,
        encoder,
        hasher,
    };

    let store_ptr = &mut local_store as *mut FileStore;
    let store_ref = unsafe { &mut *store_ptr as &mut dyn ObjectStore };
    let refs_ref = unsafe { &mut *store_ptr as &mut dyn RefStore };
    merge_cmd
        .execute(store_ref, refs_ref)
        .map_err(|e| PosVaultError::Sync(e.to_string()))?;

    Ok(())
}

struct VctrlUnionAdapter(pub UnionCsvResolver);

impl VctrlConflictResolver for VctrlUnionAdapter {
    fn resolve(&self, base: &[u8], ours: &[u8], theirs: &[u8]) -> Option<Vec<u8>> {
        ConflictResolver::resolve(&self.0, base, ours, theirs).ok()
    }
}
