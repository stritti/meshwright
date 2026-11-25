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
      `cutout_<n>_<i>_len`) and an `Angle` from the sketch's X axis on the longest slanted side
      with its own parameter (`outline_<i>_angle`, `cutout_<n>_<i>_angle`) in
      `polygon_sketch_dimensioned`/`polygons_sketch_dimensioned`
      (`crates/revmesh-core/src/design/sketch.rs`; parallel slanted sides share their angle and a
      loop's turns sum to 360°, so one angle per polygon is the independent one — and that side's
      length is then derived, driving both would over-constrain the sketch; likewise, when the
      remaining lengths and orientations already fix that side's direction, as a right triangle's
      legs held horizontal and vertical do for the diagonal, the longest axis-aligned side's
      length is derived too (`dimensioned_sides_for_angle`), so the angle parameter stays
      editable)
   2. arcs in outlines and cut-outs (`SketchCurve::Arc` with a `Diameter` parameter) instead of
      only fillet features — round outlines and D-shaped cut-outs, today a mesh
   3. (done for pockets of any straight-sided outline and for steps down from the top face,
      several levels deep, along Z, but not for headed holes opening on a step's level — those
      stay a mesh or a stack; steps along X or Y and the combined extrusions' sketches
      follow — a part stepped along X or Y is a stack of outlines along that axis, recognised
      by the stack recogniser) one sketch per face level: generalise `split_faces` beyond
      bottom and top — arbitrary pockets and steps, both written as `Pocket<n>Sketch` +
      `Pocket<n>` cut with `pocket_<n>_<i>_len` per side
      (`recognize/to_design.rs::block_features`; the levels in `plate/prism.rs::step_levels`),
      and with 3.12 combined extrusions
   4. (done for plates and profiles along Z) per-face surface deviation in the import report,
      computed in the sandbox at import: each recognised face's label with the farthest its mesh
      points lie from the ideal surface (`plate::face_deviations`, `Walls::deviation`; the volume
      within 1% stays the gate, Hausdorff in `cargo xtask check`; combined and stacked parts
      follow)
6. **More parts, fitted robustly** — from the code review of the recognisers against the
   references (the extracted know-how is packaged in the `revmesh-recognition-knowhow` agent
   skill; the concept → code mapping with the sources is [Comparable tools → Concepts from the
   list, applied to revmesh](/guide/comparison#concepts-from-the-list-applied-to-revmesh)):
   1. an arbitrary prism axis: detect the two dominant parallel planes with fixed-seed RANSAC
      (Schnabel et al. 2007) and rotate the mesh so their normal is +Z, instead of trying only
      X, Y and Z (`recognize/axis.rs`) — today a part rotated in the file stays a mesh
   2. robust circle fits on rim loops: a fixed-seed RANSAC circle (3-point minimal sample plus
      inliers) or a geometric fit, augmenting the algebraic `fit_circle`
      (`plate/geometry.rs::fit_circle`); with it, round holes with fewer than eight facets
      (`MIN_HOLE_SEGMENTS`, `plate/mod.rs`) when their steps stay even — a D-shape's flat side
      stays one long step and is still refused
   3. analytic torus detection for the rounds at hole rims and boss feet (today measured band by
      band in `plate/bosses.rs::round_foot`)
   4. per-corner radii in outlines (`plate/outline.rs` already fits one circle per rounded corner
      with `CircleSums`; today all corners must share one radius)
   5. hole rows at any angle (a pattern's direction is not an expression in SolidCraft — write
      them as single holes or on a rotated sketch)
   6. ComplexGen-style topology gates alongside exact-or-nothing and the volume check: recognised
      faces close into loops, walls meet their face at 90°
   plus what is still open in 3.10 and 3.12: bosses on side faces, roofs and bevels on stacks
7. **Import and conversion**:
   1. an in-sandbox self-check: request the `evaluate@0.2` capability, stage the produced design
      and rebuild it, and on feature errors fall back to the mesh body with the reason in the
      report (closes item 4 at import time — today the rebuild check runs only in
      `cargo xtask check`, and the plugin requests only `formats@0.2`)
   2. OBJ and PLY input next to STL, with the same chunked streaming and size limits as
      `StlReader` (`plugin.json` accepts only `stl` today)
   3. double-precision coordinates in mesh bodies beyond the single-precision range (the design
      is written with `f32` coordinates today, `design/mod.rs`) — parts far from the origin lose
      precision in the mesh fallback
   4. a higher `auto` input ceiling (8 MiB today vs 32 MiB in mesh mode, `revmesh-plugin`) by
      slimming the face and wall indexes recognition builds
   5. localised ASCII-STL syntax reasons (they stay English today, `stl.rs`) and an optional
      `units` import option (STL is unitless and assumed to be mm)

Contributions are very welcome: bug reports, STL files that import badly, recognisers, docs.
Start with [CONTRIBUTING.md](https://github.com/stritti/revmesh/blob/main/CONTRIBUTING.md).
