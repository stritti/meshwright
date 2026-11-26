---
name: solidcraft-wasm-limits
description: >-
  Respect and design around SolidCraft's WebAssembly runtime limits: memory (128 MiB default, limits.memory_mib), fuel and deadline (limits.fuel / limits.deadline_ms), the 64 MiB component and 256 MiB design caps, and the 1 MiB per-host-call ceiling — with the streaming patterns (chunked source::read / design::write, progress + cancellation, no double buffering, f32 coordinates) that make large imports fit. Use when writing the import loop, choosing input size ceilings, or debugging out-of-memory traps.
license: MIT
---

# Runtime limits and streaming discipline

SolidCraft runs the component in a sandbox with hard budgets. `docs/architecture.md` ("Limits
inside SolidCraft") is the source of truth; the manifest declares what the plugin asks for
(`plugin/revmesh/plugin.json` → `runtime.limits`).

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

- Declared `runtime.limits` must fit the host ceilings: `fuel` ≤ 10,000,000,000,000 (hard approval
  ceiling; the user's default Max plugin fuel is 100,000,000,000), `memory_mib` ≤ the 128 MiB
  default ceiling unless the user approves more at install, `deadline_ms` ≤ 10 min.
- If the plugin uses listeners (`design.changed` / `design.links_changed`) or an importer
  `dialog_hook`, their separate smaller budgets apply too (listeners: ≤ 1,000,000 fuel, 16 MiB
  memory, 512 KiB stack, 100 ms; dialog hook: ≤ 5,000,000 fuel, 500 ms, 64 KiB prefix reads) —
  confirm against plugins.md which of these exist at the pinned tag.
- Per-call payload caps (1 MiB per `source::read` / `design::write` / `evaluate::stage` / storage
  read, 256 MiB design file, 256 MiB export bytes, 1 MiB JSON reply) are host-side and cannot be
  raised — design around them.

## The budgets

| Limit | Default | Max | Declared as |
|---|---|---|---|
| Memory | 128 MiB | 4 GiB, only with user approval at install | `limits.memory_mib` |
| Fuel | — | — | `limits.fuel` (this repo: 100000000000) |
| Deadline | — | 10 min | `limits.deadline_ms` (this repo: 120000 = 2 min) |
| Component size | 64 MiB | — | enforced by CI, not the manifest |
| Design file | 256 MiB | — | enforced by the host (`design.write`, `evaluate.stage` total) |
| Per host call | 1 MiB payload | — | `source.read` len, `design.write` bytes, `evaluate.stage` bytes, `storage.get-blob`/`get-document` len |

## Rules that make big inputs fit

1. **Never hold the data twice.** Stream the file in ≤ 1 MiB chunks and parse incrementally
   (`source::read("", offset, CHUNK)` + `StlReader` in `crates/revmesh-plugin/src/lib.rs`), move
   the mesh into the result without copying, and serialise the design straight into 1 MiB
   `design.write` calls via an `io::Write` adapter (the `HostWriter` in `lib.rs`: buffer to
   `CHUNK = 1 << 20`, flush with `design::write`, keep a host error in an `Option` because
   `io::Error` cannot carry a `PluginError`).
2. **Check size before reading.** Use `source::FileInfo.size` (or `source::size("")`) and refuse
   early with `limit-exceeded`/`invalid-input`. This repo uses 16 MiB in `auto` mode and 32 MiB
   in `mesh` mode (`AUTO_MAX_BYTES`/`MESH_MAX_BYTES` in `lib.rs`): recognition builds face/wall
   indexes on top of the mesh (~250 bytes per triangle in all, so 16 MiB peaks at ≈ 90 MiB), so
   its ceiling is lower.
3. **Progress and cancellation in every loop**: `control::progress(fraction, message)` and
   `if control::cancelled() { return Err(PluginError::Cancelled) }`.
4. **Write coordinates as f32** in large designs; keep recognition tolerances explicit
   (1 µm or 1e-5 of the part's size, whichever is larger).
5. **Budget the expansion**: binary STL is 50 bytes/triangle on disk, 72 bytes once expanded —
   plus face/wall indexes in recognition. Reference measurement (SolidCraft 0.49, 128 MiB):
   a 1.5 M-triangle STL (75 MB) imports as a mesh in ~8 s; 2 M triangles exceed the memory.
6. **Be deterministic**: no threads, network, subprocesses or system randomness exist in the
   sandbox — seed randomized algorithms (e.g. RANSAC) with a fixed value.

## Chunked reads of the imported file

```rust
const CHUNK: usize = 1 << 20; // largest payload one host call accepts
let mut offset = 0u64;
while offset < file.size {
    if control::cancelled() { return Err(PluginError::Cancelled); }
    let chunk = source::read("", offset, CHUNK as u32)?; // "" = the file being imported
    if chunk.is_empty() { break; }                      // EOF
    /* feed the incremental parser */
    offset += chunk.len() as u64;
    control::progress(0.4 * offset as f32 / file.size.max(1) as f32, "Reading");
}
```

`source::read` with any path but `""` addresses sibling files of the imported one and needs the
`input_folder` permission; `source::resolve(path)` maps a path to its real spelling
(case-insensitive fallback).

## Pitfalls

- `design.write` calls must arrive **in order**, ≤ 1 MiB each; an empty read at offset ≥ size
  means EOF.
- Exceeding memory traps the component — the user sees `trap`, not a helpful message, so enforce
  your own ceilings up front.
- Holding the whole file plus the parsed mesh plus the serialised design is the classic way to
  blow 128 MiB; stream all three stages.

## References in this repo

- `crates/revmesh-plugin/src/lib.rs` (`read_mesh`, `HostWriter`, `CHUNK`, size ceilings)
- `crates/revmesh-core/src/stl.rs` (`StlReader` incremental parse)
- `plugin/revmesh/plugin.json` (`runtime.limits`)
- `docs/architecture.md` (limits, measured numbers, rules of thumb)
