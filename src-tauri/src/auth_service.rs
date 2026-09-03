use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use keyring::Entry;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, TransactionTrait};
use serde::Serialize;
use tokio::time::sleep;

use crate::{
    auth::{
        ensure_active_admin_remains, generate_session_secret, generate_temporary_password,
        hash_password, hash_session_secret, new_uuid, normalize_username, now_unix_millis,
        validate_new_password, verify_password, SessionSecret, TemporaryPassword, UserIdentity,
        UserRole, SESSION_LIFETIME,
    },
    errors::AppError,
};

const CREDENTIAL_SERVICE: &str = "com.remolinodelpez.inventario";
const CREDENTIAL_ACCOUNT: &str = "remembered-session";

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthStartup {
    pub state: &'static str,
    pub identity: Option<UserIdentity>,
    pub temporary_password: Option<String>,
    pub persistence_warning: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthResult {
    pub identity: UserIdentity,
    pub persistence_warning: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminUser {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub role: UserRole,
    pub active: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminMutationResult {
    pub user: AdminUser,
    pub temporary_password: Option<String>,
}

#[derive(Clone, Debug)]
struct UserRecord {
    id: String,
    username: String,
    display_name: String,
    password_hash: String,
    role: UserRole,
    active: bool,
    must_change_password: bool,
}

#[derive(Clone, Debug)]
struct ActiveSession {
    id: String,
    secret: SessionSecret,
}

#[derive(Clone, Copy, Debug, Default)]
struct FailedLogins {
    consecutive: u32,
}

pub trait CredentialStore: Send + Sync {
    fn load(&self) -> Result<Option<String>, String>;
    fn save(&self, secret: &str) -> Result<(), String>;
    fn delete(&self);
}

struct WindowsCredentialStore;

impl CredentialStore for WindowsCredentialStore {
    fn load(&self) -> Result<Option<String>, String> {
        match Entry::new(CREDENTIAL_SERVICE, CREDENTIAL_ACCOUNT)
            .map_err(|error| error.to_string())?
            .get_password()
        {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }

    fn save(&self, secret: &str) -> Result<(), String> {
        Entry::new(CREDENTIAL_SERVICE, CREDENTIAL_ACCOUNT)
            .map_err(|error| error.to_string())?
            .set_password(secret)
            .map_err(|error| error.to_string())
    }

    fn delete(&self) {
        if let Ok(entry) = Entry::new(CREDENTIAL_SERVICE, CREDENTIAL_ACCOUNT) {
            let _ = entry.delete_credential();
        }
    }
}

pub struct AuthService {
    active_session: Mutex<Option<ActiveSession>>,
    failed_logins: Mutex<HashMap<String, FailedLogins>>,
    credential_store: Arc<dyn CredentialStore>,
}

impl AuthService {
    pub fn new() -> Self {
        Self::with_credential_store(Arc::new(WindowsCredentialStore))
    }

    pub fn with_credential_store(credential_store: Arc<dyn CredentialStore>) -> Self {
        Self {
            active_session: Mutex::new(None),
            failed_logins: Mutex::new(HashMap::new()),
            credential_store,
        }
    }

    pub async fn startup(&self, database: &DatabaseConnection) -> Result<AuthStartup, AppError> {
        if user_count(database).await? == 0 {
            let (user, temporary_password) = create_initial_admin(database).await?;
            return Ok(AuthStartup {
                state: "bootstrap",
                identity: Some(identity(&user)),
                temporary_password: Some(temporary_password.expose_once().to_owned()),
                persistence_warning: false,
            });
        }

        if let Some((user, temporary_password)) = rotate_pending_bootstrap(database).await? {
            return Ok(AuthStartup {
                state: "bootstrap",
                identity: Some(identity(&user)),
                temporary_password: Some(temporary_password.expose_once().to_owned()),
                persistence_warning: false,
            });
        }

        let (restored, persistence_warning) = self.restore_remembered(database).await?;
        if let Some(user) = restored {
            return Ok(AuthStartup {
                state: "authenticated",
                identity: Some(identity(&user)),
                temporary_password: None,
                persistence_warning,
            });
        }

        Ok(AuthStartup {
            state: "login",
            identity: None,
            temporary_password: None,
            persistence_warning,
        })
    }

    pub async fn login(
        &self,
        database: &DatabaseConnection,
        username: &str,
        password: &str,
    ) -> Result<AuthResult, AppError> {
        let key = username.trim().to_ascii_lowercase();
        // Resolve the database lookup before applying the delay, while keeping
        // every externally visible failure indistinguishable.
        let user = match normalize_username(username) {
            Ok(normalized) => find_user_by_username(database, normalized).await?,
            Err(_) => None,
        };
        let valid = match user.as_ref() {
            Some(user) if user.active => {
                verify_password(password, &user.password_hash).unwrap_or(false)
            }
            _ => false,
        };

        if !valid {
            let delay = self.record_failed_attempt(&key)?;
            if let Some(delay) = delay {
                sleep(delay).await;
            }
            return Err(invalid_credentials());
        }

        self.reset_failed_attempt(&key)?;
        let user = user.expect("valid login must have a user");
        let result = self.create_session(database, &user).await?;
        Ok(result)
    }

    pub async fn restore_identity(
        &self,
        database: &DatabaseConnection,
    ) -> Result<UserIdentity, AppError> {
        let active = self.active_user(database).await?;
        Ok(identity(&active))
    }

    pub async fn change_password(
        &self,
        database: &DatabaseConnection,
        current_password: Option<&str>,
        new_password: &str,
    ) -> Result<AuthResult, AppError> {
        let active = self.active_user(database).await?;
        if !active.must_change_password {
            let current_password = current_password
                .ok_or_else(|| password_validation("La contraseña actual es obligatoria."))?;
            if !verify_password(current_password, &active.password_hash).unwrap_or(false) {
                return Err(password_validation("La contraseña actual no es válida."));
            }
        }
        let new_hash = validate_new_password(new_password, Some(&active.password_hash))?;
        let now = now_unix_millis()?;
        let transaction = database
            .begin()
            .await
            .map_err(|error| database_error(format!("could not begin password change: {error}")))?;
        execute(
            &transaction,
            "UPDATE users SET password_hash = ?, must_change_password = 0, bootstrap_pending = 0, password_changed_at = ?, updated_at = ? WHERE id = ?",
            [value(&new_hash), value_i64(now), value_i64(now), value(&active.id)],
        )
        .await?;
        revoke_user_sessions(&transaction, &active.id, now).await?;
        transaction.commit().await.map_err(|error| {
            database_error(format!("could not commit password change: {error}"))
        })?;

        let refreshed = find_user_by_id(database, &active.id)
            .await?
            .ok_or_else(user_not_found)?;
        self.create_session(database, &refreshed).await
    }

    pub async fn logout(&self, database: &DatabaseConnection) -> Result<(), AppError> {
        let active = self.take_active_session()?;
        if let Some(active) = active {
            let now = now_unix_millis()?;
            execute(
                database,
                "UPDATE sessions SET revoked_at = ? WHERE id = ? AND revoked_at IS NULL",
                [value_i64(now), value(&active.id)],
            )
            .await?;
        }
        self.credential_store.delete();
        Ok(())
    }

    pub async fn has_active_session(
        &self,
        database: &DatabaseConnection,
    ) -> Result<bool, AppError> {
        Ok(self.active_user(database).await.is_ok())
    }

    async fn restore_remembered(
        &self,
        database: &DatabaseConnection,
    ) -> Result<(Option<UserRecord>, bool), AppError> {
        let (secret, persistence_warning) = match self.credential_store.load() {
            Ok(secret) => (secret, false),
            Err(_) => (None, true),
        };
        let Some(secret) = secret else {
            return Ok((None, persistence_warning));
        };
        let token_hash = hash_stored_secret(&secret);
        let now = now_unix_millis()?;
        let row = database
            .query_one(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                "SELECT s.id, s.user_id, u.id, u.username, u.display_name, u.password_hash, u.role, u.active, u.must_change_password FROM sessions s JOIN users u ON u.id = s.user_id WHERE s.token_hash = ? AND s.revoked_at IS NULL AND s.expires_at > ? AND u.active = 1",
                [value(&token_hash), value_i64(now)],
            ))
            .await
            .map_err(|error| database_error(format!("could not restore session: {error}")))?;
        let Some(row) = row else {
            self.credential_store.delete();
            return Ok((None, persistence_warning));
        };
        let session_id: String = row_get(&row, 0)?;
        let user = user_from_row(&row, 2)?;
        self.set_active_session(ActiveSession {
            id: session_id,
            secret: SessionSecret::from_string(secret),
        })?;
        Ok((Some(user), persistence_warning))
    }

    async fn create_session(
        &self,
        database: &DatabaseConnection,
        user: &UserRecord,
    ) -> Result<AuthResult, AppError> {
        let secret = generate_session_secret();
        let session_id = new_uuid();
        let now = now_unix_millis()?;
        let expires = now
            .checked_add(
                i64::try_from(SESSION_LIFETIME.as_millis())
                    .map_err(|_| internal_error("session lifetime overflow"))?,
            )
            .ok_or_else(|| internal_error("session expiry overflow"))?;
        let transaction = database.begin().await.map_err(|error| {
            database_error(format!("could not begin session creation: {error}"))
        })?;
        revoke_user_sessions(&transaction, &user.id, now).await?;
        execute(
            &transaction,
            "INSERT INTO sessions (id, user_id, token_hash, created_at, expires_at) VALUES (?, ?, ?, ?, ?)",
            [
                value(&session_id),
                value(&user.id),
                value(&hash_session_secret(&secret)),
                value_i64(now),
                value_i64(expires),
            ],
        )
        .await?;
        transaction.commit().await.map_err(|error| {
            database_error(format!("could not commit session creation: {error}"))
        })?;

        let persistence_warning = match self.credential_store.save(secret.expose_to_secure_store())
        {
            Ok(()) => false,
            Err(_) => true,
        };
        self.set_active_session(ActiveSession {
            id: session_id,
            secret,
        })?;
        Ok(AuthResult {
            identity: identity(user),
            persistence_warning,
        })
    }

    async fn active_user(&self, database: &DatabaseConnection) -> Result<UserRecord, AppError> {
        let active = self
            .active_session
            .lock()
            .map_err(|_| internal_error("active session lock is poisoned"))?
            .clone()
            .ok_or_else(|| {
                AppError::new(
                    "AUTH_SESSION_REQUIRED",
                    "Necesitas iniciar sesión.",
                    "no active session",
                )
            })?;
        let now = now_unix_millis()?;
        let row = database
            .query_one(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                "SELECT u.id, u.username, u.display_name, u.password_hash, u.role, u.active, u.must_change_password FROM sessions s JOIN users u ON u.id = s.user_id WHERE s.id = ? AND s.token_hash = ? AND s.revoked_at IS NULL AND s.expires_at > ? AND u.active = 1",
                [value(&active.id), value(&hash_session_secret(&active.secret)), value_i64(now)],
            ))
            .await
            .map_err(|error| database_error(format!("could not validate active session: {error}")))?;
        let Some(row) = row else {
            self.take_active_session()?;
            self.credential_store.delete();
            return Err(AppError::new(
                "AUTH_SESSION_EXPIRED",
                "Tu sesión ya no es válida. Inicia sesión nuevamente.",
                "session is expired, revoked, or inactive",
            ));
        };
        user_from_row(&row, 0)
    }

    fn record_failed_attempt(&self, key: &str) -> Result<Option<Duration>, AppError> {
        let mut failures = self
            .failed_logins
            .lock()
            .map_err(|_| internal_error("failed login state lock is poisoned"))?;
        let state = failures.entry(key.to_owned()).or_default();
        state.consecutive = state.consecutive.saturating_add(1);
        if state.consecutive < 4 {
            return Ok(None);
        }
        let exponent = state.consecutive.saturating_sub(4).min(4);
        let seconds = (1_u64 << exponent).min(30);
        Ok(Some(Duration::from_secs(seconds)))
    }

    fn reset_failed_attempt(&self, key: &str) -> Result<(), AppError> {
        self.failed_logins
            .lock()
            .map_err(|_| internal_error("failed login state lock is poisoned"))?
            .remove(key);
        Ok(())
    }

    fn set_active_session(&self, session: ActiveSession) -> Result<(), AppError> {
        *self
            .active_session
            .lock()
            .map_err(|_| internal_error("active session lock is poisoned"))? = Some(session);
        Ok(())
    }

    fn take_active_session(&self) -> Result<Option<ActiveSession>, AppError> {
        Ok(self
            .active_session
            .lock()
            .map_err(|_| internal_error("active session lock is poisoned"))?
            .take())
    }
}

impl AuthService {
    pub async fn list_users(
        &self,
        database: &DatabaseConnection,
    ) -> Result<Vec<AdminUser>, AppError> {
        self.require_admin(database).await?;
        let rows = database
            .query_all(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT id, username, display_name, role, active, created_at, updated_at FROM users ORDER BY username",
            ))
            .await
            .map_err(|error| database_error(format!("could not list users: {error}")))?;
        rows.iter().map(admin_user_from_row).collect()
    }

    pub async fn create_user(
        &self,
        database: &DatabaseConnection,
        username: &str,
        display_name: &str,
        role: UserRole,
    ) -> Result<AdminMutationResult, AppError> {
        self.require_admin(database).await?;
        let username = normalize_username(username)?;
        let display_name = crate::auth::normalize_display_name(display_name)?;
        let temporary_password = generate_temporary_password();
        let password_hash = hash_password(temporary_password.expose_once())?;
        let id = new_uuid();
        let now = now_unix_millis()?;
        let transaction = database
            .begin()
            .await
            .map_err(|error| database_error(format!("could not begin user creation: {error}")))?;
        let result = execute(
            &transaction,
            "INSERT INTO users (id, username, display_name, password_hash, role, active, must_change_password, bootstrap_pending, created_at, updated_at) VALUES (?, ?, ?, ?, ?, 1, 1, 0, ?, ?)",
            [value(&id), value(&username), value(&display_name), value(&password_hash), value(role.as_str()), value_i64(now), value_i64(now)],
        )
        .await;
        if let Err(error) = result {
            let _ = transaction.rollback().await;
            if error.technical().to_ascii_lowercase().contains("unique") {
                return Err(username_exists());
            }
            return Err(error);
        }
        transaction
            .commit()
            .await
            .map_err(|error| database_error(format!("could not commit user creation: {error}")))?;
        let user = find_admin_user_by_id(database, &id)
            .await?
            .ok_or_else(user_not_found)?;
        Ok(AdminMutationResult {
            user,
            temporary_password: Some(temporary_password.expose_once().to_owned()),
        })
    }

    pub async fn update_user(
        &self,
        database: &DatabaseConnection,
        user_id: &str,
        username: &str,
        display_name: &str,
        role: UserRole,
    ) -> Result<AdminMutationResult, AppError> {
        let admin = self.require_admin(database).await?;
        let username = normalize_username(username)?;
        let display_name = crate::auth::normalize_display_name(display_name)?;
        if admin.id == user_id && (admin.username != username || admin.role != role) {
            return Err(self_management_error());
        }
        let now = now_unix_millis()?;
        let transaction = database
            .begin()
            .await
            .map_err(|error| database_error(format!("could not begin user update: {error}")))?;
        let target = find_user_by_id_on(&transaction, user_id)
            .await?
            .ok_or_else(user_not_found)?;
        ensure_active_admin_remains(
            active_admin_count_on(&transaction).await?,
            target.active && target.role == UserRole::Admin,
            role,
            target.active,
        )?;
        let result = execute(
            &transaction,
            "UPDATE users SET username = ?, display_name = ?, role = ?, updated_at = ? WHERE id = ? AND (role <> 'ADMIN' OR active = 0 OR (? = 'ADMIN' AND active = 1) OR (SELECT COUNT(*) FROM users WHERE role = 'ADMIN' AND active = 1) > 1)",
            [value(&username), value(&display_name), value(role.as_str()), value_i64(now), value(user_id), value(role.as_str())],
        )
        .await;
        if let Err(error) = result {
            let _ = transaction.rollback().await;
            if error.technical().to_ascii_lowercase().contains("unique") {
                return Err(username_exists());
            }
            return Err(error);
        }
        if target.role != role || target.username != username {
            revoke_user_sessions(&transaction, user_id, now).await?;
        }
        transaction
            .commit()
            .await
            .map_err(|error| database_error(format!("could not commit user update: {error}")))?;
        let user = find_admin_user_by_id(database, user_id)
            .await?
            .ok_or_else(user_not_found)?;
        Ok(AdminMutationResult {
            user,
            temporary_password: None,
        })
    }

    pub async fn set_user_active(
        &self,
        database: &DatabaseConnection,
        user_id: &str,
        active: bool,
    ) -> Result<AdminMutationResult, AppError> {
        let admin = self.require_admin(database).await?;
        if admin.id == user_id {
            return Err(self_management_error());
        }
        let now = now_unix_millis()?;
        let transaction = database
            .begin()
            .await
            .map_err(|error| database_error(format!("could not begin status update: {error}")))?;
        let target = find_user_by_id_on(&transaction, user_id)
            .await?
            .ok_or_else(user_not_found)?;
        ensure_active_admin_remains(
            active_admin_count_on(&transaction).await?,
            target.active && target.role == UserRole::Admin,
            target.role,
            active,
        )?;
        execute(
            &transaction,
            "UPDATE users SET active = ?, updated_at = ? WHERE id = ? AND (role <> 'ADMIN' OR active = 0 OR ? = 1 OR (SELECT COUNT(*) FROM users WHERE role = 'ADMIN' AND active = 1) > 1)",
            [value_i64(if active { 1 } else { 0 }), value_i64(now), value(user_id), value_i64(if active { 1 } else { 0 })],
        )
        .await?;
        revoke_user_sessions(&transaction, user_id, now).await?;
        transaction
            .commit()
            .await
            .map_err(|error| database_error(format!("could not commit status update: {error}")))?;
        let user = find_admin_user_by_id(database, user_id)
            .await?
            .ok_or_else(user_not_found)?;
        Ok(AdminMutationResult {
            user,
            temporary_password: None,
        })
    }

    pub async fn reset_password(
        &self,
        database: &DatabaseConnection,
        user_id: &str,
    ) -> Result<AdminMutationResult, AppError> {
        let admin = self.require_admin(database).await?;
        if admin.id == user_id {
            return Err(self_management_error());
        }
        let target = find_user_by_id(database, user_id)
            .await?
            .ok_or_else(user_not_found)?;
        let temporary_password = generate_temporary_password();
        let password_hash = hash_password(temporary_password.expose_once())?;
        let now = now_unix_millis()?;
        let transaction = database
            .begin()
            .await
            .map_err(|error| database_error(format!("could not begin password reset: {error}")))?;
        execute(
            &transaction,
            "UPDATE users SET password_hash = ?, must_change_password = 1, bootstrap_pending = 0, password_changed_at = NULL, updated_at = ? WHERE id = ?",
            [value(&password_hash), value_i64(now), value(user_id)],
        )
        .await?;
        revoke_user_sessions(&transaction, &target.id, now).await?;
        transaction
            .commit()
            .await
            .map_err(|error| database_error(format!("could not commit password reset: {error}")))?;
        let user = find_admin_user_by_id(database, user_id)
            .await?
            .ok_or_else(user_not_found)?;
        Ok(AdminMutationResult {
            user,
            temporary_password: Some(temporary_password.expose_once().to_owned()),
        })
    }

    async fn require_admin(&self, database: &DatabaseConnection) -> Result<UserRecord, AppError> {
        let user = self.active_user(database).await?;
        if user.must_change_password {
            return Err(AppError::new(
                "AUTH_PASSWORD_CHANGE_REQUIRED",
                "Debes cambiar tu contraseña antes de continuar.",
                "temporary password session attempted an administrative operation",
            ));
        }
        if user.role != UserRole::Admin {
            return Err(AppError::new(
                "AUTH_FORBIDDEN",
                "No tienes permisos para realizar esta operación.",
                "user role is not ADMIN",
            ));
        }
        Ok(user)
    }
}

async fn create_initial_admin(
    database: &DatabaseConnection,
) -> Result<(UserRecord, TemporaryPassword), AppError> {
    let temporary_password = generate_temporary_password();
    let password_hash = hash_password(temporary_password.expose_once())?;
    let id = new_uuid();
    let now = now_unix_millis()?;
    let transaction = database
        .begin()
        .await
        .map_err(|error| database_error(format!("could not begin bootstrap: {error}")))?;
    execute(
        &transaction,
        "INSERT INTO users (id, username, display_name, password_hash, role, active, must_change_password, bootstrap_pending, created_at, updated_at) VALUES (?, 'admin', 'Administrador', ?, 'ADMIN', 1, 1, 1, ?, ?)",
        [value(&id), value(&password_hash), value_i64(now), value_i64(now)],
    )
    .await?;
    transaction
        .commit()
        .await
        .map_err(|error| database_error(format!("could not commit bootstrap: {error}")))?;
    let user = find_user_by_id(database, &id)
        .await?
        .ok_or_else(|| internal_error("bootstrap user was not found after creation"))?;
    Ok((user, temporary_password))
}

async fn rotate_pending_bootstrap(
    database: &DatabaseConnection,
) -> Result<Option<(UserRecord, TemporaryPassword)>, AppError> {
    let row = database
        .query_one(Statement::from_string(
            DbBackend::Sqlite,
            "SELECT id FROM users WHERE bootstrap_pending = 1 LIMIT 1",
        ))
        .await
        .map_err(|error| database_error(format!("could not inspect bootstrap state: {error}")))?;
    let Some(row) = row else { return Ok(None) };
    let id: String = row_get(&row, 0)?;
    let temporary_password = generate_temporary_password();
    let password_hash = hash_password(temporary_password.expose_once())?;
    let now = now_unix_millis()?;
    let transaction = database
        .begin()
        .await
        .map_err(|error| database_error(format!("could not begin bootstrap rotation: {error}")))?;
    execute(
        &transaction,
        "UPDATE users SET password_hash = ?, must_change_password = 1, bootstrap_pending = 1, updated_at = ? WHERE id = ?",
        [value(&password_hash), value_i64(now), value(&id)],
    )
    .await?;
    revoke_user_sessions(&transaction, &id, now).await?;
    transaction
        .commit()
        .await
        .map_err(|error| database_error(format!("could not commit bootstrap rotation: {error}")))?;
    let user = find_user_by_id(database, &id)
        .await?
        .ok_or_else(|| internal_error("bootstrap user disappeared during rotation"))?;
    Ok(Some((user, temporary_password)))
}

async fn user_count(database: &DatabaseConnection) -> Result<i64, AppError> {
    let row = database
        .query_one(Statement::from_string(
            DbBackend::Sqlite,
            "SELECT COUNT(*) FROM users",
        ))
        .await
        .map_err(|error| database_error(format!("could not count users: {error}")))?
        .ok_or_else(|| internal_error("user count query returned no row"))?;
    row_get(&row, 0)
}

async fn find_user_by_id_on<C: ConnectionTrait>(
    connection: &C,
    id: &str,
) -> Result<Option<UserRecord>, AppError> {
    let row = connection
        .query_one(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT id, username, display_name, password_hash, role, active, must_change_password FROM users WHERE id = ?",
            [value(id)],
        ))
        .await
        .map_err(|error| database_error(format!("could not find user: {error}")))?;
    row.map(|row| user_from_row(&row, 0)).transpose()
}

