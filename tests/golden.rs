use std::path::Path;

#[test]
fn diagnostic_contract_matches_golden_file() {
    let stream = include_str!("../fixtures/diagnostic-stream.jsonl");
    let expected = include_str!("../fixtures/diagnostic-stream.expected.json").trim_end();
    let report = strictrs::parse_cargo_messages(stream, Path::new("."));
    let actual = serde_json::to_string_pretty(&report).expect("report should serialize");

    assert_eq!(actual, expected);
}
