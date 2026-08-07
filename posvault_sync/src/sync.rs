use libvctrl::domain::user::UserID;
use posvault_handler::errors::{PosVaultError, Result};
use std::path::Path;

pub fn pull_and_merge(
    _local_store_path: &Path,
    _remote_store_path: &Path,
    _author: UserID,
) -> Result<()> {
    Err(PosVaultError::Sync(
        "pull_and_merge is not yet safe due to limitations in libvctrl".into(),
    ))
}
