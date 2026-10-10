//! Files a Lore into the library as `<id>/LORE.md`.
//!
//! An AI's output arrives under whatever name the user saved it with
//! (`LORE (3).md`). Filing it under its own `id` gives every Lore the same
//! place and name, which is what the folder scan looks for. Nothing is ever
//! overwritten or deleted: the source is copied, identical content is not
//! stored twice, and a different Lore with the same `id` is kept beside the
//! first one as `<id>-2`, `<id>-3`, ...

use crate::lore;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    /// Copied to `<id>/LORE.md`.
    Imported,
    /// The library already holds a file with exactly this content.
    Duplicate,
    /// The file is already in the library, at its own `<id>/LORE.md`.
    Already,
    /// Same `id`, different content: kept as a further version, `<id>-N`.
    NewVersion,
    Failed,
}

#[derive(Debug, Serialize)]
pub struct ImportResult {
    pub source: String,
    pub id: String,
    pub outcome: Outcome,
    /// Where the Lore is now (the new file, or the existing one for a duplicate).
    pub dest: Option<String>,
    pub message: String,
}

/// Turns an `id` into a safe single directory name: letters, digits, `-`, `_`
/// and `.` are kept, everything else (including path separators) becomes `-`.
pub fn dir_name_for(id: &str) -> String {
    let cleaned: String = id
        .chars()
        .map(|c| if c.is_alphanumeric() || matches!(c, '-' | '_' | '.') { c } else { '-' })
        .collect();
    let cleaned = cleaned.trim_matches(|c| c == '.' || c == '-').to_string();
    let cleaned: String = cleaned.chars().take(100).collect();
    if cleaned.is_empty() {
        "lore".to_string()
    } else {
        cleaned
    }
}

fn fail(src: &Path, id: &str, message: impl Into<String>) -> ImportResult {
    ImportResult {
        source: src.to_string_lossy().into_owned(),
        id: id.to_string(),
        outcome: Outcome::Failed,
        dest: None,
        message: message.into(),
    }
}

/// An existing `LORE.md` in the library whose bytes equal `content`.
fn find_identical(library: &Path, content: &[u8]) -> Option<PathBuf> {
    WalkDir::new(library)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file() && e.file_name() == "LORE.md")
        .map(|e| e.into_path())
        .find(|p| fs::metadata(p).map_or(false, |m| m.len() == content.len() as u64) && fs::read(p).map_or(false, |b| b == content))
}

