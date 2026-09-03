//! Authentication domain primitives shared by the persistence and command layers.
//!
//! Secrets deliberately live in types that do not implement `Serialize`. Only
//! the safe identity types below are allowed to cross the Tauri boundary.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use argon2::{
    password_hash::{PasswordHash, PasswordHasher as _, PasswordVerifier as _, SaltString},
    Algorithm, Argon2, Params, Version,
};
use rand::{distributions::Alphanumeric, rngs::OsRng, Rng, RngCore};
use sea_orm::{ConnectionTrait, DatabaseTransaction, DbBackend, Statement};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::errors::AppError;

pub const SESSION_LIFETIME: Duration = Duration::from_secs(7 * 24 * 60 * 60);
const TEMPORARY_PASSWORD_LENGTH: usize = 32;
const ARGON2_MEMORY_KIB: u32 = 19 * 1024;
const ARGON2_TIME_COST: u32 = 2;
const ARGON2_PARALLELISM: u32 = 1;
const ARGON2_OUTPUT_LENGTH: usize = 32;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UserRole {
    Admin,
    WarehouseManager,
}

impl UserRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Admin => "ADMIN",
            Self::WarehouseManager => "WAREHOUSE_MANAGER",
        }
    }

    pub fn parse(value: &str) -> Result<Self, AppError> {
        match value {
            "ADMIN" => Ok(Self::Admin),
            "WAREHOUSE_MANAGER" => Ok(Self::WarehouseManager),
            _ => Err(user_validation_error(
                "role must be ADMIN or WAREHOUSE_MANAGER",
            )),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserIdentity {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub role: UserRole,
    pub must_change_password: bool,
}

/// A secret intended for one controlled hand-off to the UI. It must never be
/// serialized, logged, or stored in the database.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemporaryPassword(String);

impl TemporaryPassword {
    pub fn expose_once(&self) -> &str {
        &self.0
    }
}

/// An opaque session credential. Only its hash is persisted.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionSecret(String);

impl SessionSecret {
    pub fn expose_to_secure_store(&self) -> &str {
        &self.0
    }

    pub(crate) fn from_string(value: String) -> Self {
        Self(value)
    }
}

pub fn new_uuid() -> String {
    Uuid::new_v4().to_string()
}

pub fn generate_temporary_password() -> TemporaryPassword {
    let mut rng = OsRng;
    let password = (0..TEMPORARY_PASSWORD_LENGTH)
        .map(|_| rng.sample(Alphanumeric) as char)
        .collect();
    TemporaryPassword(password)
}

pub fn generate_session_secret() -> SessionSecret {
    let mut bytes = [0_u8; 32];
    OsRng.fill_bytes(&mut bytes);
    SessionSecret(hex_encode(&bytes))
}

pub fn hash_session_secret(secret: &SessionSecret) -> String {
    let digest = Sha256::digest(secret.0.as_bytes());
    hex_encode(&digest)
}

pub fn session_expiry(created_at: SystemTime) -> Result<SystemTime, AppError> {
    created_at
        .checked_add(SESSION_LIFETIME)
        .ok_or_else(|| internal_security_error("session expiry overflow"))
}

pub fn now_unix_millis() -> Result<i64, AppError> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| internal_security_error("system clock is before Unix epoch"))?;
    i64::try_from(duration.as_millis())
        .map_err(|_| internal_security_error("system clock does not fit in timestamp"))
}

pub fn normalize_username(value: &str) -> Result<String, AppError> {
    let normalized = value.trim().to_ascii_lowercase();
    let length = normalized.chars().count();
    if !(3..=32).contains(&length)
        || !normalized
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
    {
        return Err(user_validation_error(
            "username must contain 3 to 32 ASCII letters, numbers, dots, dashes, or underscores",
        ));
    }
    Ok(normalized)
}

pub fn normalize_display_name(value: &str) -> Result<String, AppError> {
    let normalized = value.trim().to_string();
    if !(1..=100).contains(&normalized.chars().count()) {
        return Err(user_validation_error(
            "display name must contain 1 to 100 characters",
        ));
    }
    Ok(normalized)
}

pub fn validate_password(password: &str) -> Result<(), AppError> {
    let length = password.chars().count();
    if !(12..=128).contains(&length) {
        return Err(AppError::new(
            "PASSWORD_VALIDATION_FAILED",
            "La contraseña debe tener entre 12 y 128 caracteres.",
            "password length is outside the permitted range",
        ));
    }
    Ok(())
}

pub fn hash_password(password: &str) -> Result<String, AppError> {
    validate_password(password)?;
    let salt = SaltString::generate(&mut OsRng);
    argon2()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| internal_security_error(format!("password hashing failed: {error}")))
}

