use crate::errors::{PosVaultError, Result};
use crate::validation::Validate;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use zeroize::Zeroizing;

// ---------------------------------------------------------------------------
// ---------------------------------------------------------------------------
#[derive(Clone)]
pub struct SecretData(Zeroizing<Vec<u8>>);

impl SecretData {
    pub fn new(data: Vec<u8>) -> Result<Self> {
        if data.is_empty() {
            return Err(PosVaultError::invalid_input(
                "SecretData tidak boleh kosong",
            ));
        }
        Ok(SecretData(Zeroizing::new(data)))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(&self.0)
    }

    pub fn from_hex(hex_str: &str) -> Result<Self> {
        let bytes =
            hex::decode(hex_str).map_err(|_| PosVaultError::invalid_input("hex tidak valid"))?;
        SecretData::new(bytes)
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(String);

impl EventId {
    pub fn new(id: impl Into<String>) -> Result<Self> {
        let id = id.into();
        if id.is_empty() || id.len() > 64 {
            return Err(PosVaultError::invalid_input("EventId harus 1..64 karakter"));
        }
        if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err(PosVaultError::invalid_input(
                "EventId hanya boleh alfanumerik + '-'",
            ));
        }
        Ok(EventId(id))
    }

    pub fn generate() -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        EventId::new(id).expect("UUID v4 harus selalu valid untuk EventId")
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Fingerprint(String);

impl Fingerprint {
    pub fn new(hex: impl Into<String>) -> Result<Self> {
        let hex = hex.into();
        if hex.len() != 64 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(PosVaultError::invalid_input(
                "Fingerprint harus 64 hex karakter",
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

#[derive(Clone)]
pub struct Signature(Zeroizing<Vec<u8>>);

impl Signature {
    pub fn new(bytes: Vec<u8>) -> Result<Self> {
        if bytes.len() != 64 {
            return Err(PosVaultError::invalid_input("Signature harus 64 bytes"));
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
            return Err(PosVaultError::invalid_input(
                "EncryptedPayload tidak boleh kosong",
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
        let event = Event {
            id,
            timestamp,
            author,
            payload,
            signature,
        };
        event.validate()?;
        Ok(event)
    }
}

impl Validate for Event {
    fn validate(&self) -> Result<()> {
        if self.timestamp <= 0 {
            return Err(PosVaultError::invalid_input("timestamp harus > 0"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recipient(String);

impl Recipient {
    pub fn new(key: impl Into<String>) -> Result<Self> {
        let key = key.into();
        if !key.starts_with("age1") || key.len() <= 4 || key.len() > 512 {
            return Err(PosVaultError::invalid_input(
                "Kunci publik age tidak valid (prefix atau panjang)",
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
            return Err(PosVaultError::invalid_input(
                "Nama branch harus 1..255 karakter",
            ));
        }
        if !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '/')
        {
            return Err(PosVaultError::invalid_input(
                "Nama branch mengandung karakter tidak valid",
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
            hex::decode(hex_str).map_err(|_| PosVaultError::invalid_input("hex tidak valid"))?;
        if bytes.len() != 64 {
            return Err(PosVaultError::invalid_input("Commit hash harus 64 bytes"));
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

    pub fn is_zero(&self) -> bool {
        self.0.iter().all(|&b| b == 0)
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
    pub fn new(version: u64, data: EncryptedPayload, hash: CommitHash) -> Result<Self> {
        let snap = Snapshot {
            version,
            data,
            hash,
        };
        snap.validate()?;
        Ok(snap)
    }
}

impl Validate for Snapshot {
    fn validate(&self) -> Result<()> {
        if self.version == 0 {
            return Err(PosVaultError::invalid_input("Versi snapshot harus > 0"));
        }
        if self.hash.is_zero() {
            return Err(PosVaultError::invalid_input("Commit hash tidak boleh nol"));
        }
        Ok(())
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
        let entry = JournalEntry {
            id,
            timestamp,
            action,
            author,
            details,
            signature,
        };
        entry.validate()?;
        Ok(entry)
    }
}

impl Validate for JournalEntry {
    fn validate(&self) -> Result<()> {
        if self.timestamp <= 0 {
            return Err(PosVaultError::invalid_input("timestamp harus > 0"));
        }
        if self.action.is_empty() || self.action.len() > 256 {
            return Err(PosVaultError::invalid_input("Action harus 1..256 karakter"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_id_generate_always_valid() {
        for _ in 0..100 {
            let id = EventId::generate();
            assert!(EventId::new(id.as_str()).is_ok());
        }
    }

    #[test]
    fn recipient_allows_valid_age_keys() {
        assert!(
            Recipient::new("age1yt4hxqdqp2vt0zr0h6z6f0g4f6z5x5p6xjxpyf6v7z6qk8w0e3srs9m0j").is_ok()
        );
        assert!(Recipient::new("abc123").is_err());
    }

    #[test]
    fn snapshot_rejects_zero_hash() {
        let payload = EncryptedPayload::new(vec![1, 2, 3]).unwrap();
        let hash = CommitHash::from_bytes([0u8; 64]);
        assert!(Snapshot::new(1, payload, hash).is_err());
    }
}