pub fn import_file(library: &Path, src: &Path) -> ImportResult {
    let src_str = src.to_string_lossy().into_owned();
    if src.extension().map_or(true, |e| e != "md") {
        return fail(src, "", "Markdown(.md)のファイルではありません");
    }
    let content = match fs::read(src) {
        Ok(c) => c,
        Err(e) => return fail(src, "", format!("読み込めません: {e}")),
    };
    let item = lore::inspect(src);
    if !item.readable {
        return fail(src, &item.id, item.error.unwrap_or_else(|| "読み込めません".into()));
    }
    let id = item.id;
    let name = dir_name_for(&id);

    // Already filed in the library under its own name.
    if let (Ok(lib), Ok(file)) = (library.canonicalize(), src.canonicalize()) {
        if file.starts_with(&lib) && src.file_name().is_some_and(|n| n == "LORE.md") {
            return ImportResult {
                source: src_str,
                id,
                outcome: Outcome::Already,
                dest: Some(file.to_string_lossy().into_owned()),
                message: "すでにライブラリの中にあります".into(),
            };
        }
    }

    if let Some(existing) = find_identical(library, &content) {
        return ImportResult {
            source: src_str,
            id,
            outcome: Outcome::Duplicate,
            dest: Some(existing.to_string_lossy().into_owned()),
            message: "まったく同じ内容が、すでにライブラリにあるため、取り込みませんでした".into(),
        };
    }

    // The first free `<id>`, `<id>-2`, `<id>-3`, ...: nothing existing is touched.
    let mut dir = library.join(&name);
    let mut version = 1;
    while dir.join("LORE.md").exists() {
        version += 1;
        dir = library.join(format!("{name}-{version}"));
    }
    if let Err(e) = fs::create_dir_all(&dir) {
        return fail(src, &id, format!("フォルダを作れません: {e}"));
    }
    let dest = dir.join("LORE.md");
    if let Err(e) = fs::write(&dest, &content) {
        return fail(src, &id, format!("保存できません: {e}"));
    }
    let (outcome, message) = if version == 1 {
        (Outcome::Imported, format!("{name}/LORE.md に取り込みました"))
    } else {
        (
            Outcome::NewVersion,
            format!("同じIDで内容が違うため、別の版として {name}-{version}/LORE.md に取り込みました"),
        )
    };
    ImportResult {
        source: src_str,
        id,
        outcome,
        dest: Some(dest.to_string_lossy().into_owned()),
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lore_text(id: &str, topic: &str) -> String {
        format!(
            "---\nlorespec: \"0.1\"\nid: \"{id}\"\ndate: \"2026-10-11\"\nsource: \"claude\"\ntopic: \"{topic}\"\ntags: [a]\nclassification:\n  type: strategy\n  domains: [d]\n  value: high\ntrails: [x]\n---\n\n## Session Arc\n\n### Started\n\nx\n\n### Ended\n\ny\n\n## Connections\n"
        )
    }

    struct Dirs {
        root: PathBuf,
        library: PathBuf,
        inbox: PathBuf,
    }

    fn dirs(name: &str) -> Dirs {
        let root = std::env::temp_dir().join(format!("loreshelf-test-import-{name}"));
        fs::remove_dir_all(&root).ok();
        let library = root.join("library");
        let inbox = root.join("inbox");
        fs::create_dir_all(&library).unwrap();
        fs::create_dir_all(&inbox).unwrap();
        Dirs { root, library, inbox }
    }

    fn save(inbox: &Path, file: &str, text: &str) -> PathBuf {
        let p = inbox.join(file);
        fs::write(&p, text).unwrap();
        p
    }

    #[test]
    fn files_a_lore_under_its_id_whatever_the_file_was_called() {
        let d = dirs("new");
        let src = save(&d.inbox, "LORE (3).md", &lore_text("session-a", "t"));
        let r = import_file(&d.library, &src);
        assert_eq!(r.outcome, Outcome::Imported, "{}", r.message);
        let dest = d.library.join("session-a").join("LORE.md");
        assert_eq!(fs::read(&dest).unwrap(), fs::read(&src).unwrap());
        assert!(src.exists(), "the source is copied, never moved");
        fs::remove_dir_all(&d.root).ok();
    }

    #[test]
    fn identical_content_is_not_stored_twice_even_under_another_name() {
        let d = dirs("dup");
        let text = lore_text("session-a", "t");
        let first = save(&d.inbox, "one.md", &text);
        let second = save(&d.inbox, "LORE (4).md", &text);
        assert_eq!(import_file(&d.library, &first).outcome, Outcome::Imported);
        let r = import_file(&d.library, &second);
        assert_eq!(r.outcome, Outcome::Duplicate);
        let existing = r.dest.expect("a duplicate points at the copy that already exists");
        assert!(existing.ends_with("session-a/LORE.md") || existing.ends_with("session-a\\LORE.md"), "{existing}");
        assert_eq!(fs::read_dir(&d.library).unwrap().count(), 1);
        fs::remove_dir_all(&d.root).ok();
    }

    #[test]
    fn same_id_with_different_content_is_kept_as_a_further_version() {
        let d = dirs("version");
        let a = save(&d.inbox, "a.md", &lore_text("session-a", "first"));
        let b = save(&d.inbox, "b.md", &lore_text("session-a", "second"));
        let c = save(&d.inbox, "c.md", &lore_text("session-a", "third"));
        assert_eq!(import_file(&d.library, &a).outcome, Outcome::Imported);
        let rb = import_file(&d.library, &b);
        assert_eq!(rb.outcome, Outcome::NewVersion);
        assert!(d.library.join("session-a-2").join("LORE.md").exists());
        assert_eq!(import_file(&d.library, &c).outcome, Outcome::NewVersion);
        assert!(d.library.join("session-a-3").join("LORE.md").exists());
        // The first one is untouched.
        assert!(fs::read_to_string(d.library.join("session-a").join("LORE.md")).unwrap().contains("first"));
        fs::remove_dir_all(&d.root).ok();
    }

    #[test]
    fn a_file_already_in_the_library_is_left_alone() {
        let d = dirs("already");
        let src = save(&d.inbox, "a.md", &lore_text("session-a", "t"));
        import_file(&d.library, &src);
        let inside = d.library.join("session-a").join("LORE.md");
        let r = import_file(&d.library, &inside);
        assert_eq!(r.outcome, Outcome::Already);
        assert_eq!(fs::read_dir(&d.library).unwrap().count(), 1);
        fs::remove_dir_all(&d.root).ok();
    }

    #[test]
    fn an_id_cannot_escape_the_library() {
        for hostile in ["../../outside", "a/b", "..", "/etc/passwd", "a\\b", ""] {
            let name = dir_name_for(hostile);
            assert!(!name.contains('/') && !name.contains('\\'), "{hostile} -> {name}");
            assert_ne!(name, "..");
            assert!(!name.is_empty());
        }
        let d = dirs("escape");
        let src = save(&d.inbox, "x.md", &lore_text("../../escaped", "t"));
        let r = import_file(&d.library, &src);
        assert_eq!(r.outcome, Outcome::Imported, "{}", r.message);
        let dest = PathBuf::from(r.dest.unwrap());
        assert!(dest.canonicalize().unwrap().starts_with(d.library.canonicalize().unwrap()));
        assert!(!d.root.join("escaped").exists());
        fs::remove_dir_all(&d.root).ok();
    }

    #[test]
    fn files_that_are_not_markdown_or_cannot_be_read_fail_without_writing() {
        let d = dirs("fail");
        let txt = save(&d.inbox, "notes.txt", "x");
        assert_eq!(import_file(&d.library, &txt).outcome, Outcome::Failed);
        let missing = d.inbox.join("missing.md");
        assert_eq!(import_file(&d.library, &missing).outcome, Outcome::Failed);
        assert_eq!(fs::read_dir(&d.library).unwrap().count(), 0);
        fs::remove_dir_all(&d.root).ok();
    }

    #[test]
    fn a_file_without_frontmatter_is_filed_under_its_file_name() {
        let d = dirs("plain");
        let src = save(&d.inbox, "plain-notes.md", "# just notes\n");
        let r = import_file(&d.library, &src);
        assert_eq!(r.outcome, Outcome::Imported, "{}", r.message);
        assert!(d.library.join("plain-notes").join("LORE.md").exists());
        fs::remove_dir_all(&d.root).ok();
    }
}
