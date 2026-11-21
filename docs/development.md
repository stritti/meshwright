# Development

## What an import produces

```sh
cargo run -q -p meshwright-core --example params -- part.stl                      # parameters and report
cargo run -q -p meshwright-core --example params -- part.stl --design part.json   # and the design
```

`part.json` opens in SolidCraft (File › Open) and can be measured headless with
`solidcraft-cli eval part.json`; `solidcraft-cli eval script.json --in part.json` measures it
after a script of `parameters.change` commands, which is how a new feature type is checked to
rebuild when its parameters change.

## Comparing with mesh2cad

meshwright follows mesh2cad's recognition and parameter names, so every recogniser is compared
with mesh2cad's output on the same STL files. Install mesh2cad with CadQuery once, for example in
its own virtual environment:

```sh
uv venv ~/.venvs/mesh2cad && VIRTUAL_ENV=~/.venvs/mesh2cad uv pip install "mesh2cad[cadquery]"
export MESH2CAD=~/.venvs/mesh2cad/bin/mesh2cad
```

Then

```sh
scripts/compare-mesh2cad.sh plugin/meshwright/tests/*.stl
```

prints a Markdown table per file: every parameter name either tool writes, mesh2cad's value
(from the CadQuery script it writes) and meshwright's, with `≠` where they differ by more than
0.001. Below each table are mesh2cad's verdict (the script and its deviation from the mesh, or
why it refused) and meshwright's import summary. A pull request that adds or changes a
recogniser includes this table for its test files.

Differences that are expected, and why:

- **Numbering.** The two tools may number holes in a different order; compare the sets.
- **Position.** mesh2cad keeps the part where it is in the file (`base_cx`, `base_x0`,
  `base_z0`, …); meshwright moves it to the origin and says so in the import report (being
  changed, see the roadmap in the README).

## Fixtures

STL fixtures are generated, never edited by hand: `crates/meshwright-core/examples/plate_stl.rs`
writes them from a spec (`synth` tessellates the way CAD exporters do), and prints the volume
SolidCraft should measure after rebuilding. Each fixture in `plugin/meshwright/tests` has a
`.json` case that `cargo xtask plugin-test` runs in SolidCraft.