async fn active_admin_count_on<C: ConnectionTrait>(connection: &C) -> Result<u64, AppError> {
    let row = connection
        .query_one(Statement::from_string(
            DbBackend::Sqlite,
            "SELECT COUNT(*) FROM users WHERE role = 'ADMIN' AND active = 1",
        ))
        .await
        .map_err(|error| database_error(format!("could not count active admins: {error}")))?
        .ok_or_else(|| internal_error("active admin count query returned no row"))?;
    let count: i64 = row_get(&row, 0)?;
    u64::try_from(count).map_err(|_| internal_error("active admin count was negative"))
}

async fn find_admin_user_by_id(
    database: &DatabaseConnection,
    id: &str,
) -> Result<Option<AdminUser>, AppError> {
    let row = database
        .query_one(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT id, username, display_name, role, active, created_at, updated_at FROM users WHERE id = ?",
            [value(id)],
        ))
        .await
        .map_err(|error| database_error(format!("could not find user: {error}")))?;
    row.map(|row| admin_user_from_row(&row)).transpose()
}

async fn find_user_by_id(
    database: &DatabaseConnection,
    id: &str,
) -> Result<Option<UserRecord>, AppError> {
    let row = database
        .query_one(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT id, username, display_name, password_hash, role, active, must_change_password FROM users WHERE id = ?",
            [value(id)],
        ))
        .await
        .map_err(|error| database_error(format!("could not find user: {error}")))?;
    row.map(|row| user_from_row(&row, 0)).transpose()
}

