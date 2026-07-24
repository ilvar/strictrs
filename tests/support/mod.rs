use std::fs;
use std::path::{Path, PathBuf};

pub struct CargoFixture {
    directory: tempfile::TempDir,
}

impl CargoFixture {
    pub fn path(&self) -> &Path {
        self.directory.path()
    }
}

pub fn cargo_fixture(name: &str) -> CargoFixture {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name);
    let directory = tempfile::tempdir().expect("temp fixture directory should be created");

    for entry in walkdir::WalkDir::new(&source_root) {
        let entry = entry.expect("fixture should be readable");
        if !entry.file_type().is_file() {
            continue;
        }

        let relative = entry
            .path()
            .strip_prefix(&source_root)
            .expect("fixture file should be below fixture root");
        let destination_relative = if relative == Path::new("Cargo.fixture.toml") {
            PathBuf::from("Cargo.toml")
        } else {
            relative.to_path_buf()
        };
        let destination = directory.path().join(destination_relative);

        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).expect("fixture directory should be created");
        }
        fs::copy(entry.path(), destination).expect("fixture file should be copied");
    }

    CargoFixture { directory }
}
