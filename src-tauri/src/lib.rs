use sea_orm::{Database, DatabaseConnection};
use tauri::State;

pub struct AppState {
    pub database: DatabaseConnection,
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn test_database_connection(state: State<'_, AppState>) -> Result<(), String> {
    state
        .database
        .ping()
        .await
        .map_err(|error| error.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let database = tauri::async_runtime::block_on(async {
        Database::connect("sqlite://inventory.db?mode=rwc")
            .await
            .expect("failed to connect to SQLite")
    });

    tauri::Builder::default()
        .manage(AppState { database })
        .invoke_handler(tauri::generate_handler![greet, test_database_connection])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
