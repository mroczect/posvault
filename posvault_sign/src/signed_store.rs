use posvault_handler::errors::Result;
use posvault_handler::traits::{EventStore, Signer};
use posvault_handler::types::Event;
use std::fmt;

pub struct SignedEventStore<S: EventStore, G: Signer> {
    inner: S,
    signer: G,
}

impl<S: EventStore, G: Signer> SignedEventStore<S, G> {
    pub fn new(inner: S, signer: G) -> Self {
        SignedEventStore { inner, signer }
    }
}

impl<S: EventStore + fmt::Debug, G: Signer + fmt::Debug> fmt::Debug for SignedEventStore<S, G> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SignedEventStore")
            .field("inner", &self.inner)
            .field("signer", &self.signer)
            .finish()
    }
}

impl<S: EventStore, G: Signer> EventStore for SignedEventStore<S, G> {
    fn append_event(&mut self, mut event: Event) -> Result<()> {
        let data = serde_json::to_vec(&event)
            .map_err(|e| posvault_handler::errors::PosVaultError::Serialization(e.to_string()))?;
        let signature = self.signer.sign(&data)?;
        event.signature = posvault_handler::types::Signature::new(signature)?;
        self.inner.append_event(event)
    }

    fn get_events_since(&self, checkpoint: u64) -> Result<Vec<Event>> {
        self.inner.get_events_since(checkpoint)
    }

    fn latest_checkpoint(&self) -> Result<u64> {
        self.inner.latest_checkpoint()
    }
}
