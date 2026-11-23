# Origin and license

## Where the recognition comes from

The recognition follows [mesh2cad](https://github.com/emirmrkaljevic/mesh2cad) (MIT): its
approach, the parts it targets and its parameter names. mesh2cad is written in Python on numpy,
scipy, trimesh and OpenCascade, none of which can run inside SolidCraft's WebAssembly plugin
sandbox (no WASI, no subprocess, no network). revmesh therefore ports the recognition to Rust
and, instead of building a B-rep, writes SolidCraft's own feature history, which SolidCraft then
rebuilds. Each recogniser is compared with mesh2cad's output before it lands.

## How it works

```text
 STL bytes ──► stl::parse ──► Mesh ──► recognize ──► PlateSpec … ──► Design (JSON) ──► SolidCraft
                                            │                               ▲
                                            └── nothing found ──► mesh body ┘   + loss report
```

| Crate | What it does |
|---|---|
| [`revmesh-core`](https://github.com/stritti/revmesh/tree/main/crates/revmesh-core) | STL parsing, recognition and the SolidCraft document model. Pure Rust, no host calls, tested natively with `cargo test`. |
| [`revmesh-plugin`](https://github.com/stritti/revmesh/tree/main/crates/revmesh-plugin) | The WebAssembly component (`craft:plugin@0.2.0`, `importer-plugin` world). Reads the file from SolidCraft, calls the core, writes the design back. |
| [`xtask`](https://github.com/stritti/revmesh/tree/main/xtask) | Builds the component and the installable zip, and runs SolidCraft's plugin tests. |
| [`plugin/revmesh`](https://github.com/stritti/revmesh/tree/main/plugin/revmesh) | The plugin package: `plugin.json` and the test cases SolidCraft runs. |

More in [Architecture](/architecture) and [Design format](/design-format).

## License

revmesh is licensed under the
[MIT License](https://github.com/stritti/revmesh/blob/main/LICENSE). Code ported from
mesh2cad keeps its original MIT copyright notice; see
[NOTICE](https://github.com/stritti/revmesh/blob/main/NOTICE). SolidCraft's plugin interface
(`crates/revmesh-plugin/wit/plugin.wit`) is © the SolidCraft authors, Apache-2.0.

revmesh is an independent project and not affiliated with SolidCraft or mesh2cad.
