use std::fs;
use std::path::{Path, PathBuf};

use rtt_core::{BuildOptions, Built, Diagnostic, Plan, Severity, build, check};

fn example() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/sigilli")
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rtt-test-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            if entry.file_name() != "out" {
                copy_dir(&entry.path(), &target);
            }
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn patched_example(name: &str, from: &str, to: &str) -> PathBuf {
    let root = scratch(name).join("sigilli");
    copy_dir(&example(), &root);
    let data = root.join("data/units.yaml");
    let text = fs::read_to_string(&data).unwrap();
    assert!(text.contains(from), "patch anchor `{from}` not found");
    fs::write(&data, text.replacen(from, to, 1)).unwrap();
    root
}

fn build_into(root: &Path, out: &Path) -> Built {
    build(
        root,
        &BuildOptions {
            out: out.to_path_buf(),
            ppi: 20.0,
        },
    )
    .unwrap()
}

fn line_of(file: &Path, needle: &str) -> usize {
    fs::read_to_string(file)
        .unwrap()
        .lines()
        .position(|l| l.contains(needle))
        .expect("needle present")
        + 1
}

#[test]
fn sigilli_builds_from_the_manifest() {
    let out = scratch("sigilli");
    let built = build_into(&example(), &out);
    assert!(built.ok, "{:#?}", built.diagnostics);
    assert!(built.diagnostics.is_empty(), "{:#?}", built.diagnostics);

    let project = check(&example()).project.unwrap();
    let plan = Plan::new(&project).unwrap();
    assert_eq!(plan.sheet_count(), 6);
    assert_eq!(plan.preview_count(), 20);

    assert_eq!(built.artifacts.len(), 21);
    let pdf = out.join("sigilli-pnp.pdf");
    assert_eq!(built.artifacts[0], pdf.display().to_string());
    assert!(fs::read(&pdf).unwrap().starts_with(b"%PDF"));
    assert!(out.join("unit-card/void-oracle-front.png").is_file());
    assert!(out.join("unit-card/void-oracle-back.png").is_file());
}

#[test]
fn build_is_deterministic() {
    let (a, b) = (scratch("det-a"), scratch("det-b"));
    assert!(build_into(&example(), &a).ok);
    assert!(build_into(&example(), &b).ok);
    let pdf = |dir: &Path| fs::read(dir.join("sigilli-pnp.pdf")).unwrap();
    assert!(pdf(&a) == pdf(&b), "PDF bytes differ between two builds");
    let png = |dir: &Path| fs::read(dir.join("unit-card/ash-titan-front.png")).unwrap();
    assert!(png(&a) == png(&b));
}

#[test]
fn overflowing_text_points_at_the_record() {
    let root = patched_example(
        "overflow",
        "perde un Sigillo.'",
        &format!(
            "perde un Sigillo.{}'",
            " Poi ripeti tutto da capo, ignorando ogni Sigillo già perso.".repeat(15)
        ),
    );
    let built = build_into(&root, &root.join("out"));
    assert!(built.ok, "{:#?}", built.diagnostics);

    let overflows: Vec<&Diagnostic> = built
        .diagnostics
        .iter()
        .filter(|d| d.code == "render.overflow")
        .collect();
    assert_eq!(overflows.len(), 1, "{:#?}", built.diagnostics);
    let d = overflows[0];
    assert_eq!(d.severity, Severity::Warning);
    assert_eq!(
        d.message,
        "`text` of unit-card/void-oracle does not fit at 5.5pt"
    );
    let location = d.location.as_ref().unwrap();
    assert_eq!(location.file, "data/units.yaml");
    let record = line_of(&root.join("data/units.yaml"), "id: void-oracle");
    assert_eq!(location.line, record + 4);
    let subject = d.subject.as_ref().unwrap();
    assert_eq!(subject.instance.as_deref(), Some("void-oracle"));
    assert_eq!(subject.field.as_deref(), Some("text"));
}

#[test]
fn unknown_icon_is_reported_with_suggestion() {
    let root = patched_example(
        "icon",
        "infliggi 1 danno. :fuoco:",
        "infliggi 1 danno. :fuco:",
    );
    let built = build_into(&root, &root.join("out"));
    assert!(!built.ok);
    assert!(built.artifacts.is_empty());
    let errors: Vec<&Diagnostic> = built.diagnostics.iter().filter(|d| d.is_error()).collect();
    assert_eq!(errors.len(), 1, "{errors:#?}");
    let d = errors[0];
    assert_eq!(d.code, "data.unknown-icon");
    assert_eq!(d.message, "`text`: unknown icon `:fuco:`");
    assert_eq!(d.location.as_ref().unwrap().line, 6);
    assert_eq!(
        d.subject.as_ref().unwrap().instance.as_deref(),
        Some("ember-adept")
    );
    assert!(d.hints.iter().any(|h| h == "did you mean `fuoco`?"));
}

#[test]
fn missing_face_function_and_asset_are_reported() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/missing-face");
    let checked = check(&root);
    let got: Vec<(&str, usize)> = checked
        .diagnostics
        .iter()
        .map(|d| (d.code, d.location.as_ref().unwrap().line))
        .collect();
    assert_eq!(
        got,
        [("data.missing-asset", 3), ("manifest.face-function", 11)]
    );
    let asset = &checked.diagnostics[0];
    assert_eq!(asset.hints, ["did you mean `hero.svg`?"]);
}
