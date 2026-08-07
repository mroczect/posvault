use posvault_handler::errors::{PosVaultError, Result};
use posvault_handler::traits::{Journal, Signer};
use posvault_handler::types::{EventId, Identity, JournalEntry, Signature};
use serde::Serialize;
use std::fmt;

#[derive(Serialize)]
struct SignableJournalEntry<'a> {
    id: &'a EventId,
    timestamp: i64,
    action: &'a str,
    author: &'a Identity,
    details: &'a str,
}

pub struct SignedJournal<J: Journal, G: Signer> {
    inner: J,
    signer: G,
}

impl<J: Journal, G: Signer> SignedJournal<J, G> {
    pub fn new(inner: J, signer: G) -> Self {
        SignedJournal { inner, signer }
    }
}

impl<J: Journal + fmt::Debug, G: Signer + fmt::Debug> fmt::Debug for SignedJournal<J, G> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SignedJournal")
            .field("inner", &self.inner)
            .field("signer", &self.signer)
            .finish()
    }
}

impl<J: Journal, G: Signer> Journal for SignedJournal<J, G> {
    fn record(&mut self, mut entry: JournalEntry) -> Result<()> {
        let signable = SignableJournalEntry {
            id: &entry.id,
            timestamp: entry.timestamp,
            action: &entry.action,
            author: &entry.author,
            details: &entry.details,
        };
        let data = serde_json::to_vec(&signable)
            .map_err(|e| PosVaultError::Serialization(e.to_string()))?;
        let signature_bytes = self.signer.sign(&data)?;
        entry.signature = Signature::new(signature_bytes)?;
        self.inner.record(entry)
    }

    fn read_all(&self) -> Result<Vec<JournalEntry>> {
        let entries = self.inner.read_all()?;
        for entry in &entries {
            let signable = SignableJournalEntry {
                id: &entry.id,
                timestamp: entry.timestamp,
                action: &entry.action,
                author: &entry.author,
                details: &entry.details,
            };
            let data = serde_json::to_vec(&signable)
                .map_err(|e| PosVaultError::Serialization(e.to_string()))?;
            if !self.signer.verify(&data, entry.signature.as_bytes()) {
                return Err(PosVaultError::Auth(format!(
                    "Signature verification failed for journal entry {}",
                    entry.id.as_str()
                )));
            }
        }
        Ok(entries)
    }
}
