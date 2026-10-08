//! LoreSpec v0.1 checks for a single LORE.md.
//!
//! The specification fixes the frontmatter, the section names, the object
//! types, their enumerated values and the connection types, but not the exact
//! Markdown an object is written in. The official example and real model
//! output differ (`### D1 — title`, `**D1: title**`, inline `**I1:** text`),
//! so this check accepts those shapes and rejects only what cannot be read or
//! what contradicts the specification.
//!
//! This is a diagnosis, not a gate: a file is loaded, shown and searched
//! whatever it contains, because people and AI read loose structure without
//! trouble. The findings tell the user how well the file follows the
//! specification, and mark the places that later features which read the
//! structure (filters, per-object search, the link graph) would have to skip.
//!
//! Errors: contradict the specification or cannot be read as structure
//! (unknown enumerated value, unknown connection type, link to an undefined
//! ID, a section whose objects cannot be recognized, ...).
//! Warnings: deviations that cost nothing when reading (unknown section, a
//! Qualifier written as free text, a recommended field left out).

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
const QUALIFIERS: &[&str] = &["always", "usually", "in this case", "tentatively"];
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

/// Section heading -> object kind (None for sections without objects).
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

/// What a section may contain instead of objects, when it has nothing to say.
const NO_CONTENT: &[&str] = &["なし", "none", "n/a", "-", "—", "(none)", "（なし）"];

/// The ID letters accepted in each kind of section. The official example
/// writes Next Steps as `NS1`, and Open Questions are commonly `OQ1`.
fn id_prefixes(kind: char) -> &'static [&'static str] {
    match kind {
        'Q' => &["Q", "OQ"],
        'N' => &["N", "NS"],
        'A' => &["A"],
        'D' => &["D"],
        'I' => &["I"],
        'P' => &["P"],
        'R' => &["R"],
        'S' => &["S"],
        _ => &[],
    }
}

fn enum_values(kind: char, label: &str) -> Option<&'static [&'static str]> {
    Some(match (kind, label) {
        ('A', "Type") => &["doc", "spec", "code", "plan", "framework", "template", "analysis"],
        ('A', "Status") => &["draft", "final", "abandoned"],
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

/// Fields whose absence is worth a warning. Title and name are not listed:
/// the official example puts them in the object's heading.
fn required_fields(kind: char) -> &'static [&'static str] {
    match kind {
        'A' => &["Type", "Status", "Summary"],
        'D' => &["Decision", "Issue", "Warrant", "Qualifier", "Status"],
        'P' => &["Description", "Scope", "Origin"],
        'R' => &["Type", "Relevance"],
        'N' => &["Urgency"],
        _ => &[],
    }
}

/// Splits `D1`, `NS3`, `OQ12` off the start of `s`. The ID must not run on
/// into a longer word, so `B2B` and `D1x` are not IDs.
fn id_prefix(s: &str) -> Option<(&str, &str)> {
    let letters = s.bytes().take_while(u8::is_ascii_uppercase).count();
    if !(1..=3).contains(&letters) {
        return None;
    }
    let digits = s[letters..].bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return None;
    }
    let end = letters + digits;
    if let Some(next) = s[end..].chars().next() {
        if next.is_ascii_alphanumeric() || next == '_' {
            return None;
        }
    }
    Some((&s[..end], &s[end..]))
}

fn is_id(s: &str) -> bool {
    matches!(id_prefix(s), Some((_, rest)) if rest.is_empty())
}

fn id_letters(id: &str) -> &str {
    id.trim_end_matches(|c: char| c.is_ascii_digit())
}

/// Words of `text`, split at spaces and the punctuation Lore uses between
/// references (ASCII and full-width).
fn tokens(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| c.is_whitespace() || ",;、，；()（）[]「」".contains(c))
        .map(|t| t.trim_end_matches(['.', '。', ':']))
        .filter(|t| !t.is_empty())
}

fn id_tokens(text: &str) -> Vec<String> {
    tokens(text).filter(|t| is_id(t)).map(String::from).collect()
}

/// A reference into another session, e.g. `session-x#D3`.
fn has_external_ref(text: &str) -> bool {
    tokens(text).any(|t| t.contains('#'))
}

/// True if `value` starts with one of `allowed` (ignoring case), so that
/// `settled（ただし…）` and `provisional — revisit when …` count as values.
fn starts_with_value(value: &str, allowed: &[&str]) -> bool {
    let v = value.trim().trim_start_matches('`').to_lowercase();
    allowed.iter().any(|a| match v.strip_prefix(*a) {
        None => false,
        Some(rest) => rest
            .chars()
            .next()
            .map_or(true, |c| !(c.is_ascii_alphanumeric() || c == '_' || c == '-')),
    })
}

