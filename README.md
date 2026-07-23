# strictrs

`strictrs` is a strict subset of Rust plus a tooling layer that turns the compiler into a deterministic, machine-readable oracle for LLM code generation.

It is not a new language, parser, compiler fork, or standard library. The project keeps Rust syntax and semantics, then adds a constrained coding profile and a closed diagnostic loop for agents.

## Thesis

Rust already provides ownership, exhaustive matching, explicit errors, strong static typing, and machine-readable diagnostics. `strictrs` focuses on the missing feedback channel: compact, stable diagnostics that an agent can consume in a check → patch → re-check loop.

## Current status

M0 is implemented on this branch:

- wraps `cargo check --message-format=json`
- emits one deterministic JSON document
- preserves exact primary spans
- includes compiler suggestions only when mechanically applicable
- removes non-actionable compiler noise
- normalizes paths for stable snapshots
- sorts diagnostics by `(file, line, col, code)`
- includes a fixture with four simultaneous compiler errors
- snapshot-tests the exact diagnostic contract

## Usage

Run the checker against a Cargo project:

```bash
cargo run -- check path/to/project
```

The command exits with status `0` only when no errors are present. Diagnostics are written to stdout as JSON.

Example shape:

```json
{
  "ok": false,
  "error_count": 1,
  "warning_count": 0,
  "diagnostics": [
    {
      "level": "error",
      "source": "rustc",
      "code": "E0599",
      "message": "no method named `frist` found",
      "at": {
        "file": "src/main.rs",
        "line": 15,
        "col": 15,
        "end_line": 15,
        "end_col": 20,
        "snippet": "let x = v.frist();"
      },
      "fixes": [
        {
          "hint": "there is a method with a similar name",
          "replace_with": "first",
          "line": 15,
          "col": 15,
          "end_line": 15,
          "end_col": 20
        }
      ]
    }
  ]
}
```

## Development

Required checks:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

Fixtures live under `fixtures/`. Each deliberately broken project should have a matching golden JSON file. Changes to the output schema or ordering are breaking changes and must update the relevant snapshots intentionally.

See [`AGENTS.md`](AGENTS.md) for repository-specific implementation rules.

## Roadmap

- **M0:** deterministic `cargo check` diagnostic oracle
- **M1:** strict-subset lint pass, using Clippy where possible
- **M2:** mechanical fix loop with no-progress detection
- **M3:** footprint-locked project template
- **M4:** property-testing integration

M0 and M1 are the priority. New syntax, a custom parser, a rustc fork, and macro-based language extensions are explicit non-goals.
