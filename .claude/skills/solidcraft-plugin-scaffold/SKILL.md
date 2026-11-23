---
name: solidcraft-plugin-scaffold
description: Scaffold a new SolidCraft plugin in this workspace: a Rust crate compiled to a WebAssembly component for wasm32-unknown-unknown, the vendored craft:plugin WIT interface, the plugin.json manifest and the xtask build wiring. Use when creating a new plugin crate or plugin package, adding a manifest entry, or setting up a fresh clone's toolchain.
license: MIT
---

# Scaffolding a SolidCraft plugin

A SolidCraft plugin is a WebAssembly **component** (not a bare module) built for
`wasm32-unknown-unknown`, packaged with a `plugin.json` manifest into a zip that users install
via **Preferences › Plugins › Install plugin…**. This repo is the reference implementation:
`revmesh` (`crates/revmesh-plugin` + `plugin/revmesh` + `xtask`).

## Layout to create

```text
crates/<name>-plugin/
  Cargo.toml          # cdylib, publish = false, deps: <name>-core + wit-bindgen
  wit/plugin.wit      # vendored craft:plugin interface (see solidcraft-wit-worlds)
  src/lib.rs          # #![cfg(target_arch = "wasm32")] + wit_bindgen::generate! + export!
plugin/<id>/
  plugin.json         # the manifest
  tests/              # optional: *.json cases + fixtures for `cargo xtask plugin-test`
xtask/src/main.rs     # add component/dist/plugin-test tasks for the new crate/package
```

## Steps

1. **Workspace**: add the crate to `members` in the root `Cargo.toml` (only to
   `default-members` if native `cargo test`/`clippy` should cover it — plugin crates are empty
   on native targets, so they usually are not). Inherit `version`, `edition`, `license` and
   `[lints] workspace = true` from `[workspace.package]`.
2. **Crate `Cargo.toml`**:

   ```toml
   [lib]
   crate-type = ["cdylib"]

   [dependencies]
   <name>-core.workspace = true   # all testable logic lives here
   wit-bindgen.workspace = true    # "0.63"

   [lints]
   workspace = true
   ```

   `publish = false`: plugins are not published to crates.io.
3. **Vendor the WIT**: copy `plugin.wit` from the SolidCraft release pinned in
   `SOLIDCRAFT_VERSION` in `.github/workflows/ci.yml` (download the
   `solidcraft-<version>-<platform>` release of `tangency-labs/solidcraft`) into
   `crates/<name>-plugin/wit/plugin.wit`. The `solidcraft-version.yml` workflow re-vendors it
   when SolidCraft releases; update `SOLIDCRAFT_VERSION` in the same PR.
4. **`src/lib.rs` skeleton**:

   ```rust
   #![cfg(target_arch = "wasm32")] // empty crate on native targets: logic is tested in core
   #![allow(unsafe_code, missing_docs, unreachable_pub, clippy::pedantic)] // generated bindings

   wit_bindgen::generate!({ path: "wit", world: "importer-plugin" });

   // ... implement the world's Guest trait ...

   export!(MyPlugin);
   ```

5. **Toolchain**: the root `rust-toolchain.toml` already pins stable + rustfmt + clippy and
   adds the `wasm32-unknown-unknown` target; `rustup` installs it on first `cargo` use. The root
   `.cargo/config.toml` already aliases `cargo xtask`.
6. **xtask**: extend `xtask/src/main.rs` so `component`/`dist`/`plugin-test` take the crate and
   package names (see how `PLUGIN_CRATE`/`PACKAGE_DIR` are used for revmesh).
7. **Manifest** `plugin/<id>/plugin.json`:

   ```json
   {
     "id": "<id>", "name": "<id>", "version": "0.1.0",
     "description": "…",
     "capabilities": ["formats@0.2"],
     "runtime": {
       "module": "<id>.wasm",
       "commands": [],
       "entries": [ { "name": "…", "kind": "importer", "extensions": ["stl"], "input_schema": { … } } ],
       "limits": { "fuel": 100000000000, "memory_mib": 128, "deadline_ms": 120000 }
     }
   }
   ```

   `runtime.module` must be a **bare file name** ending in `.wasm` (xtask validates: no path
   separators). `version` is managed by release-please (`x-release-please-version` markers in
   `Cargo.toml`) — do not hand-bump it. See `solidcraft-wit-worlds` for capabilities and
   `solidcraft-importer-workflow` for importer entries.
8. **Build and install**: `cargo xtask component` (writes `plugin/<id>/<id>.wasm`),
   `cargo xtask dist` (`target/dist/<id>-<version>.zip`), then install the zip in SolidCraft.

## Pitfalls

- The plugin crate must compile to *nothing* on native targets (`#![cfg(target_arch = "wasm32")]`),
  because the generated bindings export symbols a native linker refuses. Everything worth
  testing goes into the core crate. Lint it with
  `cargo clippy -p <name>-plugin --target wasm32-unknown-unknown -- -D warnings`.
- Workspace lints deny `unsafe` outside generated bindings and warn on `missing_docs`,
  `unreachable_pub`, clippy `pedantic` (with repo-specific allows) and `unwrap_used`.
- Keep the `[profile.release]` in the root `Cargo.toml` (`opt-level = "s"`, `lto`, `strip`,
  `codegen-units = 1`): the component must stay under 64 MiB (CI enforces it).

## References in this repo

- `crates/revmesh-plugin/Cargo.toml`, `crates/revmesh-plugin/src/lib.rs`, `crates/revmesh-plugin/wit/plugin.wit`
- `plugin/revmesh/plugin.json`
- `xtask/src/main.rs`
- `Cargo.toml` (workspace lints + release profile), `rust-toolchain.toml`, `.cargo/config.toml`
- `CONTRIBUTING.md` (setup and CI gates), `docs/architecture.md`