fn parse_field(line: &str) -> Option<(&str, &str)> {
    let rest = line.strip_prefix("- **")?;
    let end = rest.find(":**")?;
    Some((&rest[..end], rest[end + 3..].trim()))
}

/// Recognizes an object's first line, in the three shapes seen in practice:
/// `### D1 — title`, `**D1: title**`, and inline `**I1:** text`.
/// Returns the ID and whether the object's text follows on the same line.
fn parse_object_line(line: &str) -> Option<(&str, bool)> {
    if let Some(rest) = line.strip_prefix("### ") {
        let (id, _) = id_prefix(rest.trim_start())?;
        return Some((id, false));
    }
    let rest = line.strip_prefix("**")?;
    let (id, after) = id_prefix(rest)?;
    let heading_like = after.starts_with(':')
        || after.starts_with('：')
        || after.starts_with('.')
        || after.starts_with("**");
    // `**I1:** text` and `**R1: Name** — description` both carry the object's
    // text on the first line, so no separate fields are expected.
    let inline = after.starts_with(":**")
        || after.starts_with("：**")
        || after
            .find("**")
            .is_some_and(|close| !after[close + 2..].trim().is_empty());
    heading_like.then_some((id, inline))
}

struct Object {
    id: String,
    kind: char,
    line: usize,
    inline: bool,
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
        if obj.inline {
            return; // the text is on the heading line; there are no fields to expect
        }
        for label in required_fields(obj.kind) {
            if !obj.fields.contains(*label) {
                self.warn(obj.line, format!("{}: 項目 {label} がありません", obj.id));
            }
        }
    }

    /// A section that holds objects but in which none could be recognized
    /// would otherwise pass unnoticed, so it is an error.
    fn finish_section(&mut self, name: &str, kind: Option<char>, line: usize, objects: usize, content: &[String]) {
        if kind.is_none() || objects > 0 || content.is_empty() {
            return;
        }
        let only_placeholder = content
            .iter()
            .all(|l| NO_CONTENT.contains(&l.trim().trim_end_matches(['.', '。']).to_lowercase().as_str()));
        if !only_placeholder {
            self.error(
                line,
                format!("セクション {name} のオブジェクトを認識できません(見出しは `### D1 — タイトル` または `**D1: タイトル**` の形)"),
            );
        }
    }

    /// A value that must be one of `allowed`, possibly followed by a note.
    fn check_value(&mut self, line: usize, field: &str, value: &str, allowed: &[&str]) {
        if !starts_with_value(value, allowed) {
            self.error(
                line,
                format!("{field} の値 \"{value}\" は仕様にありません(使える値: {})", allowed.join(" | ")),
            );
        }
    }

    /// Frontmatter values are compared exactly.
    fn check_exact(&mut self, line: usize, field: &str, value: &str, allowed: &[&str]) {
        if !allowed.contains(&value) {
            self.error(
                line,
                format!("{field} の値 \"{value}\" は仕様にありません(使える値: {})", allowed.join(" | ")),
            );
        }
    }

    /// One `- A —[type]→ B` line. The official example also writes several
    /// targets, chains, several links separated by `;`, and a note in
    /// parentheses; all of those are read. Every end must name an ID.
    fn check_connection(&mut self, line_no: usize, line: &str, refs: &mut Vec<(usize, String)>) {
        let rest = line.strip_prefix("- ").unwrap_or(line);
        let mut segments: Vec<&str> = vec![];
        let mut kinds: Vec<&str> = vec![];
        let mut cur = rest;
        while let Some(i) = cur.find("—[") {
            segments.push(&cur[..i]);
            let after = &cur[i + "—[".len()..];
            let Some(j) = after.find("]→") else {
                self.error(line_no, "Connections の書式が不正です(例: D1 —[led_to]→ A1)".into());
                return;
            };
            kinds.push(after[..j].trim());
            cur = &after[j + "]→".len()..];
        }
        if kinds.is_empty() {
            self.error(line_no, "Connections の書式が不正です(例: D1 —[led_to]→ A1)".into());
            return;
        }
        segments.push(cur);

        for kind in &kinds {
            if !CONNECTION_TYPES.contains(kind) {
                self.error(
                    line_no,
                    format!("接続の種類 \"{kind}\" は仕様にありません(使える値: {})", CONNECTION_TYPES.join(" | ")),
                );
            }
        }
        for segment in segments {
            // A note in parentheses belongs to the link, not to an endpoint.
            let end = segment.find(['(', '（']).unwrap_or(segment.len());
            let region = &segment[..end];
            let ids = id_tokens(region);
            if ids.is_empty() && !has_external_ref(region) {
                self.error(
                    line_no,
                    format!("接続の端点にIDがありません: \"{}\"", region.trim()),
                );
            }
            refs.extend(ids.into_iter().map(|id| (line_no, id)));
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
        c.check_exact(base_line, "source", &v, SOURCES);
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
                Some(v) => c.check_exact(base_line, "classification.type", &v, SESSION_TYPES),
            }
            if let Some(v) = get_str(class, "secondary_type") {
                c.check_exact(base_line, "classification.secondary_type", &v, SESSION_TYPES);
            }
            match get_str(class, "value") {
                None => c.error(base_line, "classification に value がありません".into()),
                Some(v) => c.check_exact(base_line, "classification.value", &v, VALUES),
            }
            if class.get("domains").and_then(|x| x.as_sequence()).is_none() {
                c.warn(base_line, "classification に domains(リスト)がありません".into());
            }
        }
    }
}

