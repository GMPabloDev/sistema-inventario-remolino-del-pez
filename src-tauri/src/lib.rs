pub mod auth;
mod auth_service;
mod database;
mod errors;
mod logging;
mod migration;

use std::{path::PathBuf, sync::Mutex};

use auth_service::{AdminMutationResult, AdminUser, AuthResult, AuthService, AuthStartup};
use database::initialize_database;
use errors::AppError;
use logging::Logger;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct AppState {
    pub database: Mutex<Option<DatabaseConnection>>,
    pub auth: AuthService,
    database_path: Mutex<Option<PathBuf>>,
    initialization_error: Mutex<Option<AppError>>,
    logger: Mutex<Option<Logger>>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatus {
    pub state: &'static str,
    pub version: &'static str,
}

impl AppStatus {
    fn ready() -> Self {
        Self {
            state: "ready",
            version: APP_VERSION,
        }
    }
}

impl AppState {
    fn new() -> Self {
        Self {
            database: Mutex::new(None),
            auth: AuthService::new(),
            database_path: Mutex::new(None),
            initialization_error: Mutex::new(None),
            logger: Mutex::new(None),
        }
    }
}

fn internal_error(technical: impl Into<String>) -> AppError {
    AppError::new(
        "INTERNAL_ERROR",
        "Ocurrió un error interno. Inténtalo nuevamente.",
        technical,
    )
}

fn log(state: &AppState, level: &str, context: &str, message: &str) {
    if let Ok(logger) = state.logger.lock() {
        if let Some(logger) = logger.as_ref() {
            logger.log(level, context, message);
            return;
        }
    }

    eprintln!("[{level}] {context}: {message}");
}

fn record_initialization_error(state: &AppState, error: &AppError) {
    if let Ok(mut initialization_error) = state.initialization_error.lock() {
        *initialization_error = Some(error.clone());
    }
    log(state, "ERROR", "database.initialization", error.technical());
}

fn set_database_path(state: &AppState, path: PathBuf) {
    if let Ok(mut database_path) = state.database_path.lock() {
        *database_path = Some(path);
    }
}

async fn retry_database_initialization(state: &AppState) -> Result<AppStatus, AppError> {
    {
        let database = state
            .database
            .lock()
            .map_err(|_| internal_error("database state lock is poisoned"))?;
        if database.is_some() {
            return Ok(AppStatus::ready());
        }
    }

    let path = state
        .database_path
        .lock()
        .map_err(|_| internal_error("database path state lock is poisoned"))?
        .clone()
        .ok_or_else(|| {
            AppError::new(
                "APP_DATA_DIR_UNAVAILABLE",
                "No se pudo preparar el directorio de datos de la aplicación.",
                "database path is not configured",
            )
        })?;

    match initialize_database(path).await {
        Ok(database) => {
            let mut state_database = state
                .database
                .lock()
                .map_err(|_| internal_error("database state lock is poisoned"))?;
            *state_database = Some(database);
            drop(state_database);

            if let Ok(mut initialization_error) = state.initialization_error.lock() {
                *initialization_error = None;
            }
            log(
                state,
                "INFO",
                "database.initialization",
                "database initialized",
            );
            Ok(AppStatus::ready())
        }
        Err(error) => {
            record_initialization_error(state, &error);
            Err(error)
        }
    }
}

fn database_connection(state: &AppState) -> Result<DatabaseConnection, AppError> {
    state
        .database
        .lock()
        .map_err(|_| internal_error("database state lock is poisoned"))?
        .as_ref()
        .cloned()
        .ok_or_else(|| {
            AppError::new(
                "DATABASE_UNAVAILABLE",
                "La base de datos no está disponible.",
                "database is not initialized",
            )
        })
}

fn current_status(state: &AppState) -> Result<AppStatus, AppError> {
    let database = state
        .database
        .lock()
        .map_err(|_| internal_error("database state lock is poisoned"))?;
    if database.is_some() {
        return Ok(AppStatus::ready());
    }
    drop(database);

    let initialization_error = state
        .initialization_error
        .lock()
        .map_err(|_| internal_error("initialization error state lock is poisoned"))?;
    if let Some(error) = initialization_error.clone() {
        return Err(error);
    }

    Err(AppError::new(
        "DATABASE_UNAVAILABLE",
        "La base de datos no está disponible.",
        "database has not been initialized",
    ))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChangePasswordRequest {
    current_password: Option<String>,
    new_password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateUserRequest {
    username: String,
    display_name: String,
    role: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateUserRequest {
    id: String,
    username: String,
    display_name: String,
    role: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserStatusRequest {
    id: String,
    active: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResetPasswordRequest {
    id: String,
}

#[tauri::command]
async fn get_auth_startup(state: State<'_, AppState>) -> Result<AuthStartup, AppError> {
    let database = database_connection(&state)?;
    state.auth.startup(&database).await
}

#[tauri::command]
async fn login(request: LoginRequest, state: State<'_, AppState>) -> Result<AuthResult, AppError> {
    let database = database_connection(&state)?;
    state
        .auth
        .login(&database, &request.username, &request.password)
        .await
}

#[tauri::command]
async fn change_password(
    request: ChangePasswordRequest,
    state: State<'_, AppState>,
) -> Result<AuthResult, AppError> {
    let database = database_connection(&state)?;
    state
        .auth
        .change_password(
            &database,
            request.current_password.as_deref(),
            &request.new_password,
        )
        .await
}

#[tauri::command]
async fn logout(state: State<'_, AppState>) -> Result<(), AppError> {
    let database = database_connection(&state)?;
    state.auth.logout(&database).await
}

#[tauri::command]
async fn get_identity(state: State<'_, AppState>) -> Result<crate::auth::UserIdentity, AppError> {
    let database = database_connection(&state)?;
    state.auth.restore_identity(&database).await
}

#[tauri::command]
async fn list_users(state: State<'_, AppState>) -> Result<Vec<AdminUser>, AppError> {
    let database = database_connection(&state)?;
    state.auth.list_users(&database).await
}

#[tauri::command]
async fn create_user(
    request: CreateUserRequest,
    state: State<'_, AppState>,
) -> Result<AdminMutationResult, AppError> {
    let database = database_connection(&state)?;
    let role = crate::auth::UserRole::parse(&request.role)?;
    state
        .auth
        .create_user(&database, &request.username, &request.display_name, role)
        .await
}

#[tauri::command]
async fn update_user(
    request: UpdateUserRequest,
    state: State<'_, AppState>,
) -> Result<AdminMutationResult, AppError> {
    let database = database_connection(&state)?;
    let role = crate::auth::UserRole::parse(&request.role)?;
    state
        .auth
        .update_user(
            &database,
            &request.id,
            &request.username,
            &request.display_name,
            role,
        )
        .await
}

#[tauri::command]
async fn set_user_active(
    request: UserStatusRequest,
    state: State<'_, AppState>,
) -> Result<AdminMutationResult, AppError> {
    let database = database_connection(&state)?;
    state
        .auth
        .set_user_active(&database, &request.id, request.active)
        .await
}

#[tauri::command]
async fn reset_user_password(
    request: ResetPasswordRequest,
    state: State<'_, AppState>,
) -> Result<AdminMutationResult, AppError> {
    let database = database_connection(&state)?;
    state.auth.reset_password(&database, &request.id).await
}

#[tauri::command]
fn get_app_status(state: State<'_, AppState>) -> Result<AppStatus, AppError> {
    current_status(&state)
}

#[tauri::command]
async fn retry_database(state: State<'_, AppState>) -> Result<AppStatus, AppError> {
    retry_database_initialization(&state).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::new())
        .setup(|app| {
            let state = app.state::<AppState>();
            let data_directory = match app.path().app_data_dir() {
                Ok(data_directory) => data_directory,
                Err(error) => {
                    let app_error = AppError::new(
                        "APP_DATA_DIR_UNAVAILABLE",
                        "No se pudo preparar el directorio de datos de la aplicación.",
                        error.to_string(),
                    );
                    record_initialization_error(&state, &app_error);
                    return Ok(());
                }
            };

            let database_path = data_directory.join("inventory.db");
            set_database_path(&state, database_path);

            match Logger::new(&data_directory) {
                Ok(logger) => {
                    if let Ok(mut state_logger) = state.logger.lock() {
                        *state_logger = Some(logger);
                    }
                }
                Err(error) => {
                    eprintln!("could not initialize technical logger: {error}");
                }
            }

            let _initialization_result =
                tauri::async_runtime::block_on(retry_database_initialization(&state));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_status,
            get_auth_startup,
            login,
            change_password,
            logout,
            get_identity,
            list_users,
            create_user,
            update_user,
            set_user_active,
            reset_user_password,
            retry_database
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{retry_database_initialization, AppState};

    fn temporary_path(label: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "remolino-pez-retry-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after unix epoch")
                .as_nanos()
        ));
        (directory.clone(), directory.join("inventory.db"))
    }

    #[test]
    fn retry_initializes_a_configured_database_and_returns_ready_status() {
        tauri::async_runtime::block_on(async {
            let (directory, path) = temporary_path("success");
            let state = AppState::new();
            *state.database_path.lock().expect("path lock should work") = Some(path);

            let status = retry_database_initialization(&state)
                .await
                .expect("retry should initialize the database");

            assert_eq!(status.state, "ready");
            assert_eq!(status.version, env!("CARGO_PKG_VERSION"));
            assert!(state
                .database
                .lock()
                .expect("database lock should work")
                .is_some());

            let database = state
                .database
                .lock()
                .expect("database lock should work")
                .take()
                .expect("database should be present");
            database.close().await.expect("database should close");
            fs::remove_dir_all(directory)
                .expect("temporary database directory should be removable");
        });
    }

    #[test]
    fn retry_returns_the_stable_error_code_for_an_invalid_database() {
        tauri::async_runtime::block_on(async {
            let (directory, path) = temporary_path("failure");
            fs::create_dir_all(&path)
                .expect("invalid database path should be creatable as a directory");
            let state = AppState::new();
            *state.database_path.lock().expect("path lock should work") = Some(path);

            let error = retry_database_initialization(&state)
                .await
                .expect_err("retry should fail for a directory database path");

            assert_eq!(error.code, "DATABASE_UNAVAILABLE");
            assert!(state
                .database
                .lock()
                .expect("database lock should work")
                .is_none());
            fs::remove_dir_all(directory)
                .expect("temporary database directory should be removable");
        });
    }
}
