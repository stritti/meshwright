# Roadmap

1. **Skeleton** (done): plugin pipeline, document writer, mesh fallback, quality gates,
   releases.
2. **Plates** (done): any outline of straight sides with round through holes and rounded
   vertical corners.
3. **Parity with mesh2cad**, one recogniser per pull request, each compared with mesh2cad's
   output (`scripts/compare-mesh2cad.sh`, see [Development](/development)):
   1. (done) parameter names as mesh2cad writes them (`base_t`, `fillet_r`, `extrude_len`, …),
      and the part kept where it is in the file
   2. (done) hole rows, grids and bolt circles as SolidCraft patterns
   3. (done) counterbored and countersunk holes
   4. (done) blind holes
   5. (done) slots
   6. (done) bosses on the top face
   7. (done) polygonal cut-outs
   8. (done) pads and rectangular pockets (steps and other pockets later)
   9. (done) extrusions along X and Y
   10. (done) boxes with every edge rounded (`edge_fillet_r`), with through holes in them;
       bosses on side faces still to do
   11. (done) chamfers, drafts and the rounds at a boss's foot or a hole's rim
   12. combined extrusions: (done) the intersection of outlines extruded along X, Y and Z, with
       openings through them, straight blind cuts into them and round holes, and (done) unions
       of outlines stacked along an axis; holes that cross, roofs and bevels on a stack still to
       do
4. **Checking**: compare the rebuilt part with the mesh through SolidCraft's `evaluate` (volume
   and deviation), as mesh2cad does with CadQuery.
5. **Reworkable sketches** — beyond parity: every sketch fully dimensioned and constrained, so it
   can be reworked in SolidCraft. The concepts behind this and their sources (products and
   papers) are mapped in [Comparable tools → Concepts from the list, applied to
   revmesh](/guide/comparison#concepts-from-the-list-applied-to-revmesh):
   1. (done for plate outlines and cut-outs; the stack's and combined's sketches follow) a
      `Length` constraint per side with its own parameter (`outline_<i>_len`,
      `cutout_<n>_<i>_len`) in `polygon_sketch_dimensioned`/`polygons_sketch_dimensioned`
      (`crates/revmesh-core/src/design/sketch.rs`); an `Angle` per slanted side still to do
   2. arcs in outlines and cut-outs (`SketchCurve::Arc` with a `Diameter` parameter) instead of
      only fillet features — round outlines and D-shaped cut-outs, today a mesh
   3. one sketch per face level: generalise `split_faces` beyond bottom and top — arbitrary
      pockets and steps, and with 3.12 combined extrusions
   4. per-face surface deviation in the import report, computed in the sandbox (today: volume
      within 1% at import, Hausdorff in `cargo xtask check`)

Contributions are very welcome: bug reports, STL files that import badly, recognisers, docs.
Start with [CONTRIBUTING.md](https://github.com/stritti/revmesh/blob/main/CONTRIBUTING.md).
