use bincode;
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
    strict_verification: bool,
}

impl<J: Journal, G: Signer> SignedJournal<J, G> {
    pub fn new(inner: J, signer: G) -> Self {
        SignedJournal {
            inner,
            signer,
            strict_verification: true,
        }
    }

    pub fn new_loose(inner: J, signer: G) -> Self {
        SignedJournal {
            inner,
            signer,
            strict_verification: false,
        }
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
        if entry.signature.as_bytes() != [0u8; 64] {
            return Err(PosVaultError::Auth("entry already has a signature".into()));
        }

        let signable = SignableJournalEntry {
            id: &entry.id,
            timestamp: entry.timestamp,
            action: &entry.action,
            author: &entry.author,
            details: &entry.details,
        };
        let data = bincode::serialize(&signable)
            .map_err(|e| PosVaultError::Serialization(e.to_string()))?;
        let signature_bytes = self.signer.sign(&data)?;
        entry.signature = Signature::new(signature_bytes)?;
        self.inner.record(entry)
    }

    fn read_all(&self) -> Result<Vec<JournalEntry>> {
        let entries = self.inner.read_all()?;
        let mut valid_entries = Vec::new();

        for entry in &entries {
            let signable = SignableJournalEntry {
                id: &entry.id,
                timestamp: entry.timestamp,
                action: &entry.action,
                author: &entry.author,
                details: &entry.details,
            };
            let data = bincode::serialize(&signable)
                .map_err(|e| PosVaultError::Serialization(e.to_string()))?;
            if self.signer.verify(&data, entry.signature.as_bytes()) {
                valid_entries.push(entry.clone());
            } else {
                log::warn!(
                    "Signature verification failed for journal entry {}",
                    entry.id.as_str()
                );
                if self.strict_verification {
                    return Err(PosVaultError::Auth(format!(
                        "Signature verification failed for journal entry {}",
                        entry.id.as_str()
                    )));
                }
            }
        }

        Ok(valid_entries)
    }
}
