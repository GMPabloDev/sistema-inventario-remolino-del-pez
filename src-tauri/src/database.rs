use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use sea_orm::{
    sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode},
    ConnectOptions, Database, DatabaseConnection,
};
use sea_orm_migration::MigratorTrait;

use crate::{errors::AppError, migration::Migrator};

const MAX_CONNECTIONS: u32 = 5;
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// Creates the database directory, opens SQLite with the application defaults,
/// and applies all pending migrations.
pub async fn initialize_database(path: impl Into<PathBuf>) -> Result<DatabaseConnection, AppError> {
    let path = path.into();
    let parent = path.parent().ok_or_else(|| {
        AppError::new(
            "APP_DATA_DIR_UNAVAILABLE",
            "No se pudo preparar el directorio de datos de la aplicación.",
            "database path has no parent directory",
        )
    })?;

    std::fs::create_dir_all(parent).map_err(|error| {
        AppError::new(
            "APP_DATA_DIR_UNAVAILABLE",
            "No se pudo preparar el directorio de datos de la aplicación.",
            format!("could not create database directory: {error}"),
        )
    })?;

    let database = connect(&path).await?;
    Migrator::up(&database, None).await.map_err(|error| {
        AppError::new(
            "DATABASE_MIGRATION_FAILED",
            "No se pudieron aplicar las actualizaciones de la base de datos.",
            format!("could not apply database migrations: {error}"),
        )
    })?;

    Ok(database)
}

fn connection_options(path: &Path) -> ConnectOptions {
    let database_path = path.to_owned();
    let mut options = ConnectOptions::new("sqlite://inventory.db");
    options
        .max_connections(MAX_CONNECTIONS)
        .min_connections(1)
        .connect_timeout(CONNECTION_TIMEOUT)
        .acquire_timeout(CONNECTION_TIMEOUT)
        .map_sqlx_sqlite_opts(move |sqlite_options: SqliteConnectOptions| {
            sqlite_options
                .filename(database_path.clone())
                .create_if_missing(true)
                .foreign_keys(true)
                .journal_mode(SqliteJournalMode::Delete)
                .busy_timeout(BUSY_TIMEOUT)
        });
    options
}

async fn connect(path: &Path) -> Result<DatabaseConnection, AppError> {
    Database::connect(connection_options(path))
        .await
        .map_err(|error| {
            AppError::new(
                "DATABASE_UNAVAILABLE",
                "La base de datos no está disponible.",
                format!("could not open SQLite database: {error}"),
            )
        })
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use sea_orm::{ConnectionTrait, DbBackend, Statement};

    use super::initialize_database;

    fn temporary_database_path(label: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "remolino-pez-{label}-{}-{suffix}",
            std::process::id()
        ));
        (
            directory.clone(),
            directory.join("nested").join("inventory.db"),
        )
    }

    #[tokio::test]
    async fn creates_database_and_applies_initial_migration() {
        let (directory, path) = temporary_database_path("create");

        let database = initialize_database(&path)
            .await
            .expect("database should initialize");
        let file_exists = path.is_file();
        let migration_table_exists = database
            .query_one(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'seaql_migrations'",
            ))
            .await
            .expect("migration query should succeed")
            .is_some();
        let foreign_keys = database
            .query_one(Statement::from_string(
                DbBackend::Sqlite,
                "PRAGMA foreign_keys",
            ))
            .await
            .expect("foreign keys pragma should succeed")
            .expect("foreign keys pragma should return a value");
        let foreign_keys_enabled: i64 = foreign_keys
            .try_get_by_index(0)
            .expect("foreign keys pragma should be an integer");
        let journal_mode = database
            .query_one(Statement::from_string(
                DbBackend::Sqlite,
                "PRAGMA journal_mode",
            ))
            .await
            .expect("journal mode pragma should succeed")
            .expect("journal mode pragma should return a value");
        let journal_mode: String = journal_mode
            .try_get_by_index(0)
            .expect("journal mode pragma should be text");
        database
            .close_by_ref()
            .await
            .expect("database should close");
        drop(database);

        assert!(file_exists);
        assert!(migration_table_exists);
        assert_eq!(foreign_keys_enabled, 1);
        assert_eq!(journal_mode.to_ascii_lowercase(), "delete");
        fs::remove_dir_all(directory).expect("temporary database directory should be removable");
    }

    #[tokio::test]
    async fn reopening_database_is_idempotent() {
        let (directory, path) = temporary_database_path("reopen");

        let first = initialize_database(&path)
            .await
            .expect("first initialization should succeed");
        first.close().await.expect("first database should close");
        let second = initialize_database(&path)
            .await
            .expect("second initialization should succeed");
        let migration_count = second
            .query_one(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT COUNT(*) AS count FROM seaql_migrations",
            ))
            .await
            .expect("migration query should succeed")
            .expect("migration count should exist");
        let count: i64 = migration_count
            .try_get_by_index(0)
            .expect("count should be an integer");
        drop(migration_count);
        second
            .close_by_ref()
            .await
            .expect("second database should close");
        drop(second);

        assert_eq!(count, 1);
        fs::remove_dir_all(directory).expect("temporary database directory should be removable");
    }

    #[tokio::test]
    async fn returns_error_without_replacing_an_invalid_database_path() {
        let (directory, path) = temporary_database_path("invalid");
        fs::create_dir_all(&path)
            .expect("invalid database path should be creatable as a directory");

        let result = initialize_database(&path).await;

        assert!(result.is_err());
        assert!(path.is_dir());
        fs::remove_dir_all(directory).expect("temporary database directory should be removable");
    }
}
