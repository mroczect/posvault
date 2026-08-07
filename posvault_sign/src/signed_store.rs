use posvault_handler::errors::{PosVaultError, Result};
use posvault_handler::traits::{EventStore, Signer};
use posvault_handler::types::{EncryptedPayload, Event, EventId, Identity, Signature};
use serde::Serialize;
use std::fmt;

#[derive(Serialize)]
struct SignableEvent<'a> {
    id: &'a EventId,
    timestamp: i64,
    author: &'a Identity,
    payload: &'a EncryptedPayload,
}

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
        let signable = SignableEvent {
            id: &event.id,
            timestamp: event.timestamp,
            author: &event.author,
            payload: &event.payload,
        };
        let data = serde_json::to_vec(&signable)
            .map_err(|e| PosVaultError::Serialization(e.to_string()))?;
        let signature_bytes = self.signer.sign(&data)?;
        event.signature = Signature::new(signature_bytes)?;
        self.inner.append_event(event)
    }

    fn get_events_since(&self, checkpoint: u64) -> Result<Vec<Event>> {
        let events = self.inner.get_events_since(checkpoint)?;
        for ev in &events {
            let signable = SignableEvent {
                id: &ev.id,
                timestamp: ev.timestamp,
                author: &ev.author,
                payload: &ev.payload,
            };
            let data = serde_json::to_vec(&signable)
                .map_err(|e| PosVaultError::Serialization(e.to_string()))?;
            if !self.signer.verify(&data, ev.signature.as_bytes()) {
                return Err(PosVaultError::Auth(format!(
                    "Signature verification failed for event {}",
                    ev.id.as_str()
                )));
            }
        }
        Ok(events)
    }

    fn latest_checkpoint(&self) -> Result<u64> {
        self.inner.latest_checkpoint()
    }
}
