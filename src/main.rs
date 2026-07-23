use std::env;
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let project = env::args().nth(1).unwrap_or_else(|| ".".to_owned());

    match strictrs::run_check(Path::new(&project)) {
        Ok(report) => {
            match serde_json::to_string_pretty(&report) {
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
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}
