use posvault_handler::types::{Fingerprint, Role};

#[derive(Debug, Clone)]
pub struct Session {
    pub fingerprint: Fingerprint,
    pub role: Role,
}

impl Session {
    pub fn new(fingerprint: Fingerprint, role: Role) -> Self {
        Session { fingerprint, role }
    }
}
