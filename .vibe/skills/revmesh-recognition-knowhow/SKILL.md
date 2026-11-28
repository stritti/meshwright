---
name: revmesh-recognition-knowhow
description: >-
  Know-how for building revmesh's recognisers, extracted from the literature and the comparable
  tools and mapped to this repo's code: which sources are portable into the wasm sandbox
  (classical/deterministic only — no GPU, no Python, no network), the building blocks with their
  concrete algorithms (fixed-seed RANSAC primitives, line/circle fits on loops, dihedral-angle
  segmentation, extrusion cylinders, parametric edges, sketch+constraint representation, topology
  and volume validity gates), what revmesh already implements for each, and the hard rules every
  recogniser follows (exact-or-nothing, volume within 1%, deterministic, mesh fallback, mesh2cad
  comparison). Use when adding or changing a recogniser in crates/revmesh-core (plate/, recognize/)
  or a sketch writer in src/design/sketch.rs, or when picking the next roadmap item.
license: MIT
---

# Recognition know-how for revmesh

Extracted from the sources listed in `docs/guide/comparison.md` ("Research and literature" and
"Concepts from the list, applied to revmesh") and from this repo's own rules
(`docs/architecture.md`, `docs/design-format.md`, `docs/guide/roadmap.md` item 5). The literature
links live in `docs/guide/comparison.md`; this file holds the extracted, implementable core.

## 0. What is portable into the sandbox

