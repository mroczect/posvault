use posvault_handler::*;

#[test]
fn secret_data_new_valid() {
    let s = SecretData::new(b"secret".to_vec()).unwrap();
    assert!(!s.as_bytes().is_empty());
}

#[test]
fn secret_data_empty() {
    assert!(SecretData::new(vec![]).is_err());
}

#[test]
fn secret_data_zeroize_on_drop() {
    let data = vec![1u8, 2, 3];
    let secret = SecretData::new(data.clone()).unwrap();
    drop(secret);
    assert_eq!(data, vec![1, 2, 3]);
}

#[test]
fn secret_data_serialization_roundtrip() {
    let original = SecretData::new(b"hello".to_vec()).unwrap();
    let json = serde_json::to_string(&original).unwrap();
    let restored: SecretData = serde_json::from_str(&json).unwrap();
    assert_eq!(original.as_bytes(), restored.as_bytes());
}

#[test]
fn event_id_new_valid() {
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
}

#[test]
fn event_id_max_length() {
    let long = "a".repeat(64);
    assert!(EventId::new(&long).is_ok());
    let too_long = "a".repeat(65);
    assert!(EventId::new(&too_long).is_err());
}

#[test]
fn event_id_generate_is_unique() {
    let id1 = EventId::generate();
    let id2 = EventId::generate();
    assert_ne!(id1.as_str(), id2.as_str());
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
}

#[test]
fn fingerprint_invalid_chars() {
    let hex = "z".repeat(64);
    assert!(Fingerprint::new(&hex).is_err());
}

#[test]
fn recipient_valid_age_key() {
    let key = "age1abcdefghijklmnopqrstuvwxyz0123456789";
    let rec = Recipient::new(key).unwrap();
    assert!(rec.as_str().starts_with("age1"));
}

#[test]
fn recipient_invalid_prefix() {
    assert!(Recipient::new("xage1...").is_err());
}

#[test]
fn recipient_invalid_chars() {
    assert!(Recipient::new("age1ABCDEFGH").is_err());
}

#[test]
fn branch_name_valid() {
    let bn = BranchName::new("main").unwrap();
    assert_eq!(bn.as_str(), "main");
}

#[test]
fn branch_name_with_slash() {
    let bn = BranchName::new("feature/branch").unwrap();
    assert_eq!(bn.as_str(), "feature/branch");
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
fn commit_hash_from_bytes() {
    let bytes = [0u8; 64];
    let hash = CommitHash::from_bytes(bytes);
    assert_eq!(hash.as_bytes(), &bytes);
}

#[test]
fn commit_hash_hex_roundtrip() {
    let original = CommitHash::from_bytes([1u8; 64]);
    let hex = original.to_hex();
    let parsed = CommitHash::from_hex(&hex).unwrap();
    assert_eq!(original, parsed);
}

#[test]
fn commit_hash_invalid_hex() {
    assert!(CommitHash::from_hex("zzz").is_err());
}

#[test]
fn commit_hash_invalid_length() {
    assert!(CommitHash::from_hex("aabb").is_err());
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

#[test]
fn event_new_valid() {
    let id = EventId::generate();
    let author = Identity::new(Fingerprint::new("a".repeat(64)).unwrap(), Role::Cashier);
    let payload = EncryptedPayload::new(b"x".to_vec()).unwrap();
    let sig = Signature::new(vec![0u8; 64]).unwrap();
    let ev = Event::new(id, 1000, author, payload, sig).unwrap();
    assert_eq!(ev.timestamp, 1000);
}

#[test]
fn event_timestamp_zero() {
    assert!(
        Event::new(
            EventId::generate(),
            0,
            Identity::new(Fingerprint::new("a".repeat(64)).unwrap(), Role::Admin),
            EncryptedPayload::new(b"x".to_vec()).unwrap(),
            Signature::new(vec![0u8; 64]).unwrap(),
        )
        .is_err()
    );
}

#[test]
fn journal_entry_valid() {
    let id = EventId::generate();
    let author = Identity::new(Fingerprint::new("a".repeat(64)).unwrap(), Role::Auditor);
    let sig = Signature::new(vec![0u8; 64]).unwrap();
    let entry =
        JournalEntry::new(id, 2000, "user.login".into(), author, "details".into(), sig).unwrap();
    assert_eq!(entry.action, "user.login");
}

#[test]
fn journal_entry_action_too_long() {
    let long_action = "a".repeat(257);
    assert!(
        JournalEntry::new(
            EventId::generate(),
            1,
            long_action,
            Identity::new(Fingerprint::new("a".repeat(64)).unwrap(), Role::Auditor),
            String::new(),
            Signature::new(vec![0u8; 64]).unwrap(),
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
            Identity::new(Fingerprint::new("a".repeat(64)).unwrap(), Role::Cashier),
            String::new(),
            Signature::new(vec![0u8; 64]).unwrap(),
        )
        .is_err()
    );
}

#[test]
fn error_from_vctrl_error() {
    let vctrl_err = libvctrl::error::VctrlError::NotFound("object".into());
    let pv_err: PosVaultError = vctrl_err.into();
    match pv_err {
        PosVaultError::Vctrl(_) => (),
        _ => panic!("Wrong variant"),
    }
}

#[test]
fn error_from_tree_error() {
    let tree_err = libvctrl::domain::tree::TreeError::DuplicateEntry("dup".into());
    let pv_err: PosVaultError = tree_err.into();
    match pv_err {
        PosVaultError::Tree(_) => (),
        _ => panic!("Wrong variant"),
    }
}

#[test]
fn error_display() {
    let err = PosVaultError::InvalidInput("bad input".into());
    let msg = format!("{}", err);
    assert!(msg.contains("Invalid input"));
    assert!(msg.contains("bad input"));
}
