mod index;
mod lore;
mod validate;

use index::{Hit, Index, IndexDoc};
use lore::LoreSummary;
use serde::Serialize;
use std::path::PathBuf;
use tauri::Manager;

/// Where the rebuildable index lives: `LORESHELF_DATA_DIR` if set (handy while
/// debugging), otherwise the operating system's application data directory.
fn open_index(app: &tauri::AppHandle) -> Result<(Index, PathBuf), String> {
    let dir = match std::env::var_os("LORESHELF_DATA_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => app.path().app_data_dir().map_err(|e| e.to_string())?,
    };
    std::fs::create_dir_all(&dir).map_err(|e| format!("データフォルダを作れません: {e}"))?;
    let db = dir.join("index.sqlite");
    Ok((Index::open(&db)?, db))
}

/// The document to index for `path`, or `None` when the file cannot be read.
/// How well the file follows LoreSpec does not matter here: full-text search
/// only needs its text.
fn indexable(path: &std::path::Path) -> Option<IndexDoc> {
    let item = lore::inspect(path);
    if !item.readable {
        return None;
    }
    let body = lore::read_body(path).ok()?;
    Some(IndexDoc {
        path: item.path,
        topic: item.topic.unwrap_or_else(|| item.id.clone()),
        id: item.id,
        tags: item.tags.join(" "),
        body,
    })
}

#[derive(Serialize)]
struct IndexReport {
    indexed: usize,
    /// Files that were listed but not indexed because LoreSpec rejects them.
    skipped: usize,
    db_path: String,
}

/// Rebuilds the index from `paths`. Each file is validated again here, so a
/// rejected file cannot reach the index even if the caller passes it.
#[tauri::command]
async fn index_library(app: tauri::AppHandle, paths: Vec<String>) -> Result<IndexReport, String> {
    let docs: Vec<IndexDoc> = paths
        .iter()
        .filter_map(|path| indexable(&PathBuf::from(path)))
        .collect();
    let skipped = paths.len() - docs.len();
    let (mut index, db) = open_index(&app)?;
    let indexed = index.rebuild(&docs)?;
    Ok(IndexReport {
        indexed,
        skipped,
        db_path: db.to_string_lossy().into_owned(),
    })
}

#[tauri::command]
async fn search_lore(app: tauri::AppHandle, query: String) -> Result<Vec<Hit>, String> {
    let (index, _) = open_index(&app)?;
    index.search(&query, 50)
}

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
        .invoke_handler(tauri::generate_handler![scan_library, inspect_files, read_lore, index_library, search_lore])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(rel: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples").join(rel)
    }

    /// The prompts tell the AI to tag each Lore with the version of the prompts
    /// that produced it, so every such tag must name the version stated at the top.
    #[test]
    fn prompts_version_tag_matches_the_stated_version() {
        let doc = include_str!("../../../docs/PROMPTS.md");
        let version = doc
            .split("**版: ")
            .nth(1)
            .and_then(|rest| rest.split("**").next())
            .expect("PROMPTS.md states its version as **版: x.y**");
        let expected = version.replace('.', "-");
        let mut checked = 0;
        for part in doc.split("loreshelf-prompts-v").skip(1) {
            let found: String = part
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '-')
                .collect();
            let found = found.trim_end_matches('-');
            if found.is_empty() {
                continue; // prose that explains the tag, not a tag itself
            }
            assert_eq!(found, expected, "a prompt tag does not match the version {version}");
            checked += 1;
        }
        assert!(checked >= 4, "expected the tag in sections 1, 2, 3 and the history");
    }

    #[test]
    fn every_readable_file_is_indexed_whatever_its_structure() {
        let doc = indexable(&sample("valid/all-object-types.md")).expect("valid sample is indexed");
        assert_eq!(doc.id, "sample-all-object-types");
        assert!(doc.body.contains("synthetic Lore"));

        for name in [
            "translated-qualifier.md",
            "bad-connections.md",
            "bad-enums-and-ids.md",
            "mixed-up-enums.md",
            "no-frontmatter.md",
            "unsupported-version.md",
            "warnings-only.md",
        ] {
            assert!(indexable(&sample(&format!("invalid/{name}"))).is_some(), "{name} should be indexed");
        }
        assert!(indexable(&sample("valid/official-style.md")).is_some());
        // Only a file that cannot be read is left out.
        assert!(indexable(&sample("does-not-exist.md")).is_none());
    }

    #[test]
    fn a_file_with_structural_problems_can_still_be_found() {
        let mut index = Index::open_in_memory().unwrap();
        let docs: Vec<IndexDoc> = ["valid/all-object-types.md", "invalid/bad-connections.md", "does-not-exist.md"]
            .iter()
            .filter_map(|p| indexable(&sample(p)))
            .collect();
        assert_eq!(docs.len(), 2);
        index.rebuild(&docs).unwrap();
        let hits = index.search("synthetic", 10).unwrap();
        assert_eq!(hits.len(), 2);
    }
}
