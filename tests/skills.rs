use std::fs;
use std::path::Path;
use std::process::{Command, Output};

const SKILL: &str = include_str!("../skills/strictrs/SKILL.md");

#[test]
fn install_skills_installs_for_detected_agents_and_is_idempotent() {
    let home = tempfile::tempdir().expect("temp home should be created");
    fs::create_dir(home.path().join(".codex")).expect("Codex marker should be created");
    fs::create_dir(home.path().join(".claude")).expect("Claude marker should be created");

    let first = run(home.path());
    assert!(first.status.success(), "stderr: {}", stderr(&first));
    assert_clean_report(&first);

    let codex = home.path().join(".agents/skills/strictrs/SKILL.md");
    let claude = home.path().join(".claude/skills/strictrs/SKILL.md");

    assert_eq!(
        fs::read_to_string(&codex).expect("Codex skill should exist"),
        SKILL
    );
    assert_eq!(
        fs::read_to_string(&claude).expect("Claude skill should exist"),
        SKILL
    );

    let second = run(home.path());
    assert!(second.status.success(), "stderr: {}", stderr(&second));
    assert_clean_report(&second);
    assert_eq!(
        fs::read_to_string(codex).expect("Codex skill should remain"),
        SKILL
    );
    assert_eq!(
        fs::read_to_string(claude).expect("Claude skill should remain"),
        SKILL
    );
}

#[test]
fn install_skills_refuses_to_overwrite_modified_content() {
    let home = tempfile::tempdir().expect("temp home should be created");
    fs::create_dir(home.path().join(".codex")).expect("Codex marker should be created");

    let destination = home.path().join(".agents/skills/strictrs/SKILL.md");
    fs::create_dir_all(destination.parent().expect("skill should have a parent"))
        .expect("skill directory should be created");
    fs::write(&destination, "custom skill\n").expect("custom skill should be written");

    let output = run(home.path());
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(stderr(&output).contains("refusing to overwrite"));
    assert_eq!(
        fs::read_to_string(destination).expect("custom skill should remain"),
        "custom skill\n"
    );
}

#[test]
fn install_skills_requires_a_detected_agent() {
    let home = tempfile::tempdir().expect("temp home should be created");
    let output = run(home.path());

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(stderr(&output).contains("no supported agent installation detected"));
}

fn run(home: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_strictrs"))
        .arg("install-skills")
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("PATH", "")
        .output()
        .expect("strictrs binary should run")
}

fn assert_clean_report(output: &Output) {
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should contain JSON");
    assert_eq!(
        report.get("ok").and_then(serde_json::Value::as_bool),
        Some(true)
    );
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
