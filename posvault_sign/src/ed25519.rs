use ed25519_dalek::{Signer as DalekSigner, SigningKey, VerifyingKey};
use posvault_handler::errors::{PosVaultError, Result};
use posvault_handler::traits::Signer;
use rand::RngCore;
use std::fmt;

pub struct Ed25519Signer {
    signing_key: SigningKey,
}

impl Ed25519Signer {
    pub fn new(signing_key: SigningKey) -> Self {
        Ed25519Signer { signing_key }
    }

    pub fn verifying_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
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

    fn verify(&self, data: &[u8], signature: &[u8]) -> Result<bool> {
        let sig_bytes: [u8; 64] = signature
            .try_into()
            .map_err(|_| PosVaultError::InvalidInput("signature must be 64 bytes".into()))?;
        let signature = ed25519_dalek::Signature::from_bytes(&sig_bytes);
        Ok(self.signing_key.verify(data, &signature).is_ok())
    }
}

pub fn generate_keypair() -> (SigningKey, VerifyingKey) {
    let mut secret_bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut secret_bytes);
    let signing_key = SigningKey::from_bytes(&secret_bytes);
    let verifying_key = signing_key.verifying_key();
    (signing_key, verifying_key)
}
