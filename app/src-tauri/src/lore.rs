use crate::validate::{self, Issue};
use serde::{Deserialize, Serialize};
use std::path::Path;
use walkdir::WalkDir;

/// LORE.md frontmatter. Every field is optional so that a partially
/// valid file still shows up in the library instead of being dropped.
#[derive(Debug, Default, Deserialize)]
struct Frontmatter {
    id: Option<String>,
    date: Option<serde_yaml_ng::Value>,
    source: Option<String>,
    topic: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    classification: Option<Classification>,
    #[serde(default)]
    trails: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
struct Classification {
    #[serde(rename = "type")]
    kind: Option<String>,
    value: Option<String>,
    #[serde(default)]
    domains: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct LoreSummary {
    pub path: String,
    pub id: String,
    pub date: Option<String>,
    pub source: Option<String>,
    pub topic: Option<String>,
    pub tags: Vec<String>,
    pub kind: Option<String>,
    pub value: Option<String>,
    /// The subject areas (shelves) the Lore belongs to.
    pub domains: Vec<String>,
    pub trails: Vec<String>,
    /// Set when the file could not be read or its frontmatter not parsed.
    pub error: Option<String>,
    /// LoreSpec v0.1 findings (errors and warnings). They describe how well
    /// the file follows the specification; they never keep it from loading.
    pub issues: Vec<Issue>,
    /// False only when the file could not be read at all.
    pub readable: bool,
}

/// Returns the YAML block between the leading `---` fences, if any.
fn split_frontmatter(text: &str) -> Option<&str> {
    let text = text.trim_start_matches('\u{feff}').trim_start();
    let rest = text.strip_prefix("---")?.trim_start_matches(['\r', '\n']);
    let end = rest.find("\n---")?;
    Some(&rest[..end])
}

/// Returns the Markdown body of a LORE.md, without the frontmatter.
/// Only Markdown files are readable, so the UI cannot be used to read
/// arbitrary files.
pub fn read_body(path: &Path) -> Result<String, String> {
    if path.extension().map_or(true, |e| e != "md") {
        return Err("Markdown(.md)以外のファイルは読み込めません".into());
    }
    let text = std::fs::read_to_string(path).map_err(|e| format!("読み込めません: {e}"))?;
    let text = text.trim_start_matches('\u{feff}').trim_start();
    let body = text
        .strip_prefix("---")
        .and_then(|rest| rest.find("\n---").map(|i| &rest[i + 4..]))
        .unwrap_or(text);
    Ok(body.trim_start_matches(['\r', '\n']).to_string())
}

fn value_to_string(v: serde_yaml_ng::Value) -> Option<String> {
    match v {
        serde_yaml_ng::Value::String(s) => Some(s),
        serde_yaml_ng::Value::Null => None,
        other => serde_yaml_ng::to_string(&other)
            .ok()
            .map(|s| s.trim().to_string()),
    }
}

/// Checks a single file, whatever its name, as when the user imports it
/// by hand. A file that departs from LoreSpec is still loaded; the departures
/// come back as `issues`.
pub fn inspect(path: &Path) -> LoreSummary {
    summarize(path)
}

fn summarize(path: &Path) -> LoreSummary {
    let path_str = path.to_string_lossy().into_owned();
    // LORE.md files are named after their directory; any other file after itself.
    let name_source = if path.file_name().is_some_and(|n| n == "LORE.md") {
        path.parent().and_then(|p| p.file_name())
    } else {
        path.file_stem()
    };
    let fallback_id = name_source
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path_str.clone());

    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            return LoreSummary {
                path: path_str,
                id: fallback_id,
                date: None,
                source: None,
                topic: None,
                tags: vec![],
                kind: None,
                value: None,
                domains: vec![],
                trails: vec![],
                error: Some(format!("読み込めません: {e}")),
                issues: vec![],
                readable: false,
            }
        }
    };
    let issues = validate::validate(&text);
    // Missing or broken frontmatter is reported in `issues`; the file still
    // loads, with the file name standing in for the missing fields.
    let fm: Frontmatter = split_frontmatter(&text)
        .and_then(|yaml| serde_yaml_ng::from_str(yaml).ok())
        .unwrap_or_default();

    let class = fm.classification.unwrap_or_default();
    LoreSummary {
        path: path_str,
        id: fm.id.unwrap_or(fallback_id),
        date: fm.date.and_then(value_to_string),
        source: fm.source,
        topic: fm.topic,
        tags: fm.tags,
        kind: class.kind,
        value: class.value,
        domains: class.domains,
        trails: fm.trails,
        error: None,
        issues,
        readable: true,
    }
}

