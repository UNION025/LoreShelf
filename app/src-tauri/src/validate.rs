//! LoreSpec v0.1 syntax checks for a single LORE.md.
//!
//! Errors are violations of the spec that would break a program reading the
//! file (unknown enum value, unknown connection type, dangling link). A file
//! with any error is rejected before it reaches the index.
//! Warnings are deviations that do not prevent reading (unknown section,
//! missing recommended field).

use serde::Serialize;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Serialize)]
pub struct Issue {
    pub line: usize,
    pub severity: Severity,
    pub message: String,
}

const SOURCES: &[&str] = &["claude", "chatgpt", "gemini", "other"];
const SESSION_TYPES: &[&str] = &[
    "strategy",
    "technical",
    "research",
    "drafting",
    "operational",
    "reflective",
];
const VALUES: &[&str] = &["high", "medium", "low", "skip"];
const CONNECTION_TYPES: &[&str] = &[
    "led_to",
    "informed_by",
    "supersedes",
    "contradicts",
    "related_to",
    "depends_on",
    "instance_of",
];
const ARC_PARTS: &[&str] = &["Started", "Pivots", "Ended"];

/// Section heading -> object ID prefix (None for sections without objects).
const SECTIONS: &[(&str, Option<char>)] = &[
    ("Session Arc", None),
    ("Artifacts", Some('A')),
    ("Decisions", Some('D')),
    ("Insights", Some('I')),
    ("Patterns", Some('P')),
    ("Open Questions", Some('Q')),
    ("References", Some('R')),
    ("Next Steps", Some('N')),
    ("Solutions", Some('S')),
    ("Connections", None),
    ("Trail Updates", None),
];

fn enum_values(kind: char, label: &str) -> Option<&'static [&'static str]> {
    Some(match (kind, label) {
        ('A', "Type") => &["doc", "spec", "code", "plan", "framework", "template", "analysis"],
        ('A', "Status") => &["draft", "final", "abandoned"],
        ('D', "Qualifier") => &["always", "usually", "in this case", "tentatively"],
        ('D', "Status") => &["settled", "provisional", "revisited"],
        ('I', "Source") => &["research", "discussion", "discovery", "analysis"],
        ('I', "Confidence") => &["established", "likely", "speculative"],
        ('P', "Scope") => &["universal", "local"],
        ('P', "Origin") => &["invented", "referenced", "evolved"],
        ('R', "Type") => &["tool", "company", "person", "article", "repo", "concept", "framework"],
        ('N', "Urgency") => &["now", "soon", "someday"],
        _ => return None,
    })
}

fn required_fields(kind: char) -> &'static [&'static str] {
    match kind {
        'A' => &["Title", "Type", "Status", "Summary"],
        'D' => &["Decision", "Issue", "Positions", "Arguments", "Warrant", "Qualifier", "Status"],
        'I' => &["Insight", "Source", "Domain", "Confidence"],
        'P' => &["Name", "Description", "Steps or components", "Scope", "Origin"],
        'Q' => &["Question"],
        'R' => &["Name", "Type", "Relevance"],
        'N' => &["Action", "Urgency"],
        'S' => &["Problem", "Fix"],
        _ => &[],
    }
}

fn is_id(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_uppercase())
        && s.len() > 1
        && chars.all(|c| c.is_ascii_digit())
}

/// A reference into another session, e.g. `session-x#D3`.
fn is_external(s: &str) -> bool {
    s.contains('#') || s.contains(':') || s.contains('/')
}

fn parse_field(line: &str) -> Option<(&str, &str)> {
    let rest = line.strip_prefix("- **")?;
    let end = rest.find(":**")?;
    Some((&rest[..end], rest[end + 3..].trim()))
}

fn parse_connection(line: &str) -> Option<(&str, &str, &str)> {
    let rest = line.strip_prefix("- ")?;
    let (from, rest) = rest.split_once(" —[")?;
    let (kind, to) = rest.split_once("]→ ")?;
    Some((from.trim(), kind.trim(), to.trim()))
}

