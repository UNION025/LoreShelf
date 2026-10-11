//! Bundles Lore into text that can be pasted into a conversation with an AI.
//!
//! A shelf is handed to an AI as context, not as training data: the files stay
//! as they are and the text is built on demand. Two forms exist: the whole
//! file, or only the parts that matter for continuing the work (decisions,
//! open questions, next steps), which is shorter.

use crate::lore;
use serde::Serialize;
use std::path::Path;

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

fn one(path: &Path, mode: Mode) -> Option<String> {
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
    Some(format!("{head}\n\n{body}\n"))
}

pub fn build(paths: &[impl AsRef<Path>], mode: Mode, with_preface: bool) -> Bundle {
    let parts: Vec<String> = paths.iter().filter_map(|p| one(p.as_ref(), mode)).collect();
    let skipped = paths.len() - parts.len();
    let mut text = String::new();
    if with_preface {
        text.push_str(PREFACE);
        text.push('\n');
    }
    text.push_str(&parts.join("\n----------\n\n"));
    Bundle {
        chars: text.chars().count(),
        files: parts.len(),
        skipped,
        text,
    }
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
