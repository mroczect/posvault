use posvault_handler::*;
use posvault_store::*;
use tempfile::TempDir;

fn setup_vault() -> (TempDir, PosVault) {
    let dir = TempDir::new().unwrap();
    let vault = PosVault::open(dir.path()).unwrap();
    (dir, vault)
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

#[test]
fn test_event_append_and_latest_checkpoint() {
    let (_dir, vault) = setup_vault();
    let store_arc = vault.store_arc();
    let mut store = VctrlEventStore::new(store_arc);
    assert_eq!(store.latest_checkpoint().unwrap(), 0);
    let event = create_test_event(1, Role::Cashier);
    store.append_event(event).unwrap();
    assert_eq!(store.latest_checkpoint().unwrap(), 1);
}

#[test]
fn test_append_multiple_events_increases_checkpoint() {
    let (_dir, vault) = setup_vault();
    let store_arc = vault.store_arc();
    let mut store = VctrlEventStore::new(store_arc);
    for i in 1..=5 {
        store
            .append_event(create_test_event(i, Role::Cashier))
            .unwrap();
    }
    assert_eq!(store.latest_checkpoint().unwrap(), 5);
}

#[test]
fn test_get_events_since_returns_events() {
    let (_dir, vault) = setup_vault();
    let store_arc = vault.store_arc();
    let mut store = VctrlEventStore::new(store_arc);
    store
        .append_event(create_test_event(1, Role::Cashier))
        .unwrap();
    store
        .append_event(create_test_event(2, Role::Manager))
        .unwrap();
    store
        .append_event(create_test_event(3, Role::Admin))
        .unwrap();

    let recent = store.get_events_since(1).unwrap();
    assert_eq!(recent.len(), 2, "harusnya ada 2 event setelah checkpoint 1");
    assert!(recent[0].timestamp <= recent[1].timestamp);
}

#[test]
fn test_save_and_load_snapshot() {
    let (_dir, vault) = setup_vault();
    let mut snap_store = VctrlSnapshotStore::new(vault.store_arc());
    let hash = CommitHash::from_bytes([1u8; 64]);
    let snapshot = Snapshot::new(1, EncryptedPayload::new(b"hello".to_vec()).unwrap(), hash)
        .expect("snapshot valid");

    snap_store.save_snapshot(snapshot.clone()).unwrap();
    let loaded = snap_store.load_snapshot().unwrap().unwrap();
    assert_eq!(loaded.data, snapshot.data);
    assert_eq!(loaded.hash, snapshot.hash);
}

#[test]
fn test_load_snapshot_empty() {
    let (_dir, vault) = setup_vault();
    let store_arc = vault.store_arc();
    let store = VctrlSnapshotStore::new(store_arc);
    assert!(store.load_snapshot().unwrap().is_none());
}

#[test]
fn test_journal_record_and_read_all() {
    let (_dir, vault) = setup_vault();
    let store_arc = vault.store_arc();
    let mut journal = VctrlJournal::new(store_arc);
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
    let store_arc = vault.store_arc();
    let threshold: u64 = 3;
    let mut journal = VctrlJournal::with_threshold(store_arc, threshold);
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
    let store_arc = vault.store_arc();
    let journal = VctrlJournal::new(store_arc);
    assert_eq!(journal.read_all().unwrap().len(), 0);
}
