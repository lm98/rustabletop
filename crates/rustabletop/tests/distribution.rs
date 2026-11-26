use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn manifest_value(manifest: &str, table: &str, key: &str) -> String {
    let section = manifest
        .split("\n[")
        .find(|section| section.starts_with(&format!("{table}]")))
        .unwrap_or_else(|| panic!("no [{table}] in Cargo.toml"));
    section
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key} = \"")))
        .and_then(|rest| rest.strip_suffix('"'))
        .unwrap_or_else(|| panic!("no {key} in [{table}]"))
        .to_owned()
}

fn archive_suffixes(pkg_fmt: &str) -> &'static [&'static str] {
    match pkg_fmt {
        "tgz" => &[".tgz", ".tar.gz"],
        "zip" => &[".zip"],
        other => panic!("unexpected pkg-fmt {other}"),
    }
}

fn matrix_targets(workflow: &str) -> Vec<&str> {
    workflow
        .lines()
        .filter_map(|line| line.trim().strip_prefix("- target: "))
        .collect()
}

#[test]
fn m3_1_exit_criterion_binstall_metadata_matches_the_release_archives() {
    let manifest = fs::read_to_string(repo().join("crates/rustabletop/Cargo.toml")).unwrap();
    let workflow = fs::read_to_string(repo().join(".github/workflows/release.yml")).unwrap();
    let binstall = "package.metadata.binstall";
    let windows = "x86_64-pc-windows-msvc";

    let pkg_url = manifest_value(&manifest, binstall, "pkg-url");
    let (release, archive) = pkg_url.rsplit_once('/').unwrap();
    assert_eq!(release, "{ repo }/releases/download/v{ version }");
    assert!(workflow.contains("tags: [\"v*\"]"));
    assert_eq!(
        manifest_value(&manifest, binstall, "bin-dir"),
        "{ bin }{ binary-ext }"
    );

    let targets = matrix_targets(&workflow);
    assert!(
        targets.len() >= 3 && targets.contains(&windows),
        "{targets:?}"
    );
    for target in targets {
        let pkg_fmt = if target == windows {
            manifest_value(
                &manifest,
                &format!("{binstall}.overrides.{windows}"),
                "pkg-fmt",
            )
        } else {
            manifest_value(&manifest, binstall, "pkg-fmt")
        };
        let built: Vec<String> = archive_suffixes(&pkg_fmt)
            .iter()
            .map(|suffix| {
                archive
                    .replace("{ name }", "rustabletop")
                    .replace("{ target }", "${{ matrix.target }}")
                    .replace("{ archive-suffix }", suffix)
            })
            .filter(|name| {
                let unix = format!("tar -czf {name} -C target/${{{{ matrix.target }}}}/release rustabletop\n");
                let windows = format!(
                    "Compress-Archive -Path target/${{{{ matrix.target }}}}/release/rustabletop.exe -DestinationPath {name}\n"
                );
                workflow.contains(if pkg_fmt == "zip" { &windows } else { &unix })
            })
            .collect();
        assert_eq!(
            built.len(),
            1,
            "{target}: no release step builds {archive} as {pkg_fmt}"
        );
    }
}

#[test]
fn release_workflow_exports_the_skill_from_the_binary() {
    let workflow = fs::read_to_string(repo().join(".github/workflows/release.yml")).unwrap();
    assert!(workflow.contains("rustabletop skill export --zip dist/rustabletop-skill.zip"));
    assert!(!workflow.contains("crates/rustabletop-core/skill"));
}

#[test]
fn skill_export_defaults_to_a_versioned_zip_in_the_current_directory() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("skill-export-default");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_rustabletop"))
        .args(["skill", "export", "--json"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let name = format!("rustabletop-skill-{}.zip", env!("CARGO_PKG_VERSION"));
    assert_eq!(report["ok"], true);
    assert_eq!(report["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(report["zip"], name.as_str());
    assert!(report.get("dir").is_none());
    assert_eq!(
        fs::read(dir.join(&name)).unwrap(),
        rustabletop_core::skill_zip().unwrap()
    );
}
