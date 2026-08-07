use posvault_handler::errors::{PosVaultError, Result};
use posvault_handler::types::{EncryptedPayload, Event};

pub fn encrypt_event(event: &mut Event, recipients: &[impl AsRef<str>]) -> Result<()> {
    if recipients.is_empty() {
        return Err(PosVaultError::Encryption(
            "recipients must not be empty".into(),
        ));
    }

    let plaintext = event.payload.as_bytes().to_vec();

    let cipherbytes = if recipients.len() > 1 {
        let keys: Vec<&str> = recipients.iter().map(|s| s.as_ref()).collect();
        let response = librage::encrypt_multiple(&plaintext, &keys);
        if !response.success {
            return Err(map_librage_error(&response.error));
        }
        let data = response
            .data
            .ok_or_else(|| PosVaultError::Encryption("response missing ciphertext data".into()))?;
        data.ciphertext.to_vec()
    } else {
        let single_key = recipients.first().expect("recipients not empty");
        let response = librage::encrypt(&plaintext, single_key.as_ref());
        if !response.success {
            return Err(map_librage_error(&response.error));
        }
        let data = response
            .data
            .ok_or_else(|| PosVaultError::Encryption("response missing ciphertext data".into()))?;
        data.ciphertext.to_vec()
    };

    let encrypted = EncryptedPayload::new(cipherbytes)?;
    event.payload = encrypted;

    Ok(())
}

pub fn decrypt_event(event: &mut Event, identity: &str) -> Result<()> {
    let cipherbytes = event.payload.as_bytes().to_vec();
    let response = librage::decrypt(&cipherbytes, identity);
    if !response.success {
        return Err(map_librage_error(&response.error));
    }
    let data = response
        .data
        .ok_or_else(|| PosVaultError::Encryption("response missing plaintext data".into()))?;
    let plaintext = data.plaintext.to_vec();
    let decrypted = EncryptedPayload::new(plaintext)?;
    event.payload = decrypted;
    Ok(())
}

fn map_librage_error(body: &Option<librage::ErrorBody>) -> PosVaultError {
    body.as_ref()
        .map(|b| PosVaultError::Encryption(format!("{}: {}", b.code, b.message)))
        .unwrap_or_else(|| PosVaultError::Encryption("unknown librage error".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use posvault_handler::types::{EventId, Fingerprint, Identity, Role};

    fn create_test_event() -> Event {
        let id = EventId::generate();
        let author = Identity::new(Fingerprint::new("a".repeat(64)).unwrap(), Role::Cashier);
        let payload = EncryptedPayload::new(b"test plaintext".to_vec()).unwrap();
        let sig = posvault_handler::types::Signature::new(vec![0u8; 64]).unwrap();
        Event::new(id, 1, author, payload, sig).unwrap()
    }

    fn generate_keys() -> (String, String) {
        let kp = librage::generate_keypair();
        assert!(kp.success);
        let data = kp.data.unwrap();
        (data.public_key, data.secret_key.to_string())
    }

    #[test]
    fn encrypt_decrypt_single_recipient() {
        let mut event = create_test_event();
        let original_payload = event.payload.as_bytes().to_vec();
        let (recipient, identity) = generate_keys();

        encrypt_event(&mut event, &[&recipient]).unwrap();
        assert_ne!(event.payload.as_bytes(), original_payload.as_slice());
        assert_eq!(event.signature.as_bytes(), &[0u8; 64]);

        decrypt_event(&mut event, &identity).unwrap();
        assert_eq!(event.payload.as_bytes(), original_payload.as_slice());
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
    fn encrypt_with_empty_recipients_fails() {
        let mut event = create_test_event();
        let recipients: &[&str] = &[];
        let err = encrypt_event(&mut event, recipients).unwrap_err();
        match err {
            PosVaultError::Encryption(msg) => assert!(msg.contains("must not be empty")),
            _ => panic!("wrong error variant"),
        }
    }

    #[test]
    fn encrypt_with_invalid_key_fails() {
        let mut event = create_test_event();
        let res = encrypt_event(&mut event, &["not-a-valid-age-key"]);
        assert!(res.is_err());
    }

    #[test]
    fn decrypt_with_wrong_key_fails() {
        let mut event = create_test_event();
        let (rec, _) = generate_keys();
        let (_, wrong_ident) = generate_keys();

        encrypt_event(&mut event, &[&rec]).unwrap();
        assert!(decrypt_event(&mut event, &wrong_ident).is_err());
    }

    #[test]
    fn decrypt_with_passphrase_fails_gracefully() {
        let mut event = create_test_event();
        let res = decrypt_event(&mut event, "not-a-valid-x25519-identity");
        assert!(res.is_err());
    }

    #[test]
    fn signature_unchanged_after_encryption() {
        let mut event = create_test_event();
        let real_sig = posvault_handler::types::Signature::new(vec![1u8; 64]).unwrap();
        event.signature = real_sig.clone();

        let (rec, _) = generate_keys();
        encrypt_event(&mut event, &[&rec]).unwrap();

        assert_eq!(event.signature, real_sig);
    }
}
