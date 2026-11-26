---
name: solidcraft-plugin-testing
description: >-
  Test a SolidCraft Rust/WASM plugin at the right level: native cargo test for the core crate (synthetic meshes, golden JSON files), headless solidcraft-cli eval on exported designs, and end-to-end plugin tests via cargo xtask plugin-test (solidcraft-cli plugin test --require-cases on plugin/<id>/tests/*.json cases with generated fixtures), plus the CI gates to run before a PR. Use when adding a recogniser, a fixture, a test case, or debugging a plugin-test failure.
license: MIT
---

# Testing SolidCraft plugins

Three levels (`docs/architecture.md` "Testing levels"); pick the narrowest that covers the change.

## Plugin-interface conformance — always check this

This skill touches SolidCraft's plugin boundary, so before committing any change here, check it
against the host's own documentation: `docs/plugins.md` in
[tangency-labs/solidcraft](https://github.com/tangency-labs/solidcraft) at the pinned release
[tag `v0.50.0`](https://github.com/tangency-labs/solidcraft/blob/v0.50.0/docs/plugins.md)
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

- Verify the package the way the host sees it: `solidcraft-cli exec plugin.check '<zip>'` for the
  problems list, a dry-run `plugin.install` for the approval/archive hashes, and `plugin.tools` /
  `engine.describe` for the resolved entries, input schemas and slots.
- Headless runs reuse grants only with `--keep-plugin-approvals` (plus `--plugins-dir` for CI);
  `plugin.grants` then shows `current` / `changed` / `not_installed` / `unreadable` per plugin.
- `cargo xtask plugin-test` runs the CLI downloaded at `SOLIDCRAFT_VERSION`; after a version bump,
  re-run it and re-check plugins.md at the new tag (the behaviour of `file.import`, import reports
  and limits may have changed).

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
cargo llvm-cov --workspace --fail-under-lines 80   # coverage gate >= 80% lines, whole workspace (llvm-tools-preview + cargo-llvm-cov)
cargo xtask dist          # component build + 64 MiB size gate
cargo xtask plugin-test   # if you have SolidCraft
```

## Coverage gate: at least 80% lines, whole workspace

The CI `coverage` job (`.github/workflows/ci.yml`) is a required check (`ci-ok` needs it) and
fails below 80% total line coverage: `cargo llvm-cov --workspace --lcov --fail-under-lines 80`
(exit 1 under the threshold). The lcov is also uploaded to Codecov (badge in the README). Run it
locally with `llvm-tools-preview` and `cargo-llvm-cov` installed. Current coverage is about 95%;
when the gate trips, the `coverage-lcov` artifact has per-file line numbers. New code must come
with tests that keep coverage >= 80%. The whole workspace is measured, no source excluded — the
plugin crate is empty on native targets, so effectively `revmesh-core` plus `xtask` count.

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