struct Object {
    id: String,
    kind: char,
    line: usize,
    fields: HashSet<String>,
}

struct Checker {
    issues: Vec<Issue>,
}

impl Checker {
    fn push(&mut self, line: usize, severity: Severity, message: String) {
        self.issues.push(Issue { line, severity, message });
    }
    fn error(&mut self, line: usize, message: String) {
        self.push(line, Severity::Error, message);
    }
    fn warn(&mut self, line: usize, message: String) {
        self.push(line, Severity::Warning, message);
    }

    fn finish_object(&mut self, obj: Option<Object>) {
        let Some(obj) = obj else { return };
        for label in required_fields(obj.kind) {
            if !obj.fields.contains(*label) {
                self.warn(obj.line, format!("{}: 項目 {label} がありません", obj.id));
            }
        }
    }

    fn check_enum_value(&mut self, line: usize, field: &str, value: &str, allowed: &[&str]) {
        if !allowed.contains(&value) {
            self.error(
                line,
                format!("{field} の値 \"{value}\" は仕様にありません(使える値: {})", allowed.join(" | ")),
            );
        }
    }
}

fn check_frontmatter(c: &mut Checker, yaml: &str, base_line: usize) {
    let value: serde_yaml_ng::Value = match serde_yaml_ng::from_str(yaml) {
        Ok(v) => v,
        Err(e) => {
            c.error(base_line, format!("フロントマターを解析できません: {e}"));
            return;
        }
    };
    let get_str = |v: &serde_yaml_ng::Value, key: &str| v.get(key).and_then(|x| x.as_str()).map(String::from);

    for key in ["lorespec", "id", "date", "source", "topic"] {
        if get_str(&value, key).is_none() {
            c.error(base_line, format!("フロントマターに {key} がありません"));
        }
    }
    if let Some(v) = get_str(&value, "lorespec") {
        if v != "0.1" {
            // The checks below encode v0.1 only; other versions cannot be judged.
            c.error(base_line, format!("lorespec \"{v}\" は未対応です(対応: 0.1)"));
        }
    }
    if let Some(v) = get_str(&value, "source") {
        c.check_enum_value(base_line, "source", &v, SOURCES);
    }
    for key in ["tags", "trails"] {
        if value.get(key).and_then(|x| x.as_sequence()).is_none() {
            c.warn(base_line, format!("フロントマターに {key}(リスト)がありません"));
        }
    }

    match value.get("classification") {
        None => c.error(base_line, "フロントマターに classification がありません".into()),
        Some(class) => {
            match get_str(class, "type") {
                None => c.error(base_line, "classification に type がありません".into()),
                Some(v) => c.check_enum_value(base_line, "classification.type", &v, SESSION_TYPES),
            }
            if let Some(v) = get_str(class, "secondary_type") {
                c.check_enum_value(base_line, "classification.secondary_type", &v, SESSION_TYPES);
            }
            match get_str(class, "value") {
                None => c.error(base_line, "classification に value がありません".into()),
                Some(v) => c.check_enum_value(base_line, "classification.value", &v, VALUES),
            }
            if class.get("domains").and_then(|x| x.as_sequence()).is_none() {
                c.warn(base_line, "classification に domains(リスト)がありません".into());
            }
        }
    }
}

pub fn has_errors(issues: &[Issue]) -> bool {
    issues.iter().any(|i| i.severity == Severity::Error)
}

