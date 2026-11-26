# Example parts

STL files to try revmesh on: everyday parts, tessellated the way a CAD program exports them.
Open one in SolidCraft with **Plugins › revmesh › stl** and change its parameters.

| File | Part | What revmesh makes of it |
|---|---|---|
| [`sbc-mount.stl`](sbc-mount.stl) | 95 x 66 x 3 mm mounting plate for a single-board computer | plate, two 2 x 2 hole grids, rounded corners |
| [`nema17-motor-plate.stl`](nema17-motor-plate.stl) | 60 x 60 x 5 mm plate for a NEMA 17 stepper | plate, pilot bore, two 2 x 2 hole grids, rounded corners |
| [`square-flange.stl`](square-flange.stl) | 80 x 80 x 8 mm flange | plate, bore, bolt circle of six holes, rounded corners |
| [`corner-plate.stl`](corner-plate.stl) | L-shaped corner plate, 4 mm | L outline sketch, all corners rounded, hole row, single hole |
| [`chamfered-cover.stl`](chamfered-cover.stl) | 80 x 50 x 4 mm cover with chamfered corners | eight-sided outline sketch, hole row, cable hole |
| [`spacer-bar.stl`](spacer-bar.stl) | 20 x 12 x 70 mm bar along X with a bore | outline extruded along X, hole along X, kept in place |
| [`rounded-block.stl`](rounded-block.stl) | 60 x 40 x 25 mm block, all edges rounded | box with all twelve edges filleted |

Pictures, the features and every parameter: the docs'
[examples gallery](https://stritti.github.io/revmesh/guide/examples).

The files are generated, never edited by hand:

```sh
cargo run -q -p revmesh-core --example examples_stl -- examples
cargo test -p revmesh-core --test examples   # each imports as the gallery describes
```

More parts, one feature each (counterbores, slots, bosses, pockets, …), are the plugin's test
files in [`plugin/revmesh/tests/`](../plugin/revmesh/tests/).

## Headless, from a script

[`cli/`](https://github.com/stritti/revmesh/tree/main/examples/cli) shows how to import without
the app: install the `cargo xtask dist` zip, then `file.import` with the importer entry
`revmesh.stl` through `solidcraft-cli`, and export the rebuilt design as STL. One command, also
run by CI (job `example-cli`): `scripts/example-cli.sh`.
