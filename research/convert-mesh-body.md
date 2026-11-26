# Converting a mesh body in the open design — spike for issue #70

Issue: [#70](https://github.com/stritti/revmesh/issues/70) — today revmesh only runs as a file
importer (STL/OBJ/PLY file → new design); the ask is to select a mesh body already in the open
design (from SolidCraft's own *File › Open*, or revmesh's mesh fallback) and convert it in place.

Sources checked 2026-11-26: the vendored `crates/revmesh-plugin/wit/plugin.wit`
(`craft:plugin@0.2.0`, from SolidCraft **v0.50.0**), SolidCraft's `docs/plugins.md` at tag v0.50.0,
command schemas via `solidcraft-cli 0.50.0 exec engine.describe`, and
[gridfinity-solidcraft](https://github.com/tharje/gridfinity-solidcraft)'s package as a working
`commands@0.2` example (its `edit` entry edits a picked body in the open design).

## Finding 1 — packaging: one component = one world, so a second package

- The WIT defines separate worlds: `importer-plugin` (exports only `importer.import-file`) and
  `command-plugin` (imports `commands`, `files`, `storage`, …; exports only `entry.execute`).
  No world combines them, and the manifest has a single `runtime.module` — one component per
  package.
- **Recommendation:** keep `revmesh` as the importer package and ship a second package, e.g. id
  `revmesh-convert`, zip `revmesh-convert-<version>.zip`, implementing world `command-plugin` with
  capabilities `commands@0.2` (+ `storage@0.1` for per-design data, optionally `design@0.1`, and
  later `ui@0.1` for a panel). It links the same `revmesh-core`, so all recognition is reused
  unchanged and stays natively testable (`cargo test`), and `xtask dist` packages both.

## Finding 2 — reading the selected body's mesh

- The entry's `input_schema` declares a selection property; the host passes the picked body as a
  `geometry-ref` in the entry's `params` (plugins.md "x-solidcraft-selection"; gridfinity's `edit`
  entry does exactly this):

  ```json
  "body": {"type": "array", "title": "Body to convert", "minItems": 1, "maxItems": 1,
            "x-solidcraft-selection": {"version": 1, "accept": ["body"]}}
  ```

- Map the ref to a body name with `document.inspect` (output has `bodies` and `selection`), then
  read the mesh with **`model.mesh`** `{body: <name>, tolerance: 0.05}` →
  `{positions: [[x,y,z]…], triangles: […], frame, component}` (millimetres, owning component's
  coordinates). List both in `runtime.commands`; model commands need no `command_groups` grant
  (only `file`/`doc`/`app`/`plugin` groups are gated). Note: `model.mesh` is *not* in the
  `evaluate` sandbox allowlist (`document.inspect`, `inspect.measure`, `model.edges`,
  `model.faces`, `timeline.roll_to`) — it runs on the open design via `commands.run`.
- Rebuild `revmesh_core::mesh::Mesh` from `positions` + `triangles` (as f64; apply `frame` when it
  is not identity), then call `import_mesh(&mesh, name, &options)` unchanged. Apply the
  `AUTO_MAX_BYTES`/`MESH_MAX_BYTES` ceilings to the triangle count instead of file bytes. A binary
  STL is 50 bytes per triangle, so the 16 MiB file ceiling is ≈ 335k triangles; the measured
  in-memory peak is ≈ 90 MiB at that size (≈ 270 B per triangle, `docs/architecture.md` "Limits
  inside SolidCraft"), inside the 128 MiB budget — so state the `auto` ceiling directly as
  **≈ 335k triangles**, matching today's 16 MiB file limit. Note that `model.mesh` returns
  positions and indices as JSON, whose parse buffer adds to the peak that a file import streams
  instead — keep a margin below the budget.

## Finding 3 — writing the result back into the open design

- A command run is one transaction with one undo step; `runtime.commands` may list write commands
  on the open design. gridfinity's manifest lists `sketch.create`, `solid.extrude`, `solid.box`,
  `timeline.group`, `timeline.delete`, `parameters.add`, `storage.get`, `storage.patch`, …
- **`timeline.add`** `{feature: …, params: …}` appends one recorded feature "as the design file or
  `timeline.inspect` has it — every feature kind replays exactly" — the same recorded format the
  importer's design writer emits. So the write-back path is: append the recognized features one by
  one (`timeline.add`), add the named parameters (`parameters.add`), then delete the mesh body's
  import feature (`timeline.delete`). If nothing is recognized, leave the body and return the loss
  report as the entry's JSON result (no changes).
- **`storage@0.1`** (`put-blob`/`get-blob`/`get-document`) keeps the plugin's own files in the open
  design — there the import options and report per body belong (the "settings stored with the
  design" pattern from gridfinity's Edit bin, which stores its settings in the design via
  `storage.get`/`storage.patch`). `user-storage@0.1` covers per-user settings outside designs.

## Recommendation

1. New package `revmesh-convert` (world `command-plugin`, capabilities `commands@0.2` +
   `storage@0.1`), one entry `convert` with the selection field plus `mode`/`units` options,
   reusing `revmesh-core::import_mesh`. `runtime.commands`: `document.inspect`, `model.mesh`,
   `timeline.add`, `timeline.delete`, `parameters.add`, `parameters.list`, `storage.get`,
   `storage.patch` (start minimal; extend per recogniser).
2. A panel (`ui@0.1`, world `panel-plugin`: `ui.declarations` with `panel-spec`/`workspace-spec`,
   `set-data`, `show-panel`) can live in the same package later — gridfinity's `panel.rs` is the
   working reference (Parts/Quick/Layout/Settings tabs → our "imported parts + reports" list).
3. Testing: extend `cargo xtask plugin-test` with a command case — open a design containing a mesh
   body (import `tetra.stl` via `file.import` first), run the entry via `plugin.run` with the body
   ref, then check timeline, parameters and the loss report; keep the ≥ 80 % line-coverage gate.

## Open questions for the implementation

- `geometry-ref` → body name: confirm the exact field shapes in `document.inspect`'s `bodies` /
  `selection` on 0.50.0 (the ref is "stable across edits"; the inspect reply uses names).
- `model.mesh`'s `frame`: apply it when positions are in the owning component's coordinates.
- Verify `timeline.add` accepts every feature kind the design writer emits (plate sketch, extrude,
  holes, fillet, patterns) with a plugin-test case.
- Set the byte→triangle ceilings from the measured memory profile (`docs/architecture.md`
  "Limits inside SolidCraft"): ≈ 335k triangles ≈ 16 MiB binary STL ≈ 90 MiB peak, plus the
  `model.mesh` JSON parse buffer on top.