The plugin runs as a wasm component on `wasm32-unknown-unknown`: no WASI, no threads, no
network, no system randomness, 128 MiB memory by default. The neural methods (SPFN, CPFN, HPNet,
ParSeNet, Point2CAD, CAD-Recode, MeshWright's AI assistant) need a GPU and Python and **cannot
run here** — adopt their output *representations* and the classical deterministic algorithms only.
Every sampling algorithm (RANSAC) uses a **fixed seed**; results must be reproducible bit for bit.

## 1. Building blocks, extracted and mapped to code

### 1.1 RANSAC primitive detection — Schnabel, Wahl, Klein, CGF 26(2) 2007, doi:10.1111/j.1467-8659.2007.01016.x

Algorithm: repeatedly sample a minimal sample set (plane: 3 points, sphere: 4, cylinder: 5,
cone: 6, torus: 7; where facet normals exist, sample points+normals), fit the primitive, count
inliers within the error band ε, keep the best model by a score that rewards many inliers and
small residuals (reject samples whose own minimal set already disagrees). Iterations
`N = log(1 − p) / log(1 − w^m)` with confidence `p` (e.g. 0.99), inlier ratio `w`, minimal-set
size `m`. Deterministic variant: drive the sampler with a fixed-seed PRNG.

revmesh today: fixed-seed RANSAC for planes and cylinders; `plate/faces.rs::split_faces` splits
horizontal faces from walls, `plate/walls.rs::on_wall` checks corners against the wall loops.
Tolerance model: exact-or-nothing, `tol = max(1 µm, 10⁻⁵ · size)` (`plate/mod.rs::tolerance`).
Next: torus detection, so rounds at hole rims and boss feet become analytically measured
surfaces instead of per-band fits (`plate/bosses.rs::round_foot`).

### 1.2 Line and circle fits on 2D loops — the classical core of PIE-Net

PIE-Net (Wang et al., NeurIPS 2020, arXiv:2007.04883) represents feature edges as parametric
curves — lines, circles, B-splines — inferred from the points; traditional pipelines fit them
directly. For STL input that means: read loops of facet corners and fit lines (collinearity via
chord distance, `drop_collinear`/`merge_collinear` in `plate/holes.rs`, `plate/outline.rs`) and
circles (algebraic least-squares fit, `plate/geometry.rs::fit_circle`) to them.

revmesh today: circles fitted to hole rims and boss rims (`fit_circle`, used from
`plate/holes.rs`, `plate/bosses.rs::round_foot`, and per rounded corner via `CircleSums` in
`plate/outline.rs`); loops from face boundaries (`plate/geometry.rs::boundary_loops`). Facet
corners are read as lying on the true surfaces — never facet normals, because exporters mesh
walls in bands and space arc corners unevenly (`docs/architecture.md`, "Take exporters as they
are").
Next: curved outline sides as arcs in the sketch — roadmap 5.2.

### 1.3 Segmentation by sharp edges / dihedral angle — Sunil & Pande, CAD 40(4) 2008

Algorithm: two adjacent facets form a sharp edge when the angle between their normals exceeds a
threshold; group facets into faces by connectivity and smoothness (dihedral angle, local
curvature), build the face adjacency graph, and recognise features (holes, slots, beads, darts,
pockets) by matching subgraphs against feature rules.

revmesh today: `split_faces` (top/bottom faces vs walls), the wall index (`walls.rs`), outline =
the counter-clockwise boundary loop of the top face, holes = clockwise loops
(`plate/mod.rs` pipeline step 2), holes recognised as circles, corner rounds as runs of corners
on one circle meeting the sides tangentially (`plate/outline.rs::outline_shape`).
Next: more feature classes from the same segmentation — chamfers, drafts, beads, blind and
slanted slots (roadmap 3.11, partly done).

### 1.4 Extrusion cylinders — Point2Cyl (Uy et al., CVPR 2022, arXiv:2112.09329)

Representation: a CAD model is a set of **extrusion cylinders** — each a 2D sketch (base) plus a
barrel surface plus an extrusion axis and range — combined by booleans. Point2Cyl predicts
per-point base/barrel labels and normals, then estimates the extrusion parameters in closed,
differentiable form; the portable core is the representation and the closed-form fitting.

revmesh today: one prism per part, the axis tried along X, Y and Z by proper rotation
(`plate/prism.rs::recognize_prism`, rotations in `recognize/axis.rs`), profile read from the top
face; unions/intersections of outlines stacked along an axis are recognised too
(`recognize/stack.rs`, `recognize/combined.rs`).
Next: several extrusion cylinders per part — combined extrusions with holes that cross, roofs
and bevels on a stack (roadmap 3.12), each cylinder its own sketch + extrude.

### 1.5 Sketch-extrude representation — DeepCAD (Wu et al., ICCV 2021, arXiv:2105.09492), Fusion 360 Gallery (Willis et al., SIGGRAPH 2021, arXiv:2010.02392), ABC (Koch et al., CVPR 2019, arXiv:1812.06216)

Representation: a CAD model is a **sequence of sketch operations and extrudes**; a sketch is 2D
curves (line, arc, circle) plus constraints (coincident, horizontal, vertical, tangent, equal,
…). The datasets define "recover the CAD program from the geometry": ABC (~1M models with
parametric ground truth — tessellated to STL they are a recogniser test corpus), DeepCAD (178k
construction sequences), Fusion 360 Gallery (8,625 human sketch-extrude parts).

SolidCraft's sketch is exactly this subset: points, `SketchCurve::Line`/`Arc`, and the
`Constraint` enum (`Horizontal`, `Vertical`, `Coincident`, `Length`, `Tangent`, `Equal`,
`Midpoint`, `Diameter`, `DistanceX`, `DistanceY`, `Angle`) in `crates/revmesh-core/src/design/sketch.rs`,
where `Length`/`Diameter`/`DistanceX`/`DistanceY`/`Angle` are driven by model parameters
(`docs/design-format.md`).

revmesh today: `rectangle_sketch` (two `Length` constraints via model params `d1`/`d2` →
`base_x`/`base_y`), `slot_sketch` (tangent ×4, equal, midpoint, `Length`, `Diameter`, coincident
position, `Angle` — all parameter-driven), but `polygon_sketch`/`polygons_sketch` write free
points with only `Horizontal`/`Vertical`, used for outlines and cut-outs
(`recognize/to_design.rs`).
Next: per-side `Length`/`Angle` (or per-corner `DistanceX`/`DistanceY`) each with its own model
parameter in `polygon_sketch`/`add_polygons` — roadmap 5.1. All constraint types already exist;
no new dependency is needed.

### 1.6 Validity gates — ComplexGen (Guo et al., SIGGRAPH 2022, arXiv:2205.14573) and mesh2cad's refuse rule

Algorithm: a reconstruction is only valid under structural constraints — closed face loops,
consistent adjacency/correspondence between primitives, perpendicularity — enforced by global
optimization plus geometric refinement. mesh2cad refuses any part its scripts cannot express
exactly and reports Hausdorff/RMS/volume.

revmesh today (the deterministic analogue): exact-or-nothing recognition
(`plate/mod.rs::tolerance`), the rebuilt part's volume must match the mesh's within
`MAX_VOLUME_ERROR = 1%` (`plate/mod.rs`), the rebuild is checked in SolidCraft and re-exported
compared by Hausdorff in `cargo xtask check` (`src/check.rs`; gates as mesh2cad: deviation ≤
`max(4 · 0.1% · diagonal, 0.1 mm)`, volume ≤ 1%). Anything not recognised becomes a mesh body
with the reason in the loss report — the part is never lost.
Next: ComplexGen-style topology checks as recognition gates (closed loops, walls ⊥ their face)
and per-face deviation in the import report — roadmap 5.4.

## 2. Hard rules for every recogniser (docs/architecture.md)

- Thin plugin crate: all logic in `revmesh-core`, tested natively; the plugin crate only moves
  bytes. No `unsafe` outside the generated `wit_bindgen` bindings; no panics in plugin code.
- Pure recognition math never crosses the plugin boundary; anything that does (design JSON fields,
  host calls, report JSON) must pass the plugin-interface conformance check in the `solidcraft-*`
  skills against SolidCraft's `docs/plugins.md` — [at the pinned tag
  `v0.51.2`](https://github.com/tangency-labs/solidcraft/blob/v0.51.2/docs/plugins.md) and [at
  `main`](https://github.com/tangency-labs/solidcraft/blob/main/docs/plugins.md): compatible with
  the host, and using every part of the interface that applies.
- Deterministic: fixed seeds, no threads, no network, no system randomness in the sandbox.
- Exact-or-nothing: measure the mesh as a CAD exporter wrote it (facet corners on the true
  surfaces), tolerance `max(1 µm, 10⁻⁵ · size)`; rebuilt volume within 1%.
- Never lose the user's part: mesh fallback + reason in the import report.
- Never hand-edit golden files (`crates/revmesh-core/tests/data/*.json`) or STL fixtures
  (`plugin/revmesh/tests/*.stl`); regenerate with the documented commands.
- Coverage gate ≥ 80% lines across the workspace (CI); new code ships with tests — see the
  `solidcraft-plugin-testing` skill.
- A recogniser PR includes the mesh2cad comparison table: `scripts/compare-mesh2cad.sh`
  (`docs/development.md`).

## 3. Where to go next

`docs/guide/roadmap.md` item 5 (reworkable sketches) and open items of 3.12; the concept → code
mapping with all sources is in `docs/guide/comparison.md` → "Concepts from the list, applied to
revmesh".

## 4. Sources

All with links and venues in `docs/guide/comparison.md` → "Research and literature": Efficient
RANSAC (Schnabel et al. 2007); SPFN (2019), CPFN (2021), ParSeNet (2021), HPNet (2021), PIE-Net
(2020) — neural, representation only; Sunil & Pande (CAD 2008), Muraleedharan &
Muthuganapathy (2021) — classical STL feature recognition; Point2Cyl (CVPR 2022), ComplexGen
(SIGGRAPH 2022), Point2CAD (CVPR 2024), CAD-Recode (ICCV 2025) — mesh/point cloud → parametric
CAD; ABC (CVPR 2019), DeepCAD (ICCV 2021), Fusion 360 Gallery (SIGGRAPH 2021) — datasets and the
sketch-extrude representation; surveys: Lin et al. (Appl. Sci. 2025), Debnath et al.
(Metrology 2025), ML feature recognition (Int. J. Manuf. Res. 2026).
