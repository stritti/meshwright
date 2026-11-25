---
name: solidcraft-wasm-build
description: >-
  Build, wrap, package and lint a SolidCraft plugin's WebAssembly component: cargo xtask component / dist / plugin-test (cargo build --release --locked --target wasm32-unknown-unknown, wit-component wrapping, the installable zip), the release profile and 64 MiB component limit, and the fmt/clippy/deny gates. Use when building the .wasm, packaging the install zip, fixing a wasm-target clippy error, or shrinking component size.
license: MIT
---

# Building and packaging the component

SolidCraft runs plugins as **components** for `wasm32-unknown-unknown` (no WASI, no threads, no
network). A bare module is not enough: it must be wrapped with the component model.

## Plugin-interface conformance — always check this

This skill touches SolidCraft's plugin boundary, so before committing any change here, check it
against the host's own documentation: `docs/plugins.md` in
[tangency-labs/solidcraft](https://github.com/tangency-labs/solidcraft) at the pinned release
[tag `v0.49.1`](https://github.com/tangency-labs/solidcraft/blob/v0.49.1/docs/plugins.md)
(`SOLIDCRAFT_VERSION` in `.github/workflows/ci.yml`, matching the vendored
`crates/revmesh-plugin/wit/plugin.wit`) and at
[`main`](https://github.com/tangency-labs/solidcraft/blob/main/docs/plugins.md) for what is
coming next. Both questions are mandatory:

1. **Compatible?** The host at `SOLIDCRAFT_VERSION` accepts what you declare and build: known
   capabilities only (unknown or newer required capabilities are refused at install, and
   `min_host_version` is checked), entry fields valid for that version, limits under the host
   ceilings (fuel ≤ 10,000,000,000,000 approved, memory ≤ the 128 MiB ceiling unless the user
   approves more, deadline ≤ 10 min, zip ≤ 256 entries and ≤ 16 MiB per file, importer component
   ≤ 64 MiB), and a component whose imports the declared capabilities cover. Prove it with
   `plugin.check`, a dry-run `plugin.install` and `cargo xtask plugin-test`.
2. **Fully utilised?** Walk the plugins.md feature list and for each feature that applies to this
   plugin either use it or record why not: capabilities beyond `formats@0.2` (`evaluate@0.2`
   self-check, `design@0.1` with `get-token`/`expected_design` and the
   `design.changed`/`design.links_changed` listeners, `storage@0.1`, `forms@0.1`, `dialog@0.1`,
   `ui@0.1` panels), `inspect.classify_points` through `commands.run` on `read` entries, manifest
   conveniences (menu/ribbon slots, `x-solidcraft-field-order`, `$defs`, `oneOf` titles,
   `x-solidcraft-visible-when`, importer `file_first`/`dialog_hook`, `updates.github`,
   `min_host_version`), the headless grant flow (`--keep-plugin-approvals`), and the complete
   `plugin-error` mapping.
   When `SOLIDCRAFT_VERSION` moves, re-run both parts against the new tag's plugins.md, diff
   `main` for upcoming interface changes, and state the outcome in the PR.

For this skill:

- The zip must pass the host's install checks: one plugin with `plugin.json` + component at the
  top or inside one top-level folder, `safe_zip` rules (no absolute paths, `..`, symlinks or
  device entries), ≤ 256 entries, ≤ 16 MiB per file (importer component ≤ 64 MiB), the manifest
  under its size cap, and `runtime.module` a bare `.wasm` file name. CI's 64 MiB gate mirrors the
  importer-component cap.
- Validate before releasing: `solidcraft-cli exec plugin.check '<zip-or-folder>'`, then a dry-run
  `plugin.install` to review permissions, limits and the approval/archive hashes. `plugin.run`
  checks stored grants against the current manifest on every call, so a changed component needs
  `plugin.install` with `replace: true` and a fresh approval.

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
