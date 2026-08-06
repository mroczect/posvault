use posvault_handler::errors::{PosVaultError, Result};
use posvault_handler::traits::Transport;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct FileTransport {
    local_store_path: PathBuf,
    remote_store_path: PathBuf,
}

impl FileTransport {
    pub fn new(local: impl AsRef<Path>, remote: impl AsRef<Path>) -> Self {
        FileTransport {
            local_store_path: local.as_ref().to_path_buf(),
            remote_store_path: remote.as_ref().to_path_buf(),
        }
    }
}

impl Transport for FileTransport {
    fn push(&mut self, _refs: &[String]) -> Result<()> {
        let source = &self.local_store_path;
        let dest = &self.remote_store_path;
        if !source.exists() {
            return Err(PosVaultError::NotFound("local store not found".into()));
        }
        copy_dir_recursive(source, dest)
            .map_err(|e| PosVaultError::Sync(format!("push failed: {}", e)))?;
        Ok(())
    }

    fn pull(&mut self, _refs: &[String]) -> Result<()> {
        let source = &self.remote_store_path;
        let dest = &self.local_store_path;
        if !source.exists() {
            return Err(PosVaultError::NotFound("remote store not found".into()));
        }
        copy_dir_recursive(source, dest)
            .map_err(|e| PosVaultError::Sync(format!("pull failed: {}", e)))?;
        Ok(())
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    if src.is_dir() {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            let new_dst = dst.join(entry.file_name());
            if ty.is_dir() {
                copy_dir_recursive(&entry.path(), &new_dst)?;
            } else {
                fs::copy(entry.path(), new_dst)?;
            }
        }
    } else {
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(src, dst)?;
    }
    Ok(())
}
