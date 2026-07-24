#[test]
fn generated_project_pins_and_scaffolds_property_testing() {
    let manifest = include_str!("../fixtures/m3-template/Cargo.fixture.toml");
    let lockfile = include_str!("../fixtures/m3-template/Cargo.lock");
    let properties = include_str!("../fixtures/m3-template/tests/properties.rs");

    assert!(manifest.contains(
        r#"proptest = { version = "=1.11.0", default-features = false, features = ["std"] }"#
    ));
    assert!(lockfile.contains("name = \"proptest\"\nversion = \"1.11.0\""));
    assert!(lockfile.contains(
        "name = \"hello-strictrs\"\nversion = \"0.1.0\"\ndependencies = [\n \"proptest\",\n]"
    ));
    assert!(properties.contains("proptest!"));
    assert!(properties.contains("prop_assert_eq!"));
    assert!(properties.contains("0..256"));
    assert!(!properties.contains("::*"));
}
