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

    let (command, project) = match first.as_deref() {
        None => ("check", ".".to_owned()),
        Some("check") => ("check", arguments.next().unwrap_or_else(|| ".".to_owned())),
        Some("fix") => ("fix", arguments.next().unwrap_or_else(|| ".".to_owned())),
        Some(path) => ("check", path.to_owned()),
    };

    if arguments.next().is_some() {
        eprintln!("too many arguments");
        print_usage();
        return ExitCode::from(2);
    }

    let result = match command {
        "fix" => strictrs::run_fix(Path::new(&project)),
        _ => strictrs::run_check(Path::new(&project)),
    };

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
    eprintln!("usage: strictrs [check|fix] [path]");
}
