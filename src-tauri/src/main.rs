#![allow(clippy::needless_return)]
mod api;
use tauri::Manager;

fn main() {
  tauri::Builder::default()
    .setup(|_app| Ok(()))
    .invoke_handler(tauri::generate_handler![
      api::create_project,
      api::list_projects
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}