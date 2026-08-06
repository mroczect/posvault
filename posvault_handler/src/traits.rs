use crate::errors::Result;
use crate::types::{Event, JournalEntry, Snapshot};
use std::fmt::Debug;

pub trait EventStore: Debug + Send + Sync {
    fn append_event(&mut self, event: Event) -> Result<()>;
    fn get_events_since(&self, checkpoint: u64) -> Result<Vec<Event>>;
    fn latest_checkpoint(&self) -> Result<u64>;
}

pub trait SnapshotStore: Debug + Send + Sync {
    fn save_snapshot(&mut self, snapshot: Snapshot) -> Result<()>;
    fn load_snapshot(&self) -> Result<Option<Snapshot>>;
}

pub trait Journal: Debug + Send + Sync {
    fn record(&mut self, entry: JournalEntry) -> Result<()>;
    fn read_all(&self) -> Result<Vec<JournalEntry>>;
}

pub trait Signer: Debug + Send + Sync {
    fn sign(&self, data: &[u8]) -> Result<Vec<u8>>;
    fn verify(&self, data: &[u8], signature: &[u8]) -> Result<bool>;
}

pub trait ConflictResolver: Debug + Send + Sync {
    fn resolve(&self, base: &[u8], ours: &[u8], theirs: &[u8]) -> Result<Vec<u8>>;
}

pub trait Transport: Debug + Send + Sync {
    fn push(&mut self, refs: &[String]) -> Result<()>;
    fn pull(&mut self, refs: &[String]) -> Result<()>;
}

pub trait EventCodec: Debug + Send + Sync {
    fn encode(&self, event: &Event) -> Result<Vec<u8>>;
    fn decode(&self, data: &[u8]) -> Result<Event>;
}
