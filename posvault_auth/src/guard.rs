use posvault_handler::errors::{PosVaultError, Result};
use posvault_handler::types::Role;

use crate::session::Session;

pub fn require_role(session: &Session, allowed: &[Role]) -> Result<()> {
    if session.is_expired() {
        return Err(PosVaultError::Auth("session expired".into()));
    }
    if !allowed.contains(&session.role) {
        return Err(PosVaultError::Auth(format!(
            "role {:?} is not allowed; required one of {:?}",
            session.role, allowed
        )));
    }
    Ok(())
}
