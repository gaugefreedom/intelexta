// In src-tauri/src/main.rs
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use tauri::Manager;
// Use our new lib.rs as the entry point for all modules
use intelexta::{api, keychain, runtime, store};

fn main() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
        keychain::initialize_backend();

        runtime::initialize().expect("failed to initialize runtime");

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
    });

    #[cfg(feature = "interactive")]
    let builder = builder.invoke_handler(tauri::generate_handler![
        api::create_project,
        api::rename_project,
        api::delete_project,
        api::list_projects,
        api::list_local_models,
        api::create_run,
        api::rename_run,
        api::delete_run,
        api::list_runs,
        api::list_checkpoints,
        api::get_checkpoint_details,
        api::download_checkpoint_artifact,
        api::open_interactive_checkpoint_session,
        api::list_run_steps,
        api::create_run_step,
        api::update_run_step,
        api::delete_run_step,
        api::reorder_run_steps,
        api::submit_interactive_checkpoint_turn,
        api::finalize_interactive_checkpoint,
        api::start_run,
        api::clone_run,
        api::estimate_run_cost,
        api::get_policy,
        api::update_policy,
        api::replay_run,
        api::emit_car,
        api::export_project,
        api::import_project,
        api::import_car
    ]);

    #[cfg(not(feature = "interactive"))]
    let builder = builder.invoke_handler(tauri::generate_handler![
        api::create_project,
        api::rename_project,
        api::delete_project,
        api::list_projects,
        api::list_local_models,
        api::create_run,
        api::rename_run,
        api::delete_run,
        api::list_runs,
        api::list_checkpoints,
        api::get_checkpoint_details,
        api::download_checkpoint_artifact,
        api::list_run_steps,
        api::create_run_step,
        api::update_run_step,
        api::delete_run_step,
        api::reorder_run_steps,
        api::start_run,
        api::clone_run,
        api::estimate_run_cost,
        api::get_policy,
        api::update_policy,
        api::replay_run,
        api::emit_car,
        api::export_project,
        api::import_project,
        api::import_car
    ]);

    builder
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app_handle, _event| {});
}
