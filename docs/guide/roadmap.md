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
   1. (done for plate outlines and cut-outs, and for the stack's and combined's sketches: a
      stack band `k`'s outline as `outline_<k>_<i>_len`/`_angle`, a combined extrusion along
      axis `a` as `outline_<a>_<i>_len`/`_angle`, their openings as `cutout_<n>_<i>_len`/`_angle`
      and a combined cut's outline as `cut_<n>_<i>_len`/`_angle`) a
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
   2. (done for outlines and cut-outs) arc sides in outlines and in cut-outs as
      `SketchCurve::Arc` with a `Diameter` constraint driven by `arc_<i>_d` (an outline's) or
      `cutout_<n>_arc_<i>_d` (a cut-out's), the centre a free sketch point the solver
      places equidistant from the side's corners (like a slot's arc centres)
      (`polygon_sketch_arcs`, `crates/revmesh-core/src/design/sketch.rs`; recognised as
      `ProfileSpec::arcs` in `plate/outline.rs::outline_shape` and as `CutoutSpec::arcs` in
      `plate/holes.rs::cutout_shape`, which reads a cut-out's loop like an outline and accepts
      it only with arc sides and no rounded corners) — round outlines, D-shaped cut-outs,
      rounded-corner cut-outs and concave arc sides stay a mesh
   3. (done for pockets of any straight-sided outline, for steps down from the top face
      several levels deep along Z, for headed holes opening on a step's level — measured from
      the level and drilled from it, a rounded rim filleted on the pocket's start edge — and
      for steps along X or Y, which the stack recogniser reads as outlines stacked along that
      axis) one sketch per face level: generalise `split_faces` beyond
      bottom and top — arbitrary pockets and steps, both written as `Pocket<n>Sketch` +
      `Pocket<n>` cut with `pocket_<n>_<i>_len` per side
      (`recognize/to_design.rs::block_features`; the levels in `plate/prism.rs::step_levels`,
      each top loop's level tagged in `face_outlines`, heads measured from their mouth's face in
      `stepped_hole`),
      and with 3.12 combined extrusions
   4. (done for every recognised kind) per-face surface deviation in the import report,
      computed in the sandbox at import: each recognised face's label with the farthest its mesh
      points lie from the ideal surface (`recognize::deviations`: `plate::face_deviations` for
      prisms, a box rounded all over reports its rounded envelope as "rounded surface",
      `plate::walls_deviations` per extrusion and per band for combined and stacked parts,
      labelled `extrude <a> side <i>` / `band <k> side <i>`; `Walls::deviation`; the volume
      within 1% stays the gate, Hausdorff in `cargo xtask check`)
6. **More parts, fitted robustly** — from the code review of the recognisers against the
   references (the extracted know-how is packaged in the `revmesh-recognition-knowhow` agent
   skill; the concept → code mapping with the sources is [Comparable tools → Concepts from the
   list, applied to revmesh](/guide/comparison#concepts-from-the-list-applied-to-revmesh)):
   1. an arbitrary prism axis: detect the two dominant parallel planes with fixed-seed RANSAC
      (Schnabel et al. 2007) and rotate the mesh so their normal is +Z, instead of trying only
      X, Y and Z (`recognize/axis.rs`) — today a part rotated in the file stays a mesh.
      Recognition-side only for now: SolidCraft 0.50.0's design format has origin-plane sketches
      and translate-only moves, so a slanted prism cannot be written parametrically at the
      pinned version (needs construction planes or a newer pin)
   2. (done: a fixed-seed RANSAC circle in `plate/geometry.rs::ransac_circle` augmenting the
      algebraic `fit_circle` — 3-point minimal samples, inliers within tolerance, the
      iteration count adapted to the inlier ratio — behind `round_hole` in `plate/holes.rs`:
      a rim loop with a few corners off the circle is fitted on its inliers when at least
      `MIN_RING_SEGMENTS` (6) of them are a strict majority of the loop, their steps stay even
      and the excluded corners lie within one facet's sagitta of the circle, while the ring's
      own edges keep the original 50° gate, so an exporter's uneven rim reads as a round
      hole but a real flat or lobe stays refused: in a face's own loops fewer than
      `MIN_HOLE_SEGMENTS` (8) corners are read as a polygonal cut-out first, so a hexagon stays
      a cut-out there, and a D-shape's flat side and an omitted facet's long step are still
      one long step) robust circle fits on rim loops
   3. (done: a torus about the known axis is a circle in the (radial, axial) profile plane, so
      `plate/geometry.rs::fit_torus` fits it as one — least-squares circle through the round's
      band corners, refused when they lie on no circle — in `rounded_rim` for hole rims
      (`plate/prism.rs`, with tangency to the bore's wall and to the face checked) and in
      `round_foot` for boss feet (`plate/bosses.rs`), replacing the per-corner averaging) analytic
      torus detection for the rounds at hole rims and boss feet
   4. per-corner radii in outlines (`plate/outline.rs` already fits one circle per rounded corner
      with `CircleSums`; today all corners must share one radius)
   5. hole rows at any angle (a pattern's direction is not an expression in SolidCraft — write
      them as single holes or on a rotated sketch)
   6. (done: `closed_loops` in `crates/revmesh-core/src/recognize/mod.rs` refuses, before any axis
      is tried, a mesh whose edges are not covered by two facets in opposite directions —
      corners welded on a sub-tolerance grid, an exporter's T-junctions counted by their sub-edges'
      length — so the recognised faces close into loops; and `check_walls` in `plate/prism.rs`
      refuses a wall facet whose plan is not a straight line within the tolerance plus the round
      walls' sagitta, so walls meet their face at 90°) ComplexGen-style topology gates
      alongside exact-or-nothing and the volume check: recognised faces close into loops, walls
      meet their face at 90°
   plus what is still open in 3.10 and 3.12: bosses on side faces, roofs and bevels on stacks
7. **Import and conversion**:
   1. (done: the manifest requests `evaluate@0.2`; in auto mode the serialised design is staged
      and rebuilt in the app's private sandbox (`document.inspect`) before it is written, and a
      design the app cannot open falls back to the mesh body with the reason in the loss report,
      code `revmesh.selfcheck`; mesh-mode imports are not checked — `self_check` in
      `crates/revmesh-plugin/src/lib.rs`, closing item 4 at import time) an in-sandbox self-check:
      request the `evaluate@0.2` capability, stage the produced design
      and rebuild it, and on feature errors fall back to the mesh body with the reason in the
      report
   2. (done: `plugin.json` accepts `stl`, `obj` and `ply`; `format.rs` tells the format by the
      file's name or its first bytes, and `MeshReader` streams all three in chunks with the same
      size limits as `StlReader`; `obj.rs` and `ply.rs` (ASCII and binary little-endian) parse
      into the same `Mesh` with localised line/record errors; `import`, the plugin's `read_mesh`
      and `cargo xtask check` take any of them; the plugin-test fixtures `block-60x40x6.obj`,
      `block-60x40x6.ply` and `block-60x40x6-bin.ply` import the STL block unchanged, and
      `obj-bad.obj`/`ply-bad.ply` cover the error cases) OBJ and PLY input next to STL
   3. double-precision coordinates in mesh bodies beyond the single-precision range (the design
      is written with `f32` coordinates today, `design/mod.rs`) — parts far from the origin lose
      precision in the mesh fallback
   4. (done: `auto` accepts 16 MiB vs 32 MiB in mesh mode — `import_mesh` takes the mesh by
      reference and clones only in the mesh/unrecognized fallback paths, and `Faces.chords` is
      slimmed from (a, b, middle, length) to (a, b, mid_z, length); measured peak ≈ 90 MiB at
      16 MiB, inside the 128 MiB budget (`AUTO_MAX_BYTES` in `crates/revmesh-plugin/src/lib.rs`);
      32 MiB would need index-based face storage) a higher `auto` input ceiling (16 MiB today vs
      32 MiB in mesh mode, `revmesh-plugin`) by slimming the face and wall indexes recognition
      builds
   5. (done: ASCII-STL syntax errors carry a `SyntaxReason` (`stl.rs`) rendered in the import
      language via `stl.syntax.reason.*` in the locale catalogs; the dialog's `units` option
      (`plugin.json`) reads inch files as inches and scales them to millimetres in place
      (`convert_units`, range-checked afterwards), noted in the report as `revmesh.units`).

Contributions are very welcome: bug reports, STL files that import badly, recognisers, docs.
Start with [CONTRIBUTING.md](https://github.com/stritti/revmesh/blob/main/CONTRIBUTING.md).
