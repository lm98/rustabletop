use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Serialize;

pub const SKILL_DIR: &str = ".claude/skills/rustabletop";

pub const SKILL_FILES: &[(&str, &str)] = &[
    (
        "SKILL.md",
        include_str!("../../../skills/rustabletop/SKILL.md"),
    ),
    (
        "reference/manifest.md",
        include_str!("../../../skills/rustabletop/reference/manifest.md"),
    ),
    (
        "reference/templates.md",
        include_str!("../../../skills/rustabletop/reference/templates.md"),
    ),
    (
        "reference/diagnostics.md",
        include_str!("../../../skills/rustabletop/reference/diagnostics.md"),
    ),
    (
        "reference/printing.md",
        include_str!("../../../skills/rustabletop/reference/printing.md"),
    ),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillStatus {
    Created,
    Unchanged,
    Updated,
}

#[derive(Debug, Serialize)]
pub struct SkillInstall {
    pub ok: bool,
    pub dir: String,
    pub status: SkillStatus,
    pub version: &'static str,
    pub files: Vec<String>,
}

pub fn install_skill(project: &Path) -> Result<SkillInstall> {
    let dir = project.join(SKILL_DIR);
    let existing: Vec<Option<String>> = SKILL_FILES
        .iter()
        .map(|(rel, _)| fs::read_to_string(dir.join(rel)).ok())
        .collect();
    let status = if existing.iter().all(Option::is_none) {
        SkillStatus::Created
    } else if existing
        .iter()
        .zip(SKILL_FILES)
        .all(|(old, (_, text))| old.as_deref() == Some(*text))
    {
        SkillStatus::Unchanged
    } else {
        SkillStatus::Updated
    };
    let mut files = Vec::new();
    for (rel, text) in SKILL_FILES {
        let path = dir.join(rel);
        if status != SkillStatus::Unchanged {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("cannot create {}", parent.display()))?;
            }
            fs::write(&path, text).with_context(|| format!("cannot write {}", path.display()))?;
        }
        files.push(path.display().to_string());
    }
    Ok(SkillInstall {
        ok: true,
        dir: dir.display().to_string(),
        status,
        version: env!("CARGO_PKG_VERSION"),
        files,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::*;

    const CODE_PREFIXES: &[&str] = &["csv", "data", "manifest", "print", "render", "yaml"];

    fn repo() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn reference(name: &str) -> String {
        SKILL_FILES
            .iter()
            .find(|(rel, _)| *rel == format!("reference/{name}"))
            .map(|(_, text)| (*text).to_owned())
            .unwrap()
    }

    fn string_literals(text: &str) -> Vec<&str> {
        text.split('"').skip(1).step_by(2).collect()
    }

    fn is_code(literal: &str) -> bool {
        literal == "typst"
            || literal.split_once('.').is_some_and(|(prefix, name)| {
                CODE_PREFIXES.contains(&prefix)
                    && !name.is_empty()
                    && name.chars().all(|c| c.is_ascii_lowercase() || c == '-')
            })
    }

    fn emitted_codes() -> Vec<String> {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut codes = Vec::new();
        for entry in fs::read_dir(src).unwrap() {
            let path = entry.unwrap().path();
            if path.file_name().is_some_and(|n| n == "skill.rs") {
                continue;
            }
            let text = fs::read_to_string(&path).unwrap();
            for constructor in ["Diagnostic::error(", "Diagnostic::warning(", "violation(\n"] {
                for call in text.split(constructor).skip(1) {
                    let first = call.trim_start();
                    if let Some(literal) = first.strip_prefix('"').and_then(|s| s.split('"').next())
                    {
                        assert!(
                            is_code(literal),
                            "{}: `{literal}` does not look like a diagnostic code",
                            path.display()
                        );
                    }
                }
            }
            codes.extend(
                string_literals(&text)
                    .into_iter()
                    .filter(|l| is_code(l))
                    .map(str::to_owned),
            );
        }
        codes.sort();
        codes.dedup();
        codes
    }

    #[test]
    fn m3_exit_criterion_every_diagnostic_code_is_documented() {
        let codes = emitted_codes();
        assert!(codes.len() > 30, "{codes:?}");
        let doc = reference("diagnostics.md");
        let missing: Vec<&String> = codes
            .iter()
            .filter(|c| !doc.contains(&format!("`{c}`")))
            .collect();
        assert!(
            missing.is_empty(),
            "undocumented in diagnostics.md: {missing:?}"
        );
    }

    #[test]
    fn m3_exit_criterion_every_manifest_key_is_documented() {
        let keys = crate::project::manifest_keys();
        for key in [
            "game",
            "duplex-edge",
            "orientation",
            "crop-marks",
            "overlap",
            "origin",
        ] {
            assert!(keys.iter().any(|k| k == key), "{key} missing from {keys:?}");
        }
        let doc = reference("manifest.md");
        let missing: Vec<&String> = keys
            .iter()
            .filter(|k| !doc.contains(&format!("`{k}`")))
            .collect();
        assert!(
            missing.is_empty(),
            "undocumented in manifest.md: {missing:?}"
        );
    }

    #[test]
    fn skill_entry_point_is_short_and_links_every_reference() {
        let skill = fs::read_to_string(repo().join("skills/rustabletop/SKILL.md")).unwrap();
        assert!(skill.lines().count() <= 150);
        assert!(skill.starts_with("---\nname: rustabletop\ndescription: "));
        for name in [
            "manifest.md",
            "templates.md",
            "diagnostics.md",
            "printing.md",
        ] {
            assert!(skill.contains(&format!("reference/{name}")), "{name}");
        }
    }
}
