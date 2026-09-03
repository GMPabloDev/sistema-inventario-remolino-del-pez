use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE TABLE users (
                    id TEXT PRIMARY KEY NOT NULL,
                    username TEXT NOT NULL UNIQUE COLLATE NOCASE,
                    display_name TEXT NOT NULL,
                    password_hash TEXT NOT NULL,
                    role TEXT NOT NULL CHECK (role IN ('ADMIN', 'WAREHOUSE_MANAGER')),
                    active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
                    must_change_password INTEGER NOT NULL DEFAULT 1 CHECK (must_change_password IN (0, 1)),
                    password_changed_at TEXT NULL,
                    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
                    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
                );

                CREATE INDEX idx_users_role_active ON users (role, active);

                CREATE TRIGGER trg_users_updated_at
                AFTER UPDATE ON users
                FOR EACH ROW
                BEGIN
                    UPDATE users
                    SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                    WHERE id = OLD.id;
                END;

                CREATE TABLE sessions (
                    id TEXT PRIMARY KEY NOT NULL,
                    user_id TEXT NOT NULL,
                    token_hash TEXT NOT NULL UNIQUE,
                    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
                    expires_at TEXT NOT NULL,
                    revoked_at TEXT NULL,
                    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE RESTRICT
                );

                CREATE INDEX idx_sessions_user_id ON sessions (user_id);
                CREATE INDEX idx_sessions_validity ON sessions (token_hash, expires_at, revoked_at);
                "#,
            )
            .await
            .map(|_| ())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                DROP TABLE IF EXISTS sessions;
                DROP TRIGGER IF EXISTS trg_users_updated_at;
                DROP TABLE IF EXISTS users;
                "#,
            )
            .await
            .map(|_| ())
    }
}
