---
name: solidcraft-wasm-build
description: >-
  Build, wrap, package and lint a SolidCraft plugin's WebAssembly component: cargo xtask component / dist / plugin-test (cargo build --release --locked --target wasm32-unknown-unknown, wit-component wrapping, the installable zip), the release profile and 64 MiB component limit, and the fmt/clippy/deny gates. Use when building the .wasm, packaging the install zip, fixing a wasm-target clippy error, or shrinking component size.
license: MIT
---

# Building and packaging the component

SolidCraft runs plugins as **components** for `wasm32-unknown-unknown` (no WASI, no threads, no
network). A bare module is not enough: it must be wrapped with the component model.

## Commands (this repo's xtask, see `xtask/src/main.rs`)

```sh
cargo xtask component     # build + wrap  -> plugin/revmesh/revmesh.wasm
cargo xtask dist          # component + zip -> target/dist/revmesh-<version>.zip (installable)
cargo xtask plugin-test   # component + solidcraft-cli plugin test --require-cases revmesh
```

What `component` does, so you can reproduce it by hand or for a new crate:

1. `cargo build --release --locked --target wasm32-unknown-unknown -p <plugin-crate>`
2. Wrap the module with `wit-component` (a dependency of `xtask`):
   `wit_component::ComponentEncoder::default().module(&module).validate(true).encode()`.
   Equivalent CLI: `wasm-tools component new target/wasm32-unknown-unknown/release/<crate_with_underscores>.wasm -o plugin/<id>/<module>.wasm`
3. Write the component to `plugin/<id>/<module>.wasm`, where `<module>` is the bare file name from
   `plugin.json`'s `runtime.module` (xtask rejects path separators and non-`.wasm` names).

`dist` zips exactly `plugin.json` + the component (no tests, no hidden files) into
`target/dist/<id>-<version>.zip` — that zip is what **Preferences › Plugins › Install plugin…**
takes, and what CI attaches to GitHub releases.

## Size and profile

- **64 MiB component limit** — CI's `component` job fails above 67,108,864 bytes
  (`.github/workflows/ci.yml`). Keep the root `[profile.release]` in `Cargo.toml`:
  `opt-level = "s"`, `lto = true`, `strip = true`, `codegen-units = 1`.
- Prefer small, dependency-light crates for the plugin path; `cargo deny check` (deny.toml)
  gates licenses, advisories, bans and sources. The binary STL reader etc. belong in the core
  crate; the component only pays for what it links.
- Measure: after `cargo xtask dist`, `stat -c %s plugin/revmesh/revmesh.wasm`.

## Lint and test gates (run locally before opening a PR — see CONTRIBUTING.md)

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p revmesh-plugin --target wasm32-unknown-unknown -- -D warnings   # CI also adds -p revmesh-core
cargo test --workspace
cargo deny check
cargo llvm-cov --workspace --fail-under-lines 80   # coverage gate >= 80% lines, whole workspace
cargo xtask dist          # exercises the component build + 64 MiB gate locally
cargo xtask plugin-test   # needs solidcraft-cli (set SOLIDCRAFT_CLI)
```

CI sets `RUSTFLAGS=-D warnings`, runs tests on Linux/Windows/macOS, an MSRV job from
`rust-version` in the root `Cargo.toml`, and `cargo doc -p revmesh-core --no-deps` with
`RUSTDOCFLAGS=-D warnings` (so keep doc comments valid).

## wasm-specific rules

- Target `wasm32-unknown-unknown` only; it is in the root `rust-toolchain.toml`
  (`targets = ["wasm32-unknown-unknown"]`, profile minimal) and rustup installs it automatically.
- No WASI, no threads, no network, no subprocesses, no system randomness → write deterministic
  code (fixed seeds), and **panics abort** (no unwinding across the boundary): return `Result`,
  never `unwrap`/`expect` in plugin code (workspace lints warn on `unwrap_used`).
- The plugin crate is `#![cfg(target_arch = "wasm32")]`: on native targets it compiles empty, so
  `cargo build`/`cargo test` natively never exercise it — that is why the wasm clippy target and
  `cargo xtask plugin-test` exist.
- Workspace lints deny `unsafe` outside the generated `wit_bindgen` bindings.

## References in this repo

- `xtask/src/main.rs` (component/dist/plugin-test, module-name validation)
- `Cargo.toml` (workspace members, lints, `[profile.release]`), `rust-toolchain.toml`, `.cargo/config.toml`
- `.github/workflows/ci.yml` (fmt/clippy/test/msrv/docs/deny/component/plugin-test jobs, 64 MiB check)
- `deny.toml`, `CONTRIBUTING.md`
