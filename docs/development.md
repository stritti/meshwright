# Development

## What an import produces

```sh
cargo run -q -p revmesh-core --example params -- part.stl                      # parameters and report
cargo run -q -p revmesh-core --example params -- part.stl --design part.json   # and the design
```

`part.json` opens in SolidCraft (File › Open) and can be measured headless with
`solidcraft-cli eval part.json`; `solidcraft-cli eval script.json --in part.json` measures it
after a script of `parameters.change` commands, which is how a new feature type is checked to
rebuild when its parameters change. `solidcraft-cli run --in part.json --out rebuilt.stl` exports
the rebuilt part.

`cargo xtask check` does all of that for every fixture in `plugin/revmesh/tests` and compares the
rebuilt part with the source mesh: `solidcraft-cli eval` for feature errors and volume, the
re-export for the surface deviation (Hausdorff, `check.rs`), gated as mesh2cad gates its scripts
(see `docs/architecture.md`, testing level 3).

## Comparing with mesh2cad

revmesh follows mesh2cad's recognition and parameter names, so every recogniser is compared
with mesh2cad's output on the same STL files. Install mesh2cad with CadQuery once, for example in
its own virtual environment:

```sh
uv venv ~/.venvs/mesh2cad && VIRTUAL_ENV=~/.venvs/mesh2cad uv pip install "mesh2cad[cadquery]"
export MESH2CAD=~/.venvs/mesh2cad/bin/mesh2cad
```

Then

```sh
scripts/compare-mesh2cad.sh plugin/revmesh/tests/*.stl
```

prints a Markdown table per file: every parameter name either tool writes, mesh2cad's value
(from the CadQuery script it writes) and revmesh's, with `≠` where they differ by more than
0.001. Below each table are mesh2cad's verdict (the script and its deviation from the mesh, or
why it refused) and revmesh's import summary. A pull request that adds or changes a
recogniser includes this table for its test files.

Differences that are expected, and why:

- **Numbering.** The two tools may number holes in a different order; compare the sets.
- **L-brackets.** mesh2cad writes an L with every corner rounded alike as `base_x`, `base_y`,
  `leg_x`, `leg_y`, `fillet_r` and `base_hole_N_*` / `leg_hole_N_*`; revmesh writes it as
  any other outline (`extrude_len`, `outline_fillet_r`, `hole_N_*`).
- **The outline itself.** mesh2cad's `outline` is a list of points; SolidCraft has no list
  parameters, so revmesh's outline lives in its sketch, where each side can be dimensioned.

## Fixtures

STL fixtures are generated, never edited by hand: `crates/revmesh-core/examples/plate_stl.rs`
writes them from a spec (`synth` tessellates the way CAD exporters do), and prints the volume
SolidCraft should measure after rebuilding. Each fixture in `plugin/revmesh/tests` has a
`.json` case that `cargo xtask plugin-test` runs in SolidCraft.
