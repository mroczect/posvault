use posvault_handler::errors::Result;
use posvault_handler::traits::{EventStore, Journal, Signer};
use posvault_handler::types::{
    EncryptedPayload, Event, EventId, Fingerprint, Identity, JournalEntry, Role, Signature,
};
use posvault_sign::ed25519::{Ed25519Signer, generate_keypair};
use posvault_sign::signed_journal::SignedJournal;
use posvault_sign::signed_store::SignedEventStore;

#[derive(Debug)]
struct DummyEventStore;
impl EventStore for DummyEventStore {
    fn append_event(&mut self, _event: Event) -> Result<()> {
        Ok(())
    }
    fn get_events_since(&self, _checkpoint: u64) -> Result<Vec<Event>> {
        Ok(vec![])
    }
    fn latest_checkpoint(&self) -> Result<u64> {
        Ok(0)
    }
}

#[derive(Debug)]
struct DummyJournal;
impl Journal for DummyJournal {
    fn record(&mut self, _entry: JournalEntry) -> Result<()> {
        Ok(())
    }
    fn read_all(&self) -> Result<Vec<JournalEntry>> {
        Ok(vec![])
    }
}

fn create_test_event() -> Event {
    let id = EventId::generate();
    let author = Identity::new(Fingerprint::new("a".repeat(64)).unwrap(), Role::Cashier);
    let payload = EncryptedPayload::new(b"data".to_vec()).unwrap();
    let sig = Signature::new(vec![0u8; 64]).unwrap();
    Event::new(id, 1, author, payload, sig).unwrap()
}

fn create_test_journal_entry() -> JournalEntry {
    JournalEntry::new(
        EventId::generate(),
        1,
        "test".into(),
        Identity::new(Fingerprint::new("a".repeat(64)).unwrap(), Role::Admin),
        "".into(),
        Signature::new(vec![0u8; 64]).unwrap(),
    )
    .unwrap()
}

#[test]
fn test_sign_and_append_event() {
    let (signing_key, _verifying_key) = generate_keypair();
    let signer = Ed25519Signer::new(signing_key);
    let store = DummyEventStore;
    let mut signed_store = SignedEventStore::new(store, signer);
    let event = create_test_event();
    signed_store.append_event(event).unwrap();
}

#[test]
fn test_sign_and_record_journal() {
    let (signing_key, _verifying_key) = generate_keypair();
    let signer = Ed25519Signer::new(signing_key);
    let journal = DummyJournal;
    let mut signed_journal = SignedJournal::new(journal, signer);
    let entry = create_test_journal_entry();
    signed_journal.record(entry).unwrap();
}

#[test]
fn test_signature_verification() {
    let (signing_key, _verifying_key) = generate_keypair();
    let signer = Ed25519Signer::new(signing_key);
    let data = b"hello world";
    let sig = signer.sign(data).unwrap();
    let mut sig_bytes = [0u8; 64];
    sig_bytes.copy_from_slice(&sig);
    let valid = signer.verify(data, &sig_bytes);
    assert!(valid);

    let wrong_data = b"wrong data";
    let invalid = signer.verify(wrong_data, &sig_bytes);
    assert!(!invalid);
}
