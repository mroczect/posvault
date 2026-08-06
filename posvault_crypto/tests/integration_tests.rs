use posvault_crypto::{decrypt_event, encrypt_event};
use posvault_handler::types::EncryptedPayload;
use posvault_handler::types::{Event, EventId, Fingerprint, Identity, Role, Signature};

fn create_test_event() -> Event {
    let id = EventId::generate();
    let author = Identity::new(Fingerprint::new("a".repeat(64)).unwrap(), Role::Cashier);
    let payload = EncryptedPayload::new(b"test plaintext".to_vec()).unwrap();
    let sig = Signature::new(vec![0u8; 64]).unwrap();
    Event::new(id, 1, author, payload, sig).unwrap()
}

#[test]
fn encrypt_decrypt_single_recipient() {
    let mut event = create_test_event();
    let original = event.payload.as_bytes().to_vec();

    let kp = librage::generate_keypair();
    let data = kp.data.as_ref().unwrap();
    let recipient = data.public_key.clone();
    let identity = data.secret_key.as_str().to_owned();

    let recipients = vec![recipient];
    encrypt_event(&mut event, &recipients).unwrap();
    assert_ne!(event.payload.as_bytes(), original.as_slice());

    decrypt_event(&mut event, &identity).unwrap();
    assert_eq!(event.payload.as_bytes(), original.as_slice());
}

#[test]
fn encrypt_multiple_recipients() {
    let mut event = create_test_event();
    let original = event.payload.as_bytes().to_vec();

    let kp1 = librage::generate_keypair();
    let kp2 = librage::generate_keypair();
    let data1 = kp1.data.as_ref().unwrap();
    let data2 = kp2.data.as_ref().unwrap();

    let recipients = vec![data1.public_key.clone(), data2.public_key.clone()];
    encrypt_event(&mut event, &recipients).unwrap();
    assert_ne!(event.payload.as_bytes(), original.as_slice());

    decrypt_event(&mut event, data1.secret_key.as_str()).unwrap();
    assert_eq!(event.payload.as_bytes(), original.as_slice());

    let mut event2 = create_test_event();
    encrypt_event(&mut event2, &recipients).unwrap();
    decrypt_event(&mut event2, data2.secret_key.as_str()).unwrap();
    assert_eq!(event2.payload.as_bytes(), original.as_slice());
}

#[test]
fn decrypt_wrong_key_fails() {
    let mut event = create_test_event();
    let kp_enc = librage::generate_keypair();
    let kp_wrong = librage::generate_keypair();
    let data_enc = kp_enc.data.as_ref().unwrap();
    let data_wrong = kp_wrong.data.as_ref().unwrap();

    encrypt_event(&mut event, std::slice::from_ref(&data_enc.public_key)).unwrap();

    let result = decrypt_event(&mut event, data_wrong.secret_key.as_str());
    assert!(result.is_err());
}

#[test]
fn encrypt_empty_recipients_fails() {
    let mut event = create_test_event();
    let result = encrypt_event(&mut event, &[]);
    assert!(result.is_err());
}

#[test]
fn decrypt_with_garbage_identity_fails() {
    let mut event = create_test_event();
    let kp = librage::generate_keypair();
    let data = kp.data.as_ref().unwrap();
    encrypt_event(&mut event, std::slice::from_ref(&data.public_key)).unwrap();

    let result = decrypt_event(&mut event, "AGE-SECRET-KEY-INVALID...");
    assert!(result.is_err());
}

#[test]
fn roundtrip_empty_payload() {
    let mut event = create_test_event();
    event.payload = EncryptedPayload::new(b"x".to_vec()).unwrap();
    let original = event.payload.as_bytes().to_vec();

    let kp = librage::generate_keypair();
    let data = kp.data.as_ref().unwrap();
    encrypt_event(&mut event, std::slice::from_ref(&data.public_key)).unwrap();
    assert_ne!(event.payload.as_bytes(), original.as_slice());

    decrypt_event(&mut event, data.secret_key.as_str()).unwrap();
    assert_eq!(event.payload.as_bytes(), original.as_slice());
}
