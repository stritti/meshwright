---
name: solidcraft-host-calls
description: Call SolidCraft's host interfaces from Rust/WASM correctly and map every failure to the right plugin-error variant: source/design/evaluate (chunked file I/O and self-checking), log, control, clock, files, storage, commands, forms, design-state; options/params arrive as JSON strings; keep the plugin crate thin and panic-free. Use when writing any host call, error mapping, progress reporting, or the plugin-core split in crates/revmesh-plugin/src/lib.rs.
license: MIT
---

# Host calls and error mapping

Rule of thumb (see `docs/architecture.md`): **the plugin crate only moves bytes between
SolidCraft and the core crate**. All logic lives in `revmesh-core`, which compiles and tests
natively; the plugin crate is `#![cfg(target_arch = "wasm32")]` and empty elsewhere.

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
