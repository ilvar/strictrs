use std::fs;
use std::path::Path;

#[test]
fn fix_loop_applies_compiler_suggestions_until_clean() {
    let directory = tempfile::tempdir().expect("temp directory should be created");
    copy_fixture("fixable", directory.path());

    let report = strictrs::run_fix(directory.path()).expect("fix loop should run");
    let source = fs::read_to_string(directory.path().join("src/main.rs"))
        .expect("fixed source should be readable");

    assert!(report.ok, "final report was: {report:#?}");
    assert!(source.contains("values.first()"));
    assert!(source.contains("values.last()"));
    assert!(!source.contains("frist"));
    assert!(!source.contains("lsat"));
}

#[test]
fn fix_loop_stops_when_no_machine_fix_exists() {
    let directory = tempfile::tempdir().expect("temp directory should be created");
    copy_fixture("no-progress", directory.path());
    let path = directory.path().join("src/main.rs");
    let before = fs::read_to_string(&path).expect("fixture should be readable");

    let report = strictrs::run_fix_with_limit(directory.path(), 3)
        .expect("no-progress fixture should be checked");
    let after = fs::read_to_string(path).expect("fixture should remain readable");

    assert!(!report.ok);
    assert_eq!(before, after);
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code.as_deref() == Some("strictrs::no_panic_api")
    }));
}

#[test]
fn zero_iteration_limit_does_not_modify_sources() {
    let directory = tempfile::tempdir().expect("temp directory should be created");
    copy_fixture("fixable", directory.path());
    let path = directory.path().join("src/main.rs");
    let before = fs::read_to_string(&path).expect("fixture should be readable");

    let report = strictrs::run_fix_with_limit(directory.path(), 0)
        .expect("zero-iteration check should run");
    let after = fs::read_to_string(path).expect("fixture should remain readable");

    assert!(!report.ok);
    assert_eq!(before, after);
}

fn copy_fixture(name: &str, destination: &Path) {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name);

    copy_file(&fixture, destination, Path::new("Cargo.toml"));
    copy_file(&fixture, destination, Path::new("src/main.rs"));
}

fn copy_file(source_root: &Path, destination_root: &Path, relative: &Path) {
    let destination = destination_root.join(relative);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).expect("fixture directory should be created");
    }
    fs::copy(source_root.join(relative), &destination).expect("fixture file should be copied");
}
