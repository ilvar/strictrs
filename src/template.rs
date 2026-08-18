use std::fs;
use std::path::{Path, PathBuf};

const CARGO_CONFIG: &str = r#"[alias]
release-small = "build --release --locked --target x86_64-unknown-linux-musl"
"#;

const GITIGNORE: &str = "/target\n/dist\n__pycache__/\n";
const LOCKFILE_TEMPLATE: &str = include_str!("../fixtures/m3-template/Cargo.lock");

const PROPERTIES: &str = r#"use proptest::collection::vec;
use proptest::prelude::{any, prop_assert_eq, proptest};

proptest! {
    #[test]
    fn reversing_twice_preserves_values(values in vec(any::<u8>(), 0..256)) {
        let mut transformed = values.clone();
        transformed.reverse();
        transformed.reverse();

        prop_assert_eq!(transformed, values);
    }
}
"#;

const TOOLCHAIN: &str = r#"[toolchain]
channel = "1.97.1"
components = ["clippy", "rustfmt"]
profile = "minimal"
targets = ["x86_64-unknown-linux-musl"]
"#;

// Placeholder substituted with the crate name when a template is rendered.
const CRATE_PLACEHOLDER: &str = "__CRATE__";

const DOCKERFILE: &str = include_str!("../templates/Dockerfile");
const DOCKERIGNORE: &str = include_str!("../templates/dockerignore");
const MAKEFILE: &str = include_str!("../templates/Makefile");
const PRE_COMMIT: &str = include_str!("../templates/pre-commit-config.yaml");
const CI_WORKFLOW: &str = include_str!("../templates/ci.yml");
const COMMIT_SH: &str = include_str!("../templates/commit.sh");
const BUMP_VERSION: &str = include_str!("../templates/bump_version.py");
const AGENTS_MD: &str = include_str!("../templates/AGENTS.md");
const CLAUDE_MD: &str = include_str!("../templates/CLAUDE.md");
const README_TEMPLATE: &str = include_str!("../templates/README.md");

pub fn create_project(parent: &Path, name: &str) -> Result<PathBuf, String> {
    validate_name(name)?;

    let destination = parent.join(name);
    if destination.exists() {
        return Err(format!(
            "destination already exists: {}",
            destination.display()
        ));
    }

    let staging = parent.join(format!(".{name}.strictrs-tmp"));
    if staging.exists() {
        return Err(format!(
            "staging path already exists: {}",
            staging.display()
        ));
    }

    fs::create_dir(&staging)
        .map_err(|error| format!("failed to create {}: {error}", staging.display()))?;

    let result = write_project(&staging, name);
    if let Err(error) = result {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }

    fs::rename(&staging, &destination).map_err(|error| {
        let _ = fs::remove_dir_all(&staging);
        format!(
            "failed to move {} to {}: {error}",
            staging.display(),
            destination.display()
        )
    })?;

    Ok(destination)
}

fn validate_name(name: &str) -> Result<(), String> {
    let mut characters = name.chars();
    let Some(first) = characters.next() else {
        return Err("project name cannot be empty".to_owned());
    };

    if !first.is_ascii_lowercase() {
        return Err("project name must start with a lowercase ASCII letter".to_owned());
    }

    if characters.any(|character| {
        !character.is_ascii_lowercase()
            && !character.is_ascii_digit()
            && character != '-'
            && character != '_'
    }) {
        return Err(
            "project name may contain only lowercase ASCII letters, digits, '-' and '_'".to_owned(),
        );
    }

    Ok(())
}

fn write_project(project: &Path, name: &str) -> Result<(), String> {
    fs::create_dir(project.join(".cargo"))
        .map_err(|error| format!("failed to create .cargo: {error}"))?;
    fs::create_dir(project.join("src"))
        .map_err(|error| format!("failed to create src: {error}"))?;
    fs::create_dir(project.join("tests"))
        .map_err(|error| format!("failed to create tests: {error}"))?;
    fs::create_dir(project.join("scripts"))
        .map_err(|error| format!("failed to create scripts: {error}"))?;
    fs::create_dir_all(project.join(".github/workflows"))
        .map_err(|error| format!("failed to create .github/workflows: {error}"))?;

    write_file(project, ".cargo/config.toml", CARGO_CONFIG)?;
    write_file(project, ".dockerignore", DOCKERIGNORE)?;
    write_file(
        project,
        ".github/workflows/ci.yml",
        &render(CI_WORKFLOW, name),
    )?;
    write_file(project, ".gitignore", GITIGNORE)?;
    write_file(project, ".pre-commit-config.yaml", PRE_COMMIT)?;
    write_file(project, "AGENTS.md", AGENTS_MD)?;
    write_file(project, "CLAUDE.md", CLAUDE_MD)?;
    write_file(project, "Cargo.lock", &render_lockfile(name))?;
    write_file(project, "Cargo.toml", &render_manifest(name))?;
    write_file(project, "Dockerfile", &render(DOCKERFILE, name))?;
    write_file(project, "Makefile", &render(MAKEFILE, name))?;
    write_file(project, "README.md", &render_readme(name))?;
    write_file(project, "rust-toolchain.toml", TOOLCHAIN)?;
    write_file(project, "src/main.rs", &render_main(name))?;
    write_file(project, "tests/properties.rs", PROPERTIES)?;

    write_executable(
        project,
        "scripts/bump_version.py",
        &render(BUMP_VERSION, name),
    )?;
    write_executable(project, "scripts/commit.sh", COMMIT_SH)?;

    Ok(())
}

fn render(template: &str, name: &str) -> String {
    template.replace(CRATE_PLACEHOLDER, name)
}

fn write_executable(project: &Path, relative: &str, content: &str) -> Result<(), String> {
    write_file(project, relative, content)?;
    set_executable(&project.join(relative))
}

#[cfg(unix)]
fn set_executable(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path)
        .map_err(|error| format!("failed to stat {}: {error}", path.display()))?
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)
        .map_err(|error| format!("failed to chmod {}: {error}", path.display()))
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> Result<(), String> {
    Ok(())
}

fn write_file(project: &Path, relative: &str, content: &str) -> Result<(), String> {
    let path = project.join(relative);
    fs::write(&path, content)
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn render_manifest(name: &str) -> String {
    format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"
rust-version = "1.97.1"
publish = false

[dependencies]

[dev-dependencies]
proptest = {{ version = "=1.11.0", default-features = false, features = ["std"] }}

[lints.rust]
unsafe_code = "deny"
unused_must_use = "deny"

[lints.clippy]
as_conversions = "deny"
wildcard_imports = "deny"

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
strip = true
"#
    )
}

fn render_lockfile(name: &str) -> String {
    LOCKFILE_TEMPLATE.replacen(
        "name = \"hello-strictrs\"",
        &format!("name = \"{name}\""),
        1,
    )
}

fn render_main(name: &str) -> String {
    format!(
        r#"#![cfg_attr(
    not(test),
    deny(clippy::expect_used, clippy::indexing_slicing, clippy::unwrap_used)
)]

fn main() {{
    println!("Hello from {name}.");
}}
"#
    )
}

fn render_readme(name: &str) -> String {
    render(README_TEMPLATE, name)
}
