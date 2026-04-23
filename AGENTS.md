# AGENTS.md

This file describes the current project conventions for the `tailcall` workspace.

## Workspace Overview

The workspace is intentionally split by responsibility:

- `tailcall`
  - Published runtime crate.
  - Owns runtime implementation and runtime-internal unit tests.
- `tailcall-proc-macro`
  - Proc-macro crate directory in this repo.
  - Published package name remains `tailcall-impl`.
  - Owns macro analysis, lowering, expansion, and proc-macro unit tests.
- `std-integration`
  - Non-published downstream `std` crate.
  - Owns public API integration tests and benchmarks.
- `no-std-integration`
  - Non-published downstream `#![no_std]` crate.
  - Owns public API integration tests for no-std usage.

Do not add new public integration tests back under `tailcall/tests`; that bucket was intentionally removed so each location has a clear purpose.

## Toolchain

The repo is pinned with `rust-toolchain.toml`.

- Plain `cargo` commands in the repo should use the pinned toolchain.
- Prefer using plain `cargo ...` rather than manually specifying `+toolchain` unless you are intentionally checking another toolchain.

Current pinned toolchain:

- `1.94.1`
- components: `rustfmt`, `clippy`

The workspace uses:

- Edition `2021`
- `resolver = "2"`

## Where Tests Belong

Use the following rules when adding coverage.

### `tailcall/src/...`

Keep tests here only for runtime internals:

- layout/size invariants
- slot storage behavior
- destructor/drop behavior
- panic message behavior for runtime internals
- other private implementation details

### `tailcall-proc-macro/src/...`

Keep tests here only for proc-macro internals:

- syntax recognition
- analyzer eligibility rules
- loop lowering
- token expansion shape
- hidden helper naming / white-box expansion details

### `std-integration/tests/...`

Put public API behavior tests here:

- `#[tailcall]` usage from a downstream crate
- methods, mutual recursion, borrowed input, `Option` / `Result`, recursive examples
- stack-depth behavior
- direct runtime API usage from a downstream crate

Public integration tests should avoid asserting implementation details like generated hidden helper names unless there is a strong reason. Hidden helper names are implementation details, not the public contract.

### `no-std-integration/tests/...`

Put downstream `#![no_std]` coverage here:

- macro usage in no-std
- direct runtime usage in no-std
- deep recursion behavior in no-std

Keep this crate focused. Do not duplicate the full std integration suite here without a specific no-std reason.

## Benchmarks

Benchmarks live in `std-integration/benches`.

- Use Criterion, not `bencher`.
- Treat benchmarks as downstream/public-usage performance checks.
- Do not benchmark generated hidden helper functions directly from public integration crates.
- Prefer benchmarking:
  - handwritten loop baselines
  - manual `runtime::Thunk` usage
  - public `#[tailcall]` functions
  - mutual recursion and representative public API cases

Current bench command:

```bash
cargo bench -p std-integration
```

For compile-only verification:

```bash
cargo bench -p std-integration --no-run
```

## CI Expectations

CI currently runs:

- `cargo bench --all`
- `cargo build --all --release`
- `cargo fmt --all -- --check`
- `cargo clippy --all`
- `cargo test --all`
- `cargo +nightly miri test --all`
- `cargo doc --no-deps --all`

Keep new changes compatible with those checks unless the workflow is intentionally updated.

## Release / Versioning

Only the published crates carry release versions:

- `tailcall/Cargo.toml`
- `tailcall-proc-macro/Cargo.toml`

The root workspace manifest does not own the crate version anymore.

When doing a release:

1. Update versions in:
   - `tailcall/Cargo.toml`
   - `tailcall-proc-macro/Cargo.toml`
2. Run release checks:
   - `cargo test`
   - `cargo test --doc`
   - `cargo doc --no-deps`
3. Commit the release bump.
4. Publish `tailcall-impl` first.
5. Publish `tailcall` second.
6. Tag and push the release commit.

`tailcall` depends on the published proc-macro package `tailcall-impl` by explicit path + version, so both crate versions must stay aligned for release.

## Documentation / README Conventions

The README now reflects the workspace structure and pinned toolchain.

When updating docs:

- keep Development and Publishing sections aligned with actual workspace structure
- do not reintroduce references to removed `std-smoke` / `no-std-smoke` crates
- do not describe a root workspace version that no longer exists

## Notes For Future Changes

- If a change only affects runtime internals, prefer runtime unit tests over new integration tests.
- If a change only affects macro expansion shape, prefer `tailcall-proc-macro` unit tests.
- If a change affects how downstream users write or run recursive code, prefer integration coverage.
- Be cautious with tests that depend on compiler-specific closure layout details; those can be brittle across toolchains.
