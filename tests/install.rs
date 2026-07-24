use std::path::Path;

#[test]
fn repository_contains_only_the_root_cargo_manifest() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut manifests = walkdir::WalkDir::new(repository)
        .into_iter()
        .filter_entry(|entry| entry.file_name() != "target" && entry.file_name() != ".git")
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file() && entry.file_name() == "Cargo.toml")
        .map(|entry| {
            entry
                .path()
                .strip_prefix(repository)
                .expect("manifest should be inside repository")
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect::<Vec<_>>();

    manifests.sort();
    assert_eq!(manifests, ["Cargo.toml"]);
}
