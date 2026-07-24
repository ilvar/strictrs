---
name: strictrs
description: Use strictrs to create, check, and conservatively fix Rust projects with deterministic JSON diagnostics and strict-subset rules.
---

# strictrs

Use this skill whenever you create or modify Rust code in a project that uses `strictrs`.

## Required workflow

1. Run `strictrs --help` and treat its embedded manual as the current source of truth.
2. Inspect the project and make the smallest coherent change that advances the task.
3. Run `strictrs check <path>` and parse the single JSON report on stdout.
4. Address every error diagnostic. Never invent a fix or applicability level.
5. Use `strictrs fix <path>` only for compiler- or Clippy-supplied `MachineApplicable` replacements.
6. Repeat the check until `ok` is `true`.
7. Run formatting, Clippy, unit tests, and property tests before submitting changes.
8. Inspect the final diff and do not submit known failures.

## Strict-subset expectations

Avoid unsafe Rust, panic APIs outside tests, catch-all arms for local enums, numeric `as` casts, glob imports, mutable globals, omitted public return types, ignored `must_use` values, and unmarked filesystem/network/process access.

Keep capability access inside modules preceded by the exact marker:

```rust
// strictrs: capability
mod filesystem {
    // capability implementation
}
```

## Generated projects

Use `strictrs new <name>` for a deterministic project pinned to Rust 1.97.1 with strict lints, a locked property-test scaffold, a MUSL release target, and the footprint-oriented release profile.
