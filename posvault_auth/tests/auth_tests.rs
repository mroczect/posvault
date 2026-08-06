use age_auth::OtpGenerator;
use age_credentials::backend::traits::AccountBackend;
use age_credentials::crypto;
use age_credentials::domain::error::Result as AccountResult;
use age_credentials::domain::fingerprint::Fingerprint;
use age_credentials::domain::identity::Identity;
use age_credentials::domain::types::{KeyGenData, UserID};
use posvault_auth::{Session, login, require_role};
use posvault_handler::types::Role;
use std::collections::HashMap;
use zeroize::Zeroizing;

struct MockBackend {
    identities: HashMap<Fingerprint, Identity>,
    encrypted_keys: HashMap<Fingerprint, Vec<u8>>,
}

impl MockBackend {
    fn new() -> Self {
        MockBackend {
            identities: HashMap::new(),
            encrypted_keys: HashMap::new(),
        }
    }

    fn add_account(
        &mut self,
        email: &str,
        passphrase: &str,
        _totp_secret_base32: &str,
    ) -> Identity {
        let key_data = generate_keypair();
        let fingerprint = {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(key_data.public_key.as_bytes());
            let digest = hasher.finalize();
            hex::encode(digest)
        };
        let fp = Fingerprint::new(&fingerprint).unwrap();

        let encrypted_private =
            crypto::encrypt_with_passphrase(key_data.secret_key.as_bytes(), passphrase).unwrap();

        let user_id = UserID::new("Test User", email).unwrap();
        let identity = Identity::new(fp.clone(), user_id, key_data.public_key.clone(), None);

        self.identities.insert(fp.clone(), identity.clone());
        self.encrypted_keys.insert(fp.clone(), encrypted_private);

        identity
    }
}

impl AccountBackend for MockBackend {
    fn save_identity(&mut self, identity: &Identity) -> AccountResult<()> {
        self.identities
            .insert(identity.fingerprint.clone(), identity.clone());
        Ok(())
    }

    fn load_identity(&self, fingerprint: &Fingerprint) -> AccountResult<Option<Identity>> {
        Ok(self.identities.get(fingerprint).cloned())
    }

    fn delete_identity(&mut self, fingerprint: &Fingerprint) -> AccountResult<()> {
        self.identities.remove(fingerprint);
        self.encrypted_keys.remove(fingerprint);
        Ok(())
    }

    fn store_encrypted_private_key(
        &mut self,
        fingerprint: &Fingerprint,
        encrypted_key: &[u8],
    ) -> AccountResult<()> {
        self.encrypted_keys
            .insert(fingerprint.clone(), encrypted_key.to_vec());
        Ok(())
    }

    fn load_encrypted_private_key(
        &self,
        fingerprint: &Fingerprint,
    ) -> AccountResult<Option<Zeroizing<Vec<u8>>>> {
        Ok(self
            .encrypted_keys
            .get(fingerprint)
            .map(|v| Zeroizing::new(v.clone())))
    }

    fn list_fingerprints(&self) -> AccountResult<Vec<Fingerprint>> {
        Ok(self.identities.keys().cloned().collect())
    }

    fn find_by_email(&self, email: &str) -> AccountResult<Option<Fingerprint>> {
        for (fp, identity) in &self.identities {
            if identity.user_id.email == email {
                return Ok(Some(fp.clone()));
            }
        }
        Ok(None)
    }
}

fn generate_keypair() -> KeyGenData {
    let kp = librage::generate_keypair();
    let data = kp.data.as_ref().unwrap();
    KeyGenData {
        public_key: data.public_key.clone(),
        secret_key: data.secret_key.clone(),
    }
}

#[test]
fn login_success() {
    let mut backend = MockBackend::new();
    backend.add_account("test@example.com", "correct_pass", "JBSWY3DPEHPK3PXP");

    let session = login(
        &backend,
        "test@example.com",
        "correct_pass",
        &age_auth::AgeAuthenticator::totp_now_from_base32(
            &libage_auth_handler::types::Base32String::new("JBSWY3DPEHPK3PXP").unwrap(),
        )
        .unwrap(),
        "JBSWY3DPEHPK3PXP",
    )
    .unwrap();

    assert!(session.fingerprint.as_str().len() == 64);
}

#[test]
fn login_invalid_passphrase() {
    let mut backend = MockBackend::new();
    backend.add_account("test@example.com", "correct_pass", "JBSWY3DPEHPK3PXP");

    let result = login(
        &backend,
        "test@example.com",
        "wrong_pass",
        "000000",
        "JBSWY3DPEHPK3PXP",
    );
    assert!(result.is_err());
}

#[test]
fn login_invalid_otp() {
    let mut backend = MockBackend::new();
    backend.add_account("test@example.com", "correct_pass", "JBSWY3DPEHPK3PXP");

    let result = login(
        &backend,
        "test@example.com",
        "correct_pass",
        "000000",
        "JBSWY3DPEHPK3PXP",
    );
    assert!(result.is_err());
}

#[test]
fn login_unknown_user() {
    let backend = MockBackend::new();
    let result = login(
        &backend,
        "nobody@example.com",
        "pass",
        "000000",
        "JBSWY3DPEHPK3PXP",
    );
    assert!(result.is_err());
}

#[test]
fn guard_requires_role() {
    let fp = Fingerprint::new("a".repeat(64)).unwrap();
    let admin_session = Session::new(
        posvault_handler::types::Fingerprint::new(fp.as_str()).unwrap(),
        Role::Admin,
    );

    let cashier_session = Session::new(
        posvault_handler::types::Fingerprint::new(fp.as_str()).unwrap(),
        Role::Cashier,
    );

    assert!(require_role(&admin_session, &[Role::Admin]).is_ok());
    assert!(require_role(&cashier_session, &[Role::Admin]).is_err());
    assert!(require_role(&admin_session, &[Role::Admin, Role::Manager]).is_ok());
    assert!(require_role(&cashier_session, &[Role::Cashier]).is_ok());
}
