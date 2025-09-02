#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use tauri::Manager;
// Import from our library. The unused `DbPool` has been removed.
use intelexta::{api, store};

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // Tauri v2 API for getting app paths has changed from `path_resolver()`.
            let app_data_dir = app.path()
                .app_local_data_dir()
                .expect("failed to find app data dir");

            std::fs::create_dir_all(&app_data_dir)?;

            let db_path = app_data_dir.join("intelexta.sqlite");

            let manager = r2d2_sqlite::SqliteConnectionManager::file(db_path);
            let pool = r2d2::Pool::new(manager)
                .expect("failed to create db pool");

            store::migrate_db(&pool.get()?)?;

            app.manage(pool);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            api::create_project,
            api::list_projects
        ])
        // The application startup sequence has changed in Tauri v2.
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app_handle, _event| {
            // The .run() method now requires a closure.
        });
}
