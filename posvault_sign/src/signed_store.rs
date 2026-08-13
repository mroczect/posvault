use posvault_handler::errors::{PosVaultError, Result};
use posvault_handler::traits::{EventStore, Signer};
use posvault_handler::types::{EncryptedPayload, Event, EventId, Identity, Signature};
use serde::Serialize;
use std::fmt;

/// Serializable subset of an event used for signing.
#[derive(Serialize)]
struct SignableEvent<'a> {
    id: &'a EventId,
    timestamp: i64,
    author: &'a Identity,
    payload: &'a EncryptedPayload,
}

/// Wraps an [`EventStore`] and signs events before appending them.
///
/// The signature is computed over a JSON representation of the event's
/// stable fields, excluding the signature itself.
pub struct SignedEventStore<S: EventStore, G: Signer> {
    inner: S,
    signer: G,
    strict_verification: bool,
}

impl<S: EventStore, G: Signer> SignedEventStore<S, G> {
    /// Creates a new signed event store with strict verification.
    pub fn new(inner: S, signer: G) -> Self {
        SignedEventStore {
            inner,
            signer,
            strict_verification: true,
        }
    }

    /// Creates a new signed event store that skips invalid events during reads.
    pub fn new_loose(inner: S, signer: G) -> Self {
        SignedEventStore {
            inner,
            signer,
            strict_verification: false,
        }
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
        if event.signature.as_bytes() != [0u8; 64] {
            return Err(PosVaultError::Auth("event already has a signature".into()));
        }

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
        let mut valid_events = Vec::new();

        for ev in &events {
            let signable = SignableEvent {
                id: &ev.id,
                timestamp: ev.timestamp,
                author: &ev.author,
                payload: &ev.payload,
            };
            let data = serde_json::to_vec(&signable)
                .map_err(|e| PosVaultError::Serialization(e.to_string()))?;
            if self.signer.verify(&data, ev.signature.as_bytes()) {
                valid_events.push(ev.clone());
            } else {
                log::warn!("Signature verification failed for event {}", ev.id.as_str());
                if self.strict_verification {
                    return Err(PosVaultError::Auth(format!(
                        "Signature verification failed for event {}",
                        ev.id.as_str()
                    )));
                }
            }
        }

        Ok(valid_events)
    }

    fn latest_checkpoint(&self) -> Result<u64> {
        self.inner.latest_checkpoint()
    }
}
