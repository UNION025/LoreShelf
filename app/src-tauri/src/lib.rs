mod lore;
mod validate;

use lore::LoreSummary;
use std::path::PathBuf;

#[tauri::command]
fn scan_library(dir: String) -> Result<Vec<LoreSummary>, String> {
    let dir = PathBuf::from(dir);
    if !dir.is_dir() {
        return Err(format!("フォルダが見つかりません: {}", dir.display()));
    }
    Ok(lore::scan(&dir))
}

#[tauri::command]
fn inspect_files(paths: Vec<String>) -> Vec<LoreSummary> {
    paths.iter().map(|p| lore::inspect(&PathBuf::from(p))).collect()
}

#[tauri::command]
fn read_lore(path: String) -> Result<String, String> {
    lore::read_body(&PathBuf::from(path))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![scan_library, inspect_files, read_lore])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