async fn find_user_by_username(
    database: &DatabaseConnection,
    username: String,
) -> Result<Option<UserRecord>, AppError> {
    let row = database
        .query_one(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT id, username, display_name, password_hash, role, active, must_change_password FROM users WHERE username = ? COLLATE NOCASE",
            [value(&username)],
        ))
        .await
        .map_err(|error| database_error(format!("could not find user by username: {error}")))?;
    row.map(|row| user_from_row(&row, 0)).transpose()
}

async fn execute<C: ConnectionTrait>(
    connection: &C,
    sql: &str,
    values: impl IntoIterator<Item = sea_orm::Value>,
) -> Result<(), AppError> {
    connection
        .execute(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            sql,
            values,
        ))
        .await
        .map(|_| ())
        .map_err(|error| database_error(format!("database operation failed: {error}")))
}

async fn revoke_user_sessions<C: ConnectionTrait>(
    connection: &C,
    user_id: &str,
    now: i64,
) -> Result<(), AppError> {
    execute(
        connection,
        "UPDATE sessions SET revoked_at = ? WHERE user_id = ? AND revoked_at IS NULL",
        [value_i64(now), value(user_id)],
    )
    .await
}

fn admin_user_from_row(row: &sea_orm::QueryResult) -> Result<AdminUser, AppError> {
    Ok(AdminUser {
        id: row_get(row, 0)?,
        username: row_get(row, 1)?,
        display_name: row_get(row, 2)?,
        role: UserRole::parse(&row_get::<String>(row, 3)?)?,
        active: row_get::<i64>(row, 4)? != 0,
        created_at: row_get(row, 5)?,
        updated_at: row_get(row, 6)?,
    })
}

