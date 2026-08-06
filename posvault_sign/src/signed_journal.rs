use posvault_handler::errors::Result;
use posvault_handler::traits::{Journal, Signer};
use posvault_handler::types::JournalEntry;
use std::fmt;

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
        let data = serde_json::to_vec(&entry)
            .map_err(|e| posvault_handler::errors::PosVaultError::Serialization(e.to_string()))?;
        let signature = self.signer.sign(&data)?;
        entry.signature = posvault_handler::types::Signature::new(signature)?;
        self.inner.record(entry)
    }

    fn read_all(&self) -> Result<Vec<JournalEntry>> {
        self.inner.read_all()
    }
}
