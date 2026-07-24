mod skills;
mod template;

use std::env;
use std::path::Path;
use std::process::ExitCode;

const AGENT_HELP: &str = include_str!("help.txt");
const USAGE: &str = "usage: strictrs [--help] | strictrs [check|fix] [path] | strictrs new <name> | strictrs install-skills";

fn main() -> ExitCode {
    let mut arguments = env::args().skip(1);
    let first = arguments.next();

    let operation = match parse_operation(first, &mut arguments) {
        Ok(operation) => operation,
        Err(error) => {
            eprintln!("{error}");
            print_usage_error();
            return ExitCode::from(2);
        }
    };

    if arguments.next().is_some() {
        eprintln!("too many arguments");
        print_usage_error();
        return ExitCode::from(2);
    }

    match operation {
        Operation::Help => {
            print!("{AGENT_HELP}");
            ExitCode::SUCCESS
        }
        Operation::Check(project) => emit_result(strictrs::run_check(Path::new(&project))),
        Operation::Fix(project) => emit_result(strictrs::run_fix(Path::new(&project))),
        Operation::New(name) => match template::create_project(Path::new("."), &name) {
            Ok(_) => emit_report(&clean_report()),
            Err(error) => {
                eprintln!("{error}");
                ExitCode::from(2)
            }
        },
        Operation::InstallSkills => match skills::install_detected() {
            Ok(messages) => {
                for message in messages {
                    eprintln!("{message}");
                }
                emit_report(&clean_report())
            }
            Err(error) => {
                eprintln!("{error}");
                ExitCode::from(2)
            }
        },
    }
}

enum Operation {
    Help,
    Check(String),
    Fix(String),
    New(String),
    InstallSkills,
}

fn parse_operation(
    first: Option<String>,
    arguments: &mut impl Iterator<Item = String>,
) -> Result<Operation, String> {
    match first.as_deref() {
        None => Ok(Operation::Check(".".to_owned())),
        Some("-h" | "--help" | "help") => Ok(Operation::Help),
        Some("check") => parse_path_or_help(arguments, Operation::Check),
        Some("fix") => parse_path_or_help(arguments, Operation::Fix),
        Some("new") => match arguments.next().as_deref() {
            Some("-h" | "--help") => Ok(Operation::Help),
            Some(name) => Ok(Operation::New(name.to_owned())),
            None => Err("new requires a project name".to_owned()),
        },
        Some("install-skills") => parse_install_skills(arguments),
        Some(path) => Ok(Operation::Check(path.to_owned())),
    }
}

fn parse_path_or_help(
    arguments: &mut impl Iterator<Item = String>,
    operation: impl FnOnce(String) -> Operation,
) -> Result<Operation, String> {
    match arguments.next().as_deref() {
        Some("-h" | "--help") => Ok(Operation::Help),
        Some(path) => Ok(operation(path.to_owned())),
        None => Ok(operation(".".to_owned())),
    }
}

fn parse_install_skills(
    arguments: &mut impl Iterator<Item = String>,
) -> Result<Operation, String> {
    match arguments.next().as_deref() {
        None => Ok(Operation::InstallSkills),
        Some("-h" | "--help") => Ok(Operation::Help),
        Some(_) => Err("install-skills does not accept arguments".to_owned()),
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

fn print_usage_error() {
    eprintln!("{USAGE}");
}