fn user_from_row(row: &sea_orm::QueryResult, offset: usize) -> Result<UserRecord, AppError> {
    let role = UserRole::parse(&row_get::<String>(row, offset + 4)?)?;
    Ok(UserRecord {
        id: row_get(row, offset)?,
        username: row_get(row, offset + 1)?,
        display_name: row_get(row, offset + 2)?,
        password_hash: row_get(row, offset + 3)?,
        role,
        active: row_get::<i64>(row, offset + 5)? != 0,
        must_change_password: row_get::<i64>(row, offset + 6)? != 0,
    })
}

fn identity(user: &UserRecord) -> UserIdentity {
    UserIdentity {
        id: user.id.clone(),
        username: user.username.clone(),
        display_name: user.display_name.clone(),
        role: user.role,
        must_change_password: user.must_change_password,
    }
}

fn row_get<T: sea_orm::TryGetable>(
    row: &sea_orm::QueryResult,
    index: usize,
) -> Result<T, AppError> {
    row.try_get_by_index(index)
        .map_err(|error| database_error(format!("invalid authentication row: {error}")))
}

fn value(value: &str) -> sea_orm::Value {
    sea_orm::Value::String(Some(Box::new(value.to_owned())))
}

fn value_i64(value: i64) -> sea_orm::Value {
    sea_orm::Value::BigInt(Some(value))
}

