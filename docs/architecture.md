# Architecture

## The constraint that shapes everything

SolidCraft runs plugins as WebAssembly components built for `wasm32-unknown-unknown`: no WASI,
no threads, no network, no subprocesses, no system randomness. mesh2cad itself (Python with
numpy, scipy, trimesh and OpenCascade) cannot run there, so revmesh ports its recognition to
Rust.

It does **not** port mesh2cad's OpenCascade part, which stitches recognised faces into a solid.
The importer writes a *feature history* instead (sketch, extrusion, holes, fillets) and lets
SolidCraft's own kernel build the solid. That is both less code and what makes the result
editable.

## Layout

```text
crates/
  revmesh-core/     host-independent: STL → Mesh → Recognition → Design + LossReport
    src/stl.rs         binary and ASCII STL
    src/mesh.rs        triangle soup, bounds, volume, area
    src/recognize/     what recognition finds (`Recognition`), the extrusion axis and the
                       rotations that align it with Z (axis.rs), the specs (spec.rs) and
                       PlateSpec/ProfileSpec → Design (to_design.rs)
    src/plate/         the plate recogniser, one file per concern: mod.rs (entry point, shared
                       constants), geometry.rs (2D measurements), faces.rs and walls.rs (face
                       and wall classification), outline.rs (outline, rounded corners), holes.rs
                       (round holes, slots, polygonal cut-outs), bosses.rs, prism.rs (prism
                       check, face loops, hole pairing, the rounded rim of an all-over-rounded
                       box), tests/ (the tests, one file per concern)
    src/synth.rs       synthetic meshes of known parts (plates, profiles, all-over-rounded
                       boxes), for tests and STL fixtures
    src/design/        the SolidCraft document model (the subset revmesh writes):
                       mod.rs (Design, features, DesignBuilder), sketch.rs (sketches and
                       their constructors)
    src/report.rs      the loss report SolidCraft shows after an import
    src/import.rs      the whole pipeline, and the import options
    tests/data/        fixtures and golden designs
  revmesh-plugin/   the component: craft:plugin@0.2.0, importer-plugin world
    wit/plugin.wit     vendored from SolidCraft v0.50.0
plugin/revmesh/     the package: plugin.json, icons/ (the SVG SolidCraft shows in its Plugins menu), the built .wasm, SolidCraft test cases
xtask/                 component wrapping, packaging, plugin tests
```

## Rules of thumb

- **Everything testable lives in the core.** The plugin crate compiles to nothing on native
  targets; it only reads the source in 1 MiB chunks, calls `revmesh_core::import` and writes
  the design in 1 MiB chunks.
- **The output side is fixed before the recognisers.** `PlateSpec::to_design` already writes the
  history a recognised plate becomes, and its golden file has been opened and rebuilt in
  SolidCraft. A recogniser's job is only to fill in a `PlateSpec` (and its successors).
- **Never lose the user's part.** Whatever is not recognised is imported as a mesh body, with an
  entry in the loss report.
- **Recognise exactly or not at all.** A recogniser measures the mesh as a CAD exporter wrote it
  (facet corners on the true surfaces), within 1 µm or 10⁻⁵ of the part's size, whichever is
  larger, and the rebuilt part must match the mesh's volume within 1%. Anything less certain
  stays a mesh, with the reason in the loss report.
- **Take exporters as they are.** Exporters differ in how they mesh: SolidCraft's own STL
  export meshes tall walls in bands (corners between the top and the bottom face, facets that
  lean a little on a curved wall), meshes the top and bottom faces apart, and spaces the corners
  of arcs and rims unevenly. So a corner between the faces is checked against the walls
  themselves (it must lie on a hole's circle, a side or a corner's arc), not against its facet's
  normal, and arcs and holes are read from where their corners lie, not from equal facets.
  `plugin/revmesh/tests/solidcraft-l-bracket.stl` is such an export.
