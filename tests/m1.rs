use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

#[test]
fn strict_subset_lints_share_the_unified_contract() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/strict-subset");
    let report = strictrs::run_check(&fixture).expect("fixture lint pass should run");

    assert!(!report.ok);

    let codes: BTreeSet<_> = report
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.source == "strictrs")
        .filter_map(|diagnostic| diagnostic.code.as_deref())
        .collect();

    for expected in [
        "strictrs::no_unsafe",
        "strictrs::no_panic_api",
        "strictrs::no_as_cast",
        "strictrs::no_glob_import",
        "strictrs::no_mutable_global",
        "strictrs::explicit_return_type",
        "strictrs::no_catchall_arm",
        "strictrs::capability_boundary",
    ] {
        assert!(
            codes.contains(expected),
            "missing {expected}; report was: {report:#?}"
        );
    }
}

#[test]
fn marked_capability_module_is_exempt() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/strict-subset");
    let diagnostics = strictrs::scan_strict_subset(&fixture).expect("source scan should run");

    let process_violation = diagnostics.iter().any(|diagnostic| {
        diagnostic.code.as_deref() == Some("strictrs::capability_boundary")
            && diagnostic
                .at
                .as_ref()
                .is_some_and(|at| at.snippet.contains("std::process::"))
    });

    assert!(!process_violation, "capability module should be exempt");
}

#[test]
fn panic_apis_in_cfg_test_code_are_exempt() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/test-exemption");
    let report = strictrs::run_check(&fixture).expect("test exemption fixture should run");

    assert!(report.ok, "test-only panic API was linted: {report:#?}");
}

#[test]
fn catchall_on_an_unrelated_type_is_not_flagged() {
    let directory = tempfile::tempdir().expect("temp directory should be created");
    let src = directory.path().join("src");
    fs::create_dir(&src).expect("src directory should be created");
    fs::write(
        src.join("main.rs"),
        "enum Local { A }\nfn main() {\n    let value = Some(1);\n    match value {\n        Some(_) => {}\n        _ => {}\n    }\n    let _ = Local::A;\n}\n",
    )
    .expect("source fixture should be written");

    let diagnostics =
        strictrs::scan_strict_subset(directory.path()).expect("source scan should run");

    assert!(!diagnostics.iter().any(|diagnostic| {
        diagnostic.code.as_deref() == Some("strictrs::no_catchall_arm")
    }));
}
