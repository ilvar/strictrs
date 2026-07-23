use std::collections::BTreeSet;
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
