use regex::Regex;
use serde::Serialize;
use serde_json::Value;
use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

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

const CLIPPY_LINTS: &[&str] = &[
    "clippy::unwrap_used",
    "clippy::expect_used",
    "clippy::indexing_slicing",
    "clippy::as_conversions",
    "clippy::wildcard_imports",
    "unsafe_code",
    "unused_must_use",
];

pub fn run_check(project_dir: &Path) -> Result<Report, String> {
    let mut command = Command::new("cargo");
    command
        .arg("clippy")
        .arg("--message-format=json")
        .arg("--all-targets")
        .arg("--all-features")
        .arg("--");

    for lint in CLIPPY_LINTS {
        command.arg("-D").arg(lint);
    }

    let output = command
        .current_dir(project_dir)
        .output()
        .map_err(|error| format!("failed to execute cargo clippy: {error}"))?;

    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| format!("cargo emitted invalid UTF-8 on stdout: {error}"))?;

    let mut diagnostics = parse_cargo_messages(&stdout, project_dir).diagnostics;
    diagnostics.extend(scan_strict_subset(project_dir)?);
    Ok(build_report(diagnostics))
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

        let raw_code = inner
            .get("code")
            .and_then(|value| value.get("code"))
            .and_then(Value::as_str);
        let (source, code) = map_lint_code(raw_code);

        let primary_span = inner
            .get("spans")
            .and_then(Value::as_array)
            .and_then(|spans| {
                spans
                    .iter()
                    .find(|span| span.get("is_primary").and_then(Value::as_bool) == Some(true))
            });

        diagnostics.push(Diagnostic {
            level: level.to_owned(),
            source: source.to_owned(),
            code,
            message: text.to_owned(),
            at: primary_span.and_then(|span| location_from_span(span, project_dir)),
            fixes: collect_fixes(inner),
        });
    }

    build_report(diagnostics)
}

pub fn scan_strict_subset(project_dir: &Path) -> Result<Vec<Diagnostic>, String> {
    let src = project_dir.join("src");
    if !src.exists() {
        return Ok(Vec::new());
    }

    let public_fn = Regex::new(r"^\s*pub(?:\([^)]*\))?\s+(?:async\s+)?fn\s+[A-Za-z_][A-Za-z0-9_]*\s*\([^)]*\)\s*(?:where\b[^\{]*)?\{")
        .map_err(|error| format!("invalid public function regex: {error}"))?;
    let mutable_static = Regex::new(r"^\s*(?:pub\s+)?static\s+mut\b")
        .map_err(|error| format!("invalid mutable static regex: {error}"))?;

    let mut diagnostics = Vec::new();

    for entry in WalkDir::new(&src).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if !entry.file_type().is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }

        let source = fs::read_to_string(path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        let lines: Vec<&str> = source.lines().collect();
        let has_local_enum = lines.iter().any(|line| line.trim_start().starts_with("enum ") || line.trim_start().starts_with("pub enum "));
        let capability_ranges = capability_ranges(&lines);

        for (index, line) in lines.iter().enumerate() {
            let line_no = index as u64 + 1;
            let trimmed = line.trim();

            if mutable_static.is_match(line) {
                diagnostics.push(custom_diagnostic(
                    "strictrs::no_mutable_global",
                    "module-level mutable state obscures effects; use owned state passed explicitly",
                    path,
                    project_dir,
                    line_no,
                    line,
                ));
            }

            if public_fn.is_match(line) && !line.contains("->") {
                diagnostics.push(custom_diagnostic(
                    "strictrs::explicit_return_type",
                    "public functions must declare an explicit return type so intent is compiler-checked",
                    path,
                    project_dir,
                    line_no,
                    line,
                ));
            }

            if has_local_enum && (trimmed.starts_with("_ =>") || trimmed.contains(" _ =>")) {
                diagnostics.push(custom_diagnostic(
                    "strictrs::no_catchall_arm",
                    "catch-all arms can hide newly added enum variants; match variants explicitly",
                    path,
                    project_dir,
                    line_no,
                    line,
                ));
            }

            let uses_capability = line.contains("std::fs::")
                || line.contains("std::net::")
                || line.contains("std::process::");
            if uses_capability && !inside_ranges(index, &capability_ranges) {
                diagnostics.push(custom_diagnostic(
                    "strictrs::capability_boundary",
                    "filesystem, network, and process effects must live in a capability module",
                    path,
                    project_dir,
                    line_no,
                    line,
                ));
            }
        }
    }

    Ok(diagnostics)
}

