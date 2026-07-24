use std::fs;
use std::path::Path;
use std::process::Command;

const EXPECTED_FILES: &[&str] = &[
    ".cargo/config.toml",
    ".gitignore",
    "Cargo.lock",
    "Cargo.toml",
    "README.md",
    "rust-toolchain.toml",
    "src/main.rs",
    "tests/properties.rs",
];

#[test]
fn new_command_matches_the_golden_project() {
    let directory = tempfile::tempdir().expect("temp directory should be created");
    let output = run_new(directory.path(), "hello-strictrs");

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should contain JSON");
    assert_eq!(
        report.get("ok").and_then(serde_json::Value::as_bool),
        Some(true)
    );

    let generated = directory.path().join("hello-strictrs");
    let expected = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/m3-template");

    let expected_files: Vec<String> = EXPECTED_FILES
        .iter()
        .map(|path| (*path).to_owned())
        .collect();
    assert_eq!(relative_files(&generated), expected_files);

    for relative in EXPECTED_FILES {
        let actual = read(&generated.join(relative));
        let golden = read(&expected.join(relative));
        assert_eq!(actual, golden, "generated {relative} differs");
    }
}

#[test]
fn new_command_refuses_invalid_names_and_existing_destinations() {
    let directory = tempfile::tempdir().expect("temp directory should be created");

    let invalid = run_new(directory.path(), "../escape");
    assert_eq!(invalid.status.code(), Some(2));
    assert!(!directory.path().join("escape").exists());

    fs::create_dir(directory.path().join("already-there"))
        .expect("existing destination should be created");
    let existing = run_new(directory.path(), "already-there");
    assert_eq!(existing.status.code(), Some(2));
}

fn run_new(directory: &Path, name: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_strictrs"))
        .current_dir(directory)
        .arg("new")
        .arg(name)
        .output()
        .expect("strictrs binary should run")
}

fn relative_files(root: &Path) -> Vec<String> {
    let mut files = Vec::new();

    for entry in walkdir::WalkDir::new(root) {
        let entry = entry.expect("generated project should be readable");
        if !entry.file_type().is_file() {
            continue;
        }

        let relative = entry
            .path()
            .strip_prefix(root)
            .expect("generated file should be below project root")
            .to_string_lossy()
            .replace('\\', "/");
        files.push(relative);
    }

    files.sort();
    files
}

fn read(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
