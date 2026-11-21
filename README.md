# meshwright

[![CI](https://github.com/stritti/meshwright/actions/workflows/ci.yml/badge.svg)](https://github.com/stritti/meshwright/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**Open STL files in [SolidCraft](https://github.com/tangency-labs/solidcraft) as editable designs.**

meshwright is a SolidCraft importer plugin. Instead of a dead triangle mesh, it recognises what
the part was made of (plates, holes, slots, fillets, …) and writes a parametric feature history:
a sketch, an extrusion, holes and fillets, driven by named parameters such as `base_x` or
`hole_1_d`. Change a parameter and SolidCraft rebuilds the part.

The recognition is a Rust port of [mesh2cad](https://github.com/emirmrkaljevic/mesh2cad)
(MIT), so it runs inside SolidCraft's WebAssembly sandbox without Python, numpy or OpenCascade.

> **Status: early.** The plugin pipeline works end to end: STL in, design out, tested in
> SolidCraft 0.45. Recognition currently finds plain blocks; everything else is imported as a
> mesh body with a note in the import report. Porting mesh2cad's recognisers is the next step;
> see the [roadmap](#roadmap).

## Install

1. Download `meshwright-<version>.zip` from the [releases](https://github.com/stritti/meshwright/releases).
2. In SolidCraft, open **Preferences › Plugins › Install plugin…** and pick the zip.
3. Open an STL through the **Plugins** menu (meshwright › Open .stl). SolidCraft's own
   *File › Open* keeps using its built-in mesh import for `.stl`.

The import dialog offers **Import as**: `auto` (recognise features, keep the mesh where none are
found) or `mesh` (keep the mesh as it is).

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
2. **Plates**: port mesh2cad's plate, through-hole and outline-fillet recognition.
3. **More features**: slots, counterbores and countersinks, hole patterns, chamfers, pockets.
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