#[cfg(test)]
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
    let mut section_line = 0;
    let mut section_objects = 0usize;
    let mut section_content: Vec<String> = vec![];
    let mut object: Option<Object> = None;
    let mut in_fence = false;
    // (line, id) pairs to resolve once every object ID is known.
    let mut refs: Vec<(usize, String)> = vec![];

    for (idx, raw) in lines.iter().enumerate().skip(close + 1) {
        let line_no = idx + 1;
        let line = raw.trim_end();

        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            if kind.is_some() {
                section_content.push(line.trim().to_string());
            }
            continue;
        }
        if in_fence {
            if kind.is_some() && !line.trim().is_empty() {
                section_content.push(line.trim().to_string());
            }
            continue;
        }

        if let Some(title) = line.strip_prefix("## ") {
            c.finish_object(object.take());
            if let Some(name) = section {
                c.finish_section(name, kind, section_line, section_objects, &section_content);
            }
            section_objects = 0;
            section_content.clear();
            section_line = line_no;
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

        if section == Some("Session Arc") {
            if let Some(title) = line.strip_prefix("### ") {
                let title = title.trim();
                match ARC_PARTS.iter().find(|p| **p == title) {
                    Some(p) => {
                        arc_seen.insert(p);
                    }
                    None => c.warn(line_no, format!("Session Arc に仕様にない小見出し \"{title}\" があります")),
                }
            }
            continue;
        }

        if let Some(k) = kind {
            if !line.trim().is_empty() {
                section_content.push(line.trim().to_string());
            }
            if line.starts_with("### ") || line.starts_with("**") {
                match parse_object_line(line) {
                    Some((id, inline)) => {
                        c.finish_object(object.take());
                        let prefixes = id_prefixes(k);
                        if !prefixes.contains(&id_letters(id)) {
                            c.error(
                                line_no,
                                format!("ID {id} はこのセクションでは {} で始まる必要があります", prefixes.join(" か ")),
                            );
                        } else if let Some(first) = ids.insert(id.to_string(), line_no) {
                            c.error(line_no, format!("ID {id} が重複しています(最初の定義は {first} 行目)"));
                        }
                        section_objects += 1;
                        object = Some(Object {
                            id: id.to_string(),
                            kind: k,
                            line: line_no,
                            inline,
                            fields: HashSet::new(),
                        });
                        continue;
                    }
                    None if line.starts_with("### ") => {
                        c.finish_object(object.take());
                        c.error(
                            line_no,
                            format!("オブジェクトIDの形式が不正です: \"{}\"", line.trim_start_matches("### ").trim()),
                        );
                        continue;
                    }
                    None => {}
                }
            }
        }

        if section == Some("Connections") && line.starts_with("- ") {
            c.check_connection(line_no, line, &mut refs);
            continue;
        }

        if let (Some(obj), Some((label, value))) = (object.as_mut(), parse_field(line)) {
            obj.fields.insert(label.to_string());
            if obj.kind == 'D' && label == "Qualifier" {
                // The official example writes a sentence here, so a value
                // outside the four is a warning, not a reason to refuse.
                if !starts_with_value(value, QUALIFIERS) {
                    c.warn(
                        line_no,
                        format!(
                            "Qualifier の値 \"{value}\" は、仕様の値({})で始まっていません",
                            QUALIFIERS.join(" | ")
                        ),
                    );
                }
            } else if let Some(allowed) = enum_values(obj.kind, label) {
                c.check_value(line_no, label, value, allowed);
            }
            if label == "Links" || label == "Depends on" {
                refs.extend(id_tokens(value).into_iter().map(|id| (line_no, id)));
            }
        }
    }
    c.finish_object(object.take());
    if let Some(name) = section {
        c.finish_section(name, kind, section_line, section_objects, &section_content);
    }

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
    let mut reported: HashSet<(usize, String)> = HashSet::new();
    for (line, id) in refs {
        if !ids.contains_key(&id) && reported.insert((line, id.clone())) {
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

    fn issues(body: &str, severity: Severity) -> Vec<String> {
        validate(&format!("{HEADER}{body}"))
            .into_iter()
            .filter(|i| i.severity == severity)
            .map(|i| i.message)
            .collect()
    }
    fn errors(body: &str) -> Vec<String> {
        issues(body, Severity::Error)
    }
    fn warnings(body: &str) -> Vec<String> {
        issues(body, Severity::Warning)
    }

    const DECISION: &str = "## Decisions\n\n### D1 — t\n\n- **Decision:** x\n- **Issue:** x\n- **Positions:**\n  - a\n- **Arguments:**\n  - a\n- **Warrant:** x\n- **Qualifier:** usually\n- **Status:** settled\n- **Links:** A9\n\n## Connections\n\n- D1 —[led_to]→ D1\n";

    /// The shapes used by the official example, written with invented content.
    const OFFICIAL_STYLE: &str = "## Artifacts\n\n**A1: Plan**\n- **Type:** spec\n- **Status:** final\n- **Summary:** s\n- **Links:** informed_by D1; depends_on R1\n\n## Decisions\n\n**D1: Choose X**\n- **Decision:** x\n- **Issue:** i\n- **Positions:** (1) a (2) b\n- **Arguments:** a\n- **Warrant:** Because w.\n- **Qualifier:** Settled for now. Revisit later.\n- **Status:** provisional — revisit when load grows\n- **Links:** led_to A1; informed_by I1\n\n## Insights\n\n**I1:** An insight written as one line.\n\n## Patterns\n\n**P1: Method**\n- **Description:** d\n- **Steps:** (1) a (2) b\n- **Scope:** universal — applies widely\n- **Origin:** Invented during this session\n- **Links:** D1 is an instance_of this pattern\n\n## Open Questions\n\n**OQ1: What next?**\n- **Question:** q\n\n## References\n\n**R1: Tool**\n- **Type:** tool\n- **Relevance:** r\n\n## Next Steps\n\n**NS1:** Do the thing\n- **Why:** w\n- **Urgency:** now\n- **Links:** depends_on D1\n\n## Connections\n\n- D1 —[led_to]→ A1 (the plan follows the decision)\n- I1 —[informed_by]→ D1, A1\n- P1, I1 —[led_to]→ OQ1\n- D1 —[led_to]→ NS1 —[depends_on]→ R1（chain with a note）\n- I1 —[led_to]→ OQ1；R1 —[related_to]→ I1\n";

    #[test]
    fn accepts_valid_lore_except_dangling_link() {
        let e = errors(DECISION);
        assert_eq!(e.len(), 1, "{e:?}");
        assert!(e[0].contains("A9"));
    }

    #[test]
    fn accepts_the_official_example_style() {
        let e = errors(OFFICIAL_STYLE);
        assert!(e.is_empty(), "{e:?}");
    }

    #[test]
    fn a_free_text_qualifier_is_a_warning_not_an_error() {
        let e = errors(OFFICIAL_STYLE);
        assert!(e.is_empty(), "{e:?}");
        let w = warnings(OFFICIAL_STYLE);
        assert!(w.iter().any(|m| m.contains("Qualifier")), "{w:?}");

        let translated = DECISION.replace("usually", "原則として").replace("A9", "D1");
        assert!(errors(&translated).is_empty());
        assert!(warnings(&translated).iter().any(|m| m.contains("Qualifier")));
    }

    #[test]
    fn enumerated_values_may_carry_a_note_and_any_case() {
        let body = DECISION
            .replace("- **Status:** settled", "- **Status:** settled（ただし一部のみ）")
            .replace("A9", "D1");
        assert!(errors(&body).is_empty(), "{:?}", errors(&body));
        let body = OFFICIAL_STYLE.replace("Invented during", "invented, during");
        assert!(errors(&body).is_empty());
    }

    #[test]
    fn values_borrowed_from_another_object_type_are_still_errors() {
        let body = OFFICIAL_STYLE.replace("- **Status:** final", "- **Status:** provisional");
        let e = errors(&body);
        assert!(e.iter().any(|m| m.contains("Status") && m.contains("provisional")), "{e:?}");
        let body = OFFICIAL_STYLE.replace("Invented during", "Research during");
        assert!(errors(&body).iter().any(|m| m.contains("Origin")));
        // A value that merely begins with letters of a valid one is not valid.
        let body = OFFICIAL_STYLE.replace("- **Urgency:** now", "- **Urgency:** nowhere");
        assert!(errors(&body).iter().any(|m| m.contains("Urgency")));
    }

    #[test]
    fn flags_unknown_connection_type_and_bad_format() {
        let body = DECISION.replace("led_to", "causes").replace("A9", "D1") + "- D1 -> D1\n";
        let e = errors(&body);
        assert!(e.iter().any(|m| m.contains("causes")), "{e:?}");
        assert!(e.iter().any(|m| m.contains("書式")), "{e:?}");
    }

    #[test]
    fn an_object_described_on_its_heading_line_needs_no_fields() {
        let body = "## References\n\n**R1: Some tool** — tool。What it is used for.\n\n## Connections\n";
        assert!(errors(body).is_empty());
        assert!(!warnings(body).iter().any(|m| m.contains("R1")), "{:?}", warnings(body));
        // A bare heading with no fields still draws the advisory warning.
        let bare = "## References\n\n**R1: Some tool**\n\n## Connections\n";
        assert!(warnings(bare).iter().any(|m| m.contains("R1")));
    }

    #[test]
    fn a_connection_end_without_an_id_is_an_error() {
        let body = DECISION.replace("A9", "D1") + "- hero_main —[led_to]→ D1\n";
        let e = errors(&body);
        assert!(e.iter().any(|m| m.contains("端点にIDがありません")), "{e:?}");
    }

    #[test]
    fn a_section_whose_objects_cannot_be_recognized_is_an_error() {
        let e = errors("## Decisions\n\nProse that names no object ID.\n\n## Connections\n");
        assert!(e.iter().any(|m| m.contains("オブジェクトを認識できません")), "{e:?}");
        // An explicit "nothing here" is fine.
        assert!(errors("## Decisions\n\nなし\n\n## Connections\n").is_empty());
    }

    #[test]
    fn flags_duplicate_and_misplaced_ids() {
        let body = format!("{}\n## Insights\n\n### D1 — dup\n", DECISION.replace("A9", "D1"));
        let e = errors(&body);
        assert!(e.iter().any(|m| m.contains("D1 はこのセクション")), "{e:?}");
        let body = DECISION.replace("A9", "D1") + "\n## Insights\n\n**I1:** a\n\n**I1:** b\n";
        assert!(errors(&body).iter().any(|m| m.contains("重複")));
    }

    #[test]
    fn allows_cross_session_references() {
        let body = DECISION.replace("A9", "other-session#D3");
        assert!(errors(&body).is_empty());
        let body = DECISION.replace("A9", "D1") + "- other-session#D3 —[led_to]→ D1\n";
        assert!(errors(&body).is_empty());
    }

    #[test]
    fn ignores_headings_inside_code_fences() {
        let body = format!("{}\n```\n## Not A Section\n```\n", DECISION.replace("A9", "D1"));
        assert!(validate(&format!("{HEADER}{body}")).iter().all(|i| !i.message.contains("Not A Section")));
    }

    #[test]
    fn id_recognition() {
        for ok in ["D1", "NS12", "OQ3", "A100"] {
            assert!(is_id(ok), "{ok}");
        }
        for bad in ["D", "1D", "B2B", "D1x", "hero_main", "ABCD1", "d1"] {
            assert!(!is_id(bad), "{bad}");
        }
    }

    #[test]
    fn valid_sample_with_every_object_type_has_no_issues() {
        let text = include_str!("../../../samples/valid/all-object-types.md");
        let issues = validate(text);
        assert!(issues.is_empty(), "{issues:?}");
    }

    #[test]
    fn official_style_sample_has_no_errors() {
        let text = include_str!("../../../samples/valid/official-style.md");
        let errs: Vec<_> = validate(text).into_iter().filter(|i| i.severity == Severity::Error).collect();
        assert!(errs.is_empty(), "{errs:?}");
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
}
