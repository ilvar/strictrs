use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

const SKILL_CONTENT: &str = include_str!("../skills/strictrs/SKILL.md");

pub fn install_detected() -> Result<Vec<String>, String> {
    let home = home_directory()?;
    let targets = detected_targets(&home);

    if targets.is_empty() {
        return Err(
            "no supported agent installation detected; start Codex or Claude Code once, then rerun `strictrs install-skills`"
                .to_owned(),
        );
    }

    preflight(&targets)?;

    let mut created = Vec::new();
    let mut messages = Vec::new();

    for target in targets {
        if target.path.is_file() {
            messages.push(format!(
                "{} skill is already current: {}",
                target.agent,
                target.path.display()
            ));
            continue;
        }

        if let Err(error) = write_skill(&target.path) {
            for path in &created {
                let _ = fs::remove_file(path);
            }
            return Err(error);
        }

        created.push(target.path.clone());
        messages.push(format!(
            "installed {} skill: {}",
            target.agent,
            target.path.display()
        ));
    }

    Ok(messages)
}

struct Target {
    agent: &'static str,
    path: PathBuf,
}

fn home_directory() -> Result<PathBuf, String> {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or_else(|| "cannot determine the user home directory".to_owned())
}

fn detected_targets(home: &Path) -> Vec<Target> {
    let mut targets = Vec::new();

    if home.join(".codex").is_dir()
        || home.join(".agents").is_dir()
        || command_exists("codex")
    {
        targets.push(Target {
            agent: "Codex",
            path: home
                .join(".agents")
                .join("skills")
                .join("strictrs")
                .join("SKILL.md"),
        });
    }

    if home.join(".claude").is_dir() || command_exists("claude") {
        targets.push(Target {
            agent: "Claude Code",
            path: home
                .join(".claude")
                .join("skills")
                .join("strictrs")
                .join("SKILL.md"),
        });
    }

    targets
}

fn preflight(targets: &[Target]) -> Result<(), String> {
    for target in targets {
        if !target.path.exists() {
            continue;
        }

        let existing = fs::read_to_string(&target.path).map_err(|error| {
            format!(
                "failed to inspect existing {} skill at {}: {error}",
                target.agent,
                target.path.display()
            )
        })?;

        if existing != SKILL_CONTENT {
            return Err(format!(
                "refusing to overwrite a modified {} skill at {}; remove it explicitly and rerun the command",
                target.agent,
                target.path.display()
            ));
        }
    }

    Ok(())
}

fn write_skill(path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("invalid skill destination: {}", path.display()))?;

    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("failed to create {}: {error}", path.display()))?;

    if let Err(error) = file.write_all(SKILL_CONTENT.as_bytes()) {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(format!("failed to write {}: {error}", path.display()));
    }

    Ok(())
}

fn command_exists(name: &str) -> bool {
    let Some(path) = env::var_os("PATH") else {
        return false;
    };

    env::split_paths(&path).any(|directory| executable_exists(&directory, name))
}

fn executable_exists(directory: &Path, name: &str) -> bool {
    if directory.join(name).is_file() {
        return true;
    }

    #[cfg(windows)]
    {
        for extension in ["exe", "cmd", "bat"] {
            if directory.join(format!("{name}.{extension}")).is_file() {
                return true;
            }
        }
    }

    false
}
