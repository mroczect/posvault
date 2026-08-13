use posvault_handler::*;

#[test]
fn secret_data_new_valid() {
    let s = SecretData::new(b"secret".to_vec()).unwrap();
    assert!(!s.as_bytes().is_empty());
}

#[test]
fn secret_data_empty_fails() {
    assert!(SecretData::new(vec![]).is_err());
}

#[test]
fn secret_data_hex_roundtrip() {
    let original = SecretData::new(b"test".to_vec()).unwrap();
    let hex = original.to_hex();
    let restored = SecretData::from_hex(&hex).unwrap();
    assert_eq!(original.as_bytes(), restored.as_bytes());
}

#[test]
fn secret_data_hex_invalid() {
    assert!(SecretData::from_hex("nothex!").is_err());
}

#[test]
fn event_id_valid() {
    let id = EventId::new("abc-123").unwrap();
    assert_eq!(id.as_str(), "abc-123");
}

#[test]
fn event_id_empty() {
    assert!(EventId::new("").is_err());
}

#[test]
fn event_id_invalid_chars() {
    assert!(EventId::new("abc def").is_err());
    assert!(EventId::new("abc.def").is_err());
}

#[test]
fn event_id_max_length() {
    let long = "a".repeat(64);
    assert!(EventId::new(&long).is_ok());
    let too_long = "a".repeat(65);
    assert!(EventId::new(&too_long).is_err());
}

#[test]
fn event_id_generate_is_valid() {
    for _ in 0..100 {
        let id = EventId::generate();
        assert!(EventId::new(id.as_str()).is_ok());
    }
}

#[test]
fn event_id_generate_is_unique() {
    let mut set = std::collections::HashSet::new();
    for _ in 0..1000 {
        set.insert(EventId::generate().as_str().to_owned());
    }
    assert!(set.len() > 999);
}

#[test]
fn fingerprint_valid() {
    let hex = "a".repeat(64);
    let fp = Fingerprint::new(&hex).unwrap();
    assert_eq!(fp.as_str().len(), 64);
}

#[test]
fn fingerprint_invalid_length() {
    assert!(Fingerprint::new("abc").is_err());
    assert!(Fingerprint::new("a".repeat(63)).is_err());
    assert!(Fingerprint::new("a".repeat(65)).is_err());
}

#[test]
fn fingerprint_invalid_chars() {
    assert!(Fingerprint::new("z".repeat(64)).is_err());
}

#[test]
fn recipient_valid_minimal() {
    let key = "age1abcde";
    let rec = Recipient::new(key).unwrap();
    assert!(rec.as_str().starts_with("age1"));
}

#[test]
fn recipient_invalid_prefix() {
    assert!(Recipient::new("xage1abcde").is_err());
    assert!(Recipient::new("AGE1...").is_err());
}

#[test]
fn recipient_length_boundary() {
    assert!(Recipient::new("age1a").is_ok());
    let long = "age1".to_owned() + &"a".repeat(508);
    assert!(Recipient::new(&long).is_ok());
    let too_long = "age1".to_owned() + &"a".repeat(509);
    assert!(Recipient::new(&too_long).is_err());
}

#[test]
fn recipient_allows_bech32_chars() {
    let key = "age1qpzry9x8gf2tvdw0s3jn54khce6mua7l";
    assert!(Recipient::new(key).is_ok());
}

#[test]
fn recipient_rejects_uppercase_after_prefix() {
    assert!(Recipient::new("age1ABCDEFGH").is_err());
    assert!(Recipient::new("age1abcDef").is_err());
}

#[test]
fn branch_name_valid() {
    let bn = BranchName::new("main").unwrap();
    assert_eq!(bn.as_str(), "main");
}

#[test]
fn branch_name_with_slash_and_dash() {
    let bn = BranchName::new("feature/branch-v2").unwrap();
    assert_eq!(bn.as_str(), "feature/branch-v2");
}

#[test]
fn branch_name_empty() {
    assert!(BranchName::new("").is_err());
}

#[test]
fn branch_name_too_long() {
    let long = "b".repeat(256);
    assert!(BranchName::new(&long).is_err());
}

#[test]
fn branch_name_invalid_chars() {
    assert!(BranchName::new("branch name").is_err());
    assert!(BranchName::new("branch.name").is_err());
}

#[test]
fn commit_hash_from_bytes_and_as_bytes() {
    let bytes = [42u8; 64];
    let hash = CommitHash::from_bytes(bytes);
    assert_eq!(hash.as_bytes(), &bytes);
}

#[test]
fn commit_hash_hex_roundtrip() {
    let original = CommitHash::from_bytes([255u8; 64]);
    let hex = original.to_hex();
    let parsed = CommitHash::from_hex(&hex).unwrap();
    assert_eq!(original, parsed);
}