- **Deterministic numerics.** No threads and no system randomness in the sandbox: algorithms
  such as RANSAC use a fixed seed. The literature behind the recognition approach — RANSAC shape
  detection, feature recognition from STL meshes, extrusion-cylinder decomposition — is listed
  under [Research and literature](/guide/comparison#research-and-literature).
- **One pipeline, three axes.** Like mesh2cad, which tries a prism along X, Y and Z, `recognize`
  tries each axis in turn: the mesh is rotated by a proper rotation (so volumes and windings
  survive) into a frame where the axis is +Z, the Z pipeline runs unchanged, and the result is
  rotated back. Z is tried first, so a box keeps the `base_*` plate. The profile's coordinates in
  that frame are exactly the sketch coordinates on SolidCraft's matching origin plane (YZ: y, z;
  XZ: x, -z; XY: x, y), so the design writer only has to pick the plane and map directions.

## Limits inside SolidCraft

128 MiB memory by default (up to 4 GiB if the user approves it at install), an import deadline
of up to 10 minutes, a 64 MiB component and a 256 MiB design. `plugin.json` asks for 128 MiB and
2 minutes, the most a plugin gets without the user approving more at install.

Memory decides how large a file can be. The plugin therefore holds nothing twice: the STL is
parsed chunk by chunk as it is read (`stl::StlReader`), `import_mesh` takes the mesh by reference
(it clones only in the mesh and unrecognized fallback paths), and the design is serialised
straight into 1 MiB writes to SolidCraft, with coordinates in single precision. Auto mode
therefore accepts STL up to 16 MiB — measured peak ≈ 90 MiB at about 250 bytes per triangle,
inside the 128 MiB the manifest asks for (32 MiB would need index-based face storage); mesh mode
accepts up to 32 MiB. A rotated X/Y recognition attempt would hold a second mesh copy plus the
face/wall indexes on top of the original (measured up to ~420 bytes per triangle peak on a
densely banded mesh — 133 MiB at 333k triangles), so `recognize` only tries Z above 200,000
triangles and a larger part lying along X or Y falls back to a mesh body instead of trapping the
component. Measured with SolidCraft 0.50: a 1.5 million triangle STL (75 MB) imports
as a mesh in about 8 s; 2 million triangles exceed 128 MiB (before streaming, 600,000 already
did).

## Self-check in the app's sandbox

In auto mode the serialised design is staged in SolidCraft's private sandbox (capability
`evaluate@0.2`) and rebuilt there (`document.inspect`) before it is written to the design file.
A command failure or a timeline feature with an `error` in the `document.inspect` reply falls
back to the mesh body, with the reason in the loss report (code `revmesh.selfcheck`); a call error
(a design the app cannot open, a reply over 1 MiB) is an error for that call only, so the design
is written unchecked. The reply parsing and the fallback policy live in `revmesh_core::import`
(`SelfCheck`, `apply_self_check`), tested natively; the plugin only stages and runs. Mesh-mode
imports are not checked: the design is the mesh, and holding its JSON twice would break the
memory budget above.

## Testing levels

1. `cargo test`: unit tests of the core with synthetic meshes, and golden designs.
2. `cargo xtask plugin-test`: `solidcraft-cli plugin test` installs the real component in a
   scratch folder and imports each file in `plugin/revmesh/tests` through SolidCraft's
   `file.import`, then checks bodies, features, the loss report and the measured volume.
3. `cargo xtask check`: imports each fixture, rebuilds the design in SolidCraft
   (`solidcraft-cli eval`: a feature error fails the check) and re-exports it
   (`solidcraft-cli run --out`), then compares the rebuilt mesh with the source STL in
   `check.rs`: the volume error and the surface deviation (Hausdorff of the two
   triangulations), gated as mesh2cad gates its scripts (deviation at most
   `max(4 * 0.1% of the diagonal, 0.1 mm)`, volume error at most 1%).
4. By hand: open a golden design (`cargo run -p revmesh-core --example plate_design`) in
   SolidCraft, change parameters and watch it rebuild.
