---
name: solidcraft-importer-workflow
description: >-
  Implement a SolidCraft importer plugin end to end: declare an importer entry in plugin.json (kind, extensions, label, input_schema options, capabilities formats@0.2 [+ evaluate@0.2]), implement importer.import-file in the importer-plugin world — parse options, enforce an input ceiling, stream the file via source.read, write the design JSON via design.write in 1 MiB chunks, return the loss report — with the mesh fallback and the evaluate-based self-check. Use when adding a new file-format import slot or changing what an import returns.
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

## Plugin-interface conformance — always check this

This skill touches SolidCraft's plugin boundary, so before committing any change here, check it
against the host's own documentation: `docs/plugins.md` in
[tangency-labs/solidcraft](https://github.com/tangency-labs/solidcraft) at the pinned release
[tag `v0.51.2`](https://github.com/tangency-labs/solidcraft/blob/v0.51.2/docs/plugins.md)
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

- Entry fields must be valid for the pinned host: `kind` / `extensions` / `label` /
  `input_schema` / `needs_plugin_data` everywhere; `file_first` and `dialog_hook` only where the
  host knows them (set `min_host_version` accordingly), and only on importer entries.
- Use the dialog features the host actually renders from `input_schema`: `enum` vs `oneOf`
  `{const, title}` dropdowns, `x-solidcraft-field-order: "declared"`, shared `$defs` + `$ref`,
  `x-solidcraft-visible-when` conditions (file facts such as `extension` need no new capability
  on current hosts).
- The design itself only ever reaches the host through `design::write`; the return value is the
  loss-report JSON. If `evaluate@0.2` is declared, self-check with `evaluate::stage`/`run` (§4);
  if not, say in the report why the mesh fallback is enough.

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
        "units": { "type": "string", "title": "Units", "enum": ["mm", "inch"],
                   "default": "mm", "description": "…" }
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
2. Check `file.size` against your ceiling **before reading** → `limit-exceeded` (this repo: a single
   32 MiB ceiling, `MAX_BYTES` in `crates/revmesh-plugin/src/lib.rs`; recognition is gated
   separately in the core at `MAX_RECOGNITION_TRIANGLES`)
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
        let mesh = read_mesh(&file, MAX_BYTES)?;
        let result = import_mesh(mesh, &file.name, &options);        // core crate
        let mut out = HostWriter::new();                             // buffers to 1 MiB, then design::write
        let written = result.design.write_json(&mut out)
            .and_then(|()| out.flush_buffer());                      // flush the final partial chunk
        if let Some(error) = out.error { return Err(error); }         // host error kept in the writer
        written.map_err(|e| PluginError::Trap(format!("writing the design: {e}")))?;
        Ok(result.report.to_json())                                   // loss report
    }
}
```

## 3. Write a valid SolidCraft design

The document format is internal; `docs/design-format.md` records the subset revmesh writes
(verified against 0.51.2, re-check every SolidCraft release): `"format": "solidcraft/63"`,
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
- `HostWriter` sends only full 1 MiB buffers during `write`: always flush it explicitly at the end and check its stored host error, or designs under 1 MiB are never sent and larger ones arrive truncated.
- An empty `source::read` at `offset >= size` is EOF, not an error.
- Return `cancelled` promptly when `control::cancelled()`; a hung import burns the deadline
  (`limits.deadline_ms`, max 10 min).

## References in this repo

- `crates/revmesh-plugin/src/lib.rs` (complete importer)
- `plugin/revmesh/plugin.json` (entry + input_schema + limits)
- `crates/revmesh-core/src/import.rs` (pipeline, `ImportOptions`, report)
- `crates/revmesh-core/src/report.rs` (loss report), `src/design/` (document model)
- `docs/design-format.md`, `docs/architecture.md`, `README.md` ("Use")
