use std::fs;
use std::path::{Path, PathBuf};

use rustabletop_core::{BuildOptions, Built, Diagnostic, Page, Plan, Severity, build, check};

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
    patched_file(name, "data/units.yaml", from, to)
}

fn patched_file(name: &str, file: &str, from: &str, to: &str) -> PathBuf {
    let root = scratch(name).join("sigilli");
    copy_dir(&example(), &root);
    let path = root.join(file);
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.contains(from), "patch anchor `{from}` not found");
    fs::write(&path, text.replacen(from, to, 1)).unwrap();
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
fn m2_exit_criterion_sigilli_builds_from_the_manifest() {
    let out = scratch("sigilli");
    let built = build_into(&example(), &out);
    assert!(built.ok, "{:#?}", built.diagnostics);
    assert!(built.diagnostics.is_empty(), "{:#?}", built.diagnostics);

    let project = check(&example()).project.unwrap();
    let plan = Plan::new(&project).unwrap();
    assert_eq!(plan.sheets_of(0).len(), 6);
    assert_eq!(plan.page_count(), 8 + 1 + 4);
    assert_eq!(plan.preview_count(), 20 + 8 + 1 + 1);

    assert_eq!(built.artifacts.len(), 31);
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

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn codes(root: &Path) -> Vec<(&'static str, usize, Vec<String>)> {
    check(root)
        .diagnostics
        .into_iter()
        .map(|d| (d.code, d.location.map_or(0, |l| l.line), d.hints))
        .collect()
}

#[test]
fn m2_5_exit_criterion_sigilli_tokens_are_imposed_with_cut_lines() {
    let project = check(&example()).project.unwrap();
    let plan = Plan::new(&project).unwrap();

    let seals = plan.sheets_of(1);
    assert_eq!(seals.len(), 1);
    let seal = seals[0];
    assert!(seal.cut_lines && seal.fold.is_some() && seal.marks.is_empty());
    let fronts = seal.cells.iter().filter(|c| c.face == "front").count();
    assert_eq!(fronts, 12);
    assert_eq!(seal.cells.len(), 24);

    let vortex = plan.sheets_of(2);
    assert_eq!(vortex.len(), 1);
    assert!(vortex[0].cut_lines && vortex[0].fold.is_none());
    assert_eq!(vortex[0].cells.len(), 6);
    assert!(vortex[0].cells.iter().all(|c| c.face == "front"));

    let vortex = &project.components[2];
    assert_eq!(vortex.instances.len(), 1);
    assert_eq!(vortex.instances[0].id, "vortex");
    assert_eq!(vortex.instances[0].qty, 6);
}

#[test]
fn m2_5_exit_criterion_unknown_face_on_a_token() {
    let got = codes(&fixture("token-bad-face"));
    assert_eq!(got.len(), 1, "{got:#?}");
    let (code, line, hints) = &got[0];
    assert_eq!((*code, *line), ("manifest.face", 11));
    assert!(hints.iter().any(|h| h == "faces: front, back"));
}

#[test]
fn token_manifest_errors_point_at_the_option() {
    let got: Vec<(&str, usize)> = codes(&fixture("token-bad-options"))
        .into_iter()
        .map(|(code, line, _)| (code, line))
        .collect();
    assert_eq!(
        got,
        [
            ("manifest.shape", 6),
            ("manifest.print", 18),
            ("manifest.data", 26),
        ]
    );
}

#[test]
fn inline_fields_are_validated_by_the_schema() {
    let got = codes(&fixture("inline-fields"));
    assert_eq!(got.len(), 1, "{got:#?}");
    assert_eq!((got[0].0, got[0].1), ("data.unknown-field", 13));
    assert!(got[0].2.iter().any(|h| h == "did you mean `title`?"));
}

const MANIFEST: &str = "game.yaml";

#[test]
fn m2_6_exit_criterion_board_is_tiled_with_an_assembly_page() {
    let project = check(&example()).project.unwrap();
    let plan = Plan::new(&project).unwrap();
    let pages = plan.pages_of(3);
    assert_eq!(pages.len(), 5);
    let Page::Assembly(assembly) = pages[0] else {
        panic!("the assembly page comes first");
    };
    assert_eq!(assembly.sections.len(), 4);
    let labels: Vec<&str> = pages[1..]
        .iter()
        .map(|p| match p {
            Page::Tile(t) => t.label.as_str(),
            _ => panic!("sections follow the assembly page"),
        })
        .collect();
    assert_eq!(labels, ["A1", "A2", "B1", "B2"]);
    let Page::Tile(a1) = pages[1] else {
        unreachable!()
    };
    assert_eq!(a1.page, (297.0, 210.0));
    assert_eq!(a1.under.len(), 2);
    assert_eq!(a1.registration.len(), 4);
    assert!(!a1.marks.is_empty());

    let map = &project.components[3];
    let grid = map.grid.as_ref().unwrap();
    assert_eq!((grid.cols, grid.rows), (14, 9));
    assert_eq!(grid.cell, 30.0);
    assert_eq!(map.instances[0].id, "dungeon-map");
}

#[test]
fn m2_6_exit_criterion_poster_is_a_single_page() {
    let root = patched_file("poster", MANIFEST, "mode: tiles", "mode: poster");
    let project = check(&root).project.unwrap();
    let plan = Plan::new(&project).unwrap();
    let pages = plan.pages_of(3);
    assert_eq!(pages.len(), 1);
    let Page::Sheet(sheet) = pages[0] else {
        panic!("a poster is a single sheet");
    };
    assert_eq!(sheet.page, (506.0, 356.0));
    assert!(build_into(&root, &root.join("out")).ok);
}

#[test]
fn m2_6_exit_criterion_grid_cell_outside_the_grid() {
    let root = patched_file(
        "grid-cell",
        MANIFEST,
        "{ at: [12, 7], terrain: lava }",
        "{ at: [3, 12], terrain: lava }",
    );
    let checked = check(&root);
    let errors: Vec<&Diagnostic> = checked
        .diagnostics
        .iter()
        .filter(|d| d.is_error())
        .collect();
    assert_eq!(errors.len(), 1, "{errors:#?}");
    let d = errors[0];
    assert_eq!(d.code, "data.grid-cell");
    let line = line_of(&root.join(MANIFEST), "at: [3, 12]");
    assert_eq!(d.location.as_ref().unwrap().line, line);
    let subject = d.subject.as_ref().unwrap();
    assert_eq!(subject.instance.as_deref(), Some("dungeon-map"));
    assert_eq!(subject.field.as_deref(), Some("spaces.7.at"));
    assert_eq!(d.hints, ["grid is 14 × 9: col 0..13, row 0..8"]);
}

#[test]
fn m2_6_exit_criterion_unknown_grid_cell_component() {
    let root = patched_file("grid-typo", MANIFEST, "cell: vortex,", "cell: vortx,");
    let got = codes(&root);
    assert_eq!(got.len(), 1, "{got:#?}");
    let (code, line, hints) = &got[0];
    assert_eq!(*code, "manifest.grid");
    assert_eq!(*line, line_of(&root.join(MANIFEST), "cell: vortx"));
    assert_eq!(hints[0], "did you mean `vortex`?");
}

#[test]
fn m2_6_exit_criterion_back_on_tiles_is_rejected() {
    let root = patched_file(
        "tiles-back",
        MANIFEST,
        "      front: templates/map.typ",
        "      front: templates/map.typ\n      back: templates/map.typ",
    );
    let got = codes(&root);
    let print: Vec<_> = got
        .iter()
        .filter(|(code, ..)| *code == "manifest.print")
        .collect();
    assert_eq!(print.len(), 1, "{got:#?}");
    assert_eq!(print[0].1, line_of(&root.join(MANIFEST), "mode: tiles"));
    assert!(print[0].2.iter().any(|h| h.contains("mode: poster")));
}

#[test]
fn grid_errors_on_other_kinds_and_formats_without_grid() {
    let got: Vec<(&str, usize)> = codes(&fixture("grid-misuse"))
        .into_iter()
        .map(|(code, line, _)| (code, line))
        .collect();
    assert_eq!(got, [("manifest.schema", 7), ("manifest.grid", 14)]);
}
