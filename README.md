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
- a committed lockfile, Rust 1.97.1 toolchain, strict lint policy, and size-oriented MUSL release profile in every generated project;
- an exact-pinned `proptest` scaffold for agent-authored invariants, shrinking, and regression persistence;
- portable agent-skill installation for detected Codex and Claude Code clients.

## Install

Install the binary from a local checkout:

```bash
cargo install --path .
```

Or install the current `main` branch directly from GitHub:

```bash
cargo install --git https://github.com/ilvar/strictrs
```

The repository and generated projects pin Rust **1.97.1**. Ensure Cargo's binary directory is on `PATH`—normally `$HOME/.cargo/bin`.

### Install the agent skill

After installing the binary, register the bundled portable skill with detected local agents:

```bash
strictrs install-skills
```

The command detects Codex through the `codex` executable or the `~/.codex`/`~/.agents` directories and installs:

```text
~/.agents/skills/strictrs/SKILL.md
```

It detects Claude Code through the `claude` executable or `~/.claude` and installs:

```text
~/.claude/skills/strictrs/SKILL.md
```

The operation is idempotent. It accepts an identical existing skill but refuses to overwrite a modified file. Remove a customized copy explicitly before reinstalling the bundled version. `cargo install` itself does not modify agent configuration or home-directory files.

## Usage

Print the complete embedded instructions for a coding agent:

```bash
strictrs --help
```

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

Install the portable skill for detected local agents:

```bash
strictrs install-skills
```

For backward compatibility, a bare path is treated as `check`:

```bash
strictrs path/to/project
```

Operational commands emit exactly one final JSON report to stdout. `--help` is the only plain-text stdout mode. Reports exit with status `0` only on success, status `1` when diagnostics remain, and status `2` for invocation or operational failures. Human-oriented failures and skill-installation notices are written to stderr.

## Generated project

`strictrs new <name>` creates these deterministic files:

- `.cargo/config.toml`
- `.dockerignore`
- `.github/workflows/ci.yml`
- `.gitignore`
- `.pre-commit-config.yaml`
- `AGENTS.md`
- `CLAUDE.md`
- `Cargo.lock`
- `Cargo.toml`
- `Dockerfile`
- `Makefile`
- `README.md`
- `rust-toolchain.toml`
- `scripts/bump_version.py`
- `scripts/commit.sh`
- `src/main.rs`
- `tests/properties.rs`

The generated project pins Rust 1.97.1 and the `x86_64-unknown-linux-musl` target. Its manifest contains the footprint profile:

```toml
[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

Generated projects have no runtime dependencies. The property-test scaffold uses an exact-pinned `proptest` dev dependency with default features disabled and only `std` enabled. The committed lockfile pins its transitive test dependencies without changing the release binary.

Alongside the crate, `new` scaffolds a development workflow that mirrors the strict gate: a `.pre-commit-config.yaml`, a `Makefile`, a reusable GitHub Actions workflow (`.github/workflows/ci.yml`: fmt, clippy, test, `strictrs check`, shellcheck, and a Docker image build + smoke test), a multi-stage `Dockerfile`, `AGENTS.md`/`CLAUDE.md`, and `scripts/` (`commit.sh`, `bump_version.py`). Image publishing is intentionally omitted — add a workflow for your own registry.

The generated small-release command uses stable Rust and the prebuilt MUSL standard library:

```bash
cargo release-small
```

**Measured stable release size:** 377,400 bytes for the generated hello-world binary targeting `x86_64-unknown-linux-musl` in GitHub Actions. CI enforces a **400,000-byte** ceiling and publishes the exact byte count as the `template-size` artifact.

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

The repository pins Rust 1.97.1 in `rust-toolchain.toml`. Required checks:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

Fixtures live under `fixtures/`. Changes to the diagnostic schema, ordering, generated project contents, embedded agent help, or bundled skill are public-contract changes and must update their focused tests intentionally.

See [`AGENTS.md`](AGENTS.md) for repository-specific implementation rules.

## Roadmap

- **M0:** deterministic compiler diagnostic oracle — complete
- **M1:** strict-subset lint pass — complete
- **M2:** mechanical fix loop with no-progress detection — complete
- **M3:** footprint-locked project template — complete
- **M4:** property-testing integration — complete

New syntax, a custom parser, a rustc fork, and macro-based language extensions are explicit non-goals.
