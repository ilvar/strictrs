# AGENTS.md

This file defines repository-specific rules for coding agents working on `strictrs`.

## Project intent

`strictrs` is not a new programming language. It is:

1. a strict subset of Rust;
2. a tooling layer around existing Rust compiler diagnostics;
3. a deterministic feedback contract for an agent-driven check → patch → re-check loop.

Do not introduce new syntax, a parser, a rustc fork, a new standard library, or macro-based language extensions.

## Priorities

Work in this order unless an issue or pull request explicitly says otherwise:

1. **M0:** deterministic compiler diagnostic oracle;
2. **M1:** strict-subset lint pass;
3. **M2:** mechanical fix loop;
4. **M3:** footprint-locked project template;
5. **M4:** property-testing integration.

Completed milestones must remain usable before later work is expanded.

## Diagnostic contract

The JSON diagnostic shape is the public API between the toolchain and coding agents. Treat incompatible changes as breaking changes.

Required properties:

- output exactly one JSON document;
- `source` is either `rustc` or `strictrs`;
- omit non-actionable summary noise;
- include fixes only when the compiler supplies a `MachineApplicable` replacement;
- never invent a replacement;
- include start and end positions for spans and fixes;
- use deterministic ordering by `(file, line, col, code)`;
- exit with status `0` only when `error_count == 0`.

Internal metadata such as file names and rustc byte offsets may be retained with `#[serde(skip)]`, but it must not alter the public JSON shape.

## Agent-help contract

`src/help.txt` is the embedded operational manual exposed by `strictrs --help`. Treat it as a public agent interface.

It must describe:

- installation and every supported command form;
- the check → patch → re-check workflow;
- stdout/stderr behavior, report fields, deterministic ordering, and exit codes;
- every stable strict-subset code and the capability-module marker;
- conservative fix-loop safety and termination;
- generated-project contents and footprint profile;
- property-testing expectations and final validation commands;
- constraints against invented fixes, hidden diagnostics, and unvalidated pushes.

`--help`, `-h`, `help`, and subcommand help aliases must print identical text to stdout and exit successfully. Invocation errors must print only concise usage and error context to stderr. Any CLI, diagnostic, lint, template, property-testing, or validation change must update the embedded help and its regression tests in the same coherent change.

## Implementation rules

- Prefer boring, dependency-light Rust.
- Reuse rustc and Clippy capabilities instead of reimplementing them.
- Use Clippy configuration for strict-subset rules when an equivalent lint exists.
- Hand-roll only rules that Clippy cannot express adequately.
- Keep path normalization and diagnostic ordering stable across machines.
- Avoid panics in production code. Return structured errors.
- Do not silently discard malformed compiler messages, Cargo failures, or directory-walk errors unless they are explicitly known non-diagnostic events.
- Keep stdout reserved for the JSON contract, except for the explicit plain-text `--help` mode. Human-oriented operational errors belong on stderr.
- Preserve the explicit exemption for panic APIs in `#[cfg(test)]` code.

## Mechanical fix rules

M2 is intentionally conservative:

- apply only compiler- or Clippy-supplied `MachineApplicable` replacements;
- retain and use rustc byte offsets rather than reconstructing edits from display columns;
- never edit a path outside the requested project root;
- validate byte ranges and UTF-8 boundaries before modifying content;
- group edits by file and apply them back-to-front;
- deduplicate identical edits;
- when alternatives overlap, keep the first deterministic candidate and discard the rest;
- re-run the full checker after every patch pass;
- stop when clean, when no applicable edit remains, when a pass makes no progress, or when the iteration cap is reached;
- keep the final stdout value as the ordinary diagnostic report, not a separate fix-result schema.

## Project-template rules

Generated projects must:

- use the exact release footprint settings in the project specification;
- include a committed `Cargo.lock`;
- contain no unpinned third-party dependency;
- pin the Rust toolchain and MUSL target used for the size gate;
- bake the strict non-panic lint policy into `Cargo.toml`;
- preserve the test-only panic-API exemption with crate-level conditional lint attributes;
- create files in deterministic order with deterministic contents;
- refuse invalid package names and existing destination paths;
- be written through a staging directory and renamed only after every file succeeds;
- pass formatting, Clippy, tests, the `strictrs` checker, property tests, and the binary-size acceptance job.

Do not record an estimated binary size. Record only a size produced by CI from the committed template.

## Property-testing rules

M4-generated projects must:

- pin `proptest` to an exact version;
- disable default features and enable only the features required by the scaffold;
- keep property testing as a dev dependency so release size and runtime dependencies remain unchanged;
- generate `tests/properties.rs` with explicit imports that comply with the strict subset;
- state properties as invariants rather than duplicating the implementation;
- use bounded strategies so tests remain fast and reproducible in CI;
- preserve generated `proptest-regressions/` files when a failure is persisted;
- run property tests through ordinary locked `cargo test`;
- keep the generated lockfile deterministic and golden-tested.

When upgrading `proptest`, update the exact manifest pin, committed generated lockfile, golden fixture, README, and CI in one coherent change.

## Fixtures and tests

Every behavior change requires a fixture or focused unit test.

- Deliberately broken Cargo projects belong under `fixtures/`.
- Golden JSON files must snapshot the exact output contract.
- Each future lint should have one fixture per stable `strictrs::` code.
- Tests must verify deterministic ordering, exact spans, counts, source attribution, and exit status.
- Fix tests must cover multiple edits in one file, overlapping alternatives, no-progress termination, and iteration caps.
- Template tests must compare every generated file against the golden template fixture.
- Property-testing tests must verify the exact dependency pin, generated scaffold, committed lockfile, and successful locked execution.
- Help tests must verify stdout/stderr separation, successful aliases, and the presence of every required agent-manual section.
- When changing a golden file, explain why the contract changed; do not refresh snapshots blindly.
- Preserve the multi-error fixture proving that at least four simultaneous compiler errors are not masked.
- Preserve the fixture proving panic APIs are allowed in test-only code.

## Validation and commit discipline

Run the complete relevant validation set before committing or pushing:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

For generated-project changes, also validate the generated project, property suite, installed-binary path, and binary-size acceptance path.

Commit and push rules:

- assemble a complete logical change before committing;
- inspect the full diff and staged file list before the commit;
- do not commit or push known formatting, compilation, lint, or test failures;
- do not create one commit per file or one commit merely to discover a CI error;
- keep commits coherent; multiple commits are acceptable when they represent distinct logical changes;
- batch related mechanical corrections and push them together after validation;
- do not push each formatting or test correction separately and trigger a torrent of CI runs;
- use concurrency cancellation in CI so a superseded branch run stops promptly;
- remove temporary logs, generated patches, and diagnostic workflow steps before the final push;
- use CI to verify a validated change, not as a substitute for validation available locally;
- do not rewrite published history solely to reduce the visible commit count.

GitHub Actions must enforce the same commands used locally.

## Scope discipline

Avoid unrelated refactors in milestone pull requests. Keep commits and pull requests focused enough that diagnostic-contract, source-editing, generated-template, property-testing, installation, and help-contract changes can be reviewed directly.