pub fn validate(text: &str) -> Vec<Issue> {
    let text = text.trim_start_matches('\u{feff}');
    let lines: Vec<&str> = text.lines().collect();
    let mut c = Checker { issues: vec![] };

    if lines.first().map(|l| l.trim_end()) != Some("---") {
        c.error(1, "ファイルの先頭に --- で始まるフロントマターがありません".into());
        return c.issues;
    }
    let Some(close) = lines.iter().skip(1).position(|l| l.trim_end() == "---").map(|i| i + 1) else {
        c.error(1, "フロントマターの終わり(---)がありません".into());
        return c.issues;
    };
    check_frontmatter(&mut c, &lines[1..close].join("\n"), 1);

    let mut ids: HashMap<String, usize> = HashMap::new();
    let mut sections_seen: HashSet<&'static str> = HashSet::new();
    let mut arc_seen: HashSet<&str> = HashSet::new();
    let mut section: Option<&'static str> = None;
    let mut kind: Option<char> = None;
    let mut object: Option<Object> = None;
    let mut in_fence = false;
    // (line, id) pairs to resolve once every object ID is known.
    let mut refs: Vec<(usize, String)> = vec![];

    for (idx, raw) in lines.iter().enumerate().skip(close + 1) {
        let line_no = idx + 1;
        let line = raw.trim_end();

        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }

        if let Some(title) = line.strip_prefix("## ") {
            c.finish_object(object.take());
            let title = title.trim();
            match SECTIONS.iter().find(|(name, _)| *name == title) {
                Some((name, k)) => {
                    if !sections_seen.insert(name) {
                        c.warn(line_no, format!("セクション \"{title}\" が重複しています"));
                    }
                    section = Some(name);
                    kind = *k;
                }
                None => {
                    c.warn(line_no, format!("仕様にないセクション \"{title}\" です"));
                    section = None;
                    kind = None;
                }
            }
            continue;
        }

        if let Some(title) = line.strip_prefix("### ") {
            c.finish_object(object.take());
            let title = title.trim();
            if section == Some("Session Arc") {
                match ARC_PARTS.iter().find(|p| **p == title) {
                    Some(p) => {
                        arc_seen.insert(p);
                    }
                    None => c.warn(line_no, format!("Session Arc に仕様にない小見出し \"{title}\" があります")),
                }
            } else if let Some(k) = kind {
                let id = title.split_whitespace().next().unwrap_or("");
                if !is_id(id) {
                    c.error(line_no, format!("オブジェクトIDの形式が不正です: \"{title}\""));
                } else if !id.starts_with(k) {
                    c.error(
                        line_no,
                        format!("ID {id} はこのセクションでは {k} で始まる必要があります"),
                    );
                } else if let Some(first) = ids.insert(id.to_string(), line_no) {
                    c.error(line_no, format!("ID {id} が重複しています(最初の定義は {first} 行目)"));
                }
                object = Some(Object {
                    id: id.to_string(),
                    kind: k,
                    line: line_no,
                    fields: HashSet::new(),
                });
            }
            continue;
        }

        if section == Some("Connections") && line.starts_with("- ") {
            match parse_connection(line) {
                None => c.error(line_no, "Connections の書式が不正です(例: D1 —[led_to]→ A1)".into()),
                Some((from, kind, to)) => {
                    if !CONNECTION_TYPES.contains(&kind) {
                        c.error(
                            line_no,
                            format!("接続の種類 \"{kind}\" は仕様にありません(使える値: {})", CONNECTION_TYPES.join(" | ")),
                        );
                    }
                    for end in [from, to] {
                        if is_id(end) {
                            refs.push((line_no, end.to_string()));
                        } else if !is_external(end) {
                            c.error(line_no, format!("接続の端点 \"{end}\" がIDの形式ではありません"));
                        }
                    }
                }
            }
            continue;
        }

        if let (Some(obj), Some((label, value))) = (object.as_mut(), parse_field(line)) {
            obj.fields.insert(label.to_string());
            if let Some(allowed) = enum_values(obj.kind, label) {
                c.check_enum_value(line_no, label, value, allowed);
            }
            if label == "Links" || label == "Depends on" {
                for token in value.split(',').map(|t| t.trim().trim_end_matches('.')) {
                    if is_id(token) {
                        refs.push((line_no, token.to_string()));
                    }
                }
            }
        }
    }
    c.finish_object(object.take());

    for part in ["Started", "Ended"] {
        if sections_seen.contains("Session Arc") && !arc_seen.contains(part) {
            c.warn(1, format!("Session Arc に {part} がありません"));
        }
    }
    for required in ["Session Arc", "Connections"] {
        if !sections_seen.contains(required) {
            c.warn(1, format!("セクション {required} がありません"));
        }
    }
    for (line, id) in refs {
        if !ids.contains_key(&id) {
            c.error(line, format!("参照先の {id} が、このファイル内に定義されていません"));
        }
    }

    c.issues.sort_by_key(|i| i.line);
    c.issues
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: &str = "---\nlorespec: \"0.1\"\nid: \"s\"\ndate: \"2026-10-05\"\nsource: \"claude\"\ntopic: \"t\"\ntags: [a]\nclassification:\n  type: strategy\n  domains: [d]\n  value: high\ntrails: [x]\n---\n\n## Session Arc\n\n### Started\n\nx\n\n### Ended\n\ny\n\n";

    fn errors(body: &str) -> Vec<String> {
        validate(&format!("{HEADER}{body}"))
            .into_iter()
            .filter(|i| i.severity == Severity::Error)
            .map(|i| i.message)
            .collect()
    }

    const DECISION: &str = "## Decisions\n\n### D1 — t\n\n- **Decision:** x\n- **Issue:** x\n- **Positions:**\n  - a\n- **Arguments:**\n  - a\n- **Warrant:** x\n- **Qualifier:** usually\n- **Status:** settled\n- **Links:** A9\n\n## Connections\n\n- D1 —[led_to]→ D1\n";

    #[test]
    fn accepts_valid_lore_except_dangling_link() {
        let e = errors(DECISION);
        assert_eq!(e.len(), 1, "{e:?}");
        assert!(e[0].contains("A9"));
    }

    #[test]
    fn flags_translated_qualifier() {
        let e = errors(&DECISION.replace("usually", "原則として"));
        assert!(e.iter().any(|m| m.contains("Qualifier")), "{e:?}");
    }

    #[test]
    fn flags_unknown_connection_type_and_bad_format() {
        let body = DECISION.replace("led_to", "causes") + "- D1 -> D1\n";
        let e = errors(&body);
        assert!(e.iter().any(|m| m.contains("causes")), "{e:?}");
        assert!(e.iter().any(|m| m.contains("書式")), "{e:?}");
    }

    #[test]
    fn flags_duplicate_and_misplaced_ids() {
        let body = format!("{DECISION}\n## Insights\n\n### D1 — dup\n");
        let e = errors(&body);
        assert!(e.iter().any(|m| m.contains("D1 はこのセクション")), "{e:?}");
    }

    #[test]
    fn allows_cross_session_references() {
        let body = DECISION.replace("A9", "other-session#D3");
        assert!(errors(&body).is_empty());
    }

    #[test]
    fn ignores_headings_inside_code_fences() {
        let body = format!("{DECISION}\n```\n## Not A Section\n```\n");
        assert!(validate(&format!("{HEADER}{body}")).iter().all(|i| !i.message.contains("Not A Section")));
    }

    #[test]
    fn rejects_unsupported_lorespec_version() {
        let text = format!("{HEADER}{DECISION}").replace("lorespec: \"0.1\"", "lorespec: \"0.2\"");
        assert!(validate(&text).iter().any(|i| i.severity == Severity::Error && i.message.contains("未対応")));
    }

    #[test]
    fn reports_missing_frontmatter() {
        let issues = validate("# no frontmatter");
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, Severity::Error);
    }

    #[test]
    fn valid_sample_with_every_object_type_has_no_issues() {
        let text = include_str!("../../../samples/valid/all-object-types.md");
        let issues = validate(text);
        assert!(issues.is_empty(), "{issues:?}");
    }
}