pub fn verify_password(password: &str, encoded_hash: &str) -> Result<bool, AppError> {
    validate_password(password)?;
    let parsed = PasswordHash::new(encoded_hash).map_err(|error| {
        internal_security_error(format!("stored password hash is invalid: {error}"))
    })?;
    Ok(argon2()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

pub fn ensure_active_admin_remains(
    active_admin_count: u64,
    target_is_active_admin: bool,
    next_role: UserRole,
    next_active: bool,
) -> Result<(), AppError> {
    let target_remains_admin =
        target_is_active_admin && matches!(next_role, UserRole::Admin) && next_active;
    if active_admin_count == 0
        || (target_is_active_admin && !target_remains_admin && active_admin_count <= 1)
    {
        return Err(AppError::new(
            "LAST_ACTIVE_ADMIN_REQUIRED",
            "Debe existir al menos un administrador activo.",
            "user operation would remove the last active administrator",
        ));
    }
    Ok(())
}

/// Checks the administrator invariant using a transaction connection. The caller
/// must perform its mutation on this same transaction before committing.
pub async fn ensure_active_admin_remains_in_transaction(
    transaction: &DatabaseTransaction,
    target_user_id: &str,
    next_role: UserRole,
    next_active: bool,
) -> Result<(), AppError> {
    let count_result = transaction
        .query_one(Statement::from_string(
            DbBackend::Sqlite,
            "SELECT COUNT(*) FROM users WHERE role = 'ADMIN' AND active = 1",
        ))
        .await
        .map_err(|error| {
            internal_security_error(format!("could not count active admins: {error}"))
        })?
        .ok_or_else(|| internal_security_error("active admin count query returned no row"))?;
    let active_admin_count: i64 = count_result
        .try_get_by_index(0)
        .map_err(|error| internal_security_error(format!("invalid active admin count: {error}")))?;

    let target_result = transaction
        .query_one(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT role, active FROM users WHERE id = ?",
            [sea_orm::Value::String(Some(Box::new(
                target_user_id.to_owned(),
            )))],
        ))
        .await
        .map_err(|error| internal_security_error(format!("could not read target user: {error}")))?;
    let target_is_active_admin = target_result
        .map(|row| {
            let role: String = row.try_get_by_index(0).unwrap_or_default();
            let active: i64 = row.try_get_by_index(1).unwrap_or_default();
            role == "ADMIN" && active == 1
        })
        .unwrap_or(false);

    ensure_active_admin_remains(
        u64::try_from(active_admin_count).unwrap_or(0),
        target_is_active_admin,
        next_role,
        next_active,
    )
}

pub fn validate_new_password(
    password: &str,
    current_hash: Option<&str>,
) -> Result<String, AppError> {
    validate_password(password)?;
    if let Some(current_hash) = current_hash {
        if verify_password(password, current_hash)? {
            return Err(AppError::new(
                "PASSWORD_VALIDATION_FAILED",
                "La nueva contraseña debe ser diferente de la actual.",
                "new password matches current password",
            ));
        }
    }
    hash_password(password)
}

fn argon2() -> Argon2<'static> {
    let params = Params::new(
        ARGON2_MEMORY_KIB,
        ARGON2_TIME_COST,
        ARGON2_PARALLELISM,
        Some(ARGON2_OUTPUT_LENGTH),
    )
    .expect("centralized Argon2 parameters must be valid");
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn user_validation_error(technical: &'static str) -> AppError {
    AppError::new(
        "USER_VALIDATION_FAILED",
        "Los datos del usuario no son válidos.",
        technical,
    )
}

fn internal_security_error(technical: impl Into<String>) -> AppError {
    AppError::new(
        "INTERNAL_ERROR",
        "Ocurrió un error interno. Inténtalo nuevamente.",
        technical,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_and_validates_usernames() {
        assert_eq!(normalize_username(" Manager_01 ").unwrap(), "manager_01");
        assert!(normalize_username("ab").is_err());
        assert!(normalize_username("no spaces").is_err());
        assert!(normalize_username("ábc").is_err());
    }

    #[test]
    fn trims_and_validates_display_names() {
        assert_eq!(
            normalize_display_name("  Ana María  ").unwrap(),
            "Ana María"
        );
        assert!(normalize_display_name("   ").is_err());
    }

    #[test]
    fn generates_cryptographically_random_secrets() {
        let first = generate_temporary_password();
        let second = generate_temporary_password();
        assert!(first.expose_once().chars().count() >= 20);
        assert_ne!(first, second);

        let session = generate_session_secret();
        assert_eq!(session.expose_to_secure_store().len(), 64);
        assert_eq!(hash_session_secret(&session).len(), 64);
    }

    #[test]
    fn hashes_and_verifies_passwords_without_reusing_salts() {
        let first = hash_password("contraseña válida 1").unwrap();
        let second = hash_password("contraseña válida 1").unwrap();
        assert_ne!(first, second);
        assert!(verify_password("contraseña válida 1", &first).unwrap());
        assert!(!verify_password("contraseña válida 2", &first).unwrap());
        assert!(hash_password("short").is_err());
        assert!(hash_password(&"x".repeat(129)).is_err());
    }

    #[test]
    fn protects_the_last_active_administrator() {
        assert!(ensure_active_admin_remains(1, true, UserRole::Admin, true).is_ok());
        assert!(ensure_active_admin_remains(1, true, UserRole::WarehouseManager, true).is_err());
        assert!(ensure_active_admin_remains(1, true, UserRole::Admin, false).is_err());
        assert!(ensure_active_admin_remains(2, true, UserRole::WarehouseManager, true).is_ok());
    }

    #[test]
    fn computes_a_fixed_seven_day_session_lifetime() {
        let created = UNIX_EPOCH;
        assert_eq!(session_expiry(created).unwrap(), created + SESSION_LIFETIME);
    }
}
