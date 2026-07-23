use serde::Serialize;
use serde_json::Value;
use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Report {
    pub ok: bool,
    pub error_count: usize,
    pub warning_count: usize,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Diagnostic {
    pub level: String,
    pub source: String,
    pub code: Option<String>,
    pub message: String,
    pub at: Option<Location>,
    pub fixes: Vec<Fix>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Location {
    pub file: String,
    pub line: u64,
    pub col: u64,
    pub end_line: u64,
    pub end_col: u64,
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Fix {
    pub hint: String,
    pub replace_with: String,
    pub line: u64,
    pub col: u64,
    pub end_line: u64,
    pub end_col: u64,
}

pub fn run_check(project_dir: &Path) -> Result<Report, String> {
    let output = Command::new("cargo")
        .arg("check")
        .arg("--message-format=json")
        .current_dir(project_dir)
        .output()
        .map_err(|error| format!("failed to execute cargo check: {error}"))?;

    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| format!("cargo emitted invalid UTF-8 on stdout: {error}"))?;

    Ok(parse_cargo_messages(&stdout, project_dir))
}

pub fn parse_cargo_messages(stream: &str, project_dir: &Path) -> Report {
    let mut diagnostics = Vec::new();

    for line in stream
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let Ok(message) = serde_json::from_str::<Value>(line) else {
            continue;
        };

        if message.get("reason").and_then(Value::as_str) != Some("compiler-message") {
            continue;
        }

        let Some(inner) = message.get("message") else {
            continue;
        };

        let Some(level) = inner.get("level").and_then(Value::as_str) else {
            continue;
        };

        if level != "error" && level != "warning" {
            continue;
        }

        let Some(text) = inner.get("message").and_then(Value::as_str) else {
            continue;
        };

        if text.starts_with("aborting due to") || text.starts_with("could not compile") {
            continue;
        }

        let code = inner
            .get("code")
            .and_then(|value| value.get("code"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);

        let primary_span = inner
            .get("spans")
            .and_then(Value::as_array)
            .and_then(|spans| {
                spans
                    .iter()
                    .find(|span| span.get("is_primary").and_then(Value::as_bool) == Some(true))
            });

        let at = primary_span.and_then(|span| location_from_span(span, project_dir));
        let fixes = collect_fixes(inner);

        diagnostics.push(Diagnostic {
            level: level.to_owned(),
            source: "rustc".to_owned(),
            code,
            message: text.to_owned(),
            at,
            fixes,
        });
    }

    diagnostics.sort_by(compare_diagnostics);

    let error_count = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.level == "error")
        .count();
    let warning_count = diagnostics.len().saturating_sub(error_count);

    Report {
        ok: error_count == 0,
        error_count,
        warning_count,
        diagnostics,
    }
}

fn location_from_span(span: &Value, project_dir: &Path) -> Option<Location> {
    let file = span.get("file_name")?.as_str()?;
    let path = normalize_path(file, project_dir);
    let snippet = span
        .get("text")
        .and_then(Value::as_array)
        .and_then(|lines| lines.first())
        .and_then(|line| line.get("text"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();

    Some(Location {
        file: path,
        line: span.get("line_start")?.as_u64()?,
        col: span.get("column_start")?.as_u64()?,
        end_line: span.get("line_end")?.as_u64()?,
        end_col: span.get("column_end")?.as_u64()?,
        snippet,
    })
}

fn collect_fixes(inner: &Value) -> Vec<Fix> {
    let mut fixes = Vec::new();

    let Some(children) = inner.get("children").and_then(Value::as_array) else {
        return fixes;
    };

    for child in children {
        let hint = child
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();

        let Some(spans) = child.get("spans").and_then(Value::as_array) else {
            continue;
        };

        for span in spans {
            let Some(replacement) = span.get("suggested_replacement").and_then(Value::as_str)
            else {
                continue;
            };

            let applicability = span
                .get("suggestion_applicability")
                .and_then(Value::as_str)
                .unwrap_or("");

            if applicability != "MachineApplicable" {
                continue;
            }

            let (Some(line), Some(col), Some(end_line), Some(end_col)) = (
                span.get("line_start").and_then(Value::as_u64),
                span.get("column_start").and_then(Value::as_u64),
                span.get("line_end").and_then(Value::as_u64),
                span.get("column_end").and_then(Value::as_u64),
            ) else {
                continue;
            };

            fixes.push(Fix {
                hint: hint.clone(),
                replace_with: replacement.to_owned(),
                line,
                col,
                end_line,
                end_col,
            });
        }
    }

    fixes
}

fn normalize_path(file: &str, project_dir: &Path) -> String {
    let path = PathBuf::from(file);
    path.strip_prefix(project_dir)
        .unwrap_or(&path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn compare_diagnostics(left: &Diagnostic, right: &Diagnostic) -> Ordering {
    let left_key = diagnostic_key(left);
    let right_key = diagnostic_key(right);
    left_key.cmp(&right_key)
}

fn diagnostic_key(diagnostic: &Diagnostic) -> (String, u64, u64, String) {
    let (file, line, col) = diagnostic
        .at
        .as_ref()
        .map(|at| (at.file.clone(), at.line, at.col))
        .unwrap_or_else(|| (String::new(), 0, 0));
    let code = diagnostic.code.clone().unwrap_or_default();
    (file, line, col, code)
}
