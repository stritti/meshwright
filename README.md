# meshwright

[![CI](https://github.com/stritti/meshwright/actions/workflows/ci.yml/badge.svg)](https://github.com/stritti/meshwright/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**Open STL files in [SolidCraft](https://github.com/tangency-labs/solidcraft) as editable designs.**

meshwright is a SolidCraft importer plugin. Instead of a dead triangle mesh, it recognises what
the part was made of (plates, holes, slots, fillets, …) and writes a parametric feature history:
a sketch, an extrusion, holes and fillets, driven by named parameters such as `base_x` or
`hole_1_d`. Change a parameter and SolidCraft rebuilds the part.

The recognition follows [mesh2cad](https://github.com/emirmrkaljevic/mesh2cad)
(MIT): its approach, the parts it targets and its parameter names. It is written in Rust, so it
runs inside SolidCraft's WebAssembly sandbox without Python, numpy or OpenCascade.

> **Status: early.** The plugin pipeline works end to end: STL in, design out, tested in
> SolidCraft 0.45. Recognition finds plates: any outline of straight sides (rectangles,
> L-shapes, brackets, chamfered or slanted sides) extruded along Z, its vertical corners sharp
> or all rounded with one radius, with round through holes. Everything else is imported as a
> mesh body, with the reason in the import report. See the [roadmap](#roadmap).

## Install

1. Download `meshwright-<version>.zip` from the [releases](https://github.com/stritti/meshwright/releases).
2. In SolidCraft, open **Preferences › Plugins › Install plugin…** (also *File › Plugins…*) and pick
   the zip.

## Use

1. On the **Model** ribbon tab, click the label of the **Plugins** group to open the Plugins menu,
   and choose **meshwright › stl**. SolidCraft's own *File › Open* keeps using its built-in mesh
   import for `.stl`, so meshwright is reached through the Plugins menu.
2. The dialog offers **Import as**: `auto` (recognise features, keep the mesh where none are
   found; the default) or `mesh` (keep the mesh as it is).
3. Its button chooses the STL file. It opens as a new, unsaved design.
4. The **Import report** follows: a summary, one line per kind of change, and everything in
   detail under *Details*. **Plugins › Last import report…** shows it again later.

A recognised part arrives as a timeline (sketch, extrusion, holes, fillet) with its sizes as
parameters in the parameter panel: change `base_z` or `hole_1_d` and SolidCraft rebuilds it. A
part that is not recognised arrives as a mesh body, and the report says why. To try it, import
[`plugin/meshwright/tests/plate-holes.stl`](plugin/meshwright/tests/plate-holes.stl).

### What `auto` recognises

| In the STL | In SolidCraft | Parameters |
|---|---|---|
| A rectangular plate, thickness along Z | `Sketch1` (rectangle on XY) and `Extrude1` | `base_x`, `base_y`, `base_z` |
| A plate with any other outline of straight sides | `Sketch1` (the outline on XY, sides along X or Y kept horizontal or vertical) and `Extrude1` | `base_z` |
| Round vertical through holes (8 or more facets going evenly round the circle) | `Hole1`, `Hole2`, … numbered from left to right | `hole_<n>_x`, `hole_<n>_y`, `hole_<n>_d` |
| Every vertical corner rounded with one radius, inside and outside corners alike | `Fillet1` (before the holes in a non-rectangular outline, so a hole may lie in an inside corner's rounding) | `outline_fillet_r` |

A chamfered corner is simply one more straight side, so it comes over exactly. The rebuilt part
must match the mesh's volume within 1%, otherwise the mesh is kept. The import report notes when
the plate was moved so that its bounding box's corner sits at the origin, and how much the volume
changed because round facets became true circles. Not recognised yet (kept as a mesh, with the
reason): curved sides and round outlines, corners with different radii or only some rounded,
slots, D-shaped and polygonal cut-outs, corners rounded almost all the way along their sides,
steps, pockets, bosses, blind holes, and parts standing upright or rotated.

## How it works

```text
 STL bytes ──► stl::parse ──► Mesh ──► recognize ──► PlateSpec … ──► Design (JSON) ──► SolidCraft
                                            │                               ▲
                                            └── nothing found ──► mesh body ┘   + loss report
```

| Crate | What it does |
|---|---|
| [`meshwright-core`](crates/meshwright-core) | STL parsing, recognition and the SolidCraft document model. Pure Rust, no host calls, tested natively with `cargo test`. |
| [`meshwright-plugin`](crates/meshwright-plugin) | The WebAssembly component (`craft:plugin@0.2.0`, `importer-plugin` world). Reads the file from SolidCraft, calls the core, writes the design back. |
| [`xtask`](xtask) | Builds the component and the installable zip, and runs SolidCraft's plugin tests. |
| [`plugin/meshwright`](plugin/meshwright) | The plugin package: `plugin.json` and the test cases SolidCraft runs. |

More in [docs/architecture.md](docs/architecture.md) and, for the document format the importer
writes, [docs/design-format.md](docs/design-format.md).

## Build from source

You need Rust (the toolchain in `rust-toolchain.toml` is picked up automatically, with the
`wasm32-unknown-unknown` target) and, for the end-to-end tests, `solidcraft-cli`, which ships
with SolidCraft.

```sh
cargo test                     # unit tests of the core, natively
cargo xtask component          # builds plugin/meshwright/meshwright.wasm
cargo xtask dist               # target/dist/meshwright-<version>.zip
cargo xtask plugin-test        # runs plugin/meshwright/tests in SolidCraft (set SOLIDCRAFT_CLI)
```

## Roadmap

1. **Skeleton** (done): plugin pipeline, document writer, mesh fallback, quality gates, releases.
2. **Plates** (done): any outline of straight sides with round through holes and rounded
   vertical corners, moved to the origin when it sits elsewhere.
3. **More features**: slots, counterbores and countersinks, hole patterns and bolt circles,
   bosses, pockets, curved sides, plates lying along X or Y.
4. **Checking**: compare the rebuilt part with the mesh through SolidCraft's `evaluate`
   (volume and deviation), as mesh2cad does with CadQuery.

## Contributing

Contributions are very welcome: bug reports, STL files that import badly, recognisers, docs.
Start with [CONTRIBUTING.md](CONTRIBUTING.md). Everyone taking part follows the
[Code of Conduct](CODE_OF_CONDUCT.md).

## License

meshwright is licensed under the [MIT License](LICENSE). Unless you state otherwise, any
contribution you submit for inclusion is licensed the same way, without any additional terms or
conditions.

Code ported from mesh2cad keeps its original MIT copyright notice; see [NOTICE](NOTICE) and
[LICENSES/mesh2cad-MIT.txt](LICENSES/mesh2cad-MIT.txt). SolidCraft's plugin interface
(`crates/meshwright-plugin/wit/plugin.wit`) is © the SolidCraft authors, Apache-2.0.
meshwright is an independent project and not affiliated with SolidCraft or mesh2cad.
