// In src-tauri/src/main.rs
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use tauri::Manager;
// Use our new lib.rs as the entry point for all modules
use intelexta::{api, keychain, store};

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            keychain::initialize_backend();

            let app_data_dir = app
                .path()
                .app_local_data_dir()
                .expect("failed to find app data dir");

            std::fs::create_dir_all(&app_data_dir)?;

            let db_path = app_data_dir.join("intelexta.sqlite");

            let manager = r2d2_sqlite::SqliteConnectionManager::file(db_path);
            let pool = r2d2::Pool::new(manager).expect("failed to create db pool");

            // --- FIX IS HERE ---
            // 1. Get a mutable connection from the pool.
            let mut conn = pool.get()?;
            // 2. Pass a mutable reference to the migrate function.
            store::migrate_db(&mut conn)?;
            // --- END FIX ---

            app.manage(pool);

            Ok(())
        })
        // Add our API commands to the handler
        .invoke_handler(tauri::generate_handler![
            api::create_project,
            api::list_projects,
            api::start_hello_run,
            api::list_runs,
            api::list_checkpoints,
            api::get_policy,
            api::update_policy,
            api::emit_car
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app_handle, _event| {});
}
