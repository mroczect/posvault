use posvault_handler::errors::{PosVaultError, Result};
use posvault_handler::types::BranchName;
use posvault_store::FileStore;

/// Creates a new branch for a store and switches HEAD to it.
///
/// The branch name is derived from `store_id` and is always placed under
/// `refs/heads/`. The new branch points to the current HEAD commit.
pub fn create_store_branch(store: &mut FileStore, store_id: &str) -> Result<BranchName> {
    let branch_name = format!("refs/heads/store-{}", store_id);
    let current_head = store
        .head()?
        .ok_or_else(|| PosVaultError::NotFound("HEAD not found".into()))?;

    store
        .set_ref(&branch_name, &current_head)
        .map_err(|e| PosVaultError::Storage(e.to_string()))?;
    store
        .set_head(&branch_name)
        .map_err(|e| PosVaultError::Storage(e.to_string()))?;

    BranchName::new(branch_name.trim_start_matches("refs/heads/"))
}

/// Switches HEAD to the given branch.
///
/// Fails if the branch does not exist.
pub fn checkout_branch(store: &mut FileStore, branch_name: &BranchName) -> Result<()> {
    let full_name = format!("refs/heads/{}", branch_name.as_str());
    store
        .get_ref(&full_name)
        .map_err(|e| PosVaultError::Storage(e.to_string()))?
        .ok_or_else(|| {
            PosVaultError::NotFound(format!("branch '{}' not found", branch_name.as_str()))
        })?;

    store
        .set_head(&full_name)
        .map_err(|e| PosVaultError::Storage(e.to_string()))?;
    Ok(())
}

/// Returns the currently checked-out branch, if any.
///
/// The returned name is the short branch name without `refs/heads/`.
pub fn current_branch(store: &FileStore) -> Result<Option<BranchName>> {
    let head_ref = store
        .head_ref_name()
        .map_err(|e| PosVaultError::Storage(e.to_string()))?;
    match head_ref {
        Some(name) if name.starts_with("refs/heads/") => {
            BranchName::new(name.trim_start_matches("refs/heads/")).map(Some)
        }
        _ => Ok(None),
    }
}
