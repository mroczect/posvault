use posvault_crypto::{decrypt_event, encrypt_event};
use posvault_handler::errors::PosVaultError;
use posvault_handler::types::{
    EncryptedPayload, Event, EventId, Fingerprint, Identity, Role, Signature,
};

fn create_test_event() -> Event {
    let id = EventId::generate();
    let author = Identity::new(Fingerprint::new("a".repeat(64)).unwrap(), Role::Cashier);
    let payload = EncryptedPayload::new(b"test plaintext".to_vec()).unwrap();
    let sig = Signature::new(vec![0u8; 64]).unwrap();
    Event::new(id, 1, author, payload, sig).unwrap()
}

fn generate_keys() -> (String, String) {
    let kp = librage::generate_keypair();
    assert!(kp.success, "Generate keypair failed");
    let data = kp.data.expect("KeyGenData harus ada");
    (data.public_key, data.secret_key.to_string())
}

#[test]
fn encrypt_decrypt_single_recipient() {
    let mut event = create_test_event();
    let original = event.payload.as_bytes().to_vec();
    let (recipient, identity) = generate_keys();

    encrypt_event(&mut event, &[&recipient]).unwrap();
    assert_ne!(event.payload.as_bytes(), original.as_slice());
    assert_eq!(event.signature.as_bytes(), &[0u8; 64]);

    decrypt_event(&mut event, &identity).unwrap();
    assert_eq!(event.payload.as_bytes(), original.as_slice());
}

#[test]
fn encrypt_multiple_recipients() {
    let mut event = create_test_event();
    let original = event.payload.as_bytes().to_vec();
    let (rec1, ident1) = generate_keys();
    let (rec2, ident2) = generate_keys();

    encrypt_event(&mut event, &[&rec1, &rec2]).unwrap();
    assert_ne!(event.payload.as_bytes(), original.as_slice());
    assert_eq!(event.signature.as_bytes(), &[0u8; 64]);

    let mut event1 = event.clone();
    decrypt_event(&mut event1, &ident1).unwrap();
    assert_eq!(event1.payload.as_bytes(), original.as_slice());

    let mut event2 = event.clone();
    decrypt_event(&mut event2, &ident2).unwrap();
    assert_eq!(event2.payload.as_bytes(), original.as_slice());
}

#[test]
fn decrypt_wrong_key_fails() {
    let mut event = create_test_event();
    let (rec, _) = generate_keys();
    let (_, wrong_ident) = generate_keys();

    encrypt_event(&mut event, &[&rec]).unwrap();

    let result = decrypt_event(&mut event, &wrong_ident);
    assert!(result.is_err());
    match result {
        Err(PosVaultError::Encryption(_)) => {}
        _ => panic!("Expected Encryption error"),
    }
}

#[test]
fn encrypt_empty_recipients_fails() {
    let mut event = create_test_event();
    let empty: &[&str] = &[];
    let result = encrypt_event(&mut event, empty);
    assert!(result.is_err());
    match result {
        Err(PosVaultError::Encryption(msg)) => assert!(msg.contains("must not be empty")),
        _ => panic!("Expected Encryption error with 'must not be empty'"),
    }
}

#[test]
fn encrypt_with_invalid_key_fails() {
    let mut event = create_test_event();
    let result = encrypt_event(&mut event, &["invalid-key"]);
    assert!(result.is_err());
}

#[test]
fn decrypt_with_garbage_identity_fails() {
    let mut event = create_test_event();
    let (rec, _) = generate_keys();
    encrypt_event(&mut event, &[&rec]).unwrap();

    let result = decrypt_event(&mut event, "AGE-SECRET-KEY-INVALID...");
    assert!(result.is_err());
}

#[test]
fn decrypt_with_passphrase_identity_fails() {
    let mut event = create_test_event();
    let (rec, _) = generate_keys();
    encrypt_event(&mut event, &[&rec]).unwrap();

    let result = decrypt_event(&mut event, "my-passphrase");
    assert!(result.is_err());
}

#[test]
fn decrypt_untouched_event_fails() {
    let mut event = create_test_event();
    let (_, ident) = generate_keys();
    let result = decrypt_event(&mut event, &ident);
    assert!(result.is_err());
}

#[test]
fn signature_preserved_after_encryption() {
    let mut event = create_test_event();
    let real_sig = Signature::new(vec![42u8; 64]).unwrap();
    event.signature = real_sig.clone();

    let (rec, _) = generate_keys();
    encrypt_event(&mut event, &[&rec]).unwrap();

    assert_eq!(event.signature, real_sig);
}

#[test]
fn signature_unchanged_after_decryption() {
    let mut event = create_test_event();
    let real_sig = Signature::new(vec![7u8; 64]).unwrap();
    event.signature = real_sig.clone();
    let (rec, ident) = generate_keys();

    encrypt_event(&mut event, &[&rec]).unwrap();
    decrypt_event(&mut event, &ident).unwrap();

    assert_eq!(event.signature, real_sig);
}

#[test]
fn roundtrip_with_small_payload() {
    let mut event = create_test_event();
    event.payload = EncryptedPayload::new(b"x".to_vec()).unwrap();
    let original = event.payload.as_bytes().to_vec();
    let (rec, ident) = generate_keys();

    encrypt_event(&mut event, &[&rec]).unwrap();
    assert_ne!(event.payload.as_bytes(), original.as_slice());

    decrypt_event(&mut event, &ident).unwrap();
    assert_eq!(event.payload.as_bytes(), original.as_slice());
}

#[test]
fn roundtrip_with_large_payload() {
    let large_data = vec![0xAB; 1_000_000];
    let mut event = create_test_event();
    event.payload = EncryptedPayload::new(large_data.clone()).unwrap();
    let original = event.payload.as_bytes().to_vec();
    let (rec, ident) = generate_keys();

    encrypt_event(&mut event, &[&rec]).unwrap();
    assert_ne!(event.payload.as_bytes(), original.as_slice());

    decrypt_event(&mut event, &ident).unwrap();
    assert_eq!(event.payload.as_bytes(), original.as_slice());
}

#[test]
fn roundtrip_empty_payload_fails() {
    assert!(EncryptedPayload::new(vec![]).is_err());
}
