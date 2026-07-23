# strictrs

`strictrs` is a strict subset of Rust plus a tooling layer that turns the compiler into a deterministic, machine-readable oracle for LLM code generation.

It is not a new language, parser, compiler fork, or standard library. The project keeps Rust syntax and semantics, then adds a constrained coding profile and a closed diagnostic loop for agents.

## Thesis

Rust already provides ownership, exhaustive matching, explicit errors, strong static typing, and machine-readable diagnostics. `strictrs` focuses on the missing feedback channel: compact, stable diagnostics that an agent can consume in a check → patch → re-check loop.

## Current status

M0 through M3 are implemented:

- deterministic compiler and Clippy diagnostics in one JSON document;
- stable `strictrs::` lint codes and focused source checks;
- conservative `MachineApplicable` fix iteration;
- a deterministic `strictrs new <name>` project generator;
- a committed lockfile, pinned toolchain, strict lint policy, and size-oriented MUSL release profile in every generated project.

## Install

Install the binary from a local checkout:

```bash
cargo install --path .
```

Or install the current `main` branch directly from GitHub:

```bash
cargo install --git https://github.com/ilvar/strictrs
```

Ensure Cargo's binary directory is on `PATH`—normally `$HOME/.cargo/bin`.

## Usage

Check a Cargo project:

```bash
strictrs check path/to/project
```

Apply compiler-supplied mechanical fixes and re-check until the loop stops:

```bash
strictrs fix path/to/project
```

Create a footprint-locked project in the current directory:

```bash
strictrs new hello-strictrs
```

For backward compatibility, a bare path is treated as `check`:

```bash
strictrs path/to/project
```

Every command emits exactly one final JSON report to stdout. It exits with status `0` only on success. Operational failures are written to stderr and exit with status `2`.

## M3 project template

`strictrs new <name>` creates these deterministic files:

- `.cargo/config.toml`
- `.gitignore`
- `Cargo.lock`
- `Cargo.toml`
- `README.md`
- `rust-toolchain.toml`
- `src/main.rs`

The generated manifest contains the required footprint profile:

```toml
[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

It has no third-party dependencies, commits its lockfile, pins the compiler and MUSL target, denies the supported strict-subset lints, and keeps panic APIs exempt only in test builds.

The generated small-release command is:

```bash
cargo release-small
```

**Measured release size:** pending the successful M3 CI measurement. No estimate is recorded.

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

## Development

Required checks:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

Fixtures live under `fixtures/`. Changes to the diagnostic schema, ordering, or generated project contents are breaking changes and must update their golden fixtures intentionally.

See [`AGENTS.md`](AGENTS.md) for repository-specific implementation rules.

## Roadmap

- **M0:** deterministic compiler diagnostic oracle — complete
- **M1:** strict-subset lint pass — complete
- **M2:** mechanical fix loop with no-progress detection — complete
- **M3:** footprint-locked project template — complete after the recorded size gate passes
- **M4:** property-testing integration

New syntax, a custom parser, a rustc fork, and macro-based language extensions are explicit non-goals.
