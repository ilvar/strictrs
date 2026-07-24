mod support;

use std::collections::BTreeSet;

#[test]
fn cargo_check_surfaces_four_distinct_errors_without_masking() {
    let fixture = support::cargo_fixture("four-errors");
    let report = strictrs::run_check(fixture.path()).expect("fixture cargo check should run");

    assert!(!report.ok);
    assert!(report.error_count >= 4, "report was: {report:#?}");

    let codes: BTreeSet<_> = report
        .diagnostics
        .iter()
        .filter_map(|diagnostic| diagnostic.code.as_deref())
        .collect();

    for expected in ["E0004", "E0308", "E0425", "E0599"] {
        assert!(
            codes.contains(expected),
            "missing {expected}; report was: {report:#?}"
        );
    }
}
