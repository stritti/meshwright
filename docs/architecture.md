# Architecture

## The constraint that shapes everything

SolidCraft runs plugins as WebAssembly components built for `wasm32-unknown-unknown`: no WASI,
no threads, no network, no subprocesses, no system randomness. mesh2cad itself (Python with
numpy, scipy, trimesh and OpenCascade) cannot run there, so meshwright ports its recognition to
Rust.

It does **not** port mesh2cad's OpenCascade part, which stitches recognised faces into a solid.
The importer writes a *feature history* instead (sketch, extrusion, holes, fillets) and lets
SolidCraft's own kernel build the solid. That is both less code and what makes the result
editable.

## Layout

```text
crates/
  meshwright-core/     host-independent: STL → Mesh → Recognition → Design + LossReport
    src/stl.rs         binary and ASCII STL
    src/mesh.rs        triangle soup, bounds, volume, area
    src/recognize.rs   recognisers (ported from mesh2cad) and PlateSpec → Design
    src/design.rs      the SolidCraft document model (the subset meshwright writes)
    src/report.rs      the loss report SolidCraft shows after an import
    src/import.rs      the whole pipeline, and the import options
    tests/data/        fixtures and golden designs
  meshwright-plugin/   the component: craft:plugin@0.2.0, importer-plugin world
    wit/plugin.wit     vendored from SolidCraft v0.45.0
plugin/meshwright/     the package: plugin.json, the built .wasm, SolidCraft test cases
xtask/                 component wrapping, packaging, plugin tests
```

## Rules of thumb

- **Everything testable lives in the core.** The plugin crate compiles to nothing on native
  targets; it only reads the source in 1 MiB chunks, calls `meshwright_core::import` and writes
  the design in 1 MiB chunks.
- **The output side is fixed before the recognisers.** `PlateSpec::to_design` already writes the
  history a recognised plate becomes, and its golden file has been opened and rebuilt in
  SolidCraft. A recogniser's job is only to fill in a `PlateSpec` (and its successors).
- **Never lose the user's part.** Whatever is not recognised is imported as a mesh body, with an
  entry in the loss report.
- **Deterministic numerics.** No threads and no system randomness in the sandbox: algorithms
  such as RANSAC use a fixed seed.

## Limits inside SolidCraft

128 MiB memory by default (up to 4 GiB if the user approves it at install), an import deadline
of up to 10 minutes, a 64 MiB component and a 256 MiB design. `plugin.json` asks for 128 MiB and
2 minutes, which is enough for parts of some ten thousand triangles.

## Testing levels

1. `cargo test`: unit tests of the core with synthetic meshes, and golden designs.
2. `cargo xtask plugin-test`: `solidcraft-cli plugin test` installs the real component in a
   scratch folder and imports each file in `plugin/meshwright/tests` through SolidCraft's
   `file.import`, then checks bodies, features, the loss report and the measured volume.
3. By hand: open a golden design (`cargo run -p meshwright-core --example plate_design`) in
   SolidCraft, change parameters and watch it rebuild.
