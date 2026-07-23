# strictrs

`strictrs` is a strict subset of Rust plus a tooling layer that turns the compiler into a deterministic, machine-readable oracle for LLM code generation.

It is not a new language, parser, compiler fork, or standard library. The project keeps Rust syntax and semantics, then adds a constrained coding profile and a closed diagnostic loop for agents.

## Thesis

Rust already provides ownership, exhaustive matching, explicit errors, strong static typing, and machine-readable diagnostics. `strictrs` focuses on the missing feedback channel: compact, stable diagnostics that an agent can consume in a check → patch → re-check loop.

## Current status

M0 through M4 are implemented:

- deterministic compiler and Clippy diagnostics in one JSON document;
- stable `strictrs::` lint codes and focused source checks;
- conservative `MachineApplicable` fix iteration;
- a deterministic `strictrs new <name>` project generator;
- a committed lockfile, pinned toolchain, strict lint policy, and size-oriented MUSL release profile in every generated project;
- an exact-pinned `proptest` scaffold for agent-authored invariants, shrinking, and regression persistence.

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

## Generated project

`strictrs new <name>` creates these deterministic files:

- `.cargo/config.toml`
- `.gitignore`
- `Cargo.lock`
- `Cargo.toml`
- `README.md`
- `rust-toolchain.toml`
- `src/main.rs`
- `tests/properties.rs`

The generated manifest contains the required footprint profile:

```toml
[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

Generated projects have no runtime dependencies. The property-test scaffold uses an exact-pinned `proptest` dev dependency with default features disabled and only `std` enabled. The committed lockfile pins its transitive test dependencies without changing the release binary.

The generated small-release command is:

```bash
cargo release-small
```

**Measured release size:** 34,912 bytes for the generated hello-world binary targeting `x86_64-unknown-linux-musl` in GitHub Actions.

## Property testing

`tests/properties.rs` is the handoff point between the coding agent and the runtime:

1. the agent states invariants over generated inputs;
2. `proptest` exercises those invariants across many cases;
3. failures are shrunk to a minimal reproducer;
4. persisted regressions remain ordinary test inputs.

Run the complete generated-project test suite:

```bash
cargo test --locked
```

Run only the property suite:

```bash
cargo test --locked --test properties
```

The generated property is deliberately small and domain-neutral. Replace it with invariants about the actual program rather than duplicating the implementation inside the test.

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
- **M3:** footprint-locked project template — complete
- **M4:** property-testing integration — complete

New syntax, a custom parser, a rustc fork, and macro-based language extensions are explicit non-goals.
