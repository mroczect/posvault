use posvault_handler::errors::{PosVaultError, Result};
use posvault_handler::types::Role;

use crate::session::Session;

pub fn require_role(session: &Session, allowed: &[Role]) -> Result<()> {
    if !allowed.contains(&session.role) {
        return Err(PosVaultError::Auth(format!(
            "role {:?} is not allowed; required one of {:?}",
            session.role, allowed
        )));
    }
    Ok(())
}
