use age_auth::AgeAuthenticator;
use age_credentials::backend::traits::AccountBackend;
use age_credentials::crypto;
use libage_auth_handler::traits::OtpGenerator;
use libage_auth_handler::types::Base32String;
use posvault_handler::errors::{PosVaultError, Result};
use posvault_handler::types::{Fingerprint, Role};

use crate::session::Session;

pub fn login(
    backend: &dyn AccountBackend,
    email: &str,
    passphrase: &str,
    otp_code: &str,
    totp_secret_base32: &str,
) -> Result<Session> {
    let age_fingerprint = backend
        .find_by_email(email)
        .map_err(|e| PosVaultError::Auth(e.to_string()))?
        .ok_or_else(|| PosVaultError::Auth(format!("user '{}' not found", email)))?;

    let age_identity = backend
        .load_identity(&age_fingerprint)
        .map_err(|e| PosVaultError::Auth(e.to_string()))?
        .ok_or_else(|| PosVaultError::Auth("identity not found".into()))?;

    let encrypted_privkey = backend
        .load_encrypted_private_key(&age_fingerprint)
        .map_err(|e| PosVaultError::Auth(e.to_string()))?
        .ok_or_else(|| PosVaultError::Auth("encrypted private key not found".into()))?;

    let _privkey_bytes = crypto::decrypt_with_passphrase(&encrypted_privkey, passphrase)
        .map_err(|e| PosVaultError::Auth(format!("failed to decrypt private key: {}", e)))?;

    let base32 = Base32String::new(totp_secret_base32)
        .map_err(|e| PosVaultError::Auth(format!("invalid TOTP secret: {}", e)))?;
    let expected_otp = AgeAuthenticator::totp_now_from_base32(&base32)
        .map_err(|e| PosVaultError::Auth(format!("failed to generate TOTP: {}", e)))?;

    if otp_code != expected_otp {
        return Err(PosVaultError::Auth("invalid OTP code".into()));
    }

    let fp_hex = age_identity.fingerprint.as_str();
    let pv_fingerprint =
        Fingerprint::new(fp_hex).map_err(|e| PosVaultError::Auth(e.to_string()))?;

    let pv_role = Role::Custom(age_identity.user_id.name.clone());

    Ok(Session::new(pv_fingerprint, pv_role))
}
