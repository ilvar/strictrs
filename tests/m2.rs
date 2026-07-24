mod support;

use std::fs;

#[test]
fn fix_loop_applies_machine_suggestions_until_clean() {
    let fixture = support::cargo_fixture("fixable");

    let report = strictrs::run_fix(fixture.path()).expect("fix loop should run");
    let source = fs::read_to_string(fixture.path().join("src/main.rs"))
        .expect("fixed source should be readable");

    assert!(report.ok, "final report was: {report:#?}");
    assert!(source.contains("use first_values::FIRST;"));
    assert!(source.contains("use last_values::LAST;"));
    assert!(!source.contains("::*"));
}

#[test]
fn fix_loop_stops_after_reaching_an_unfixable_error() {
    let fixture = support::cargo_fixture("no-progress");
    let path = fixture.path().join("src/main.rs");
    let before = fs::read_to_string(&path).expect("fixture should be readable");

    let report =
        strictrs::run_fix_with_limit(fixture.path(), 3).expect("no-progress fixture should be checked");
    let after = fs::read_to_string(path).expect("fixture should remain readable");

    assert!(!report.ok);
    assert_ne!(before, after);
    assert!(after.contains("values.first().unwrap()"));
    assert!(!after.contains("values.get(0)"));
    assert!(report
        .diagnostics
        .iter()
        .any(|diagnostic| { diagnostic.code.as_deref() == Some("strictrs::no_panic_api") }));
}

#[test]
fn zero_iteration_limit_does_not_modify_sources() {
    let fixture = support::cargo_fixture("fixable");
    let path = fixture.path().join("src/main.rs");
    let before = fs::read_to_string(&path).expect("fixture should be readable");

    let report =
        strictrs::run_fix_with_limit(fixture.path(), 0).expect("zero-iteration check should run");
    let after = fs::read_to_string(path).expect("fixture should remain readable");

    assert!(!report.ok);
    assert_eq!(before, after);
}
