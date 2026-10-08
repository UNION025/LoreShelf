//! Rebuildable full-text index over accepted Lore.
//!
//! The index is derived data: it is rebuilt from the Lore files on every
//! sync, so deleting the database file never loses knowledge. Only files
//! that passed LoreSpec validation may be handed to it.

use rusqlite::{params, Connection};
use serde::Serialize;
use std::path::Path;

/// Bump when the table layout changes; an older database is dropped and rebuilt.
const SCHEMA_VERSION: i32 = 1;

/// Markers around matched text in a snippet (control characters never occur
/// in Lore text, so the UI can split on them safely).
pub const MARK_START: char = '\u{1}';
pub const MARK_END: char = '\u{2}';

/// The trigram tokenizer cannot match terms shorter than this many characters.
const MIN_TRIGRAM_CHARS: usize = 3;

pub struct IndexDoc {
    pub path: String,
    pub id: String,
    pub topic: String,
    pub tags: String,
    pub body: String,
}

#[derive(Debug, Serialize)]
pub struct Hit {
    pub path: String,
    pub id: String,
    pub topic: String,
    pub snippet: String,
}

pub struct Index {
    conn: Connection,
}

impl Index {
    pub fn open(db_path: &Path) -> Result<Self, String> {
        let conn = Connection::open(db_path).map_err(|e| format!("インデックスを開けません: {e}"))?;
        Self::init(conn)
    }

    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self, String> {
        Self::init(Connection::open_in_memory().map_err(|e| e.to_string())?)
    }

