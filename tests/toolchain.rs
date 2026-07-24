#[test]
fn repository_and_generated_project_pin_rust_1_97_1() {
    let repository = include_str!("../rust-toolchain.toml");
    let generated = include_str!("../fixtures/m3-template/rust-toolchain.toml");
    let cargo_config = include_str!("../fixtures/m3-template/.cargo/config.toml");

    for toolchain in [repository, generated] {
        assert!(toolchain.contains("channel = \"1.97.1\""));
        assert!(!toolchain.contains("nightly"));
    }

    assert!(!cargo_config.contains("-Z"));
    assert!(cargo_config.contains("x86_64-unknown-linux-musl"));
}
