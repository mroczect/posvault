use crate::errors::{PosVaultError, Result};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use zeroize::Zeroizing;

#[derive(Clone)]
pub struct SecretData(Zeroizing<Vec<u8>>);

impl SecretData {
    pub fn new(data: Vec<u8>) -> Result<Self> {
        if data.is_empty() {
            return Err(PosVaultError::InvalidInput(
                "SecretData cannot be empty".into(),
            ));
        }
        Ok(SecretData(Zeroizing::new(data)))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Debug for SecretData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecretData").finish()
    }
}

impl PartialEq for SecretData {
    fn eq(&self, other: &Self) -> bool {
        *self.0 == *other.0
    }
}

impl Eq for SecretData {}

impl Serialize for SecretData {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SecretData {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let bytes: Vec<u8> = Vec::deserialize(deserializer)?;
        SecretData::new(bytes).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: EventId,
    pub timestamp: i64,
    pub author: Identity,
    pub payload: EncryptedPayload,
    pub signature: Signature,
}

impl Event {
    pub fn new(
        id: EventId,
        timestamp: i64,
        author: Identity,
        payload: EncryptedPayload,
        signature: Signature,
    ) -> Result<Self> {
        if timestamp <= 0 {
            return Err(PosVaultError::InvalidInput("timestamp must be > 0".into()));
        }
        Ok(Event {
            id,
            timestamp,
            author,
            payload,
            signature,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(String);

impl EventId {
    pub fn new(id: impl Into<String>) -> Result<Self> {
        let id = id.into();
        if id.is_empty() || id.len() > 64 {
            return Err(PosVaultError::InvalidInput(
                "EventId must be 1..64 chars".into(),
            ));
        }
        if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err(PosVaultError::InvalidInput(
                "EventId must be alphanumeric + hyphens".into(),
            ));
        }
        Ok(EventId(id))
    }

    pub fn generate() -> Self {
        EventId(uuid::Uuid::new_v4().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identity {
    pub fingerprint: Fingerprint,
    pub role: Role,
}

impl Identity {
    pub fn new(fingerprint: Fingerprint, role: Role) -> Self {
        Identity { fingerprint, role }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Fingerprint(String);

impl Fingerprint {
    pub fn new(hex: impl Into<String>) -> Result<Self> {
        let hex = hex.into();
        if hex.len() != 64 {
            return Err(PosVaultError::InvalidInput(
                "Fingerprint must be 64 hex chars".into(),
            ));
        }
        if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(PosVaultError::InvalidInput(
                "Fingerprint must be hexadecimal".into(),
            ));
        }
        Ok(Fingerprint(hex))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    Admin,
    Manager,
    Cashier,
    Auditor,
    Branch,
    Custom(String),
}

impl Role {
    pub fn as_str(&self) -> &str {
        match self {
            Role::Admin => "admin",
            Role::Manager => "manager",
            Role::Cashier => "cashier",
            Role::Auditor => "auditor",
            Role::Branch => "branch",
            Role::Custom(s) => s.as_str(),
        }
    }
}

#[derive(Clone)]
pub struct Signature(Zeroizing<Vec<u8>>);

impl Signature {
    pub fn new(bytes: Vec<u8>) -> Result<Self> {
        if bytes.len() != 64 {
            return Err(PosVaultError::InvalidInput(
                "Signature must be 64 bytes".into(),
            ));
        }
        Ok(Signature(Zeroizing::new(bytes)))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Debug for Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Signature").finish()
    }
}

impl PartialEq for Signature {
    fn eq(&self, other: &Self) -> bool {
        *self.0 == *other.0
    }
}

impl Eq for Signature {}

impl Serialize for Signature {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Signature {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let bytes: Vec<u8> = Vec::deserialize(deserializer)?;
        Signature::new(bytes).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone)]
pub struct EncryptedPayload(Zeroizing<Vec<u8>>);

impl EncryptedPayload {
    pub fn new(data: Vec<u8>) -> Result<Self> {
        if data.is_empty() {
            return Err(PosVaultError::InvalidInput(
                "EncryptedPayload cannot be empty".into(),
            ));
        }
        Ok(EncryptedPayload(Zeroizing::new(data)))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Debug for EncryptedPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EncryptedPayload").finish()
    }
}

impl PartialEq for EncryptedPayload {
    fn eq(&self, other: &Self) -> bool {
        *self.0 == *other.0
    }
}

impl Eq for EncryptedPayload {}

impl Serialize for EncryptedPayload {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for EncryptedPayload {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let bytes: Vec<u8> = Vec::deserialize(deserializer)?;
        EncryptedPayload::new(bytes).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recipient(String);

impl Recipient {
    pub fn new(key: impl Into<String>) -> Result<Self> {
        let key = key.into();
        if !key.starts_with("age1") || key.len() <= 4 {
            return Err(PosVaultError::InvalidInput("Invalid age public key".into()));
        }
        if !key[4..]
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        {
            return Err(PosVaultError::InvalidInput(
                "Public key contains invalid characters".into(),
            ));
        }
        Ok(Recipient(key))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchName(String);

impl BranchName {
    pub fn new(name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        if name.is_empty() || name.len() > 255 {
            return Err(PosVaultError::InvalidInput(
                "Branch name must be 1..255 chars".into(),
            ));
        }
        if !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '/')
        {
            return Err(PosVaultError::InvalidInput(
                "Branch name has invalid chars".into(),
            ));
        }
        Ok(BranchName(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CommitHash([u8; 64]);

impl CommitHash {
    pub fn from_bytes(bytes: [u8; 64]) -> Self {
        CommitHash(bytes)
    }

    pub fn from_hex(hex_str: &str) -> Result<Self> {
        let bytes =
            hex::decode(hex_str).map_err(|_| PosVaultError::InvalidInput("Invalid hex".into()))?;
        if bytes.len() != 64 {
            return Err(PosVaultError::InvalidInput(
                "Commit hash must be 64 bytes".into(),
            ));
        }
        let mut arr = [0u8; 64];
        arr.copy_from_slice(&bytes);
        Ok(CommitHash(arr))
    }

    pub fn as_bytes(&self) -> &[u8; 64] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

impl Serialize for CommitHash {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        self.to_hex().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for CommitHash {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        CommitHash::from_hex(&s).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub version: u64,
    pub data: EncryptedPayload,
    pub hash: CommitHash,
}

impl Snapshot {
    pub fn new(version: u64, data: EncryptedPayload, hash: CommitHash) -> Self {
        Snapshot {
            version,
            data,
            hash,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    pub id: EventId,
    pub timestamp: i64,
    pub action: String,
    pub author: Identity,
    pub details: String,
    pub signature: Signature,
}

impl JournalEntry {
    pub fn new(
        id: EventId,
        timestamp: i64,
        action: String,
        author: Identity,
        details: String,
        signature: Signature,
    ) -> Result<Self> {
        if timestamp <= 0 {
            return Err(PosVaultError::InvalidInput("timestamp must be > 0".into()));
        }
        if action.is_empty() || action.len() > 256 {
            return Err(PosVaultError::InvalidInput(
                "Action must be 1..256 chars".into(),
            ));
        }
        Ok(JournalEntry {
            id,
            timestamp,
            action,
            author,
            details,
            signature,
        })
    }
}
