use std::process::Command;

#[test]
fn help_prints_the_complete_agent_manual() {
    let output = run(&["--help"]);

    assert!(output.status.success());
    assert!(output.stderr.is_empty());

    let help = String::from_utf8(output.stdout).expect("help should be UTF-8");
    for required in [
        "AGENT WORKFLOW",
        "OUTPUT CONTRACT",
        "EXIT STATUS",
        "STRICT SUBSET",
        "CAPABILITY MODULES",
        "FIX SAFETY",
        "GENERATED PROJECTS",
        "PROPERTY TESTING",
        "FINAL VALIDATION",
        "AGENT CONSTRAINTS",
        "MachineApplicable",
        "strictrs::capability_boundary",
        "cargo install --git https://github.com/ilvar/strictrs",
        "cargo clippy --all-targets --all-features -- -D warnings",
    ] {
        assert!(help.contains(required), "help is missing {required:?}");
    }
}

#[test]
fn help_aliases_have_identical_stdout() {
    let expected = run(&["--help"]);
    assert!(expected.status.success());

    for arguments in [
        &["-h"][..],
        &["help"][..],
        &["check", "--help"][..],
        &["fix", "--help"][..],
        &["new", "--help"][..],
    ] {
        let actual = run(arguments);
        assert!(actual.status.success(), "arguments were {arguments:?}");
        assert!(actual.stderr.is_empty(), "arguments were {arguments:?}");
        assert_eq!(actual.stdout, expected.stdout, "arguments were {arguments:?}");
    }
}

fn run(arguments: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_strictrs"))
        .args(arguments)
        .output()
        .expect("strictrs binary should run")
}
