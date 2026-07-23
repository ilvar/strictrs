use regex::Regex;
use serde::Serialize;
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

pub const DEFAULT_MAX_FIX_ITERATIONS: usize = 10;

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
    #[serde(skip)]
    edit: Option<EditSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EditSpan {
    file: String,
    byte_start: usize,
    byte_end: usize,
}

const ALL_TARGET_LINTS: &[&str] = &[
    "clippy::as_conversions",
    "clippy::wildcard_imports",
    "unsafe_code",
    "unused_must_use",
];

const PRODUCTION_PANIC_LINTS: &[&str] = &[
    "clippy::unwrap_used",
    "clippy::expect_used",
    "clippy::indexing_slicing",
];

pub fn run_check(project_dir: &Path) -> Result<Report, String> {
    let mut diagnostics = run_clippy(project_dir, true, ALL_TARGET_LINTS)?;
    diagnostics.extend(run_clippy(
        project_dir,
        false,
        PRODUCTION_PANIC_LINTS,
    )?);
    diagnostics.extend(scan_strict_subset(project_dir)?);
    Ok(build_report(diagnostics))
}

fn run_clippy(
    project_dir: &Path,
    all_targets: bool,
    denied_lints: &[&str],
) -> Result<Vec<Diagnostic>, String> {
    let mut command = Command::new("cargo");
    command
        .arg("clippy")
        .arg("--message-format=json")
        .arg("--all-features")
        .arg("--no-deps");

    if all_targets {
        command.arg("--all-targets");
    }

    command.arg("--");
    for lint in denied_lints {
        command.arg("-D").arg(lint);
    }

    let output = command
        .current_dir(project_dir)
        .output()
        .map_err(|error| format!("failed to execute cargo clippy: {error}"))?;

    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| format!("cargo emitted invalid UTF-8 on stdout: {error}"))?;
    let compiler_report = parse_cargo_messages(&stdout, project_dir);

    if !output.status.success() && compiler_report.error_count == 0 {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = stderr.trim();
        let suffix = if detail.is_empty() {
            String::new()
        } else {
            format!(": {detail}")
        };
        return Err(format!("cargo clippy failed without diagnostics{suffix}"));
    }

    Ok(compiler_report.diagnostics)
}

pub fn run_fix(project_dir: &Path) -> Result<Report, String> {
    run_fix_with_limit(project_dir, DEFAULT_MAX_FIX_ITERATIONS)
}

pub fn run_fix_with_limit(project_dir: &Path, max_iterations: usize) -> Result<Report, String> {
    let mut report = run_check(project_dir)?;

    for _ in 0..max_iterations {
        if report.ok {
            break;
        }

        let applied = apply_fixes(project_dir, &report)?;
        if applied == 0 {
            break;
        }

        let next = run_check(project_dir)?;
        if next == report {
            report = next;
            break;
        }
        report = next;
    }

    Ok(report)
}

pub fn apply_fixes(project_dir: &Path, report: &Report) -> Result<usize, String> {
    let mut fixes_by_file: BTreeMap<String, Vec<&Fix>> = BTreeMap::new();

    for diagnostic in &report.diagnostics {
        for fix in &diagnostic.fixes {
            let Some(edit) = fix.edit.as_ref() else {
                continue;
            };
            fixes_by_file
                .entry(edit.file.clone())
                .or_default()
                .push(fix);
        }
    }

    let mut updates = Vec::new();
    let mut applied_total = 0usize;

    for (file, fixes) in fixes_by_file {
        let Some(path) = safe_project_path(project_dir, &file) else {
            continue;
        };
        let source = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        let mut updated = source.clone();
        let mut selected = select_non_overlapping_fixes(fixes);

        selected.sort_by(|left, right| {
            let left_start = left
                .edit
                .as_ref()
                .map(|edit| edit.byte_start)
                .unwrap_or_default();
            let right_start = right
                .edit
                .as_ref()
                .map(|edit| edit.byte_start)
                .unwrap_or_default();
            right_start.cmp(&left_start)
        });

        let mut applied_in_file = 0usize;
        for fix in selected {
            let Some(edit) = fix.edit.as_ref() else {
                continue;
            };
            validate_edit(&updated, edit, &path)?;

            let Some(current) = updated.get(edit.byte_start..edit.byte_end) else {
                return Err(format!(
                    "invalid edit range {}..{} for {}",
                    edit.byte_start,
                    edit.byte_end,
                    path.display()
                ));
            };
            if current == fix.replace_with.as_str() {
                continue;
            }

            updated.replace_range(edit.byte_start..edit.byte_end, &fix.replace_with);
            applied_in_file += 1;
        }

        if applied_in_file > 0 {
            applied_total += applied_in_file;
            updates.push((path, updated));
        }
    }

    for (path, content) in updates {
        fs::write(&path, content)
            .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
    }

    Ok(applied_total)
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
            fixes: collect_fixes(inner, project_dir),
        });
    }

    build_report(diagnostics)
}

