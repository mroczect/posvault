use libvctrl::command::Command;
use libvctrl::command::branch::{CreateBranch, SetHead};
use libvctrl::storage::traits::{ObjectStore, RefStore};
use posvault_handler::errors::{PosVaultError, Result};
use posvault_handler::types::BranchName;

pub fn create_store_branch(
    store: &mut dyn ObjectStore,
    refs: &mut dyn RefStore,
    store_id: &str,
) -> Result<BranchName> {
    let branch_name = format!("refs/heads/store-{}", store_id);
    let current_head = refs
        .head()?
        .ok_or_else(|| PosVaultError::NotFound("HEAD not found".into()))?;

    let create = CreateBranch {
        name: branch_name.clone(),
        hash: current_head,
    };
    create
        .execute(store, refs)
        .map_err(|e| PosVaultError::Storage(e.to_string()))?;

    let set_head = SetHead {
        target: branch_name.clone(),
    };
    set_head
        .execute(store, refs)
        .map_err(|e| PosVaultError::Storage(e.to_string()))?;

    BranchName::new(branch_name.trim_start_matches("refs/heads/"))
}

pub fn checkout_branch(refs: &mut dyn RefStore, branch_name: &BranchName) -> Result<()> {
    let full_name = format!("refs/heads/{}", branch_name.as_str());
    refs.get_ref(&full_name)
        .map_err(|e| PosVaultError::Storage(e.to_string()))?
        .ok_or_else(|| {
            PosVaultError::NotFound(format!("branch '{}' not found", branch_name.as_str()))
        })?;

    let set_head = SetHead { target: full_name };
    let mut dummy_store = libvctrl::storage::memory::MemoryStore::new();
    set_head
        .execute(&mut dummy_store, refs)
        .map_err(|e| PosVaultError::Storage(e.to_string()))?;
    Ok(())
}

pub fn current_branch(refs: &dyn RefStore) -> Result<Option<BranchName>> {
    let head_ref = refs
        .head_ref_name()
        .map_err(|e| PosVaultError::Storage(e.to_string()))?;
    match head_ref {
        Some(name) if name.starts_with("refs/heads/") => {
            BranchName::new(name.trim_start_matches("refs/heads/")).map(Some)
        }
        _ => Ok(None),
    }
}