/// Recursively finds every `LORE.md` under `dir`.
pub fn scan(dir: &Path) -> Vec<LoreSummary> {
    let mut items: Vec<LoreSummary> = WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file() && e.file_name() == "LORE.md")
        .map(|e| summarize(e.path()))
        .collect();
    // Newest first; entries without a date go last.
    items.sort_by(|a, b| b.date.cmp(&a.date));
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_frontmatter() {
        let text = "---\nid: \"x\"\ndate: \"2026-10-05\"\ntags: [a, b]\nclassification:\n  type: strategy\n---\n\n## Body";
        let yaml = split_frontmatter(text).unwrap();
        let fm: Frontmatter = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(fm.id.as_deref(), Some("x"));
        assert_eq!(fm.tags, vec!["a", "b"]);
        assert_eq!(fm.classification.unwrap().kind.as_deref(), Some("strategy"));
    }

    #[test]
    fn read_body_strips_frontmatter_and_rejects_other_files() {
        let dir = std::env::temp_dir().join("loreshelf-test-read-body");
        std::fs::create_dir_all(&dir).unwrap();
        let lore = dir.join("LORE.md");
        let valid = "---\nlorespec: \"0.1\"\nid: \"s\"\ndate: \"2026-10-05\"\nsource: \"claude\"\ntopic: \"t\"\ntags: [a]\nclassification:\n  type: strategy\n  domains: [d]\n  value: high\ntrails: [x]\n---\n\n## Session Arc\n\n### Started\n\nx\n\n### Ended\n\ny\n\n## Connections\n";
        std::fs::write(&lore, valid).unwrap();
        assert!(read_body(&lore).unwrap().starts_with("## Session Arc"));

        // A hand-imported file need not be called LORE.md, but must be Markdown.
        let renamed = dir.join("my-lore.md");
        std::fs::copy(&lore, &renamed).unwrap();
        assert!(read_body(&renamed).is_ok());
        assert_eq!(inspect(&renamed).id, "s");

        // A spec violation does not keep a file from loading; it is reported.
        std::fs::write(&lore, valid.replace("strategy", "nonsense")).unwrap();
        assert!(read_body(&lore).is_ok());
        let item = inspect(&lore);
        assert!(item.readable);
        assert!(validate::has_errors(&item.issues));

        let other = dir.join("secret.txt");
        std::fs::write(&other, "x").unwrap();
        assert!(read_body(&other).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn valid_samples_load_without_errors() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/valid/all-object-types.md");
        let item = inspect(&path);
        assert!(item.readable, "{:?}", item.error);
        assert_eq!(item.id, "sample-all-object-types");
        assert!(item.issues.is_empty(), "{:?}", item.issues);

        // The official example's writing style is accepted too; its free-text
        // qualifier only draws a warning.
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/valid/official-style.md");
        let item = inspect(&path);
        assert!(item.readable, "{:?}", item.error);
        assert_eq!(item.id, "sample-official-style");
        assert!(item.issues.iter().all(|i| i.severity == validate::Severity::Warning));
    }

    #[test]
    fn scan_only_picks_up_files_named_lore_md() {
        let dir = std::env::temp_dir().join("loreshelf-test-scan");
        let nested = dir.join("a").join("b");
        std::fs::create_dir_all(&nested).unwrap();
        let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/valid/all-object-types.md");
        std::fs::copy(&sample, nested.join("LORE.md")).unwrap();
        std::fs::copy(&sample, dir.join("notes.md")).unwrap();
        let items = scan(&dir);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "sample-all-object-types");
        assert!(items[0].path.ends_with("LORE.md"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn invalid_samples_still_load_and_are_diagnosed_for_the_intended_reason() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/invalid");
        // (file, has errors, a fragment one of the messages must contain)
        let cases = [
            // A free-text qualifier is a warning, as in the official example.
            ("translated-qualifier.md", false, "Qualifier"),
            ("bad-connections.md", true, "causes"),
            ("bad-enums-and-ids.md", true, "Origin"),
            ("mixed-up-enums.md", true, "provisional"),
            ("no-frontmatter.md", true, "フロントマター"),
            ("unsupported-version.md", true, "未対応"),
            ("warnings-only.md", false, "Warrant"),
        ];
        for (name, has_errors, fragment) in cases {
            let item = inspect(&dir.join(name));
            // Every one of them loads; only the diagnosis differs.
            assert!(item.readable, "{name}");
            assert_eq!(validate::has_errors(&item.issues), has_errors, "{name}: {:?}", item.issues);
            assert!(
                item.issues.iter().any(|i| i.message.contains(fragment)),
                "{name}: no issue mentions {fragment}: {:?}",
                item.issues
            );
        }
        let broken = inspect(&dir.join("bad-connections.md"));
        assert!(broken.issues.iter().any(|i| i.message.contains("D99")));
        assert!(broken.issues.iter().any(|i| i.message.contains("書式")));
        // Both mix-ups are reported: a value borrowed from another object type.
        let mixed = inspect(&dir.join("mixed-up-enums.md"));
        assert!(mixed.issues.iter().any(|i| i.message.contains("Status") && i.message.contains("provisional")));
        assert!(mixed.issues.iter().any(|i| i.message.contains("Origin") && i.message.contains("research")));
        let dup = inspect(&dir.join("bad-enums-and-ids.md"));
        assert!(dup.issues.iter().any(|i| i.message.contains("重複")));
        assert!(dup.issues.iter().any(|i| i.message.contains("finalized")));
    }

    /// Dev helper, not part of the normal run:
    /// `LORESHELF_CHECK=path/to/LORE.md cargo test check_file -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn check_file_from_env() {
        let path = std::env::var("LORESHELF_CHECK").expect("set LORESHELF_CHECK to a file path");
        let item = inspect(Path::new(&path));
        println!("readable: {}  has_errors: {}", item.readable, validate::has_errors(&item.issues));
        for issue in &item.issues {
            println!("{:?} line {}: {}", issue.severity, issue.line, issue.message);
        }
        if let Some(e) = &item.error {
            println!("error: {e}");
        }
    }

    #[test]
    fn an_unreadable_file_is_reported_not_hidden() {
        let item = inspect(Path::new("/no/such/dir/LORE.md"));
        assert!(!item.readable);
        assert!(item.error.is_some());
    }

    #[test]
    fn a_file_without_frontmatter_still_gets_a_title_from_its_name() {
        let dir = std::env::temp_dir().join("loreshelf-test-nofm");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("plain-notes.md");
        std::fs::write(&f, "# just notes\n\nno frontmatter at all").unwrap();
        let item = inspect(&f);
        assert!(item.readable);
        assert_eq!(item.id, "plain-notes");
        assert!(validate::has_errors(&item.issues));
        assert!(read_body(&f).unwrap().contains("just notes"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn domains_are_read_from_the_classification() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/valid/all-object-types.md");
        assert_eq!(inspect(&path).domains, vec!["testing"]);
    }

    #[test]
    fn a_leading_blank_line_does_not_hide_the_frontmatter() {
        let dir = std::env::temp_dir().join("loreshelf-test-blank-first");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("whatever-name.md");
        let text = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/valid/all-object-types.md"),
        )
        .unwrap();
        std::fs::write(&f, format!("\n{text}")).unwrap();
        let item = inspect(&f);
        assert_eq!(item.id, "sample-all-object-types", "the id is read from the frontmatter, not the file name");
        assert!(!validate::has_errors(&item.issues), "{:?}", item.issues);
        assert!(read_body(&f).unwrap().starts_with("## Session Arc"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rejects_missing_frontmatter() {
        assert!(split_frontmatter("# no frontmatter").is_none());
    }
}
