use std::path::{Path, PathBuf};

use rustabletop_core::{Diagnostic, check};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn errors(name: &str) -> Vec<Diagnostic> {
    let checked = check(&fixture(name));
    assert!(!checked.ok(), "{name} should fail");
    assert!(checked.project.is_none());
    checked
        .diagnostics
        .into_iter()
        .filter(Diagnostic::is_error)
        .collect()
}

fn summary(d: &Diagnostic) -> (&str, usize, Option<&str>, Option<&str>) {
    let loc = d.location.as_ref().expect("diagnostic has a location");
    let subject = d.subject.as_ref();
    (
        d.code,
        loc.line,
        subject.and_then(|s| s.instance.as_deref()),
        subject.and_then(|s| s.field.as_deref()),
    )
}

#[test]
fn example_project_is_valid() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/sigilli");
    let checked = check(&root);
    assert!(checked.ok(), "{:#?}", checked.diagnostics);
    let project = checked.project.unwrap();
    let units = &project.components[0];
    assert_eq!(units.instances.len(), 10);
    assert_eq!(units.total_qty(), 21);
    assert_eq!(units.bleed.as_mm(), 3.0);
}

#[test]
fn m1_exit_criterion_bad_qty_and_suit_in_csv() {
    let errs = errors("broken-csv");
    let got: Vec<_> = errs.iter().map(summary).collect();
    assert_eq!(
        got,
        [
            ("data.type", 3, Some("gale-dancer"), Some("qty")),
            ("data.enum", 3, Some("gale-dancer"), Some("suit")),
        ]
    );
    assert_eq!(errs[0].location.as_ref().unwrap().file, "data/units.csv");
    assert!(
        errs[1]
            .hints
            .iter()
            .any(|h| h == "allowed values: fuoco, acqua, terra, aria")
    );
    assert!(errs[1].hints.iter().any(|h| h == "did you mean `aria`?"));
}

#[test]
fn yaml_errors_point_at_fields() {
    let errs = errors("broken-yaml");
    let got: Vec<_> = errs.iter().map(summary).collect();
    assert_eq!(
        got,
        [
            ("data.required", 5, Some("tide-seer"), Some("name")),
            ("data.required", 5, Some("tide-seer"), Some("cost")),
            ("data.unknown-field", 7, Some("tide-seer"), Some("cots")),
            ("data.duplicate-id", 8, Some("ember-adept"), Some("id")),
            ("data.id", 12, Some("Bad_Id"), Some("id")),
        ]
    );
}

#[test]
fn schema_keyword_typo_is_rejected() {
    let errs = errors("schema-typo");
    assert_eq!(errs.len(), 1);
    assert_eq!(errs[0].code, "manifest.schema");
    assert_eq!(errs[0].location.as_ref().unwrap().line, 10);
    assert_eq!(errs[0].hints, ["did you mean `enum`?"]);
}

#[test]
fn unquoted_csv_comma_has_actionable_hint() {
    let errs = errors("csv-comma");
    assert_eq!(errs.len(), 1);
    assert_eq!(errs[0].code, "csv.syntax");
    assert_eq!(errs[0].location.as_ref().unwrap().line, 3);
    assert!(errs[0].hints[0].contains("double quotes"));
}