#[test]
fn commit_hash_invalid_hex() {
    assert!(CommitHash::from_hex("zzz").is_err());
}

#[test]
fn commit_hash_invalid_length_hex() {
    assert!(CommitHash::from_hex("aabbcc").is_err());
}

#[test]
fn commit_hash_is_zero() {
    let zero = CommitHash::from_bytes([0u8; 64]);
    assert!(zero.is_zero());
    let non_zero = CommitHash::from_bytes([1u8; 64]);
    assert!(!non_zero.is_zero());
}

#[test]
fn commit_hash_serialization() {
    let hash = CommitHash::from_bytes([7u8; 64]);
    let json = serde_json::to_string(&hash).unwrap();
    let restored: CommitHash = serde_json::from_str(&json).unwrap();
    assert_eq!(hash, restored);
}

#[test]
fn signature_valid() {
    let bytes = vec![0u8; 64];
    let sig = Signature::new(bytes).unwrap();
    assert_eq!(sig.as_bytes().len(), 64);
}

#[test]
fn signature_invalid_length() {
    assert!(Signature::new(vec![0u8; 63]).is_err());
    assert!(Signature::new(vec![0u8; 65]).is_err());
}

#[test]
fn signature_serialization_roundtrip() {
    let sig = Signature::new(vec![1u8; 64]).unwrap();
    let json = serde_json::to_string(&sig).unwrap();
    let restored: Signature = serde_json::from_str(&json).unwrap();
    assert_eq!(sig, restored);
}

#[test]
fn encrypted_payload_valid() {
    let ep = EncryptedPayload::new(b"cipher".to_vec()).unwrap();
    assert!(!ep.as_bytes().is_empty());
}

#[test]
fn encrypted_payload_empty() {
    assert!(EncryptedPayload::new(vec![]).is_err());
}

#[test]
fn encrypted_payload_serialization_roundtrip() {
    let ep = EncryptedPayload::new(b"data".to_vec()).unwrap();
    let json = serde_json::to_string(&ep).unwrap();
    let restored: EncryptedPayload = serde_json::from_str(&json).unwrap();
    assert_eq!(ep, restored);
}

fn make_valid_identity() -> Identity {
    Identity::new(Fingerprint::new("a".repeat(64)).unwrap(), Role::Cashier)
}

fn make_valid_payload() -> EncryptedPayload {
    EncryptedPayload::new(b"payload".to_vec()).unwrap()
}

fn make_valid_signature() -> Signature {
    Signature::new(vec![0u8; 64]).unwrap()
}

#[test]
fn event_new_valid() {
    let id = EventId::generate();
    let author = make_valid_identity();
    let payload = make_valid_payload();
    let sig = make_valid_signature();
    let ev = Event::new(id, 1000, author, payload, sig).unwrap();
    assert_eq!(ev.timestamp, 1000);
}

#[test]
fn event_timestamp_zero() {
    assert!(
        Event::new(
            EventId::generate(),
            0,
            make_valid_identity(),
            make_valid_payload(),
            make_valid_signature(),
        )
        .is_err()
    );
}

#[test]
fn event_timestamp_negative() {
    assert!(
        Event::new(
            EventId::generate(),
            -1,
            make_valid_identity(),
            make_valid_payload(),
            make_valid_signature(),
        )
        .is_err()
    );
}

#[test]
fn event_validate_method_direct() {
    let ev = Event::new(
        EventId::generate(),
        1,
        make_valid_identity(),
        make_valid_payload(),
        make_valid_signature(),
    )
    .unwrap();
    assert!(ev.validate().is_ok());
}

#[test]
fn journal_entry_valid() {
    let id = EventId::generate();
    let author = make_valid_identity();
    let sig = make_valid_signature();
    let entry =
        JournalEntry::new(id, 2000, "user.login".into(), author, "details".into(), sig).unwrap();
    assert_eq!(entry.action, "user.login");
}

#[test]
fn journal_entry_action_empty() {
    assert!(
        JournalEntry::new(
            EventId::generate(),
            1,
            String::new(),
            make_valid_identity(),
            String::new(),
            make_valid_signature(),
        )
        .is_err()
    );
}

#[test]
fn journal_entry_action_too_long() {
    let long_action = "a".repeat(257);
    assert!(
        JournalEntry::new(
            EventId::generate(),
            1,
            long_action,
            make_valid_identity(),
            String::new(),
            make_valid_signature(),
        )
        .is_err()
    );
}

#[test]
fn journal_entry_timestamp_zero() {
    assert!(
        JournalEntry::new(
            EventId::generate(),
            0,
            "test".into(),
            make_valid_identity(),
            String::new(),
            make_valid_signature(),
        )
        .is_err()
    );
}

