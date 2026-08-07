use libvctrl::storage::traits::RefStore;
use posvault_handler::errors::{PosVaultError, Result};
use posvault_handler::types::BranchName;

pub fn create_store_branch(refs: &mut dyn RefStore, store_id: &str) -> Result<BranchName> {
    let branch_name = format!("refs/heads/store-{}", store_id);
    let current_head = refs
        .head()?
        .ok_or_else(|| PosVaultError::NotFound("HEAD not found".into()))?;

    refs.set_ref(&branch_name, &current_head)
        .map_err(|e| PosVaultError::Storage(e.to_string()))?;
    refs.set_head(&branch_name)
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

    refs.set_head(&full_name)
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
