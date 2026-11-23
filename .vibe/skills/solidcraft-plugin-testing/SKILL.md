---
name: solidcraft-plugin-testing
description: Test a SolidCraft Rust/WASM plugin at the right level: native cargo test for the core crate (synthetic meshes, golden JSON files), headless solidcraft-cli eval on exported designs, and end-to-end plugin tests via cargo xtask plugin-test (solidcraft-cli plugin test --require-cases on plugin/<id>/tests/*.json cases with generated fixtures), plus the CI gates to run before a PR. Use when adding a recogniser, a fixture, a test case, or debugging a plugin-test failure.
license: MIT
---

# Testing SolidCraft plugins

Three levels (`docs/architecture.md` "Testing levels"); pick the narrowest that covers the change.

## 1. Native unit tests — `cargo test --workspace`

- The plugin crate compiles to **nothing** on native targets (`#![cfg(target_arch = "wasm32")]`),
  so all logic must live in the core crate and be tested there.
- Build inputs synthetically: `crates/revmesh-core/src/synth.rs` tessellates known parts the way
  CAD exporters do; tests live one file per concern under `src/plate/tests/`, `src/recognize/tests.rs`,
  `src/design/tests.rs`.
- Golden designs in `crates/revmesh-core/tests/data/*.json` are **regenerated, never edited by
  hand** — the failing test's comment names the command (examples: `params`, `plate_design`).

## 2. Headless checks of a produced design

```sh
cargo run -q -p revmesh-core --example params -- part.stl                    # parameters + report
cargo run -q -p revmesh-core --example params -- part.stl --design out.json  # and the design JSON
solidcraft-cli eval out.json                                                 # open + measure headless
solidcraft-cli eval script.json --in out.json   # after parameters.change: proves parameters rebuild
```

`docs/development.md` covers this and the mesh2cad comparison
(`scripts/compare-mesh2cad.sh plugin/revmesh/tests/*.stl` — include the table in a PR that adds
or changes a recogniser).

## 3. End-to-end in the real host — `cargo xtask plugin-test`

- Builds the component, then runs `solidcraft-cli plugin test --require-cases <id>` in `plugin/`
  (see `xtask/src/main.rs`): the real component is installed in a scratch folder and every case in
  `plugin/<id>/tests/*.json` is imported through SolidCraft's `file.import`; bodies, features, the
  loss report and the measured volume are checked.
- Add a case as `plugin/<id>/tests/<name>.json` plus its fixture (`.stl`); fixtures are
  **generated, never hand-edited** (`crates/revmesh-core/examples/plate_stl.rs` writes them from a
  spec and prints the volume SolidCraft should measure). See `plugin/revmesh/tests/`
  (`plate-holes.*`, `solidcraft-l-bracket.*`, …).
- Locating the CLI: `$SOLIDCRAFT_CLI`, else `PATH`, else the default Windows install
  (`C:\Program Files\SolidCraft`, also via WSL `/mnt/c/...`). CI downloads it from the
  `tangency-labs/solidcraft` release pinned by `SOLIDCRAFT_VERSION` (`.github/workflows/ci.yml`,
  job `plugin-test`).

## CI gates to run locally first (CONTRIBUTING.md)

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p revmesh-plugin -p revmesh-core --target wasm32-unknown-unknown -- -D warnings
cargo test --workspace
cargo deny check          # cargo install cargo-deny
cargo xtask dist          # component build + 64 MiB size gate
cargo xtask plugin-test   # if you have SolidCraft
```

CI also runs tests on Windows and macOS, an MSRV job (`rust-version` in the root `Cargo.toml`),
and `cargo doc -p revmesh-core --no-deps` with `-D warnings` on rustdoc.

## Pitfalls

- Code in the plugin crate is exercised **only** by `cargo xtask plugin-test` (and wasm clippy) —
  keep it to byte-moving and error mapping; logic belongs in the core.
- A `--locked` mismatch or a stale component (`plugin/revmesh/revmesh.wasm`) makes plugin-test
  test the wrong binary: rerun `cargo xtask component` first.
- Golden files and fixtures drift when recognition changes: regenerate them with the documented
  command and commit the result together with the recogniser.

## References in this repo

- `docs/architecture.md` ("Testing levels"), `docs/development.md` (examples, fixtures, mesh2cad comparison)
- `crates/revmesh-core/src/synth.rs`, `crates/revmesh-core/examples/` (`params.rs`, `plate_stl.rs`, `plate_design.rs`)
- `plugin/revmesh/tests/` (cases + fixtures), `xtask/src/main.rs` (`plugin-test`)
- `.github/workflows/ci.yml`, `CONTRIBUTING.md`
