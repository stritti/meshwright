---
name: solidcraft-host-calls
description: >-
  Call SolidCraft's host interfaces from Rust/WASM correctly and map every failure to the right plugin-error variant: source/design/evaluate (chunked file I/O and self-checking), log, control, clock, files, storage, commands, forms, design-state; options/params arrive as JSON strings; keep the plugin crate thin and panic-free. Use when writing any host call, error mapping, progress reporting, or the plugin-core split in crates/revmesh-plugin/src/lib.rs.
license: MIT
---

# Host calls and error mapping

Rule of thumb (see `docs/architecture.md`): **the plugin crate only moves bytes between
SolidCraft and the core crate**. All logic lives in `revmesh-core`, which compiles and tests
natively; the plugin crate is `#![cfg(target_arch = "wasm32")]` and empty elsewhere.

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

- Every host call used must exist in the vendored WIT and be granted by a declared capability.
  `commands::run` may only run the read-only commands listed in the manifest — except
  `inspect.classify_points`, which any `read` entry may call without a command-group grant.
- `design-state::get-token()` (capability `design@0.1`) returns an opaque
  `{document_id, revision}` token or `null`: compare the whole token, pass it back unchanged as
  `expected_design` on editing calls, discard it when the design closes; a mismatch is refused
  with typed `stale_revision` data (`expected` vs `actual`).
- Map every `plugin-error` variant the host can produce or send, including `unsupported` (added
  in 0.2) and host refusals such as `stale_revision`.

## Using the interfaces

- `log::log(level, message)` — `level` is `debug|info|warn|error`; use it for diagnostics the
  user can read in the plugin log.
- `control::progress(fraction: f32, message)` in long loops; `control::cancelled()` → return
  `PluginError::Cancelled`.
- `clock::now()` — milliseconds since the epoch (host clock; the sandbox has no other time).
- `source::read("", offset, len)` / `size` / `resolve` — the imported file and its folder
  (see `solidcraft-wasm-limits` for the chunked loop).
- `design::write(bytes)` — append the design file, ≤ 1 MiB per call, in order. Wrap it in an
  `io::Write` adapter so `serde_json`/your writer can stream into it (`HostWriter` in
  `crates/revmesh-plugin/src/lib.rs`).
- `evaluate::stage(bytes)` / `run(command, args)` / `reset()` — self-check: stage your produced
  design (≤ 1 MiB per call, 256 MiB total) and run allowed read-only commands
  (`document.inspect`, `inspect.measure`, `model.edges`, `model.faces`, `timeline.roll_to`).
  A failing command returns `{"error": "..."}` as JSON, not a `plugin-error`. Runs in a private
  session — nothing touches the user's designs or files. Requires capability `evaluate@0.2`.
- `files::read-file(grant, path)` / `write-file(grant, path, data)` — host files the user granted
  in the install review.
- `storage::put-blob(name, data, cache, append)` / `get-blob(plugin?, name, offset, len)` /
  `get-document(plugin?, offset, len)` — the plugin's own files and main document inside the
  open design (capability `storage@0.1`); reads ≤ 1 MiB; `append` writes large files in parts.
- `commands::run(command, args)` — the app's command surface (same inputs/results as its MCP
  clients). Only the read-only commands listed in the manifest may run. Importers don't import
  `commands`; exporters use it to read the open design.
- `forms::open(entry, params)` (capability `forms@0.1`) and `design-state::get-token()`
  (capability `design@0.1`, opaque `{document_id, revision}` or null) are additive imports —
  omit them in components that don't need them.

## Errors: map every failure to the right variant

`types::plugin-error` (see `solidcraft-wit-worlds`):

- **Broken input** → `PluginError::InvalidInput(FieldFailure { field, reason })` — e.g. unparseable
  STL (`field: "file"`) or malformed options JSON (`field: "options"`). Helper pattern:
  `fn invalid(field: &str, reason: impl ToString) -> PluginError`.
- **Valid but not handled** → `PluginError::Unsupported(text)` in the user's words (no solids,
  newer file version, unconverted feature). Added in interface 0.2.
- **Internal failure** → `PluginError::Trap(string)`. `std::io::Error` cannot cross the boundary:
  keep the real host error in a side channel (e.g. `HostWriter.error: Option<PluginError>`) and
  return `Trap` with context.
- **User cancelled** → `PluginError::Cancelled` when `control::cancelled()` is true.
- **Over your ceiling** → `PluginError::LimitExceeded(text)` after checking `FileInfo.size`.
- Host-side `PermissionDenied` / `CommandFailed` from `files`/`commands` calls → propagate as-is.

## Data crossing the boundary

- `types::json` is a `string`: `import-file(entry, file, options: String)` — parse `options` and
  validate it, return `invalid-input` on error; the return value is the loss-report JSON string.
- Everything is owned: `String`, `Vec<u8>` (`list<u8>`), records by value. No lifetimes/borrows.
- **Never panic**: panics abort the component (no unwinding on `wasm32-unknown-unknown`). Return
  `Result` everywhere; workspace lints warn on `unwrap_used`. `unsafe` is denied outside the
  generated bindings.

## References in this repo

- `crates/revmesh-plugin/src/lib.rs` — the whole pattern in ~140 lines: options parsing,
  `read_mesh` chunk loop, `HostWriter`, `invalid()` helper, `export!(Importer)`
- `crates/revmesh-plugin/wit/plugin.wit` — interface contracts and payload caps
- `docs/architecture.md` — "Rules of thumb"
