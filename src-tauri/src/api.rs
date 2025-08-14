use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Project { pub id: String, pub name: String }

#[tauri::command]
pub fn create_project(name: String) -> Result<Project, String> {
  Ok(Project { id: uuid(), name })
}

#[tauri::command]
pub fn list_projects() -> Result<Vec<Project>, String> {
  Ok(vec![])
}

fn uuid() -> String { format!("{:x}", md5::compute(rand::random::<u128>().to_be_bytes())) }