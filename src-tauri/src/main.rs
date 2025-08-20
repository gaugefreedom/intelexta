#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

mod api;
mod store; // Create a new module for database logic

use tauri::Manager;

// The state for our database connection pool
pub type DbPool = r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // Get the path to the app's data directory
            let app_data_dir = app.path_resolver()
                .app_data_dir()
                .expect("failed to find app data dir");
            
            // Create the directory if it doesn't exist
            std::fs::create_dir_all(&app_data_dir)?;

            let db_path = app_data_dir.join("intelexta.sqlite");

            // Create the connection manager and the pool
            let manager = r2d2_sqlite::SqliteConnectionManager::file(db_path);
            let pool = r2d2::Pool::new(manager)
                .expect("failed to create db pool");

            // Run the schema migration
            store::migrate_db(&pool.get()?)?;

            // Add the pool to Tauri's managed state
            app.manage(pool);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            api::create_project,
            api::list_projects
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}