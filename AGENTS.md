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

1. **M0:** deterministic `cargo check` diagnostic oracle;
2. **M1:** strict-subset lint pass;
3. **M2:** mechanical fix loop;
4. **M3:** footprint-locked project template;
5. **M4:** property-testing integration.

M0 and M1 must remain usable before later milestones are expanded.

## Diagnostic contract

The JSON diagnostic shape is the public API between the toolchain and coding agents. Treat incompatible changes as breaking changes.

Required properties:

- output exactly one JSON document;
- `source` is either `rustc` or `strictrs`;
- omit non-actionable summary noise;
- include fixes only when the compiler supplies a mechanically applicable replacement;
- never invent a replacement;
- include start and end positions for spans and fixes;
- use deterministic ordering by `(file, line, col, code)`;
- exit with status `0` only when `error_count == 0`.

## Implementation rules

- Prefer boring, dependency-light Rust.
- Reuse rustc and Clippy capabilities instead of reimplementing them.
- Use Clippy configuration for strict-subset rules when an equivalent lint exists.
- Hand-roll only rules that Clippy cannot express adequately.
- Keep path normalization and diagnostic ordering stable across machines.
- Avoid panics in production code. Return structured errors.
- Do not silently discard malformed compiler messages unless they are explicitly known non-diagnostic events.
- Keep stdout reserved for the JSON contract. Human-oriented operational errors belong on stderr.

## Fixtures and tests

Every behavior change requires a fixture or focused unit test.

- Deliberately broken Cargo projects belong under `fixtures/`.
- Golden JSON files must snapshot the exact output contract.
- Each future lint should have one fixture per stable `strictrs::` code.
- Tests must verify deterministic ordering, exact spans, counts, source attribution, and exit status.
- When changing a golden file, explain why the contract changed; do not refresh snapshots blindly.
- Preserve the multi-error fixture proving that at least four simultaneous compiler errors are not masked.

## Required checks

Before proposing or merging changes, run:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

GitHub Actions must enforce the same commands.

## Scope discipline

Do not begin M2, M3, or M4 work while M0/M1 regressions remain. Avoid unrelated refactors in milestone pull requests. Keep commits and pull requests focused enough that diagnostic contract changes can be reviewed directly.
