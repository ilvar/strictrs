# strictrs

`strictrs` is a strict subset of Rust plus a tooling layer that turns the compiler into a deterministic, machine-readable oracle for LLM code generation.

It is not a new language, parser, compiler fork, or standard library. The project keeps Rust syntax and semantics, then adds a constrained coding profile and a closed diagnostic loop for agents.

## Thesis

Rust already provides ownership, exhaustive matching, explicit errors, strong static typing, and machine-readable diagnostics. `strictrs` focuses on the missing feedback channel: compact, stable diagnostics that an agent can consume in a check → patch → re-check loop.

## Current status

M0, M1, and M2 are implemented:

- runs `cargo clippy --message-format=json` as the unified compiler/lint pass
- emits one deterministic JSON document
- preserves exact primary spans
- includes suggestions only when marked `MachineApplicable` by rustc or Clippy
- maps supported Clippy/rustc lints to stable `strictrs::` codes
- adds custom checks for catch-all enum arms, mutable globals, explicit public return types, and capability boundaries
- exempts test-only code from the panic-API ban
- normalizes paths and sorts diagnostics by `(file, line, col, code)`
- applies safe fixes per file, back-to-front, then re-checks
- stops the fix loop when clean, blocked, unchanged, or capped
- includes compiler-error, strict-subset, test-exemption, fixable, and no-progress fixtures

## Usage

Check a Cargo project:

```bash
cargo run -- check path/to/project
```

Apply compiler-supplied mechanical fixes and re-check until the loop stops:

```bash
cargo run -- fix path/to/project
```

For backward compatibility, a bare path is treated as `check`:

```bash
cargo run -- path/to/project
```

Both commands emit exactly one final JSON report to stdout. They exit with status `0` only when no errors remain. Operational failures are written to stderr and exit with status `2`.

The fix command:

- applies only replacements marked `MachineApplicable`
- never invents replacement text
- retains rustc byte offsets internally while leaving the public JSON schema unchanged
- groups edits by file and applies them from the end of the file toward the start
- ignores duplicate and overlapping alternative edits deterministically
- refuses edits outside the target project or across invalid UTF-8 boundaries
- uses a default cap of 10 iterations

Example diagnostic shape:

```json
{
  "ok": false,
  "error_count": 1,
  "warning_count": 0,
  "diagnostics": [
    {
      "level": "error",
      "source": "strictrs",
      "code": "strictrs::no_panic_api",
      "message": "used `unwrap()` on an `Option` value",
      "at": {
        "file": "src/main.rs",
        "line": 15,
        "col": 15,
        "end_line": 15,
        "end_col": 23,
        "snippet": "let x = values.first().unwrap();"
      },
      "fixes": []
    }
  ]
}
```

## Strict subset

| Stable code | Banned construct | Preferred alternative |
| --- | --- | --- |
| `strictrs::no_unsafe` | `unsafe` | safe Rust APIs |
| `strictrs::no_panic_api` | `unwrap`, `expect`, indexing outside tests | `Result`, `Option`, checked access |
| `strictrs::no_catchall_arm` | `_ =>` when matching a locally defined enum | explicit variants |
| `strictrs::no_as_cast` | numeric `as` casts | `TryFrom` with handled errors |
| `strictrs::no_glob_import` | glob imports | explicit imports |
| `strictrs::no_mutable_global` | `static mut` | explicitly owned state |
| `strictrs::explicit_return_type` | omitted return type on public functions | explicit `-> ()` or value type |
| `strictrs::must_handle` | unused `must_use` values | handle or explicitly discard |
| `strictrs::capability_boundary` | filesystem/network/process calls outside boundaries | isolated capability module |

Capability modules are marked without introducing active custom Rust syntax:

```rust
// strictrs: capability
mod filesystem {
    pub fn load(path: &std::path::Path) -> std::io::Result<String> {
        std::fs::read_to_string(path)
    }
}
```

## Development

Required checks:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

Fixtures live under `fixtures/`. Each deliberately broken project should have focused assertions or a matching golden JSON file. Changes to the output schema or ordering are breaking changes and must update snapshots intentionally.

See [`AGENTS.md`](AGENTS.md) for repository-specific implementation rules, [`docs/M1.md`](docs/M1.md) for lint-pass notes, and [`docs/M2.md`](docs/M2.md) for fix-loop behavior.

## Roadmap

- **M0:** deterministic compiler diagnostic oracle — complete
- **M1:** strict-subset lint pass — complete
- **M2:** mechanical fix loop with no-progress detection — complete
- **M3:** footprint-locked project template
- **M4:** property-testing integration

New syntax, a custom parser, a rustc fork, and macro-based language extensions are explicit non-goals.
