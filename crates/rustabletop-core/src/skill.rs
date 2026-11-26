use std::fs;
use std::io::{Cursor, Write};
use std::path::Path;

use anyhow::{Context, Result};
use serde::Serialize;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, ZipWriter};

pub const SKILL_NAME: &str = "rustabletop";
pub const SKILL_DIR: &str = ".claude/skills/rustabletop";
pub const SKILL_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const SKILL_VERSION_PLACEHOLDER: &str = "{{RUSTABLETOP_VERSION}}";

const SOURCES: &[(&str, &str)] = &[
    ("SKILL.md", include_str!("../skill/SKILL.md")),
    (
        "reference/manifest.md",
        include_str!("../skill/reference/manifest.md"),
    ),
    (
        "reference/templates.md",
        include_str!("../skill/reference/templates.md"),
    ),
    (
        "reference/diagnostics.md",
        include_str!("../skill/reference/diagnostics.md"),
    ),
    (
        "reference/printing.md",
        include_str!("../skill/reference/printing.md"),
    ),
];

/// The skill as written by `skill install` and `skill export`: paths relative to the
/// skill folder, with the version placeholder replaced by this binary's version.
pub fn skill_files() -> Vec<(&'static str, String)> {
    SOURCES
        .iter()
        .map(|(rel, text)| (*rel, text.replace(SKILL_VERSION_PLACEHOLDER, SKILL_VERSION)))
        .collect()
}

pub fn default_skill_zip() -> String {
    format!("{SKILL_NAME}-skill-{SKILL_VERSION}.zip")
}

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

#[derive(Debug, Serialize)]
pub struct SkillExport {
    pub ok: bool,
    pub version: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dir: Option<String>,
    pub files: Vec<String>,
}

pub fn install_skill(project: &Path) -> Result<SkillInstall> {
    let dir = project.join(SKILL_DIR);
    let (status, files) = write_skill(&dir)?;
    Ok(SkillInstall {
        ok: true,
        dir: dir.display().to_string(),
        status,
        version: SKILL_VERSION,
        files,
    })
}

/// Writes the zip accepted by claude.ai (the `rustabletop/` folder at its root) to `zip`
/// and the same folder under `dir`.
pub fn export_skill(zip: Option<&Path>, dir: Option<&Path>) -> Result<SkillExport> {
    if let Some(path) = zip {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent)
                .with_context(|| format!("cannot create {}", parent.display()))?;
        }
        fs::write(path, skill_zip()?)
            .with_context(|| format!("cannot write {}", path.display()))?;
    }
    let dir = dir.map(|d| d.join(SKILL_NAME));
    if let Some(dir) = &dir {
        write_skill(dir)?;
    }
    Ok(SkillExport {
        ok: true,
        version: SKILL_VERSION,
        zip: zip.map(|p| p.display().to_string()),
        dir: dir.map(|d| d.display().to_string()),
        files: SOURCES
            .iter()
            .map(|(rel, _)| format!("{SKILL_NAME}/{rel}"))
            .collect(),
    })
}

pub fn skill_zip() -> Result<Vec<u8>> {
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .last_modified_time(DateTime::DEFAULT);
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let mut folders: Vec<String> = vec![format!("{SKILL_NAME}/")];
    for (rel, _) in SOURCES {
        if let Some((folder, _)) = rel.rsplit_once('/') {
            let folder = format!("{SKILL_NAME}/{folder}/");
            if !folders.contains(&folder) {
                folders.push(folder);
            }
        }
    }
    for folder in folders {
        writer.add_directory(folder, options.unix_permissions(0o755))?;
    }
    for (rel, text) in skill_files() {
        writer.start_file(
            format!("{SKILL_NAME}/{rel}"),
            options.unix_permissions(0o644),
        )?;
        writer.write_all(text.as_bytes())?;
    }
    Ok(writer.finish()?.into_inner())
}

fn write_skill(dir: &Path) -> Result<(SkillStatus, Vec<String>)> {
    let wanted = skill_files();
    let existing: Vec<Option<String>> = wanted
        .iter()
        .map(|(rel, _)| fs::read_to_string(dir.join(rel)).ok())
        .collect();
    let status = if existing.iter().all(Option::is_none) {
        SkillStatus::Created
    } else if existing
        .iter()
        .zip(&wanted)
        .all(|(old, (_, text))| old.as_deref() == Some(text.as_str()))
    {
        SkillStatus::Unchanged
    } else {
        SkillStatus::Updated
    };
    let mut files = Vec::new();
    for (rel, text) in &wanted {
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
    Ok((status, files))
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
        SOURCES
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
        let skill =
            fs::read_to_string(repo().join("crates/rustabletop-core/skill/SKILL.md")).unwrap();
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

    #[test]
    fn m3_1_exit_criterion_repository_skill_carries_the_version_placeholder() {
        let skill =
            fs::read_to_string(repo().join("crates/rustabletop-core/skill/SKILL.md")).unwrap();
        let install = skill
            .split("\n## ")
            .find(|section| section.starts_with("Install"))
            .unwrap();
        assert!(install.contains(&format!(
            "rustabletop --version   # this skill is for rustabletop {SKILL_VERSION_PLACEHOLDER}"
        )));
        assert!(install.contains(&format!("RUSTABLETOP_VERSION=v{SKILL_VERSION_PLACEHOLDER}")));
        assert!(!skill.contains(SKILL_VERSION));

        for (rel, text) in skill_files() {
            assert!(!text.contains("{{"), "{rel} still has a placeholder");
        }
        let written = &skill_files()[0].1;
        assert!(written.contains(&format!("this skill is for rustabletop {SKILL_VERSION}")));
    }
}
