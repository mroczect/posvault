use libvctrl::UserID;
use posvault_handler::errors::{PosVaultError, Result};
use std::path::Path;

/// Placeholder for cross-repository merge.
///
/// This operation is not yet implemented safely because the underlying
/// storage layer is still in-memory and does not provide atomic
/// cross-repository synchronization. It returns a descriptive error.
pub fn pull_and_merge(
    _local_store_path: &Path,
    _remote_store_path: &Path,
    _author: UserID,
) -> Result<()> {
    Err(PosVaultError::Sync(
        "pull_and_merge is not yet safe due to limitations in libvctrl".into(),
    ))
}
