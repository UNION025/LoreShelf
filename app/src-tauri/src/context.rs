//! Bundles Lore into text that can be pasted into a conversation with an AI.
//!
//! A shelf is handed to an AI as context, not as training data: the files stay
//! as they are and the text is built on demand. Two forms exist: the whole
//! file, or only the parts that matter for continuing the work (decisions,
//! open questions, next steps), which is shorter.

use crate::import::dir_name_for;
use crate::lore;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Each file as written.
    Full,
    /// Heading data, plus the Decisions, Open Questions and Next Steps sections.
    Digest,
}

impl Mode {
    pub fn parse(s: &str) -> Option<Mode> {
        match s {
            "full" => Some(Mode::Full),
            "digest" => Some(Mode::Digest),
            _ => None,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Bundle {
    pub text: String,
    /// How many files went into the text.
    pub files: usize,
    /// Length of the text in characters.
    pub chars: usize,
    /// Files that were asked for but could not be read.
    pub skipped: usize,
}

const KEPT_SECTIONS: &[&str] = &["Decisions", "Open Questions", "Next Steps"];

const PREFACE: &str = "以下は、過去のAIとの会話から作った要約(LoreSpec 形式の LORE.md)です。\n\
この内容を前提に、続きから相談に乗ってください。\n\
対象にバージョンがある場合(ゲームなど)は、各Loreに書かれたバージョンと、いまのバージョンが\n\
違っていないかを、最初に私に確認してください。違う場合は、古くなっていそうな内容を指摘してください。\n";

/// The tag that records which version of the prompts produced a Lore. It has
/// the same shape as a subject's version tag but is not one.
const PROMPTS_TAG_PREFIX: &str = "loreshelf-prompts-v";

/// Tags that name the AI model that wrote a Lore, such as `model-example-ai-v3`.
/// A model name can end in `-v` and a number too, so these are not versions.
const MODEL_TAG_PREFIX: &str = "model-";

/// A tag that says a subject's version is not known: `alexa-version-unknown`.
const VERSION_UNKNOWN_SUFFIX: &str = "-version-unknown";

/// Inside a model tag: `model-setting-high` records a run setting (such as how
/// hard the model was set to think) rather than a model name.
const SETTING_PREFIX: &str = "setting-";

/// `example-game-v2-8` style tags: the subject, `-v`, then the version number.
fn is_version_tag(tag: &str) -> bool {
    if tag.starts_with(PROMPTS_TAG_PREFIX) || tag.starts_with(MODEL_TAG_PREFIX) {
        return false;
    }
    match tag.rfind("-v") {
        Some(i) if i > 0 => {
            let rest = &tag[i + 2..];
            !rest.is_empty()
                && rest.split('-').all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
        }
        _ => false,
    }
}

/// The text after the frontmatter.
fn body_of(text: &str) -> &str {
    let text = text.trim_start_matches('\u{feff}').trim_start();
    text.strip_prefix("---")
        .and_then(|rest| rest.find("\n---").map(|i| &rest[i + 4..]))
        .unwrap_or(text)
        .trim_start_matches(['\r', '\n'])
}

/// The sections of `body` whose `## ` heading is in `names`, in file order.
/// Headings inside code fences are not headings.
fn sections(body: &str, names: &[&str]) -> String {
    let mut out = String::new();
    let mut keep = false;
    let mut in_fence = false;
    for line in body.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
        } else if !in_fence {
            if let Some(title) = line.strip_prefix("## ") {
                keep = names.contains(&title.trim());
            }
        }
        if keep {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// One Lore turned into text for an AI, with what is needed to name its file.
struct Piece {
    id: String,
    title: String,
    text: String,
}

fn piece(path: &Path, mode: Mode) -> Option<Piece> {
    if path.extension().map_or(true, |e| e != "md") {
        return None;
    }
    let text = std::fs::read_to_string(path).ok()?;
    let item = lore::inspect(path);
    let mut head = format!("### Lore: {}", item.topic.as_deref().unwrap_or(&item.id));
    head.push_str(&format!("\n- id: {}", item.id));
    if let Some(date) = &item.date {
        head.push_str(&format!("\n- 日付: {date}"));
    }
    if !item.domains.is_empty() {
        head.push_str(&format!("\n- 分野: {}", item.domains.join(", ")));
    }
    let model_tags: Vec<&str> = item.tags.iter().filter_map(|t| t.strip_prefix(MODEL_TAG_PREFIX)).collect();
    let models: Vec<&str> = model_tags
        .iter()
        .copied()
        .filter(|m| *m != "unknown" && !m.starts_with(SETTING_PREFIX))
        .collect();
    let settings: Vec<&str> = model_tags.iter().filter_map(|m| m.strip_prefix(SETTING_PREFIX)).collect();
    if !models.is_empty() || !settings.is_empty() {
        let mut line = format!("\n- 作成したAI: {}", models.join(", "));
        if !settings.is_empty() {
            line.push_str(&format!("(設定: {})", settings.join(", ")));
        }
        head.push_str(&line);
    }
    let unknown_versions: Vec<&str> = item
        .tags
        .iter()
        .filter_map(|t| t.strip_suffix(VERSION_UNKNOWN_SUFFIX))
        .filter(|name| !name.is_empty())
        .collect();
    let versions: Vec<&String> = item.tags.iter().filter(|t| is_version_tag(t)).collect();
    if !versions.is_empty() {
        head.push_str(&format!(
            "\n- 対象のバージョン: {}",
            versions.iter().map(|v| v.as_str()).collect::<Vec<_>>().join(", ")
        ));
    }
    if !unknown_versions.is_empty() {
        head.push_str(&format!("\n- バージョンが不明の対象: {}", unknown_versions.join(", ")));
    }
    let body = match mode {
        Mode::Full => text.trim_start_matches('\u{feff}').trim().to_string(),
        Mode::Digest => {
            let kept = sections(body_of(&text), KEPT_SECTIONS);
            if kept.trim().is_empty() {
                "(決定、未解決の問い、次の一手に当たる節は、このLoreにはありません)".to_string()
            } else {
                kept.trim_end().to_string()
            }
        }
    };
    Some(Piece {
        id: item.id.clone(),
        title: item.topic.clone().unwrap_or_else(|| item.id.clone()),
        text: format!("{head}\n\n{body}\n"),
    })
}

/// What a person asks of the AI in this conversation, placed after the preface.
fn task_block(task: &str) -> String {
    let task = task.trim();
    if task.is_empty() {
        String::new()
    } else {
        format!("## 今回やりたいこと\n\n{task}\n\n")
    }
}

#[cfg(test)]
pub fn build(paths: &[impl AsRef<Path>], mode: Mode, with_preface: bool) -> Bundle {
    build_with_task(paths, mode, with_preface, "")
}

pub fn build_with_task(paths: &[impl AsRef<Path>], mode: Mode, with_preface: bool, task: &str) -> Bundle {
    let pieces: Vec<Piece> = paths.iter().filter_map(|p| piece(p.as_ref(), mode)).collect();
    let skipped = paths.len() - pieces.len();
    let mut text = String::new();
    if with_preface {
        text.push_str(PREFACE);
        text.push('\n');
    }
    text.push_str(&task_block(task));
    if with_preface && !pieces.is_empty() {
        // The record of what this conversation inherits, so a Lore made from it can say so.
        let ids: Vec<&str> = pieces.iter().map(|p| p.id.as_str()).collect();
        text.push_str(&inherit_note(&ids));
        text.push('\n');
    }
    let parts: Vec<&str> = pieces.iter().map(|p| p.text.as_str()).collect();
    text.push_str(&parts.join("\n----------\n\n"));
    Bundle {
        chars: text.chars().count(),
        files: pieces.len(),
        skipped,
        text,
    }
}

// ---------------------------------------------------------------------------
// Writing the text out as files, so it can be attached to another chat.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    /// A README, and one file per Lore.
    Separate,
    /// A README, and the Lore cut into numbered parts of a given size.
    Parts,
    /// Everything in one file.
    Single,
}

impl Layout {
    pub fn parse(s: &str) -> Option<Layout> {
        match s {
            "separate" => Some(Layout::Separate),
            "parts" => Some(Layout::Parts),
            "single" => Some(Layout::Single),
            _ => None,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ExportResult {
    /// The new folder everything was written into.
    pub dir: String,
    /// The file names written, in order.
    pub files: Vec<String>,
    /// Total length of what was written, in characters.
    pub chars: usize,
    /// Files that were asked for but could not be read.
    pub skipped: usize,
}

const JOINER: &str = "\n----------\n\n";

/// The record of which Lore a new conversation inherits, with their real ids so
/// that the AI can copy them when it later writes a Lore from that conversation.
fn inherit_note(ids: &[&str]) -> String {
    let example = ids.first().copied().unwrap_or("〔id〕");
    format!(
        "## 引き継ぎの記録\n\n\
この会話は、上のLoreを引き継いでいます。この会話から、あとでLoreを作るときは、Session Arc の冒頭に\
「引き継ぎ元: {ids}」と書き、Connections に、引き継いだオブジェクトへの参照(例: `{example}#D3`)を書いてください。\n",
        ids = ids.join(", ")
    )
}

/// `entries` pairs each Lore with its file name, when it has a file of its own.
fn readme(entries: &[(Option<String>, &Piece)], task: &str, note: Option<&str>) -> String {
    let mut out = String::from("# LoreShelf からの出力\n\n");
    out.push_str(PREFACE);
    out.push('\n');
    out.push_str(&task_block(task));
    if let Some(note) = note {
        out.push_str(note);
        out.push_str("\n\n");
    }
    out.push_str(&format!("## 引き継ぐLore({} 件)\n\n", entries.len()));
    for (file, p) in entries {
        match file {
            Some(file) => out.push_str(&format!("- {file}: {}(id: {})\n", p.title, p.id)),
            None => out.push_str(&format!("- {}(id: {})\n", p.title, p.id)),
        }
    }
    out.push('\n');
    let ids: Vec<&str> = entries.iter().map(|(_, p)| p.id.as_str()).collect();
    out.push_str(&inherit_note(&ids));
    out
}

/// Cuts `text` into chunks of at most `limit` characters, at line boundaries,
/// and where possible just before a heading, so that no chunk starts mid-section.
/// A single line longer than `limit` is cut by characters. Joining the chunks
/// with a newline gives back `text`.
fn split_into_parts(text: &str, limit: usize) -> Vec<String> {
    let limit = limit.max(200);
    let mut chunks: Vec<String> = Vec::new();
    let mut current: Vec<String> = Vec::new();
    let mut current_len = 0usize; // characters, counting one newline between lines
    let mut last_heading: Option<usize> = None; // index in `current` of a heading line

    let is_heading = |l: &str| l.starts_with("## ") || l.starts_with("### Lore:");
    let flush = |lines: &[String]| lines.join("\n");

    // Pieces of text to place: each line, with over-long lines cut up.
    let mut pieces: Vec<String> = Vec::new();
    for line in text.split('\n') {
        let chars: Vec<char> = line.chars().collect();
        if chars.len() > limit {
            for c in chars.chunks(limit) {
                pieces.push(c.iter().collect());
            }
        } else {
            pieces.push(line.to_string());
        }
    }

    for line in pieces {
        let add = line.chars().count() + usize::from(!current.is_empty());
        if !current.is_empty() && current_len + add > limit {
            match last_heading {
                Some(b) if b > 0 => {
                    let tail: Vec<String> = current.split_off(b);
                    chunks.push(flush(&current));
                    current = tail;
                }
                _ => {
                    chunks.push(flush(&current));
                    current = Vec::new();
                }
            }
            current_len = current.iter().map(|l| l.chars().count()).sum::<usize>() + current.len().saturating_sub(1);
            last_heading = current.iter().rposition(|l| is_heading(l));
        }
        let add = line.chars().count() + usize::from(!current.is_empty());
        // The latest heading is the best place to cut if this part fills up.
        if is_heading(&line) {
            last_heading = Some(current.len());
        }
        current.push(line);
        current_len += add;
    }
    if !current.is_empty() {
        chunks.push(flush(&current));
    }
    chunks
}

fn width_for(count: usize) -> usize {
    count.to_string().len().max(2)
}

/// Writes the chosen Lore into a new folder under `dest_root`. Nothing that
/// exists is touched: the folder is `loreshelf-output-001`, or the first
/// number not yet in use.
pub fn export(
    paths: &[impl AsRef<Path>],
    mode: Mode,
    layout: Layout,
    task: &str,
    dest_root: &Path,
    part_limit: usize,
) -> Result<ExportResult, String> {
    if !dest_root.is_dir() {
        return Err(format!("書き出し先のフォルダが見つかりません: {}", dest_root.display()));
    }
    let pieces: Vec<Piece> = paths.iter().filter_map(|p| piece(p.as_ref(), mode)).collect();
    let skipped = paths.len() - pieces.len();
    if pieces.is_empty() {
        return Err("書き出せるLoreがありません(読めないファイルだけが選ばれています)".into());
    }

    let mut out: PathBuf = PathBuf::new();
    for n in 1..=999 {
        let candidate = dest_root.join(format!("loreshelf-output-{n:03}"));
        if std::fs::create_dir(&candidate).is_ok() {
            out = candidate;
            break;
        }
    }
    if out.as_os_str().is_empty() {
        return Err("書き出し用のフォルダを作れません(loreshelf-output-001〜999 が、すべて使われています)".into());
    }

    let mut written: Vec<(String, String)> = Vec::new(); // (file name, content)
    let width = width_for(pieces.len());
    match layout {
        Layout::Separate => {
            let names: Vec<String> = pieces
                .iter()
                .enumerate()
                .map(|(i, p)| format!("{:0width$}-{}.md", i + 1, dir_name_for(&p.id), width = width))
                .collect();
            let entries: Vec<(Option<String>, &Piece)> =
                names.iter().cloned().map(Some).zip(pieces.iter()).collect();
            written.push(("00-README.md".into(), readme(&entries, task, None)));
            for (name, p) in names.iter().zip(pieces.iter()) {
                written.push((name.clone(), p.text.clone()));
            }
        }
        Layout::Single => {
            let entries: Vec<(Option<String>, &Piece)> = pieces.iter().map(|p| (None, p)).collect();
            let mut text = readme(&entries, task, None);
            text.push_str("\n----------\n\n");
            text.push_str(&pieces.iter().map(|p| p.text.as_str()).collect::<Vec<_>>().join(JOINER));
            written.push(("bundle.md".into(), text));
        }
        Layout::Parts => {
            let body = pieces.iter().map(|p| p.text.as_str()).collect::<Vec<_>>().join(JOINER);
            let chunks = split_into_parts(&body, part_limit);
            let n = chunks.len();
            let pw = n.to_string().len(); // part01.. when there are ten or more, so the names sort
            let names: Vec<String> = (1..=n).map(|k| format!("bundle.part{k:0pw$}.md", pw = pw)).collect();
            let entries: Vec<(Option<String>, &Piece)> = pieces.iter().map(|p| (None, p)).collect();
            let note = format!(
                "## 読み込み方\n\nこのフォルダには、{n} 個のパート({}〜{})があります。パート1から順に、すべて読み終えてから、返答してください。",
                names.first().cloned().unwrap_or_default(),
                names.last().cloned().unwrap_or_default()
            );
            written.push(("00-README.md".into(), readme(&entries, task, Some(&note))));
            for (k, (name, chunk)) in names.iter().zip(chunks.iter()).enumerate() {
                let k = k + 1;
                let end = if k < n {
                    "次のパートを続けて貼ります。すべて貼り終えるまで、返答しないでください。"
                } else {
                    "ここまでが、すべてです。"
                };
                written.push((
                    name.clone(),
                    format!("(パート {k}/{n})\n\n{chunk}\n\n(パート {k}/{n} の終わり。{end})\n"),
                ));
            }
        }
    }

    let mut files = Vec::new();
    let mut chars = 0usize;
    for (name, content) in &written {
        std::fs::write(out.join(name), content).map_err(|e| format!("{name} を書き出せません: {e}"))?;
        files.push(name.clone());
        chars += content.chars().count();
    }
    Ok(ExportResult {
        dir: out.to_string_lossy().into_owned(),
        files,
        chars,
        skipped,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn sample(rel: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples").join(rel)
    }

    #[test]
    fn version_tags_are_recognized_only_in_the_agreed_form() {
        for ok in ["example-game-v2-8", "app-v3", "a-v1-2-3"] {
            assert!(is_version_tag(ok), "{ok}");
        }
        for bad in ["example-game", "version-2-8", "game-v", "game-vx", "-v2", "game-v2-", "very-good", "loreshelf-prompts-v0-5", "model-example-ai-v3", "model-setting-v2"] {
            assert!(!is_version_tag(bad), "{bad}");
        }
    }

    #[test]
    fn full_mode_keeps_the_file_and_adds_a_heading() {
        let b = build(&[sample("valid/all-object-types.md")], Mode::Full, false);
        assert_eq!(b.files, 1);
        assert_eq!(b.skipped, 0);
        assert!(b.text.starts_with("### Lore: A synthetic sample"));
        assert!(b.text.contains("- 分野: testing"));
        assert!(b.text.contains("## Session Arc"), "the whole file is kept");
        assert!(b.text.contains("lorespec: \"0.1\""), "frontmatter is kept in full mode");
        assert_eq!(b.chars, b.text.chars().count());
    }

    #[test]
    fn digest_mode_keeps_only_decisions_open_questions_and_next_steps() {
        let b = build(&[sample("valid/all-object-types.md")], Mode::Digest, false);
        assert!(b.text.contains("## Decisions"));
        assert!(b.text.contains("## Open Questions"));
        assert!(b.text.contains("## Next Steps"));
        for dropped in ["## Session Arc", "## Insights", "## Patterns", "## Connections", "lorespec:"] {
            assert!(!b.text.contains(dropped), "{dropped} should be left out");
        }
        let full = build(&[sample("valid/all-object-types.md")], Mode::Full, false);
        assert!(b.chars < full.chars);
    }

    #[test]
    fn headings_inside_code_fences_do_not_start_a_section() {
        let body = "## Decisions\n\n```\n## Insights\nnot a heading\n```\n\n## Insights\n\nleft out\n";
        let kept = sections(body, KEPT_SECTIONS);
        assert!(kept.contains("not a heading"));
        assert!(!kept.contains("left out"));
    }

    #[test]
    fn the_preface_asks_the_ai_to_check_versions_and_several_files_are_separated() {
        let paths = [sample("valid/all-object-types.md"), sample("valid/official-style.md")];
        let b = build(&paths, Mode::Digest, true);
        assert!(b.text.starts_with("以下は、過去のAIとの会話から作った要約"));
        assert!(b.text.contains("バージョン"));
        assert_eq!(b.files, 2);
        assert_eq!(b.text.matches("----------").count(), 1);
    }

    #[test]
    fn unreadable_and_non_markdown_files_are_counted_not_included() {
        let paths = [sample("valid/all-object-types.md"), sample("does-not-exist.md"), sample("invalid/README.md.txt")];
        let b = build(&paths, Mode::Full, false);
        assert_eq!(b.files, 1);
        assert_eq!(b.skipped, 2);
    }

    fn temp_dest(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("loreshelf-test-export-{name}"));
        std::fs::remove_dir_all(&d).ok();
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn read(dir: &str, file: &str) -> String {
        std::fs::read_to_string(Path::new(dir).join(file)).unwrap()
    }

    #[test]
    fn the_task_follows_the_preface() {
        let b = build_with_task(&[sample("valid/all-object-types.md")], Mode::Digest, true, "続きから設計を詰めたい");
        let preface_end = b.text.find("## 今回やりたいこと").expect("the task is shown");
        assert!(b.text[..preface_end].contains("バージョン"), "the preface comes first");
        assert!(b.text.contains("続きから設計を詰めたい"));
        assert!(!build(&[sample("valid/all-object-types.md")], Mode::Digest, true).text.contains("今回やりたいこと"));
    }

    #[test]
    fn pasted_text_records_what_the_conversation_inherits() {
        let paths = [sample("valid/all-object-types.md"), sample("valid/official-style.md")];
        let with = build(&paths, Mode::Digest, true).text;
        assert!(with.contains("引き継ぎ元: sample-all-object-types, sample-official-style"), "{with}");
        assert!(with.contains("`sample-all-object-types#D3`"));
        // It comes before the Lore themselves.
        assert!(with.find("引き継ぎの記録").unwrap() < with.find("### Lore:").unwrap());
        assert!(!build(&paths, Mode::Digest, false).text.contains("引き継ぎ"), "no preface, no record");
    }

    #[test]
    fn separate_layout_writes_a_readme_and_one_file_per_lore_and_never_overwrites() {
        let dest = temp_dest("separate");
        let paths = [sample("valid/all-object-types.md"), sample("valid/official-style.md")];
        let r = export(&paths, Mode::Digest, Layout::Separate, "続きを相談したい", &dest, 30_000).unwrap();
        assert!(r.dir.ends_with("loreshelf-output-001"), "{}", r.dir);
        assert_eq!(
            r.files,
            vec!["00-README.md", "01-sample-all-object-types.md", "02-sample-official-style.md"]
        );
        let readme = read(&r.dir, "00-README.md");
        for expected in [
            "続きを相談したい",
            "01-sample-all-object-types.md",
            "引き継ぎの記録",
            "引き継ぎ元: sample-all-object-types, sample-official-style",
            "`sample-all-object-types#D3`",
        ] {
            assert!(readme.contains(expected), "README lacks {expected}");
        }
        let one = read(&r.dir, "01-sample-all-object-types.md");
        assert!(one.starts_with("### Lore:") && one.contains("## Decisions"));
        // A second export goes to a new folder and leaves the first as it was.
        let r2 = export(&paths, Mode::Digest, Layout::Separate, "", &dest, 30_000).unwrap();
        assert!(r2.dir.ends_with("loreshelf-output-002"), "{}", r2.dir);
        assert_eq!(read(&r.dir, "00-README.md"), readme);
        assert!(!read(&r2.dir, "00-README.md").contains("今回やりたいこと"));
        std::fs::remove_dir_all(&dest).ok();
    }

    #[test]
    fn single_layout_writes_everything_into_one_file() {
        let dest = temp_dest("single");
        let paths = [sample("valid/all-object-types.md"), sample("valid/official-style.md")];
        let r = export(&paths, Mode::Full, Layout::Single, "まとめて読んでほしい", &dest, 30_000).unwrap();
        assert_eq!(r.files, vec!["bundle.md"]);
        let text = read(&r.dir, "bundle.md");
        assert!(text.contains("まとめて読んでほしい") && text.contains("### Lore: A synthetic sample"));
        assert_eq!(text.matches("### Lore:").count(), 2);
        std::fs::remove_dir_all(&dest).ok();
    }

    #[test]
    fn parts_layout_cuts_into_numbered_parts_that_can_be_put_back_together() {
        let dest = temp_dest("parts");
        let paths = [sample("valid/all-object-types.md"), sample("valid/official-style.md")];
        let limit = 1500;
        let r = export(&paths, Mode::Full, Layout::Parts, "", &dest, limit).unwrap();
        let parts: Vec<&String> = r.files.iter().filter(|f| f.starts_with("bundle.part")).collect();
        assert!(parts.len() >= 3, "{:?}", r.files);
        assert_eq!(r.files[0], "00-README.md");
        let readme = read(&r.dir, "00-README.md");
        assert!(readme.contains("パート1から順に"));
        assert!(!readme.contains("bundle.part*"), "the list names Lore, not a made-up file pattern");
        assert!(readme.contains("(id: sample-official-style)"));
        assert!(!readme.contains("〔id〕"), "the placeholder is replaced by real ids");
        let n = parts.len();
        let mut bodies = Vec::new();
        for (i, name) in parts.iter().enumerate() {
            let k = i + 1;
            let text = read(&r.dir, name);
            let head = format!("(パート {k}/{n})\n\n");
            let tail_at = text.rfind(&format!("\n\n(パート {k}/{n} の終わり")).expect("each part ends with its marker");
            assert!(text.starts_with(&head), "{name}");
            let body = &text[head.len()..tail_at];
            assert!(body.chars().count() <= limit, "{name} is {} characters", body.chars().count());
            if k < n {
                assert!(text.contains("返答しないでください"), "{name} tells the AI to wait");
            } else {
                assert!(text.contains("ここまでが、すべてです"));
            }
            bodies.push(body.to_string());
        }
        let expected = build(&paths, Mode::Full, false).text;
        assert_eq!(bodies.join("\n"), expected, "the parts, put back in order, are the whole text");
        std::fs::remove_dir_all(&dest).ok();
    }

    #[test]
    fn splitting_prefers_to_cut_before_a_heading() {
        let section = |name: &str| format!("## {name}\n{}", "aaaaaaaaaa\n".repeat(20).trim_end());
        let text = format!("{}\n{}", section("A"), section("B"));
        let parts = split_into_parts(&text, 300);
        assert_eq!(parts.len(), 2, "{parts:?}");
        assert!(parts[0].starts_with("## A") && parts[1].starts_with("## B"));
        assert_eq!(parts.join("\n"), text);
    }

    #[test]
    fn splitting_keeps_the_text_and_respects_the_limit() {
        let text: String = (0..80).map(|i| format!("line {i} {}\n", "x".repeat(i % 37))).collect();
        for limit in [200, 333, 1000] {
            let parts = split_into_parts(&text, limit);
            assert_eq!(parts.join("\n"), text, "limit {limit}");
            assert!(parts.iter().all(|p| p.chars().count() <= limit), "limit {limit}");
        }
        // A single line longer than the limit is cut by characters, and nothing is lost.
        let long = format!("short\n{}\nend", "y".repeat(700));
        let parts = split_into_parts(&long, 300);
        assert!(parts.iter().all(|p| p.chars().count() <= 300));
        assert_eq!(parts.join("").replace('\n', ""), long.replace('\n', ""));
    }

    #[test]
    fn a_hostile_id_cannot_name_a_file_outside_the_new_folder() {
        let dest = temp_dest("hostile");
        let src = dest.join("src");
        std::fs::create_dir_all(&src).unwrap();
        let f = src.join("LORE.md");
        let text = std::fs::read_to_string(sample("valid/all-object-types.md"))
            .unwrap()
            .replace("sample-all-object-types", "../../evil");
        std::fs::write(&f, text).unwrap();
        let out_root = dest.join("out");
        std::fs::create_dir_all(&out_root).unwrap();
        let r = export(&[f], Mode::Digest, Layout::Separate, "", &out_root, 30_000).unwrap();
        for name in &r.files {
            assert!(!name.contains('/') && !name.contains('\\') && !name.contains(".."), "{name}");
            assert!(Path::new(&r.dir).join(name).exists());
        }
        assert!(!dest.join("evil").exists() && !out_root.join("evil").exists());
        std::fs::remove_dir_all(&dest).ok();
    }

    #[test]
    fn nothing_is_written_when_there_is_nothing_to_write_or_nowhere_to_write() {
        let dest = temp_dest("errors");
        let e = export(&[sample("does-not-exist.md")], Mode::Full, Layout::Separate, "", &dest, 30_000).unwrap_err();
        assert!(e.contains("書き出せるLore"), "{e}");
        assert_eq!(std::fs::read_dir(&dest).unwrap().count(), 0, "no folder is left behind");
        let missing = dest.join("no-such-folder");
        assert!(export(&[sample("valid/all-object-types.md")], Mode::Full, Layout::Single, "", &missing, 30_000).is_err());
        std::fs::remove_dir_all(&dest).ok();
    }

    /// Dev helper, not part of the normal run: writes the files for one Lore into
    /// `LORESHELF_EXPORT_TO`, e.g.
    /// `LORESHELF_CHECK=a/LORE.md LORESHELF_EXPORT_TO=/tmp/x cargo test export_from_env -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn export_from_env() {
        let path = std::env::var("LORESHELF_CHECK").expect("set LORESHELF_CHECK to a file path");
        let to = std::env::var("LORESHELF_EXPORT_TO").expect("set LORESHELF_EXPORT_TO to a folder");
        let limit: usize = std::env::var("LORESHELF_PART_LIMIT").ok().and_then(|v| v.parse().ok()).unwrap_or(30_000);
        for layout in [Layout::Separate, Layout::Parts, Layout::Single] {
            let r = export(&[PathBuf::from(&path)], Mode::Full, layout, "試験", Path::new(&to), limit).unwrap();
            println!("=== {layout:?}: {} / {} 文字", r.files.join(", "), r.chars);
            for f in &r.files {
                let n = std::fs::read_to_string(Path::new(&r.dir).join(f)).unwrap().chars().count();
                println!("    {f}: {n} 文字");
            }
        }
    }

    /// Dev helper, not part of the normal run:
    /// `LORESHELF_CHECK=path/to/LORE.md cargo test print_context -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn print_context_from_env() {
        let path = std::env::var("LORESHELF_CHECK").expect("set LORESHELF_CHECK to a file path");
        for mode in [Mode::Digest, Mode::Full] {
            let b = build(&[PathBuf::from(&path)], mode, true);
            println!("=== {mode:?}: {} 件 / {} 文字", b.files, b.chars);
            if mode == Mode::Digest {
                println!("{}", b.text);
            }
        }
    }

    #[test]
    fn a_subject_whose_version_is_unknown_is_named_in_the_heading() {
        assert!(!is_version_tag("alexa-version-unknown"));
        assert!(!is_version_tag("echo-dot-gen3"));
        let dir = std::env::temp_dir().join("loreshelf-test-unknown-version");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("LORE.md");
        let text = std::fs::read_to_string(sample("valid/all-object-types.md"))
            .unwrap()
            .replace(
                "tags: [sample, test-fixture]",
                "tags: [echo-dot-gen3, alexa-version-unknown, gig-performer-v5]",
            );
        std::fs::write(&f, text).unwrap();
        let b = build(&[f], Mode::Digest, false);
        assert!(b.text.contains("- バージョンが不明の対象: alexa"), "{}", b.text);
        assert!(b.text.contains("- 対象のバージョン: gig-performer-v5"), "{}", b.text);
        assert!(!b.text.contains("echo-dot"), "a hardware generation is not a version: {}", b.text);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_version_tag_shows_up_in_the_heading() {
        let dir = std::env::temp_dir().join("loreshelf-test-context");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("LORE.md");
        let text = std::fs::read_to_string(sample("valid/all-object-types.md"))
            .unwrap()
            .replace(
                "tags: [sample, test-fixture]",
                "tags: [example-game, example-game-v2-8, loreshelf-prompts-v0-5, model-example-ai-v3, model-setting-high]",
            );
        std::fs::write(&f, text).unwrap();
        let b = build(&[f], Mode::Digest, false);
        assert!(b.text.contains("- 対象のバージョン: example-game-v2-8"), "{}", b.text);
        assert!(!b.text.contains("loreshelf-prompts"), "the prompts tag is not a subject version");
        assert!(
            b.text.contains("- 作成したAI: example-ai-v3(設定: high)"),
            "the model is shown as the author, with its setting: {}",
            b.text
        );
        assert!(!b.text.contains("対象のバージョン: example-game-v2-8, model"), "a model tag is not a subject version");
        std::fs::remove_dir_all(&dir).ok();
    }
}
