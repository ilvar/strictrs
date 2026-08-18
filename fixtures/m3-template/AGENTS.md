# AGENTS.md

Guidance for AI agents and contributors. This project was scaffolded by
`strictrs new` and is built on the strictrs strict Rust profile.

## Commands

```bash
strictrs check .                                             # strict profile oracle (JSON)
cargo fmt
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --locked
```

`strictrs check .` must report `"ok": true` before a change is done. Do not
suppress or mask diagnostics.

## Hard constraints (enforced by strictrs + lints)

- No `unsafe`.
- No panicking APIs in non-test code: no `unwrap`, `expect`, or unchecked
  indexing. Return/handle `Result`/`Option`; use `.get(...)`, `ok_or_else`, `?`.
- No numeric `as` casts — use `TryFrom`/`From`.
- No glob imports — import each item explicitly.
- Every public function has an explicit return type.
- Handle or explicitly discard `#[must_use]` values.
- Keep filesystem/network/process/environment access inside a module preceded by
  the exact comment `// strictrs: capability`.
- Match arms over locally-defined enums must name every variant; no `_` catch-all.

## Tooling

- `.pre-commit-config.yaml` mirrors the CI gate (fmt, clippy, test, strictrs,
  shellcheck, bump-version). Run `pre-commit run --all-files` before pushing.
- `scripts/commit.sh "msg"` runs that gate, bumps the version, and commits
  (`-p` to push, `-n` to skip checks).
- `scripts/bump_version.py` patch-bumps the version when a staged file reaches
  the image, so the moving `v<version>` image tag is never overwritten in place.
- `.github/workflows/ci.yml` compiles once, tests, runs `strictrs check`, and
  builds and smoke-tests the Docker image. Add image publishing separately.
- `Dockerfile`: `from-source` compiles locally; `from-artifact` copies the
  CI-built binary from `dist/`. Both share a `gcr.io/distroless/cc-debian12:nonroot` runtime.
