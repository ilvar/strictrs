use std::collections::BTreeSet;
use std::path::Path;

#[test]
fn cargo_check_surfaces_four_distinct_errors_without_masking() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/four-errors");
    let report = strictrs::run_check(&fixture).expect("fixture cargo check should run");

    assert!(!report.ok);
    assert!(report.error_count >= 4, "report was: {report:#?}");

    let codes: BTreeSet<_> = report
        .diagnostics
        .iter()
        .filter_map(|diagnostic| diagnostic.code.as_deref())
        .collect();

    for expected in ["E0004", "E0308", "E0425", "E0599"] {
        assert!(codes.contains(expected), "missing {expected}; report was: {report:#?}");
    }
}