fn hash_stored_secret(secret: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(secret.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn invalid_credentials() -> AppError {
    AppError::new(
        "AUTH_INVALID_CREDENTIALS",
        "Las credenciales no son válidas.",
        "login credentials were not accepted",
    )
}

fn username_exists() -> AppError {
    AppError::new(
        "USERNAME_ALREADY_EXISTS",
        "Ese nombre de usuario ya existe.",
        "normalized username violates the unique constraint",
    )
}

fn self_management_error() -> AppError {
    AppError::new(
        "SELF_MANAGEMENT_NOT_ALLOWED",
        "No puedes modificar tu propio usuario, rol o estado desde esta pantalla.",
        "administrator attempted forbidden self-management",
    )
}

fn password_validation(message: &'static str) -> AppError {
    AppError::new(
        "PASSWORD_VALIDATION_FAILED",
        message,
        "password change validation failed",
    )
}

fn database_error(technical: impl Into<String>) -> AppError {
    AppError::new(
        "DATABASE_UNAVAILABLE",
        "La base de datos no está disponible.",
        technical,
    )
}

fn user_not_found() -> AppError {
    AppError::new("USER_NOT_FOUND", "El usuario no existe.", "user not found")
}

fn internal_error(technical: impl Into<String>) -> AppError {
    AppError::new(
        "INTERNAL_ERROR",
        "Ocurrió un error interno. Inténtalo nuevamente.",
        technical,
    )
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{Arc, Mutex},
        time::{SystemTime, UNIX_EPOCH},
    };

    use sea_orm::{ConnectionTrait, DbBackend, Statement};

    use super::{AuthService, CredentialStore};
    use crate::{auth::UserRole, database::initialize_database};

    #[derive(Default)]
    struct MemoryStore {
        secret: Mutex<Option<String>>,
    }

    struct FailingStore;

    impl CredentialStore for FailingStore {
        fn load(&self) -> Result<Option<String>, String> {
            Err("secure store unavailable".to_owned())
        }

        fn save(&self, _secret: &str) -> Result<(), String> {
            Err("secure store unavailable".to_owned())
        }

        fn delete(&self) {}
    }

    impl CredentialStore for MemoryStore {
        fn load(&self) -> Result<Option<String>, String> {
            Ok(self
                .secret
                .lock()
                .expect("memory store lock should work")
                .clone())
        }

        fn save(&self, secret: &str) -> Result<(), String> {
            *self.secret.lock().expect("memory store lock should work") = Some(secret.to_owned());
            Ok(())
        }

        fn delete(&self) {
            *self.secret.lock().expect("memory store lock should work") = None;
        }
    }

    fn remove_temporary_directory(directory: std::path::PathBuf) {
        let mut last_error = None;
        for _ in 0..100 {
            match std::fs::remove_dir_all(&directory) {
                Ok(()) => return,
                Err(error) => {
                    last_error = Some(error);
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            }
        }
        panic!("temporary database should be removable: {last_error:?}");
    }

    fn temporary_path(label: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after Unix epoch")
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "remolino-pez-auth-{label}-{}-{suffix}",
            std::process::id()
        ));
        (directory.clone(), directory.join("inventory.db"))
    }

    #[tokio::test]
    async fn bootstraps_rotates_the_temporary_credential_and_changes_password() {
        let (directory, path) = temporary_path("bootstrap");
        let database = initialize_database(&path)
            .await
            .expect("database should initialize");
        let store = Arc::new(MemoryStore::default());
        let service = AuthService::with_credential_store(store.clone());

        let first = service
            .startup(&database)
            .await
            .expect("bootstrap should succeed");
        let first_password = first
            .temporary_password
            .expect("temporary password should be returned");
        assert_eq!(first.state, "bootstrap");
        assert!(first_password.chars().count() >= 20);

        let hash_row = database
            .query_one(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT password_hash FROM users WHERE username = 'admin'",
            ))
            .await
            .expect("hash query should succeed")
            .expect("hash row should exist");
        let hash: String = hash_row.try_get_by_index(0).expect("hash should be text");
        assert!(!hash.contains(&first_password));

        let second = service
            .startup(&database)
            .await
            .expect("bootstrap rotation should succeed");
        let second_password = second
            .temporary_password
            .expect("rotated password should exist");
        assert_ne!(first_password, second_password);
        assert!(service
            .login(&database, "ADMIN", &first_password)
            .await
            .is_err());

        let logged_in = service
            .login(&database, " admin ", &second_password)
            .await
            .expect("rotated password should work");
        assert!(logged_in.identity.must_change_password);
        let changed = service
            .change_password(&database, None, "una contraseña definitiva")
            .await
            .expect("mandatory password change should work");
        assert!(!changed.identity.must_change_password);

        let restored_service = AuthService::with_credential_store(store);
        let restored = restored_service
            .startup(&database)
            .await
            .expect("session restoration should work");
        assert_eq!(restored.state, "authenticated");
        assert_eq!(
            restored.identity.expect("identity should exist").username,
            "admin"
        );
        restored_service
            .logout(&database)
            .await
            .expect("logout should work");
        database.close().await.expect("database should close");
        remove_temporary_directory(directory);
    }

    #[tokio::test]
    async fn rejects_expired_revoked_and_inactive_sessions_without_leaking_state() {
        let (directory, path) = temporary_path("session-invalid");
        let database = initialize_database(&path)
            .await
            .expect("database should initialize");
        let store = Arc::new(MemoryStore::default());
        let service = AuthService::with_credential_store(store.clone());
        let bootstrap = service
            .startup(&database)
            .await
            .expect("bootstrap should work");
        let password = bootstrap
            .temporary_password
            .expect("temporary password should exist");

        service
            .login(&database, "admin", &password)
            .await
            .expect("login should work");
        service
            .logout(&database)
            .await
            .expect("logout should revoke the session");
        assert!(service.restore_identity(&database).await.is_err());

        service
            .login(&database, "admin", &password)
            .await
            .expect("login should work again");
        service
            .change_password(&database, None, "una contraseña definitiva")
            .await
            .expect("temporary password should be changed");
        database
            .execute(Statement::from_string(
                DbBackend::Sqlite,
                "UPDATE sessions SET expires_at = 0",
            ))
            .await
            .expect("session should be expirable in the test database");
        let restored = AuthService::with_credential_store(store.clone())
            .startup(&database)
            .await
            .expect("expired restoration should fall back to login");
        assert_eq!(restored.state, "login");

        database
            .execute(Statement::from_string(
                DbBackend::Sqlite,
                "UPDATE users SET active = 0 WHERE username = 'admin'",
            ))
            .await
            .expect("user should be deactivated in the test database");
        let error = service
            .login(&database, "admin", "una contraseña definitiva")
            .await
            .expect_err("inactive user should not log in");
        assert_eq!(error.code, "AUTH_INVALID_CREDENTIALS");
        database.close().await.expect("database should close");
        remove_temporary_directory(directory);
    }

    #[tokio::test]
    async fn reports_when_the_secure_store_is_unavailable_without_using_a_fallback() {
        let (directory, path) = temporary_path("store-failure");
        let database = initialize_database(&path)
            .await
            .expect("database should initialize");
        let service = AuthService::with_credential_store(Arc::new(FailingStore));
        let bootstrap = service
            .startup(&database)
            .await
            .expect("bootstrap should work");
        let password = bootstrap
            .temporary_password
            .expect("temporary password should exist");
        let result = service
            .login(&database, "admin", &password)
            .await
            .expect("login should work");
        assert!(result.persistence_warning);
        let session_row = database
            .query_one(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT COUNT(*) FROM sessions",
            ))
            .await
            .expect("session query should work")
            .expect("session count should exist");
        let session_count = session_row
            .try_get_by_index::<i64>(0)
            .expect("session count should be numeric");
        drop(session_row);
        assert_eq!(session_count, 1);
        database.close().await.expect("database should close");
        remove_temporary_directory(directory);
    }

    #[tokio::test]
    async fn authorizes_administration_and_revokes_security_changes() {
        let (directory, path) = temporary_path("administration");
        let database = initialize_database(&path)
            .await
            .expect("database should initialize");
        let admin_store = Arc::new(MemoryStore::default());
        let admin_service = AuthService::with_credential_store(admin_store);
        let bootstrap = admin_service
            .startup(&database)
            .await
            .expect("bootstrap should work");
        let admin_password = bootstrap
            .temporary_password
            .expect("admin password should exist");
        admin_service
            .login(&database, "admin", &admin_password)
            .await
            .expect("admin login should work");
        admin_service
            .change_password(&database, None, "contraseña admin definitiva")
            .await
            .expect("admin password should change");

        let created = admin_service
            .create_user(
                &database,
                "Manager",
                "Encargado",
                UserRole::WarehouseManager,
            )
            .await
            .expect("admin should create a manager");
        let manager_id = created.user.id.clone();
        let manager_password = created
            .temporary_password
            .expect("manager password should exist");
        assert_eq!(
            admin_service
                .list_users(&database)
                .await
                .expect("admin should list users")
                .len(),
            2
        );

        let unauthenticated = AuthService::with_credential_store(Arc::new(MemoryStore::default()));
        let required = unauthenticated
            .list_users(&database)
            .await
            .expect_err("administration requires a session");
        assert_eq!(required.code, "AUTH_SESSION_REQUIRED");

        let manager_service = AuthService::with_credential_store(Arc::new(MemoryStore::default()));
        manager_service
            .login(&database, "manager", &manager_password)
            .await
            .expect("manager login should work");
        manager_service
            .change_password(&database, None, "contraseña manager definitiva")
            .await
            .expect("manager password should change");
        let forbidden = manager_service
            .list_users(&database)
            .await
            .expect_err("warehouse manager must not administer users");
        assert_eq!(forbidden.code, "AUTH_FORBIDDEN");

        admin_service
            .update_user(
                &database,
                &manager_id,
                "MANAGER",
                "Encargado de almacén",
                UserRole::WarehouseManager,
            )
            .await
            .expect("admin should edit the manager");
        let reset = admin_service
            .reset_password(&database, &manager_id)
            .await
            .expect("admin should reset the manager password");
        assert!(manager_service.restore_identity(&database).await.is_err());
        let reset_password = reset
            .temporary_password
            .expect("reset password should exist");
        admin_service
            .set_user_active(&database, &manager_id, false)
            .await
            .expect("admin should deactivate the manager");
        let invalid = manager_service
            .login(&database, "manager", &reset_password)
            .await
            .expect_err("inactive manager must not log in");
        assert_eq!(invalid.code, "AUTH_INVALID_CREDENTIALS");
        admin_service
            .set_user_active(&database, &manager_id, true)
            .await
            .expect("admin should reactivate the manager");

        let own_update = admin_service
            .update_user(
                &database,
                "not-the-admin",
                "other-admin",
                "Other",
                UserRole::Admin,
            )
            .await
            .expect_err("unknown users should be rejected");
        assert_eq!(own_update.code, "USER_NOT_FOUND");
        database.close().await.expect("database should close");
        remove_temporary_directory(directory);
    }

    #[test]
    fn applies_progressive_delays_after_the_third_failure() {
        let service = AuthService::with_credential_store(Arc::new(MemoryStore::default()));
        assert_eq!(service.record_failed_attempt("admin").unwrap(), None);
        assert_eq!(service.record_failed_attempt("admin").unwrap(), None);
        assert_eq!(service.record_failed_attempt("admin").unwrap(), None);
        assert_eq!(
            service
                .record_failed_attempt("admin")
                .unwrap()
                .unwrap()
                .as_secs(),
            1
        );
        assert_eq!(
            service
                .record_failed_attempt("admin")
                .unwrap()
                .unwrap()
                .as_secs(),
            2
        );
        assert_eq!(
            service
                .record_failed_attempt("admin")
                .unwrap()
                .unwrap()
                .as_secs(),
            4
        );
        assert_eq!(
            service
                .record_failed_attempt("admin")
                .unwrap()
                .unwrap()
                .as_secs(),
            8
        );
        assert_eq!(
            service
                .record_failed_attempt("admin")
                .unwrap()
                .unwrap()
                .as_secs(),
            16
        );
    }
}
