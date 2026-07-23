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

M0, M1, and M2 must remain usable before later milestones are expanded.

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

## Implementation rules

- Prefer boring, dependency-light Rust.
- Reuse rustc and Clippy capabilities instead of reimplementing them.
- Use Clippy configuration for strict-subset rules when an equivalent lint exists.
- Hand-roll only rules that Clippy cannot express adequately.
- Keep path normalization and diagnostic ordering stable across machines.
- Avoid panics in production code. Return structured errors.
- Do not silently discard malformed compiler messages, Cargo failures, or directory-walk errors unless they are explicitly known non-diagnostic events.
- Keep stdout reserved for the JSON contract. Human-oriented operational errors belong on stderr.
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

## Fixtures and tests

Every behavior change requires a fixture or focused unit test.

- Deliberately broken Cargo projects belong under `fixtures/`.
- Golden JSON files must snapshot the exact output contract.
- Each future lint should have one fixture per stable `strictrs::` code.
- Tests must verify deterministic ordering, exact spans, counts, source attribution, and exit status.
- Fix tests must cover multiple edits in one file, overlapping alternatives, no-progress termination, and iteration caps.
- When changing a golden file, explain why the contract changed; do not refresh snapshots blindly.
- Preserve the multi-error fixture proving that at least four simultaneous compiler errors are not masked.
- Preserve the fixture proving panic APIs are allowed in test-only code.

## Required checks

Before proposing or merging changes, run:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

GitHub Actions must enforce the same commands.

## Scope discipline

Do not begin M3 or M4 work while M0, M1, or M2 regressions remain. Avoid unrelated refactors in milestone pull requests. Keep commits and pull requests focused enough that diagnostic-contract and source-editing changes can be reviewed directly.