fn map_lint_code(raw: Option<&str>) -> (&'static str, Option<String>) {
    match raw {
        Some("clippy::unwrap_used") | Some("clippy::expect_used") | Some("clippy::indexing_slicing") => {
            ("strictrs", Some("strictrs::no_panic_api".to_owned()))
        }
        Some("clippy::as_conversions") => ("strictrs", Some("strictrs::no_as_cast".to_owned())),
        Some("clippy::wildcard_imports") => {
            ("strictrs", Some("strictrs::no_glob_import".to_owned()))
        }
        Some("unsafe_code") => ("strictrs", Some("strictrs::no_unsafe".to_owned())),
        Some("unused_must_use") => ("strictrs", Some("strictrs::must_handle".to_owned())),
        Some(code) => ("rustc", Some(code.to_owned())),
        None => ("rustc", None),
    }
}

fn capability_ranges(lines: &[&str]) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut pending = false;
    let mut depth = 0usize;
    let mut start = None;

    for (index, line) in lines.iter().enumerate() {
        if line.contains("strictrs: capability") || line.contains("cfg_attr(any(), capability)") {
            pending = true;
        }

        if pending && line.contains("mod ") && line.contains('{') {
            start = Some(index);
            depth = brace_delta(line);
            pending = false;
            if depth == 0 {
                ranges.push((index, index));
                start = None;
            }
            continue;
        }

        if let Some(begin) = start {
            let delta = brace_delta(line);
            if delta >= 0 {
                depth = depth.saturating_add(delta as usize);
            } else {
                depth = depth.saturating_sub((-delta) as usize);
            }
            if depth == 0 {
                ranges.push((begin, index));
                start = None;
            }
        }
    }

    ranges
}

fn brace_delta(line: &str) -> isize {
    let opens = line.chars().filter(|ch| *ch == '{').count() as isize;
    let closes = line.chars().filter(|ch| *ch == '}').count() as isize;
    opens - closes
}

fn inside_ranges(line: usize, ranges: &[(usize, usize)]) -> bool {
    ranges.iter().any(|(start, end)| line >= *start && line <= *end)
}

fn custom_diagnostic(
    code: &str,
    message: &str,
    path: &Path,
    project_dir: &Path,
    line: u64,
    snippet: &str,
) -> Diagnostic {
    Diagnostic {
        level: "error".to_owned(),
        source: "strictrs".to_owned(),
        code: Some(code.to_owned()),
        message: message.to_owned(),
        at: Some(Location {
            file: normalize_path(path.to_string_lossy().as_ref(), project_dir),
            line,
            col: 1,
            end_line: line,
            end_col: snippet.chars().count() as u64 + 1,
            snippet: snippet.to_owned(),
        }),
        fixes: Vec::new(),
    }
}

fn build_report(mut diagnostics: Vec<Diagnostic>) -> Report {
    diagnostics.sort_by(compare_diagnostics);
    diagnostics.dedup();

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
    let snippet = span
        .get("text")
        .and_then(Value::as_array)
        .and_then(|lines| lines.first())
        .and_then(|line| line.get("text"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();

    Some(Location {
        file: normalize_path(file, project_dir),
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
            if span
                .get("suggestion_applicability")
                .and_then(Value::as_str)
                != Some("MachineApplicable")
            {
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
    diagnostic_key(left).cmp(&diagnostic_key(right))
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