pub fn scan_strict_subset(project_dir: &Path) -> Result<Vec<Diagnostic>, String> {
    let src = project_dir.join("src");
    if !src.exists() {
        return Ok(Vec::new());
    }

    let public_fn = Regex::new(
        r"(?ms)^[ \t]*pub(?:\([^)]*\))?[ \t]+(?:async[ \t]+)?fn[ \t]+[A-Za-z_][A-Za-z0-9_]*(?:[ \t]*<[^{};]*>)?[ \t]*\([^{};]*\)[ \t]*(?:where[^{;]*)?\{",
    )
    .map_err(|error| format!("invalid public function regex: {error}"))?;
    let mutable_static = Regex::new(r"^\s*(?:pub\s+)?static\s+mut\b")
        .map_err(|error| format!("invalid mutable static regex: {error}"))?;
    let enum_declaration =
        Regex::new(r"^\s*(?:pub(?:\([^)]*\))?\s+)?enum\s+([A-Za-z_][A-Za-z0-9_]*)")
            .map_err(|error| format!("invalid enum regex: {error}"))?;

    let mut diagnostics = Vec::new();

    for entry in WalkDir::new(&src) {
        let entry = entry.map_err(|error| format!("failed to walk {}: {error}", src.display()))?;
        let path = entry.path();
        if !entry.file_type().is_file()
            || path.extension().and_then(|extension| extension.to_str()) != Some("rs")
        {
            continue;
        }

        let source = fs::read_to_string(path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        let lines: Vec<&str> = source.lines().collect();
        let capability_ranges = capability_ranges(&lines);
        let local_enums = local_enum_names(&lines, &enum_declaration);
        let catchall_lines = catchall_arm_lines(&lines, &local_enums);

        for matched in public_fn.find_iter(&source) {
            let line_no = line_number_at(&source, matched.start());
            let snippet = lines
                .get(line_no.saturating_sub(1))
                .copied()
                .unwrap_or_default();
            diagnostics.push(custom_diagnostic(
                "strictrs::explicit_return_type",
                "public functions must declare an explicit return type so intent is compiler-checked",
                path,
                project_dir,
                line_no as u64,
                snippet,
            ));
        }

        for (index, line) in lines.iter().enumerate() {
            let line_no = index as u64 + 1;

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

            if catchall_lines.contains(&index) {
                diagnostics.push(custom_diagnostic(
                    "strictrs::no_catchall_arm",
                    "catch-all arms can hide newly added enum variants; match variants explicitly",
                    path,
                    project_dir,
                    line_no,
                    line,
                ));
            }

            let code = code_before_comment(line);
            let uses_capability = code.contains("std::fs::")
                || code.contains("std::net::")
                || code.contains("std::process::");
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
        Some("clippy::unwrap_used")
        | Some("clippy::expect_used")
        | Some("clippy::indexing_slicing") => {
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

fn local_enum_names(lines: &[&str], declaration: &Regex) -> BTreeSet<String> {
    lines
        .iter()
        .filter_map(|line| declaration.captures(line))
        .filter_map(|captures| captures.get(1))
        .map(|name| name.as_str().to_owned())
        .collect()
}

fn catchall_arm_lines(lines: &[&str], local_enums: &BTreeSet<String>) -> BTreeSet<usize> {
    let mut result = BTreeSet::new();
    let mut pending_match = false;
    let mut in_match = false;
    let mut depth = 0isize;
    let mut mentions_local_enum = false;
    let mut candidates = Vec::new();

    for (index, line) in lines.iter().enumerate() {
        let code = code_before_comment(line);

        if !in_match {
            if code.contains("match ") {
                pending_match = true;
            }
            if !pending_match || !code.contains('{') {
                continue;
            }

            in_match = true;
            pending_match = false;
            depth = brace_delta(code);
            mentions_local_enum = line_mentions_local_enum(code, local_enums);
            if code.contains("_ =>") {
                candidates.push(index);
            }
        } else {
            depth += brace_delta(code);
            mentions_local_enum |= line_mentions_local_enum(code, local_enums);
            if code.contains("_ =>") {
                candidates.push(index);
            }
        }

        if depth <= 0 {
            if mentions_local_enum {
                result.extend(candidates.iter().copied());
            }
            in_match = false;
            depth = 0;
            mentions_local_enum = false;
            candidates.clear();
        }
    }

    result
}

fn line_mentions_local_enum(line: &str, local_enums: &BTreeSet<String>) -> bool {
    local_enums
        .iter()
        .any(|name| line.contains(&format!("{name}::")))
}

fn capability_ranges(lines: &[&str]) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut pending = false;
    let mut depth = 0isize;
    let mut start = None;

    for (index, line) in lines.iter().enumerate() {
        if line.contains("strictrs: capability") || line.contains("cfg_attr(any(), capability)") {
            pending = true;
        }

        if pending && line.contains("mod ") && line.contains('{') {
            start = Some(index);
            depth = brace_delta(code_before_comment(line));
            pending = false;
            if depth <= 0 {
                ranges.push((index, index));
                start = None;
            }
            continue;
        }

        if let Some(begin) = start {
            depth += brace_delta(code_before_comment(line));
            if depth <= 0 {
                ranges.push((begin, index));
                start = None;
                depth = 0;
            }
        }
    }

    ranges
}

fn brace_delta(line: &str) -> isize {
    let opens = line.chars().filter(|character| *character == '{').count() as isize;
    let closes = line.chars().filter(|character| *character == '}').count() as isize;
    opens - closes
}

fn code_before_comment(line: &str) -> &str {
    line.split_once("//")
        .map(|(code, _comment)| code)
        .unwrap_or(line)
}

fn inside_ranges(line: usize, ranges: &[(usize, usize)]) -> bool {
    ranges
        .iter()
        .any(|(start, end)| line >= *start && line <= *end)
}

fn line_number_at(source: &str, offset: usize) -> usize {
    source
        .get(..offset)
        .unwrap_or_default()
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
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

fn collect_fixes(inner: &Value, project_dir: &Path) -> Vec<Fix> {
    let mut fixes = Vec::new();
    let message = inner.get("message").and_then(Value::as_str).unwrap_or("");

    collect_fix_spans(
        inner
            .get("spans")
            .and_then(Value::as_array)
            .map(Vec::as_slice),
        message,
        project_dir,
        &mut fixes,
    );

    if let Some(children) = inner.get("children").and_then(Value::as_array) {
        for child in children {
            let hint = child.get("message").and_then(Value::as_str).unwrap_or("");
            collect_fix_spans(
                child
                    .get("spans")
                    .and_then(Value::as_array)
                    .map(Vec::as_slice),
                hint,
                project_dir,
                &mut fixes,
            );
        }
    }

    fixes
}

fn collect_fix_spans(
    spans: Option<&[Value]>,
    hint: &str,
    project_dir: &Path,
    fixes: &mut Vec<Fix>,
) {
    let Some(spans) = spans else {
        return;
    };

    for span in spans {
        let Some(replacement) = span.get("suggested_replacement").and_then(Value::as_str) else {
            continue;
        };
        if span.get("suggestion_applicability").and_then(Value::as_str) != Some("MachineApplicable")
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

        let edit = edit_span_from_json(span, project_dir);
        fixes.push(Fix {
            hint: hint.to_owned(),
            replace_with: replacement.to_owned(),
            line,
            col,
            end_line,
            end_col,
            edit,
        });
    }
}

fn edit_span_from_json(span: &Value, project_dir: &Path) -> Option<EditSpan> {
    let file = span.get("file_name")?.as_str()?;
    let byte_start = span
        .get("byte_start")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())?;
    let byte_end = span
        .get("byte_end")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())?;

    Some(EditSpan {
        file: normalize_path(file, project_dir),
        byte_start,
        byte_end,
    })
}

fn select_non_overlapping_fixes(fixes: Vec<&Fix>) -> Vec<&Fix> {
    let mut selected = Vec::new();

    for fix in fixes {
        let Some(edit) = fix.edit.as_ref() else {
            continue;
        };

        let duplicate = selected.iter().any(|existing: &&Fix| {
            existing
                .edit
                .as_ref()
                .is_some_and(|other| other == edit && existing.replace_with == fix.replace_with)
        });
        if duplicate {
            continue;
        }

        let overlaps = selected.iter().any(|existing: &&Fix| {
            existing.edit.as_ref().is_some_and(|other| {
                ranges_overlap(
                    edit.byte_start,
                    edit.byte_end,
                    other.byte_start,
                    other.byte_end,
                )
            })
        });
        if !overlaps {
            selected.push(fix);
        }
    }

    selected
}

fn ranges_overlap(
    left_start: usize,
    left_end: usize,
    right_start: usize,
    right_end: usize,
) -> bool {
    if left_start == left_end && right_start == right_end {
        return left_start == right_start;
    }
    if left_start == left_end {
        return left_start > right_start && left_start < right_end;
    }
    if right_start == right_end {
        return right_start > left_start && right_start < left_end;
    }
    left_start < right_end && right_start < left_end
}

fn validate_edit(source: &str, edit: &EditSpan, path: &Path) -> Result<(), String> {
    if edit.byte_start > edit.byte_end || edit.byte_end > source.len() {
        return Err(format!(
            "edit range {}..{} is outside {}",
            edit.byte_start,
            edit.byte_end,
            path.display()
        ));
    }
    if !source.is_char_boundary(edit.byte_start) || !source.is_char_boundary(edit.byte_end) {
        return Err(format!(
            "edit range {}..{} splits UTF-8 in {}",
            edit.byte_start,
            edit.byte_end,
            path.display()
        ));
    }
    Ok(())
}

fn safe_project_path(project_dir: &Path, file: &str) -> Option<PathBuf> {
    let relative = Path::new(file);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
    {
        return None;
    }
    Some(project_dir.join(relative))
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

#[cfg(test)]
mod tests {
    use super::{apply_fixes, Diagnostic, EditSpan, Fix, Location, Report};
    use std::fs;

    #[test]
    fn applies_multiple_edits_back_to_front() {
        let directory = tempfile::tempdir().expect("temp directory should be created");
        let src = directory.path().join("src");
        fs::create_dir(&src).expect("src directory should be created");
        let path = src.join("main.rs");
        let source = "fn main() { let a = frist(); let b = lsat(); }\n";
        fs::write(&path, source).expect("fixture should be written");

        let first_start = source.find("frist").expect("frist should exist");
        let second_start = source.find("lsat").expect("lsat should exist");
        let fixes = vec![
            test_fix("first", first_start, first_start + "frist".len()),
            test_fix("last", second_start, second_start + "lsat".len()),
        ];
        let report = test_report(fixes);

        let applied = apply_fixes(directory.path(), &report).expect("fixes should apply");
        let updated = fs::read_to_string(path).expect("fixture should be readable");

        assert_eq!(applied, 2);
        assert!(updated.contains("first()"));
        assert!(updated.contains("last()"));
    }

    #[test]
    fn keeps_first_of_overlapping_alternatives() {
        let directory = tempfile::tempdir().expect("temp directory should be created");
        let src = directory.path().join("src");
        fs::create_dir(&src).expect("src directory should be created");
        let path = src.join("main.rs");
        let source = "fn main() { wrong(); }\n";
        fs::write(&path, source).expect("fixture should be written");

        let start = source.find("wrong").expect("wrong should exist");
        let report = test_report(vec![
            test_fix("first", start, start + "wrong".len()),
            test_fix("second", start, start + "wrong".len()),
        ]);

        let applied = apply_fixes(directory.path(), &report).expect("fix should apply");
        let updated = fs::read_to_string(path).expect("fixture should be readable");

        assert_eq!(applied, 1);
        assert!(updated.contains("first()"));
        assert!(!updated.contains("second()"));
    }

    fn test_report(fixes: Vec<Fix>) -> Report {
        Report {
            ok: false,
            error_count: 1,
            warning_count: 0,
            diagnostics: vec![Diagnostic {
                level: "error".to_owned(),
                source: "rustc".to_owned(),
                code: Some("E0000".to_owned()),
                message: "test".to_owned(),
                at: Some(Location {
                    file: "src/main.rs".to_owned(),
                    line: 1,
                    col: 1,
                    end_line: 1,
                    end_col: 1,
                    snippet: String::new(),
                }),
                fixes,
            }],
        }
    }

    fn test_fix(replacement: &str, byte_start: usize, byte_end: usize) -> Fix {
        Fix {
            hint: "test".to_owned(),
            replace_with: replacement.to_owned(),
            line: 1,
            col: 1,
            end_line: 1,
            end_col: 1,
            edit: Some(EditSpan {
                file: "src/main.rs".to_owned(),
                byte_start,
                byte_end,
            }),
        }
    }
}
