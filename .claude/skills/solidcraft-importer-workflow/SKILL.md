---
name: solidcraft-importer-workflow
description: Implement a SolidCraft importer plugin end to end: declare an importer entry in plugin.json (kind, extensions, label, input_schema options, capabilities formats@0.2 [+ evaluate@0.2]), implement importer.import-file in the importer-plugin world — parse options, enforce an input ceiling, stream the file via source.read, write the design JSON via design.write in 1 MiB chunks, return the loss report — with the mesh fallback and the evaluate-based self-check. Use when adding a new file-format import slot or changing what an import returns.
license: MIT
---

# The importer-plugin workflow

World `importer-plugin` (see `solidcraft-wit-worlds`). One exported function:

```wit
import-file: func(entry: string, file: file-info, options: json) -> result<json, plugin-error>;
```

The design is **not** returned — it is written to the host via `design::write`; the return value
is the **loss report** JSON, which SolidCraft shows as the *Import report* (again via
**Plugins › Last import report…**).

## 1. Declare the slot in plugin.json

```json
"capabilities": ["formats@0.2", "evaluate@0.2"],
"runtime": {
  "module": "revmesh.wasm",
  "entries": [{
    "name": "stl", "kind": "importer", "extensions": ["stl"],
    "label": "STL as design",
    "input_schema": {
      "type": "object",
      "properties": {
        "mode": { "type": "string", "title": "Import as", "enum": ["auto", "mesh"],
                  "default": "auto", "description": "…" }
      }
    }
  }]
}
```

`input_schema` is JSON Schema shown in the import dialog; `needs_plugin_data: true` keeps the
plugin's data across the import. The slot appears under **Model ribbon → Plugins group label →
<plugin name> › <entry>**; SolidCraft's own **File › Open** keeps using built-in importers.

## 2. Implement import-file (reference: `crates/revmesh-plugin/src/lib.rs`)

1. Parse `options` JSON → your options struct; malformed → `invalid-input("options", …)`.
2. Check `file.size` against your ceiling **before reading** → `limit-exceeded` (this repo:
   8 MiB `auto`, 32 MiB `mesh`).
3. Stream the file: `source::read("", offset, 1 MiB)` loop with `control::cancelled()` and
   `control::progress(...)`; parse incrementally (here `StlReader`).
4. Do the work in the **core crate** (native-testable): mesh → recognition → `Design` +
   loss report.
5. Write the design: serialise straight into `design::write` calls of ≤ 1 MiB via an `io::Write`
   adapter (`HostWriter`); a host error is kept aside and returned as `trap`.
6. `control::progress(1.0, "Done")`; return `Ok(report.to_json())`.

```rust
impl Guest for Importer {
    fn import_file(_entry: String, file: source::FileInfo, options: String) -> Result<String, PluginError> {
        let options = ImportOptions::from_json(&options).map_err(|e| invalid("options", e))?;
        let mesh = read_mesh(&file, max_bytes_for(options.mode))?;
        let result = import_mesh(mesh, &file.name, &options);   // core crate
        result.design.write_json(&mut HostWriter::new())?;      // chunked design::write
        Ok(result.report.to_json())                              // loss report
    }
}
```

## 3. Write a valid SolidCraft design

The document format is internal; `docs/design-format.md` records the subset revmesh writes
(verified against 0.47.0, re-check every SolidCraft release): `"format": "solidcraft/63"`,
`params` (`expr`, `unit`, `model: false` for user parameters), `features` (`sketch`, `extrude`,
`hole` simple/counterbore/countersink, `fillet`, `box`, `cylinder`, `move`, `pattern`
rectangular/circular, `mesh_import`), `next_id`, `reserved_body_names`.

## 4. Never lose the user's part

Whatever is not recognised becomes a `mesh_import` body, with the reason in the loss report.
If you rebuild parametrically, verify the result with the `evaluate` capability: `stage` your
design, `run("inspect.measure", …)`, compare the volume with the mesh's within 1 % — otherwise
keep the mesh and say so in the report.

## Pitfalls

- `design.write` ≤ 1 MiB per call, strictly in order; the design file is capped at 256 MiB.
- An empty `source::read` at `offset >= size` is EOF, not an error.
- Return `cancelled` promptly when `control::cancelled()`; a hung import burns the deadline
  (`limits.deadline_ms`, max 10 min).

## References in this repo

- `crates/revmesh-plugin/src/lib.rs` (complete importer)
- `plugin/revmesh/plugin.json` (entry + input_schema + limits)
- `crates/revmesh-core/src/import.rs` (pipeline, `Mode`, `ImportOptions`, report)
- `crates/revmesh-core/src/report.rs` (loss report), `src/design/` (document model)
- `docs/design-format.md`, `docs/architecture.md`, `README.md` ("Use")
