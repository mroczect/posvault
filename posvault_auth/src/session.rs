use posvault_handler::types::{Fingerprint, Role};
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_SESSION_DURATION: u64 = 8 * 3600;

#[derive(Debug, Clone)]
pub struct Session {
    pub fingerprint: Fingerprint,
    pub role: Role,
    created_at: u64,
    expires_at: u64,
}

impl Session {
    pub fn new(fingerprint: Fingerprint, role: Role) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            fingerprint,
            role,
            created_at: now,
            expires_at: now + DEFAULT_SESSION_DURATION,
        }
    }

    pub fn with_duration(fingerprint: Fingerprint, role: Role, duration_secs: u64) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            fingerprint,
            role,
            created_at: now,
            expires_at: now + duration_secs,
        }
    }

    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now >= self.expires_at
    }

    pub fn refresh(&mut self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.created_at = now;
        self.expires_at = now + DEFAULT_SESSION_DURATION;
    }

    pub fn created_at(&self) -> u64 {
        self.created_at
    }
    pub fn expires_at(&self) -> u64 {
        self.expires_at
    }
}
