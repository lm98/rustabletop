use std::fs;
use std::path::{Path, PathBuf};

use rustabletop_core::{
    BuildOptions, Built, Diagnostic, Paper, RenderOptions, SKILL_DIR, Severity, SkillStatus, build,
    check, describe, install_skill, new_project, render,
};

fn example() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/sigilli")
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rtt-agent-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn build_with(root: &Path, out: &Path, draft: bool) -> Built {
    build(
        root,
        &BuildOptions {
            out: out.to_path_buf(),
            ppi: 20.0,
            draft,
        },
    )
    .unwrap()
}

fn render_with(root: &Path, target: &str, out: &Path, face: Option<&str>) -> Built {
    render(
        root,
        target,
        &RenderOptions {
            out: out.to_path_buf(),
            ppi: 30.0,
            face: face.map(str::to_owned),
            draft: false,
        },
    )
    .unwrap()
}

fn codes(diagnostics: &[Diagnostic]) -> Vec<&'static str> {
    diagnostics.iter().map(|d| d.code).collect()
}

fn png_size(path: &str) -> (u32, u32) {
    let bytes = fs::read(path).unwrap();
    let be = |i: usize| u32::from_be_bytes(bytes[i..i + 4].try_into().unwrap());
    (be(16), be(20))
}

#[test]
fn m3_exit_criterion_new_project_checks_and_builds_without_diagnostics() {
    for (paper, name) in [(Paper::A4, "a4"), (Paper::Letter, "letter")] {
        let dir = scratch(&format!("new-{name}")).join("my-game");
        let files = new_project(&dir, paper).unwrap();
        assert!(files.iter().any(|f| f.ends_with(".gitignore")));
        assert!(!dir.join("fonts").exists());

        let checked = check(&dir);
        assert!(checked.diagnostics.is_empty(), "{:#?}", checked.diagnostics);
        let project = checked.project.unwrap();
        assert_eq!(project.game, "my-game");
        let kinds: Vec<&str> = project.components.iter().map(|c| c.kind.name()).collect();
        assert_eq!(kinds, ["card", "token", "board"]);

        for draft in [false, true] {
            let built = build_with(&dir, &dir.join("out"), draft);
            assert!(
                built.ok && built.diagnostics.is_empty(),
                "{:#?}",
                built.diagnostics
            );
        }
    }
}

#[test]
fn new_refuses_a_non_empty_directory() {
    let dir = scratch("new-occupied");
    fs::write(dir.join("notes.txt"), "keep me").unwrap();
    assert!(new_project(&dir, Paper::A4).is_err());
    assert_eq!(
        fs::read_to_string(dir.join("notes.txt")).unwrap(),
        "keep me"
    );
    assert!(!dir.join("game.yaml").exists());
}

#[test]
fn m3_exit_criterion_schema_describes_the_example() {
    let description = describe(&example());
    assert!(description.ok, "{:#?}", description.diagnostics);
    let json = serde_json::to_value(&description).unwrap();
    assert_eq!(json["game"], "sigilli");
    assert_eq!(json["print"]["margin"], "5mm");
    assert_eq!(json["icons"].as_array().unwrap().len(), 4);
    assert_eq!(json["formats"][2], "grid-cell");

    let component = |id: &str| {
        json["components"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == id)
            .unwrap()
            .clone()
    };
    let vortex = component("vortex");
    assert_eq!(
        vortex["shape"],
        serde_json::json!({ "type": "hex", "orientation": "pointy", "across-flats": "30mm" })
    );
    assert_eq!(vortex["bleed"], "2mm");
    assert_eq!(
        vortex["print"],
        serde_json::json!({ "sides": "single", "cut-lines": true, "gap": "2mm" })
    );
    assert_eq!(vortex["instances"], 1);
    assert!(vortex["schema"]["properties"]["id"].is_object());
    assert!(vortex["schema"]["properties"]["qty"].is_object());
    assert_eq!(vortex["schema"]["additionalProperties"], false);

    let map = component("dungeon-map");
    assert_eq!(map["grid"]["cell"], "30mm");
    assert_eq!(map["grid"]["cols"], 14);
    assert_eq!(map["print"]["mode"], "tiles");
    assert_eq!(component("unit-card")["data"], "data/units.yaml");
}

#[test]
fn m3_exit_criterion_schema_on_a_broken_project() {
    let description = describe(&fixture("token-bad-options"));
    assert!(!description.ok);
    assert!(
        description
            .diagnostics
            .iter()
            .any(|d| d.code == "manifest.print" && d.severity == Severity::Error)
    );
    assert_eq!(description.game.as_deref(), Some("fixture"));

    let missing = describe(&scratch("schema-missing"));
    assert!(!missing.ok);
    assert!(missing.game.is_none() && missing.components.is_empty());
    assert_eq!(codes(&missing.diagnostics), ["manifest.missing"]);
}