    fn init(conn: Connection) -> Result<Self, String> {
        let version: i32 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if version != SCHEMA_VERSION {
            conn.execute_batch("DROP TABLE IF EXISTS fts;")
                .map_err(|e| e.to_string())?;
        }
        conn.execute_batch(&format!(
            "CREATE VIRTUAL TABLE IF NOT EXISTS fts USING fts5(
                 path UNINDEXED, id UNINDEXED, topic, tags, body,
                 tokenize = 'trigram'
             );
             PRAGMA user_version = {SCHEMA_VERSION};"
        ))
        .map_err(|e| format!("インデックスを初期化できません: {e}"))?;
        Ok(Self { conn })
    }

    /// Replaces the whole index with `docs`. Returns how many were indexed.
    pub fn rebuild(&mut self, docs: &[IndexDoc]) -> Result<usize, String> {
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM fts", []).map_err(|e| e.to_string())?;
        {
            let mut insert = tx
                .prepare("INSERT INTO fts (path, id, topic, tags, body) VALUES (?1, ?2, ?3, ?4, ?5)")
                .map_err(|e| e.to_string())?;
            for d in docs {
                insert
                    .execute(params![d.path, d.id, d.topic, d.tags, d.body])
                    .map_err(|e| e.to_string())?;
            }
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(docs.len())
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<Hit>, String> {
        let terms: Vec<&str> = query.split_whitespace().collect();
        if terms.is_empty() {
            return Ok(vec![]);
        }
        if terms.iter().any(|t| t.chars().count() < MIN_TRIGRAM_CHARS) {
            return self.search_substring(&terms, limit);
        }

        // Each term becomes a quoted phrase, so FTS operators in the user's
        // text (AND, NEAR, -, ...) are taken literally.
        let match_expr = terms
            .iter()
            .map(|t| format!("\"{}\"", t.replace('"', "\"\"")))
            .collect::<Vec<_>>()
            .join(" ");
        let mut stmt = self
            .conn
            .prepare(&format!(
                "SELECT path, id, topic,
                        snippet(fts, 4, '{MARK_START}', '{MARK_END}', '…', 16)
                 FROM fts WHERE fts MATCH ?1 ORDER BY rank LIMIT ?2"
            ))
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![match_expr, limit as i64], |r| {
                Ok(Hit {
                    path: r.get(0)?,
                    id: r.get(1)?,
                    topic: r.get(2)?,
                    snippet: r.get::<_, String>(3)?.replace('\n', " "),
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    /// Fallback for terms the trigram index cannot match (under three
    /// characters, for example a two-character Japanese word).
    fn search_substring(&self, terms: &[&str], limit: usize) -> Result<Vec<Hit>, String> {
        let clause = (0..terms.len())
            .map(|i| {
                let n = i + 1;
                format!("(topic LIKE ?{n} ESCAPE '\\' OR tags LIKE ?{n} ESCAPE '\\' OR body LIKE ?{n} ESCAPE '\\')")
            })
            .collect::<Vec<_>>()
            .join(" AND ");
        let patterns: Vec<String> = terms.iter().map(|t| format!("%{}%", escape_like(t))).collect();
        let mut stmt = self
            .conn
            .prepare(&format!("SELECT path, id, topic, body FROM fts WHERE {clause} LIMIT {limit}"))
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(patterns.iter()), |r| {
                let body: String = r.get(3)?;
                Ok(Hit {
                    path: r.get(0)?,
                    id: r.get(1)?,
                    topic: r.get(2)?,
                    snippet: snippet_around(&body, terms),
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }
}

fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

fn fold(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

/// First index at which `needle` occurs in `hay`, ignoring case.
fn find_ci(hay: &[char], needle: &[char], from: usize) -> Option<usize> {
    if needle.is_empty() || hay.len() < needle.len() {
        return None;
    }
    (from..=hay.len() - needle.len())
        .find(|&i| hay[i..i + needle.len()].iter().zip(needle).all(|(a, b)| fold(*a) == fold(*b)))
}

/// A short excerpt around the first match, with every term marked.
fn snippet_around(body: &str, terms: &[&str]) -> String {
    let hay: Vec<char> = body.chars().collect();
    let needles: Vec<Vec<char>> = terms.iter().map(|t| t.chars().collect()).collect();
    let first = needles.iter().filter_map(|n| find_ci(&hay, n, 0)).min();
    let (start, end) = match first {
        Some(p) => (p.saturating_sub(30), (p + 90).min(hay.len())),
        None => (0, 90.min(hay.len())),
    };

    let mut marks = vec![0i8; hay.len() + 1]; // +1 = start, -1 = end
    for n in &needles {
        let mut from = start;
        while let Some(i) = find_ci(&hay[..end], n, from) {
            marks[i] += 1;
            marks[i + n.len()] -= 1;
            from = i + n.len();
        }
    }

    let mut out = String::new();
    if start > 0 {
        out.push('…');
    }
    let mut open = 0i32;
    for i in start..end {
        let delta = i32::from(marks[i]);
        if delta < 0 && open > 0 {
            open += delta;
            if open == 0 {
                out.push(MARK_END);
            }
        }
        if delta > 0 {
            if open == 0 {
                out.push(MARK_START);
            }
            open += delta;
        }
        out.push(if hay[i] == '\n' { ' ' } else { hay[i] });
    }
    if open > 0 {
        out.push(MARK_END);
    }
    if end < hay.len() {
        out.push('…');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(path: &str, topic: &str, body: &str) -> IndexDoc {
        IndexDoc {
            path: path.into(),
            id: path.into(),
            topic: topic.into(),
            tags: String::new(),
            body: body.into(),
        }
    }

    fn index() -> Index {
        let mut idx = Index::open_in_memory().unwrap();
        idx.rebuild(&[
            doc("a", "Local-first design", "The database is a rebuildable index of portable files."),
            doc("b", "日本語のメモ", "意味検索と全文検索を組み合わせて、過去の会話を再発見する。"),
            doc("c", "Other", "Nothing relevant here, only 100% plain text_with_underscores."),
        ])
        .unwrap();
        idx
    }

    fn paths(hits: &[Hit]) -> Vec<&str> {
        let mut p: Vec<&str> = hits.iter().map(|h| h.path.as_str()).collect();
        p.sort();
        p
    }

    #[test]
    fn finds_english_substrings_ignoring_case() {
        let idx = index();
        assert_eq!(paths(&idx.search("REBUILDABLE", 10).unwrap()), ["a"]);
        assert_eq!(paths(&idx.search("ortable fil", 10).unwrap()), ["a"]);
    }

    #[test]
    fn finds_japanese_with_three_or_more_characters() {
        let idx = index();
        assert_eq!(paths(&idx.search("全文検索", 10).unwrap()), ["b"]);
        assert_eq!(paths(&idx.search("再発見", 10).unwrap()), ["b"]);
    }

    #[test]
    fn falls_back_for_two_character_terms() {
        let idx = index();
        let hits = idx.search("検索", 10).unwrap();
        assert_eq!(paths(&hits), ["b"]);
        assert!(hits[0].snippet.contains(MARK_START));
    }

    #[test]
    fn all_terms_must_match() {
        let idx = index();
        assert_eq!(paths(&idx.search("database portable", 10).unwrap()), ["a"]);
        assert!(idx.search("database 全文検索", 10).unwrap().is_empty());
    }

    #[test]
    fn fts_operators_and_quotes_are_taken_literally() {
        let idx = index();
        // If AND were an operator this would match document "a"; as a literal
        // word it is absent from every body.
        assert!(idx.search("database AND", 10).unwrap().is_empty());
        assert!(idx.search("database NEAR", 10).unwrap().is_empty());
        assert!(idx.search("\"unbalanced", 10).unwrap().is_empty());
        assert!(idx.search("a*", 10).unwrap().is_empty());
    }

    #[test]
    fn like_wildcards_are_escaped_in_the_fallback() {
        let idx = index();
        // "%" only matches a literal percent sign, not everything.
        assert_eq!(paths(&idx.search("%", 10).unwrap()), ["c"]);
        assert_eq!(paths(&idx.search("0%", 10).unwrap()), ["c"]);
        // An unescaped "_" would match any character, so "x_" would hit "xt".
        assert!(idx.search("x_", 10).unwrap().is_empty());
        assert_eq!(paths(&idx.search("t_", 10).unwrap()), ["c"]);
    }

    #[test]
    fn snippet_marks_the_match() {
        let idx = index();
        let hit = &idx.search("rebuildable", 10).unwrap()[0];
        assert!(hit.snippet.contains(MARK_START) && hit.snippet.contains(MARK_END), "{:?}", hit.snippet);
    }

    #[test]
    fn rebuild_replaces_everything_and_empty_query_returns_nothing() {
        let mut idx = index();
        idx.rebuild(&[doc("z", "New", "completely different words")]).unwrap();
        assert!(idx.search("rebuildable", 10).unwrap().is_empty());
        assert_eq!(paths(&idx.search("different", 10).unwrap()), ["z"]);
        assert!(idx.search("   ", 10).unwrap().is_empty());
    }

    #[test]
    fn a_file_database_survives_reopening_and_resets_on_schema_change() {
        let dir = std::env::temp_dir().join("loreshelf-test-index");
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("index.sqlite");
        std::fs::remove_file(&db).ok();
        {
            let mut idx = Index::open(&db).unwrap();
            idx.rebuild(&[doc("a", "T", "persistent content")]).unwrap();
        }
        assert_eq!(paths(&Index::open(&db).unwrap().search("persistent", 10).unwrap()), ["a"]);
        {
            let c = Connection::open(&db).unwrap();
            c.execute_batch("PRAGMA user_version = 0;").unwrap();
        }
        // An index from another schema version is dropped, not trusted.
        assert!(Index::open(&db).unwrap().search("persistent", 10).unwrap().is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }
}
