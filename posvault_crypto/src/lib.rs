use librage::{decrypt, encrypt, encrypt_multiple};
use posvault_handler::errors::{PosVaultError, Result};
use posvault_handler::types::{EncryptedPayload, Event};

pub fn encrypt_event(event: &mut Event, recipients: &[String]) -> Result<()> {
    let plaintext = event.payload.as_bytes();
    let cipherbytes = if recipients.len() > 1 {
        let keys: Vec<&str> = recipients.iter().map(|s| s.as_str()).collect();
        let response = encrypt_multiple(plaintext, &keys);
        if !response.success {
            return Err(map_librage_error(&response.error));
        }
        let data = response
            .data
            .ok_or_else(|| PosVaultError::Encryption("no data in response".into()))?;
        data.ciphertext.to_vec()
    } else {
        let single_key = recipients
            .first()
            .ok_or_else(|| PosVaultError::Encryption("no recipients provided".into()))?;
        let response = encrypt(plaintext, single_key.as_str());
        if !response.success {
            return Err(map_librage_error(&response.error));
        }
        let data = response
            .data
            .ok_or_else(|| PosVaultError::Encryption("no data in response".into()))?;
        data.ciphertext.to_vec()
    };

    let encrypted = EncryptedPayload::new(cipherbytes)?;
    event.payload = encrypted;
    Ok(())
}

pub fn decrypt_event(event: &mut Event, identity: &str) -> Result<()> {
    let cipherbytes = event.payload.as_bytes();
    let response = decrypt(cipherbytes, identity);
    if !response.success {
        return Err(map_librage_error(&response.error));
    }
    let data = response
        .data
        .ok_or_else(|| PosVaultError::Encryption("no data in response".into()))?;
    let plaintext = data.plaintext.to_vec();
    let decrypted = EncryptedPayload::new(plaintext)?;
    event.payload = decrypted;
    Ok(())
}

fn map_librage_error(body: &Option<librage::ErrorBody>) -> PosVaultError {
    match body {
        Some(b) => PosVaultError::Encryption(format!("{}: {}", b.code, b.message)),
        None => PosVaultError::Encryption("unknown librage error".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use posvault_handler::types::{EventId, Fingerprint, Identity, Role, Signature};

    fn create_test_event() -> Event {
        let id = EventId::generate();
        let author = Identity::new(Fingerprint::new("a".repeat(64)).unwrap(), Role::Cashier);
        let payload = EncryptedPayload::new(b"test plaintext".to_vec()).unwrap();
        let sig = Signature::new(vec![0u8; 64]).unwrap();
        Event::new(id, 1, author, payload, sig).unwrap()
    }

    #[test]
    fn test_encrypt_decrypt_single_recipient() {
        let mut event = create_test_event();
        let plaintext_original = event.payload.as_bytes().to_vec();

        let kp = librage::generate_keypair();
        assert!(kp.success);
        let data = kp.data.as_ref().unwrap();
        let recipient = data.public_key.clone();
        let identity = data.secret_key.as_str().to_owned();

        let recipients = vec![recipient];
        encrypt_event(&mut event, &recipients).unwrap();
        assert_ne!(event.payload.as_bytes(), plaintext_original.as_slice());

        decrypt_event(&mut event, &identity).unwrap();
        assert_eq!(event.payload.as_bytes(), plaintext_original.as_slice());
    }

    #[test]
    fn test_encrypt_multiple_recipients() {
        let mut event = create_test_event();
        let plaintext_original = event.payload.as_bytes().to_vec();

        let kp1 = librage::generate_keypair();
        let kp2 = librage::generate_keypair();
        let data1 = kp1.data.as_ref().unwrap();
        let data2 = kp2.data.as_ref().unwrap();

        let recipients = vec![data1.public_key.clone(), data2.public_key.clone()];
        encrypt_event(&mut event, &recipients).unwrap();
        assert_ne!(event.payload.as_bytes(), plaintext_original.as_slice());

        decrypt_event(&mut event, data1.secret_key.as_str()).unwrap();
        assert_eq!(event.payload.as_bytes(), plaintext_original.as_slice());

        let mut event2 = create_test_event();
        encrypt_event(&mut event2, &recipients).unwrap();
        decrypt_event(&mut event2, data2.secret_key.as_str()).unwrap();
        assert_eq!(event2.payload.as_bytes(), plaintext_original.as_slice());
    }

    #[test]
    fn test_decrypt_with_wrong_key_fails() {
        let mut event = create_test_event();
        let kp_enc = librage::generate_keypair();
        let kp_wrong = librage::generate_keypair();
        let data_enc = kp_enc.data.as_ref().unwrap();
        let data_wrong = kp_wrong.data.as_ref().unwrap();

        encrypt_event(&mut event, std::slice::from_ref(&data_enc.public_key)).unwrap();

        let result = decrypt_event(&mut event, data_wrong.secret_key.as_str());
        assert!(result.is_err());
    }
}