#[test]
fn m3_exit_criterion_render_one_instance_per_kind() {
    let out = scratch("render-kinds");
    let cases = [
        ("unit-card/ember-adept", 2),
        ("seal/seal-fuoco", 2),
        ("vortex/vortex", 1),
        ("dungeon-map/dungeon-map", 1),
    ];
    for (target, faces) in cases {
        let rendered = render_with(&example(), target, &out, None);
        assert!(rendered.ok, "{target}: {:#?}", rendered.diagnostics);
        assert_eq!(rendered.artifacts.len(), faces, "{target}");
    }

    let board = render_with(&example(), "dungeon-map", &out, Some("front"));
    let (w, h) = png_size(&board.artifacts[0]);
    let px = |mm: f64| (mm / 25.4 * 30.0).round() as u32;
    assert!(
        w.abs_diff(px(506.0)) <= 1 && h.abs_diff(px(356.0)) <= 1,
        "{w} × {h}"
    );
    assert!(board.artifacts[0].ends_with("dungeon-map/dungeon-map-front.png"));

    let back = render_with(&example(), "unit-card/ember-adept", &out, Some("back"));
    assert_eq!(back.artifacts.len(), 1);
    assert!(back.artifacts[0].ends_with("ember-adept-back.png"));
}

#[test]
fn render_reports_unknown_targets() {
    let out = scratch("render-unknown");
    for (target, face, subject) in [
        ("unit-crd/ember-adept", None, None),
        ("unit-card/ember-adep", None, Some("ember-adep")),
        ("unit-card", None, None),
        ("vortex/vortex", Some("back"), Some("vortex")),
    ] {
        let rendered = render_with(&example(), target, &out, face);
        assert!(!rendered.ok && rendered.artifacts.is_empty(), "{target}");
        let diagnostic = rendered
            .diagnostics
            .iter()
            .find(|d| d.code == "render.target")
            .unwrap_or_else(|| panic!("{target}: {:#?}", rendered.diagnostics));
        assert_eq!(
            diagnostic
                .subject
                .as_ref()
                .and_then(|s| s.instance.as_deref()),
            subject,
            "{target}"
        );
        assert!(!diagnostic.hints.is_empty(), "{target}");
    }
}

#[test]
fn m3_exit_criterion_low_resolution_art_is_a_located_warning() {
    let root = fixture("low-res");
    let built = build_with(&root, &scratch("low-res"), false);
    assert!(built.ok, "{:#?}", built.diagnostics);
    let low: Vec<&Diagnostic> = built
        .diagnostics
        .iter()
        .filter(|d| d.code == "render.low-resolution")
        .collect();
    assert_eq!(low.len(), 1, "{:#?}", built.diagnostics);
    let d = low[0];
    assert_eq!(d.severity, Severity::Warning);
    let subject = d.subject.as_ref().unwrap();
    assert_eq!(subject.instance.as_deref(), Some("small"));
    assert_eq!(subject.field.as_deref(), Some("art"));
    let location = d.location.as_ref().unwrap();
    assert_eq!((location.file.as_str(), location.line), ("data.yaml", 3));
    assert!(
        d.hints.iter().any(|h| h.contains("394 × 315")),
        "{:?}",
        d.hints
    );

    let rendered = render_with(&root, "hero/small", &scratch("low-res-render"), None);
    assert_eq!(codes(&rendered.diagnostics), ["render.low-resolution"]);
}

#[test]
fn m3_exit_criterion_draft_reaches_templates_and_skips_images() {
    let root = fixture("low-res");
    let final_out = scratch("draft-final");
    let draft_out = scratch("draft");
    let finished = build_with(&root, &final_out, false);
    let draft = build_with(&root, &draft_out, true);
    assert!(finished.ok && draft.ok);

    assert!(!codes(&finished.diagnostics).contains(&"render.overflow"));
    assert!(codes(&draft.diagnostics).contains(&"render.overflow"));
    assert!(!codes(&draft.diagnostics).contains(&"render.low-resolution"));

    assert!(finished.artifacts[0].ends_with("fixture-pnp.pdf"));
    assert!(draft.artifacts[0].ends_with("fixture-pnp-draft.pdf"));
    let has_image = |path: &str| {
        let bytes = fs::read(path).unwrap();
        bytes.windows(13).any(|w| w == b"/Subtype /Ima")
            || bytes.windows(14).any(|w| w == b"/Subtype/Image")
    };
    assert!(has_image(&finished.artifacts[0]));
    assert!(!has_image(&draft.artifacts[0]));
}

#[test]
fn draft_build_is_deterministic() {
    let root = fixture("low-res");
    let a = build_with(&root, &scratch("draft-a"), true);
    let b = build_with(&root, &scratch("draft-b"), true);
    assert_eq!(
        fs::read(&a.artifacts[0]).unwrap(),
        fs::read(&b.artifacts[0]).unwrap()
    );
}

fn tree(root: &Path) -> Vec<(String, Vec<u8>)> {
    let mut files = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let rel = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                files.push((rel, fs::read(&path).unwrap()));
            }
        }
    }
    files.sort();
    files
}

#[test]
fn m3_exit_criterion_new_project_carries_the_repository_skill() {
    let dir = scratch("new-skill").join("game");
    new_project(&dir, Paper::A4).unwrap();
    let repo_skill = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../skills/rustabletop");
    assert_eq!(tree(&dir.join(SKILL_DIR)), tree(&repo_skill));
}

#[test]
fn skill_install_creates_keeps_and_updates() {
    let dir = scratch("skill-install");
    let first = install_skill(&dir).unwrap();
    assert_eq!(first.status, SkillStatus::Created);
    assert_eq!(install_skill(&dir).unwrap().status, SkillStatus::Unchanged);

    let skill = dir.join(SKILL_DIR).join("SKILL.md");
    fs::write(&skill, "old version").unwrap();
    assert_eq!(install_skill(&dir).unwrap().status, SkillStatus::Updated);
    assert!(
        fs::read_to_string(&skill)
            .unwrap()
            .starts_with("---\nname: rustabletop")
    );
}