#[test]
fn journal_entry_validate_direct() {
    let entry = JournalEntry::new(
        EventId::generate(),
        1,
        "test".into(),
        make_valid_identity(),
        String::new(),
        make_valid_signature(),
    )
    .unwrap();
    assert!(entry.validate().is_ok());
}

#[test]
fn snapshot_new_valid() {
    let payload = EncryptedPayload::new(b"data".to_vec()).unwrap();
    let hash = CommitHash::from_bytes([1u8; 64]);
    let snap = Snapshot::new(5, payload, hash).unwrap();
    assert_eq!(snap.version, 5);
}

#[test]
fn snapshot_version_zero() {
    let payload = make_valid_payload();
    let hash = CommitHash::from_bytes([1u8; 64]);
    assert!(Snapshot::new(0, payload, hash).is_err());
}

#[test]
fn snapshot_hash_zero() {
    let payload = make_valid_payload();
    let hash = CommitHash::from_bytes([0u8; 64]);
    assert!(Snapshot::new(1, payload, hash).is_err());
}

#[test]
fn snapshot_validate_direct() {
    let snap = Snapshot::new(1, make_valid_payload(), CommitHash::from_bytes([2u8; 64])).unwrap();
    assert!(snap.validate().is_ok());
}

#[test]
fn role_as_str() {
    assert_eq!(Role::Admin.as_str(), "admin");
    assert_eq!(Role::Cashier.as_str(), "cashier");
    assert_eq!(Role::Custom("super".into()).as_str(), "super");
}

#[test]
fn identity_new() {
    let fp = Fingerprint::new("a".repeat(64)).unwrap();
    let id = Identity::new(fp, Role::Manager);
    assert_eq!(id.role, Role::Manager);
}

#[test]
fn error_from_vctrl_error() {
    let vctrl_err = libvctrl::VctrlError::Other("object".into());
    let pv_err: PosVaultError = vctrl_err.into();
    match pv_err {
        PosVaultError::Vctrl(_) => (),
        _ => panic!("Wrong variant"),
    }
}

#[test]
fn error_from_io_error() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
    let pv_err: PosVaultError = io_err.into();
    match pv_err {
        PosVaultError::Io(_) => (),
        _ => panic!("Expected Io variant"),
    }
}

#[test]
fn error_display_invalid_input() {
    let err = PosVaultError::InvalidInput("bad input".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("Invalid input"));
    assert!(msg.contains("bad input"));
}

#[test]
fn error_display_other() {
    let err = PosVaultError::Encryption("test error".into());
    let msg = format!("{}", err);
    assert!(msg.contains("Encryption error"));
    assert!(msg.contains("test error"));
}

#[test]
fn ensure_macro_pass() {
    fn check(x: usize) -> Result<()> {
        ensure!(x > 0, "x harus > 0");
        Ok(())
    }
    assert!(check(1).is_ok());
}

#[test]
fn ensure_macro_fail() {
    fn check(x: usize) -> Result<()> {
        ensure!(x > 10, "x terlalu kecil");
        Ok(())
    }
    let err = check(5).unwrap_err();
    match err {
        PosVaultError::InvalidInput(msg) => assert!(msg.contains("x terlalu kecil")),
        _ => panic!("Invalid input expected"),
    }
}

#[test]
fn bail_macro() {
    fn failing_func() -> Result<()> {
        bail!("langsung gagal");
    }
    let err = failing_func().unwrap_err();
    match err {
        PosVaultError::InvalidInput(msg) => assert_eq!(msg, "langsung gagal"),
        _ => panic!("Expected InvalidInput"),
    }
}

#[test]
fn traits_are_object_safe() {
    fn _event_store(_: &dyn EventStore) {}
    fn _snapshot_store(_: &dyn SnapshotStore) {}
    fn _journal(_: &dyn Journal) {}
    fn _signer(_: &dyn Signer) {}
    fn _conflict_resolver(_: &dyn ConflictResolver) {}
    fn _transport(_: &dyn Transport) {}
    fn _codec(_: &dyn EventCodec) {}
}

#[test]
fn event_id_is_clone_eq_hash() {
    let id1 = EventId::new("test-1").unwrap();
    let id2 = id1.clone();
    assert_eq!(id1, id2);
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(id1);
    assert!(set.contains(&id2));
}

#[test]
fn commit_hash_is_copy() {
    let hash = CommitHash::from_bytes([1u8; 64]);
    let copy = hash;
    assert_eq!(hash, copy);
}

#[test]
fn sync_mode_periodic() {
    let mode = SyncMode::Periodic(30);
    assert_eq!(mode, SyncMode::Periodic(30));
    assert_ne!(mode, SyncMode::Realtime);
}
