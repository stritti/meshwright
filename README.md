# revmesh

[![CI](https://github.com/stritti/revmesh/actions/workflows/ci.yml/badge.svg)](https://github.com/stritti/revmesh/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/stritti/revmesh/branch/main/graph/badge.svg)](https://codecov.io/gh/stritti/revmesh)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Docs](https://img.shields.io/badge/docs-stritti.github.io%2Frevmesh-3e7bd6.svg)](https://stritti.github.io/revmesh/)

**Open STL files in [SolidCraft](https://github.com/tangency-labs/solidcraft) as editable designs.**

revmesh is a SolidCraft importer plugin. Instead of a dead triangle mesh, it recognises what
the part was made of (plates, holes, slots, fillets, …) and writes a parametric feature history:
a sketch, an extrusion, holes and fillets, driven by named parameters such as `base_x` or
`hole_1_d`. Change a parameter and SolidCraft rebuilds the part.

The recognition follows [mesh2cad](https://github.com/emirmrkaljevic/mesh2cad)
(MIT): its approach, the parts it targets and its parameter names. It is written in Rust, so it
runs inside SolidCraft's WebAssembly sandbox without Python, numpy or OpenCascade.

> **Status: early.** The plugin pipeline works end to end: STL in, design out, tested in
> SolidCraft 0.48. Recognition finds plates: any outline of straight sides (rectangles,
> L-shapes, brackets, chamfered or slanted sides) extruded along Z — or along X or Y, a part
> standing upright —, its corners parallel to that axis sharp or all rounded with one radius,
> with round through holes; and boxes with all twelve edges rounded with one radius. Everything
> else is imported as a mesh body, with the reason in the
> import report. See the [roadmap](#roadmap).

## Install

1. Download `revmesh-<version>.zip` (English) or `revmesh-<version>-de.zip` (German) from the
   [releases](https://github.com/stritti/revmesh/releases). Both are the same plugin with the same
   id, so install one of them: SolidCraft shows a plugin's texts as its package gives them.
2. In SolidCraft, open **Preferences › Plugins › Install plugin…** (also *File › Plugins…*) and pick
   the zip.

## Use

1. On the **Model** ribbon tab, click the label of the **Plugins** group to open the Plugins menu,
   and choose **revmesh › stl**. SolidCraft's own *File › Open* keeps using its built-in mesh
   import for `.stl`, so revmesh is reached through the Plugins menu.
2. The dialog offers **Import as**: `auto` (recognise features, keep the mesh where none are
   found; the default) or `mesh` (keep the mesh as it is).
3. Its button chooses the STL file. It opens as a new, unsaved design.
4. The **Import report** follows: a summary, one line per kind of change, and everything in
   detail under *Details*. **Plugins › Last import report…** shows it again later.

The plugin accepts STL files up to 8 MiB in `auto` mode and 32 MiB in `mesh` mode.

A recognised part arrives as a timeline (sketch, extrusion, holes, fillet) with its sizes as
parameters in the parameter panel: change `base_t` or `hole_1_d` and SolidCraft rebuilds it. A
part that is not recognised arrives as a mesh body, and the report says why. To try it, import
[`plugin/revmesh/tests/plate-holes.stl`](plugin/revmesh/tests/plate-holes.stl).

### What `auto` recognises

| In the STL | In SolidCraft | Parameters |
|---|---|---|
| A rectangular plate, thickness along Z | `Sketch1` (rectangle on XY), `Extrude1` and `Move1` (to where the plate is) | `base_x`, `base_y`, `base_t`; the centre `base_cx`, `base_cy` and the bottom `base_z0` |
| A plate with any other outline of straight sides | `Sketch1` (the outline on XY, sides along X or Y kept horizontal or vertical), `Extrude1` and `Move1` | `extrude_len` (thickness), `extrude_0` (bottom) |
| Any of those outlines extruded along X or Y (a part standing upright) | `Sketch1` on the YZ or XZ plane (the profile's coordinates are the sketch coordinates there), `Extrude1` along that axis, `Move1` | `extrude_len`, `extrude_0` (along the axis); the holes' and slots' positions named after the profile plane's world axes, e.g. `hole_1_y`, `hole_1_z` on an X extrusion |
| Round vertical through holes (8 or more facets going evenly round the circle) | `Hole1`, `Hole2`, … numbered from left to right | `hole_<n>_x`, `hole_<n>_y` (where the hole is in the file), `hole_<n>_d` |
| A counterbored through hole, its counterbore on the top or the bottom face | `Hole<n>` of type counterbore (drilled from that face) | `hole_<n>_d`, `hole_<n>_cbore_d`, `hole_<n>_cbore_depth` (from that face) |
| A countersunk through hole, on the top or the bottom face | `Hole<n>` of type countersink | `hole_<n>_d`, `hole_<n>_csk_d` (at the face), `hole_<n>_csk_angle` (the cone's full angle) |
| A through hole whose rim is rounded, on the top or the bottom face | `Hole<n>` and `Hole<n>Rim` (a fillet of the edge between the face and the bore) | `hole_<n>_d`, `hole_<n>_rim_r` (top) or `hole_<n>_bot_rim_r` (bottom) |
| A round blind hole with a flat floor, drilled from the top or the bottom face | `Hole<n>` with a depth (drilled from that face) | `hole_<n>_d`, `hole_<n>_depth` (from that face) |
| A straight slot through the plate (two round ends of one width), along X, Y or at any angle | `Slot<n>Sketch` (the slot on XY) and `Slot<n>` (a cut through everything) | `slot_<n>_len` (between the end centres), `_w`, the middle `_x`, `_y`, and `_angle` (of the long axis, from +X) |
| A round boss standing on the top face, straight, drafted (narrowing upwards) or with its top edge chamfered | `Boss<n>` (a cylinder joined to the plate, on its top face) | `boss_<n>_d`, `boss_<n>_h` (above the top face), `boss_<n>_fillet` (a round at its foot, if it has one; `Boss<n>Fillet`), `boss_<n>_draft_deg` (a drafted boss is a cone), `boss_<n>_chamfer` (45°) or `boss_<n>_chamfer_h` and `_w` (`Boss<n>Chamfer`), the centre `boss_<n>_x`, `boss_<n>_y` |
| A cut-out of straight sides through the plate (3 to 24 corners, each a real turn) | `CutoutsSketch` (every cut-out, on XY) and `Cutouts` (a cut through everything, after the outline's fillet) | none: its corners are as in the file (mesh2cad writes them as the point list `cutout_<n>`) |
| A rectangular block (pad) along X and Y standing on the top face | `Pad<n>` (a box joined to the plate) | `pad_<n>_w` (along X), `pad_<n>_l` (along Y), `pad_<n>_h`, the middle `pad_<n>_x`, `pad_<n>_y` |
| A rectangular blind pocket along X and Y with a flat floor, in the top or the bottom face | `Pocket<n>` (a box cut out of the plate) | `pocket_<n>_w`, `pocket_<n>_l`, `pocket_<n>_depth` (from that face), the middle `pocket_<n>_x`, `pocket_<n>_y` |
| Three or more holes of one diameter on a straight line at one pitch | `HoleRow<n>` and `HoleRow<n>Pattern` (a rectangular pattern along the row) | `hole_row_<n>_n`, `_pitch`, `_d`, and the first hole `_x`, `_y` |
| Holes of one diameter in an axis-aligned grid of at least 2 x 2 | `HoleGrid<n>` and `HoleGrid<n>Pattern` | `hole_grid_<n>_nx`, `_ny`, `_pitch_x`, `_pitch_y`, `_d`, and the corner hole `_x`, `_y` |
| Three or more holes of one diameter equally spaced on a circle (a bolt circle) | `HoleCircle<n>` and `HoleCircle<n>Pattern` (a circular pattern about the circle's centre) | `hole_circle_<n>_n`, `_d`, `_r` (centre to hole), `_cx`, `_cy`, `_angle` (first hole, from +X) |
| The four vertical corners of a rectangular plate rounded with one radius | `Fillet1` | `fillet_r` |
| Every vertical corner of any other outline rounded with one radius, inside and outside corners alike | `Fillet1` (before the holes, so a hole may lie in an inside corner's rounding) | `outline_fillet_r` |
| A rectangular box with all twelve edges rounded with one radius, with through holes | `Fillet1` on all twelve edges of the extrude (the four `side|side`, the four `side|top`, the four `side|start`), before the holes — as mesh2cad's `.edges().fillet()` does | `edge_fillet_r` |
| The intersection of outlines of straight sides extruded right across the part along two or three of X, Y and Z (a plate seen from above cut by a ramp seen from the side; every face flat and parallel to at least one axis) | `Sketch<k>` (each outline on its axis's plane) and `Extrude<k>` (the first a new body, each further one `intersect`), `Move1` | `extrude_<a>_len` (the length along axis `a`), `extrude_<a>_0` (where it starts along it) for each outline's axis; the outlines' corners as in the file (mesh2cad's point lists `outline_<a>`); openings of straight sides right through an outline (a window through the part along that axis) are drawn in its sketch and are the extruded profile's holes (mesh2cad's `outline_<a>_cutout_<i>`); straight blind cuts and pockets of straight sides along X, Y or Z are `Cut<n>Sketch` and `Cut<n>` (a cut extrusion from its start), `cut_<n>_len`, `cut_<n>_0` (where it starts in the file), the longest of the axes a rectangular pocket fits; round holes along X, Y or Z, through it or blind from a face, are `Hole<n>` (a cylinder cut out of it), `hole_<n>_d`, the hole's two coordinates across its axis (`hole_<n>_x`, `hole_<n>_y` along Z), `hole_<n>_depth` for a blind one |
| Outlines stacked along X, Y or Z, each extruded over its own span (a base, a smaller block on part of it, another on that), when no intersection of outlines fits: every face square to the axis or along it | per band `Sketch<k>` (its section on the axis's plane) and `Extrude<k>` from where the band below ends (the first a new body, the others joined), `Move1` | `extrude_<a><k>_len` and `extrude_<a><k>_0` (mesh2cad's names; a band's start is `extrude_<a><k-1>_0 + extrude_<a><k-1>_len`, so a band's new height moves the bands above); of the axes it stacks along, the one with the fewest bands |

The parameter names are mesh2cad's, and the part stays where it is in the file. Like mesh2cad,
which tries a prism along X, Y and Z, revmesh recognises an outline extruded along any of them:
Z first (a box is a prism along all three, and along Z it keeps mesh2cad's `base_*` names), then
X, then Y; an outline along X or Y is written as `extrude_len`, `extrude_0` and the outline's
corners, the way mesh2cad's extrusion module writes it. Its profile lies in the plane
perpendicular to the axis (X: y and z, Y: x and z, Z: x and y), and SolidCraft's YZ and XZ planes
sketch it directly (XZ's sketch y is -Z, so on a Y extrusion a slot's angle runs from +X towards
-Z). STL files
exported by SolidCraft itself, or by other tools that mesh walls in bands, are recognised too. Change
`hole_row_1_n` or `hole_circle_1_r` and every hole of that pattern follows. A grid wins over a
bolt circle (four corner holes are both). A complete grid can also be found alongside one
same-diameter hole that shares neither a row nor a column with it. Two holes are never a pattern, and a pattern with one
hole missing or out of place stays single holes. A row's direction is fixed (mesh2cad's
`hole_row_N_angle` is not a parameter, as a SolidCraft pattern direction is not an expression). A chamfered
corner is simply one more straight side, so it comes over exactly. The rebuilt part must match
the mesh's volume within 1%, otherwise the mesh is kept. The import report notes how much the
volume changed because round facets became true circles. Not recognised yet (kept as a mesh, with the
reason): curved sides and round outlines, corners with different radii or only some rounded,
D-shaped and other curved cut-outs, blind or slanted slots, corners rounded almost all the way along their sides,
steps, pockets other than rectangles, bosses with a draft, a round or a hole, other shapes on the top face, blind holes with a drill
point or a counterbore, and parts rotated off X, Y and Z. Blind holes, like counterbored and
countersunk ones, stay single holes, not patterns.

## How it works

```text
 STL bytes ──► stl::parse ──► Mesh ──► recognize ──► PlateSpec … ──► Design (JSON) ──► SolidCraft
                                            │                               ▲
                                            └── nothing found ──► mesh body ┘   + loss report
```

| Crate | What it does |
|---|---|
| [`revmesh-core`](crates/revmesh-core) | STL parsing, recognition and the SolidCraft document model. Pure Rust, no host calls, tested natively with `cargo test`. |
| [`revmesh-plugin`](crates/revmesh-plugin) | The WebAssembly component (`craft:plugin@0.2.0`, `importer-plugin` world). Reads the file from SolidCraft, calls the core, writes the design back. |
| [`xtask`](xtask) | Builds the component and the installable zip, and runs SolidCraft's plugin tests. |
| [`plugin/revmesh`](plugin/revmesh) | The plugin package: `plugin.json` and the test cases SolidCraft runs. |

The documentation is published at <https://stritti.github.io/revmesh/>.
More in [docs/architecture.md](docs/architecture.md), for the document format the importer
writes in [docs/design-format.md](docs/design-format.md), and for comparing a recogniser with
mesh2cad in [docs/development.md](docs/development.md).

## Build from source

You need Rust (the toolchain in `rust-toolchain.toml` is picked up automatically, with the
`wasm32-unknown-unknown` target) and, for the end-to-end tests, `solidcraft-cli`, which ships
with SolidCraft.

```sh
cargo test                     # unit tests of the core, natively
cargo xtask component          # builds plugin/revmesh/revmesh.wasm
cargo xtask dist               # target/dist/revmesh-<version>.zip (en) and revmesh-<version>-de.zip
cargo xtask plugin-test        # runs plugin/revmesh/tests in SolidCraft (set SOLIDCRAFT_CLI)
cargo xtask check              # checks every rebuild against its source mesh (volume + deviation)
```

## Roadmap

1. **Skeleton** (done): plugin pipeline, document writer, mesh fallback, quality gates, releases.
2. **Plates** (done): any outline of straight sides with round through holes and rounded
   vertical corners.
3. **Parity with mesh2cad**, one recogniser per pull request, each compared with mesh2cad's
   output (`scripts/compare-mesh2cad.sh`):
   1. (done) parameter names as mesh2cad writes them (`base_t`, `fillet_r`, `extrude_len`, …),
      and the part kept where it is in the file;
   2. (done) hole rows, grids and bolt circles as SolidCraft patterns;
   3. (done) counterbored and countersunk holes; 4. (done) blind holes; 5. (done) slots; 6. (done) bosses on the top face;
   7. (done) polygonal cut-outs; 8. (done) pads and pockets; 9. (done) extrusions along X and Y;
   10. (done) boxes with every edge rounded (`edge_fillet_r`), with through holes in them;
   bosses on side faces still to do; 11. (done) chamfers, drafts
   and the rounds at a boss's foot or a hole's rim; 12. combined extrusions: (done) the
   intersection of outlines along X, Y and Z, with openings through them, straight blind cuts
   into them and round holes, and (done) unions of outlines stacked along an axis; holes
   that cross, roofs and bevels on a stack still to do.
4. **Checking** (done): `cargo xtask check` compares the rebuilt part with the mesh through
   SolidCraft's `eval` (feature errors, volume) and an STL re-export (surface deviation,
   Hausdorff), with mesh2cad's gate, as mesh2cad checks its scripts with CadQuery.

## Agent skills

The [Agent Skills](https://agentskills.io) for professional WebAssembly-in-Rust development of
SolidCraft plugins are mirrored into the four well-known locations that skills-compatible agents
scan: [`.agents/skills/`](.agents/skills/), `.github/skills/`, `.claude/skills/` and
`.vibe/skills/` (identical copies — edit `.agents/skills/` and copy the change to the other
three). [AGENTS.md](AGENTS.md) is the short orientation for agents working on this repo.

| Skill | Use it when |
|---|---|
| [`solidcraft-plugin-scaffold`](.agents/skills/solidcraft-plugin-scaffold/SKILL.md) | creating a plugin crate, manifest or xtask wiring |
| [`solidcraft-wit-worlds`](.agents/skills/solidcraft-wit-worlds/SKILL.md) | choosing a `craft:plugin` world, capability or `plugin-error` variant |
| [`solidcraft-wasm-build`](.agents/skills/solidcraft-wasm-build/SKILL.md) | building, wrapping, packaging or shrinking the component |
| [`solidcraft-wasm-limits`](.agents/skills/solidcraft-wasm-limits/SKILL.md) | working within memory/fuel/deadline limits and streaming large files |
| [`solidcraft-host-calls`](.agents/skills/solidcraft-host-calls/SKILL.md) | calling host interfaces and mapping errors |
| [`solidcraft-importer-workflow`](.agents/skills/solidcraft-importer-workflow/SKILL.md) | implementing an importer slot end to end |
| [`solidcraft-plugin-testing`](.agents/skills/solidcraft-plugin-testing/SKILL.md) | adding unit, golden, eval or plugin-test coverage |

## Contributing

Contributions are very welcome: bug reports, STL files that import badly, recognisers, docs.
Start with [CONTRIBUTING.md](CONTRIBUTING.md). Everyone taking part follows the
[Code of Conduct](CODE_OF_CONDUCT.md).

## License

revmesh is licensed under the [MIT License](LICENSE). Unless you state otherwise, any
contribution you submit for inclusion is licensed the same way, without any additional terms or
conditions.

Code ported from mesh2cad keeps its original MIT copyright notice; see [NOTICE](NOTICE) and
[LICENSES/mesh2cad-MIT.txt](LICENSES/mesh2cad-MIT.txt). SolidCraft's plugin interface
(`crates/revmesh-plugin/wit/plugin.wit`) is © the SolidCraft authors, Apache-2.0.
revmesh is an independent project and not affiliated with SolidCraft or mesh2cad.
