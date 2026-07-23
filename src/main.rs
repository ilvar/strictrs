mod template;

use std::env;
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut arguments = env::args().skip(1);
    let first = arguments.next();

    if matches!(first.as_deref(), Some("-h" | "--help")) {
        print_usage();
        return ExitCode::SUCCESS;
    }

    let operation = match parse_operation(first, &mut arguments) {
        Ok(operation) => operation,
        Err(error) => {
            eprintln!("{error}");
            print_usage();
            return ExitCode::from(2);
        }
    };

    if arguments.next().is_some() {
        eprintln!("too many arguments");
        print_usage();
        return ExitCode::from(2);
    }

    match operation {
        Operation::Check(project) => emit_result(strictrs::run_check(Path::new(&project))),
        Operation::Fix(project) => emit_result(strictrs::run_fix(Path::new(&project))),
        Operation::New(name) => match template::create_project(Path::new("."), &name) {
            Ok(_) => emit_report(&clean_report()),
            Err(error) => {
                eprintln!("{error}");
                ExitCode::from(2)
            }
        },
    }
}

enum Operation {
    Check(String),
    Fix(String),
    New(String),
}

fn parse_operation(
    first: Option<String>,
    arguments: &mut impl Iterator<Item = String>,
) -> Result<Operation, String> {
    match first.as_deref() {
        None => Ok(Operation::Check(".".to_owned())),
        Some("check") => Ok(Operation::Check(
            arguments.next().unwrap_or_else(|| ".".to_owned()),
        )),
        Some("fix") => Ok(Operation::Fix(
            arguments.next().unwrap_or_else(|| ".".to_owned()),
        )),
        Some("new") => arguments
            .next()
            .map(Operation::New)
            .ok_or_else(|| "new requires a project name".to_owned()),
        Some(path) => Ok(Operation::Check(path.to_owned())),
    }
}

fn clean_report() -> strictrs::Report {
    strictrs::Report {
        ok: true,
        error_count: 0,
        warning_count: 0,
        diagnostics: Vec::new(),
    }
}

fn emit_result(result: Result<strictrs::Report, String>) -> ExitCode {
    match result {
        Ok(report) => emit_report(&report),
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

fn emit_report(report: &strictrs::Report) -> ExitCode {
    match serde_json::to_string_pretty(report) {
        Ok(json) => println!("{json}"),
        Err(error) => {
            eprintln!("failed to serialize report: {error}");
            return ExitCode::from(2);
        }
    }

    if report.ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn print_usage() {
    eprintln!("usage: strictrs [check|fix] [path] | strictrs new <name>");
}
