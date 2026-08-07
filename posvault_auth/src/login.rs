use age_credentials::backend::traits::AccountBackend;
use age_credentials::crypto;
use libage_auth_handler::types::Base32String;
use libage_otp::algorithms;
use posvault_handler::errors::{PosVaultError, Result};
use posvault_handler::types::{Fingerprint, Role};
use zeroize::Zeroizing;

use crate::session::Session;

pub const SESSION_DURATION_SECS: u64 = 28800;
const TOTP_DRIFT_STEPS: i64 = 1;

fn map_name_to_role(name: &str) -> Role {
    match name.to_lowercase().as_str() {
        "admin" => Role::Admin,
        "manager" => Role::Manager,
        "cashier" => Role::Cashier,
        "auditor" => Role::Auditor,
        "branch" => Role::Branch,
        other => Role::Custom(other.to_owned()),
    }
}

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

    let passphrase_owned = Zeroizing::new(passphrase.to_owned());
    let privkey_bytes =
        crypto::decrypt_with_passphrase(&encrypted_privkey, passphrase_owned.as_str())
            .map_err(|e| PosVaultError::Auth(format!("failed to decrypt private key: {}", e)))?;

    if privkey_bytes.is_empty() {
        return Err(PosVaultError::Auth("decrypted private key is empty".into()));
    }

    if privkey_bytes.len() < 32 {
        return Err(PosVaultError::Auth("private key too short".into()));
    }

    let base32 = Base32String::new(totp_secret_base32)
        .map_err(|e| PosVaultError::Auth(format!("invalid TOTP secret: {}", e)))?;
    let secret = base32
        .to_secret()
        .map_err(|e| PosVaultError::Auth(e.to_string()))?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| PosVaultError::Auth("system clock error".into()))?
        .as_secs();
    let time_step = libage_auth_handler::types::TimeStep::default();
    let digits = libage_auth_handler::types::Digits::default();
    let algo = libage_auth_handler::Algo::DEFAULT;

    let mut otp_valid = false;
    for offset in -TOTP_DRIFT_STEPS..=TOTP_DRIFT_STEPS {
        let ts = (now as i64 + offset * 30) as u64;
        let token = algorithms::compute_totp_at(&secret, ts, time_step, digits, algo)
            .map_err(|e| PosVaultError::Auth(format!("TOTP computation failed: {}", e)))?;
        if otp_code == token.format(digits.value()) {
            otp_valid = true;
            break;
        }
    }

    if !otp_valid {
        return Err(PosVaultError::Auth("invalid OTP code".into()));
    }

    let pv_role = map_name_to_role(&age_identity.user_id.name);
    let fp_hex = age_identity.fingerprint.as_str();
    let pv_fingerprint =
        Fingerprint::new(fp_hex).map_err(|e| PosVaultError::Auth(e.to_string()))?;

    Ok(Session::with_duration(
        pv_fingerprint,
        pv_role,
        SESSION_DURATION_SECS,
    ))
}
