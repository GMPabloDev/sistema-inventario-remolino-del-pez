mod database;
mod migration;

use std::sync::Mutex;

use database::initialize_database;
use sea_orm::DatabaseConnection;
use tauri::{Manager, State};

pub struct AppState {
    pub database: Mutex<Option<DatabaseConnection>>,
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!(
        "Hello, {}! You've been greeted from the Rust backend!",
        name
    )
}

#[tauri::command]
async fn test_database_connection(state: State<'_, AppState>) -> Result<(), String> {
    let database = {
        let state_database = state
            .database
            .lock()
            .map_err(|_| "database state lock is poisoned".to_string())?;
        state_database
            .as_ref()
            .cloned()
            .ok_or_else(|| "database is not initialized".to_string())?
    };

    database.ping().await.map_err(|error| error.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState {
            database: Mutex::new(None),
        })
        .setup(|app| {
            let database_path = app
                .path()
                .app_data_dir()
                .map(|directory| directory.join("inventory.db"));

            match database_path {
                Ok(database_path) => {
                    let result = tauri::async_runtime::block_on(initialize_database(database_path));
                    match result {
                        Ok(database) => {
                            let state = app.state::<AppState>();
                            let mut state_database = state
                                .database
                                .lock()
                                .map_err(|_| "database state lock is poisoned")?;
                            *state_database = Some(database);
                        }
                        Err(error) => {
                            eprintln!("database initialization failed: {error}");
                        }
                    }
                }
                Err(error) => {
                    eprintln!("could not resolve application data directory: {error}");
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet, test_database_connection])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
