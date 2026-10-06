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
    pub trails: Vec<String>,
    /// Set when the file could not be read or its frontmatter not parsed.
    pub error: Option<String>,
    /// LoreSpec v0.1 findings (errors and warnings).
    pub issues: Vec<Issue>,
    /// False when the file violates LoreSpec. Rejected files must not be
    /// opened or handed to the index.
    pub accepted: bool,
}

/// Returns the YAML block between the leading `---` fences, if any.
fn split_frontmatter(text: &str) -> Option<&str> {
    let text = text.trim_start_matches('\u{feff}');
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
    let issues = validate::validate(&text);
    if validate::has_errors(&issues) {
        return Err("LoreSpec に違反しているため読み込みを拒否しました".into());
    }
    let text = text.trim_start_matches('\u{feff}');
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
/// by hand. A file that violates LoreSpec comes back with `accepted: false`.
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

    let failed = |msg: String, issues: Vec<Issue>| LoreSummary {
        path: path_str.clone(),
        id: fallback_id.clone(),
        date: None,
        source: None,
        topic: None,
        tags: vec![],
        kind: None,
        value: None,
        trails: vec![],
        error: Some(msg),
        issues,
        accepted: false,
    };

    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => return failed(format!("読み込めません: {e}"), vec![]),
    };
    let issues = validate::validate(&text);
    let Some(yaml) = split_frontmatter(&text) else {
        return failed("フロントマターが見つかりません".into(), issues);
    };
    let fm: Frontmatter = match serde_yaml_ng::from_str(yaml) {
        Ok(fm) => fm,
        Err(e) => return failed(format!("フロントマターを解析できません: {e}"), issues),
    };

    let class = fm.classification.unwrap_or_default();
    let accepted = !validate::has_errors(&issues);
    LoreSummary {
        path: path_str,
        id: fm.id.unwrap_or(fallback_id),
        date: fm.date.and_then(value_to_string),
        source: fm.source,
        topic: fm.topic,
        tags: fm.tags,
        kind: class.kind,
        value: class.value,
        trails: fm.trails,
        error: None,
        issues,
        accepted,
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

        // A spec violation is refused, not displayed.
        std::fs::write(&lore, valid.replace("strategy", "nonsense")).unwrap();
        assert!(read_body(&lore).is_err());
        assert!(!inspect(&lore).accepted);

        let other = dir.join("secret.txt");
        std::fs::write(&other, "x").unwrap();
        assert!(read_body(&other).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn valid_sample_is_accepted() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/valid/all-object-types.md");
        let item = inspect(&path);
        assert!(item.accepted, "{:?} {:?}", item.issues, item.error);
        assert_eq!(item.id, "sample-all-object-types");
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
    fn invalid_samples_are_rejected_for_the_intended_reason() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/invalid");
        // (file, should be accepted, a fragment every run's messages must contain)
        let cases = [
            ("translated-qualifier.md", false, "Qualifier"),
            ("bad-connections.md", false, "causes"),
            ("bad-enums-and-ids.md", false, "Origin"),
            ("no-frontmatter.md", false, "フロントマター"),
            ("unsupported-version.md", false, "未対応"),
            ("warnings-only.md", true, "Warrant"),
        ];
        for (name, accepted, fragment) in cases {
            let item = inspect(&dir.join(name));
            assert_eq!(item.accepted, accepted, "{name}: {:?}", item.issues);
            assert!(
                item.issues.iter().any(|i| i.message.contains(fragment)),
                "{name}: no issue mentions {fragment}: {:?}",
                item.issues
            );
        }
        let broken = inspect(&dir.join("bad-connections.md"));
        assert!(broken.issues.iter().any(|i| i.message.contains("D99")));
        assert!(broken.issues.iter().any(|i| i.message.contains("書式")));
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
        println!("accepted: {}", item.accepted);
        for issue in &item.issues {
            println!("{:?} line {}: {}", issue.severity, issue.line, issue.message);
        }
        if let Some(e) = &item.error {
            println!("error: {e}");
        }
    }

    #[test]
    fn rejects_missing_frontmatter() {
        assert!(split_frontmatter("# no frontmatter").is_none());
    }
}
