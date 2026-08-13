use ed25519_dalek::{Signer as DalekSigner, SigningKey, Verifier, VerifyingKey};
use posvault_handler::errors::Result;
use posvault_handler::traits::Signer;
use rand::TryRngCore;
use rand::rngs::OsRng;
use std::fmt;

/// Concrete Ed25519 signer adapter.
///
/// Implements the [`Signer`] trait from `posvault_handler` using the
/// `ed25519-dalek` crate. The signer holds a private [`SigningKey`] and its
/// corresponding public [`VerifyingKey`].
#[derive(Clone)]
pub struct Ed25519Signer {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
}

impl Ed25519Signer {
    /// Creates a new signer from an existing [`SigningKey`].
    pub fn new(signing_key: SigningKey) -> Self {
        let verifying_key = signing_key.verifying_key();
        Ed25519Signer {
            signing_key,
            verifying_key,
        }
    }

    /// Returns the public verifying key.
    pub fn verifying_key(&self) -> VerifyingKey {
        self.verifying_key
    }
}

impl fmt::Debug for Ed25519Signer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Ed25519Signer").finish()
    }
}

impl Signer for Ed25519Signer {
    fn sign(&self, data: &[u8]) -> Result<Vec<u8>> {
        let signature = self.signing_key.sign(data);
        Ok(signature.to_bytes().to_vec())
    }

    fn verify(&self, data: &[u8], signature: &[u8]) -> bool {
        let sig_bytes: [u8; 64] = match signature.try_into() {
            Ok(arr) => arr,
            Err(_) => return false,
        };
        let signature = ed25519_dalek::Signature::from_bytes(&sig_bytes);
        self.verifying_key.verify(data, &signature).is_ok()
    }

    fn public_key_bytes(&self) -> &[u8] {
        self.verifying_key.as_bytes()
    }
}

/// Generates a new random Ed25519 keypair.
///
/// Uses the operating system's secure random number generator.
pub fn generate_keypair() -> (SigningKey, VerifyingKey) {
    let mut secret = [0u8; 32];
    OsRng
        .try_fill_bytes(&mut secret)
        .expect("OS random number generator failed");
    let signing_key = SigningKey::from_bytes(&secret);
    let verifying_key = signing_key.verifying_key();
    (signing_key, verifying_key)
}
