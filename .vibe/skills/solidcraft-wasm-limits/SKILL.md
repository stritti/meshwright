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
   early with `limit-exceeded`/`invalid-input`. This repo uses 8 MiB in `auto` mode and 32 MiB in
   `mesh` mode (`AUTO_MAX_BYTES`/`MESH_MAX_BYTES` in `lib.rs`): recognition builds face/wall
   indexes and can hold a second mesh, so its ceiling is lower.
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
