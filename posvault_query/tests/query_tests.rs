use posvault_handler::traits::{EventStore, SnapshotStore};
use posvault_handler::types::{
    EncryptedPayload, Event, EventId, Fingerprint, Identity, Role, Signature, Snapshot,
};
use posvault_query::engine::QueryEngine;
use posvault_query::examples::get_stock;

#[derive(Debug)]
struct DummyStore {
    events: Vec<Event>,
    snapshot: Option<Snapshot>,
}

impl EventStore for DummyStore {
    fn append_event(&mut self, _event: Event) -> posvault_handler::errors::Result<()> {
        Ok(())
    }
    fn get_events_since(&self, checkpoint: u64) -> posvault_handler::errors::Result<Vec<Event>> {
        Ok(self.events[checkpoint as usize..].to_vec())
    }
    fn latest_checkpoint(&self) -> posvault_handler::errors::Result<u64> {
        Ok(self.events.len() as u64)
    }
}

impl SnapshotStore for DummyStore {
    fn save_snapshot(&mut self, snapshot: Snapshot) -> posvault_handler::errors::Result<()> {
        self.snapshot = Some(snapshot);
        Ok(())
    }
    fn load_snapshot(&self) -> posvault_handler::errors::Result<Option<Snapshot>> {
        Ok(self.snapshot.clone())
    }
}

fn make_event(payload: &[u8]) -> Event {
    let id = EventId::generate();
    let author = Identity::new(Fingerprint::new("a".repeat(64)).unwrap(), Role::Cashier);
    let payload = EncryptedPayload::new(payload.to_vec()).unwrap();
    let sig = Signature::new(vec![0u8; 64]).unwrap();
    Event::new(id, 1, author, payload, sig).unwrap()
}

#[test]
fn test_stock_query() {
    let mut store = DummyStore {
        events: vec![],
        snapshot: None,
    };
    store.events.push(make_event(
        serde_json::to_vec(&("apple", 10i64)).unwrap().as_slice(),
    ));
    store.events.push(make_event(
        serde_json::to_vec(&("banana", 5i64)).unwrap().as_slice(),
    ));
    store.events.push(make_event(
        serde_json::to_vec(&("apple", -2i64)).unwrap().as_slice(),
    ));

    let decrypt = &|data: &[u8]| -> posvault_handler::errors::Result<Vec<u8>> { Ok(data.to_vec()) };
    let encrypt = &|data: &[u8]| -> posvault_handler::errors::Result<Vec<u8>> { Ok(data.to_vec()) };

    let mut engine = QueryEngine::new(store);
    let apple_stock = get_stock(&mut engine, decrypt, encrypt, "apple").unwrap();
    assert_eq!(apple_stock, 8);
    let banana_stock = get_stock(&mut engine, decrypt, encrypt, "banana").unwrap();
    assert_eq!(banana_stock, 5);
}
