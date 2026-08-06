use posvault_handler::*;
use posvault_store::*;
use tempfile::TempDir;

fn setup_vault() -> (TempDir, PosVault) {
    let dir = TempDir::new().unwrap();
    let vault = PosVault::open(dir.path()).unwrap();
    (dir, vault)
}

#[test]
fn test_event_append_and_latest_checkpoint() {
    let (_dir, vault) = setup_vault();
    let mut store = VctrlEventStore::new(vault);
    assert_eq!(store.latest_checkpoint().unwrap(), 0);
    let event = create_test_event(1, Role::Cashier);
    store.append_event(event).unwrap();
    assert_eq!(store.latest_checkpoint().unwrap(), 1);
}

#[test]
fn test_get_events_since_not_implemented() {
    let (_dir, vault) = setup_vault();
    let store = VctrlEventStore::new(vault);
    let result = store.get_events_since(0);
    assert!(result.is_err());
    match result.unwrap_err() {
        PosVaultError::InvalidInput(msg) => assert!(msg.contains("not yet implemented")),
        _ => panic!("Expected InvalidInput"),
    }
}

#[test]
fn test_append_multiple_events_increases_checkpoint() {
    let (_dir, vault) = setup_vault();
    let mut store = VctrlEventStore::new(vault);
    for i in 1..6 {
        store
            .append_event(create_test_event(i, Role::Cashier))
            .unwrap();
    }
    assert_eq!(store.latest_checkpoint().unwrap(), 5);
}

#[test]
fn test_save_and_load_snapshot() {
    let (_dir, vault) = setup_vault();
    let mut snap_store = VctrlSnapshotStore::new(vault);
    let snapshot = Snapshot::new(
        1,
        EncryptedPayload::new(b"hello".to_vec()).unwrap(),
        CommitHash::from_bytes([0u8; 64]),
    );
    snap_store.save_snapshot(snapshot.clone()).unwrap();
    let loaded = snap_store.load_snapshot().unwrap().unwrap();
    assert_eq!(loaded.version, 1);
    assert_eq!(loaded.data, snapshot.data);
    assert_eq!(loaded.hash, snapshot.hash);
}

#[test]
fn test_load_snapshot_empty() {
    let (_dir, vault) = setup_vault();
    let store = VctrlSnapshotStore::new(vault);
    assert!(store.load_snapshot().unwrap().is_none());
}

#[test]
fn test_journal_record_and_read_all() {
    let (_dir, vault) = setup_vault();
    let mut journal = VctrlJournal::new(vault);
    let entry1 = create_test_journal_entry(1, "action1");
    let entry2 = create_test_journal_entry(2, "action2");
    journal.record(entry1.clone()).unwrap();
    journal.record(entry2.clone()).unwrap();
    let all = journal.read_all().unwrap();
    assert_eq!(all.len(), 2);
    let actions: Vec<&str> = all.iter().map(|e| e.action.as_str()).collect();
    assert!(actions.contains(&"action1"));
    assert!(actions.contains(&"action2"));
}

#[test]
fn test_journal_compaction() {
    let (_dir, vault) = setup_vault();
    let threshold: u64 = 3;
    let mut journal = VctrlJournal::with_threshold(vault, threshold);
    for i in 0..threshold + 1 {
        let entry = create_test_journal_entry(i as i64 + 10, &format!("action{}", i));
        journal.record(entry).unwrap();
    }
    let all = journal.read_all().unwrap();
    assert_eq!(all.len(), (threshold + 1) as usize);
}

#[test]
fn test_journal_empty_read() {
    let (_dir, vault) = setup_vault();
    let journal = VctrlJournal::new(vault);
    assert_eq!(journal.read_all().unwrap().len(), 0);
}

fn create_test_event(timestamp: i64, role: Role) -> Event {
    let id = EventId::generate();
    let fingerprint = Fingerprint::new("a".repeat(64)).unwrap();
    let author = Identity::new(fingerprint, role);
    let payload = EncryptedPayload::new(b"test payload".to_vec()).unwrap();
    let signature = Signature::new(vec![0u8; 64]).unwrap();
    Event::new(id, timestamp.max(1), author, payload, signature).unwrap()
}

fn create_test_journal_entry(timestamp: i64, action: &str) -> JournalEntry {
    JournalEntry::new(
        EventId::generate(),
        timestamp.max(1),
        action.to_owned(),
        Identity::new(Fingerprint::new("a".repeat(64)).unwrap(), Role::Admin),
        "details".to_owned(),
        Signature::new(vec![0u8; 64]).unwrap(),
    )
    .unwrap()
}
