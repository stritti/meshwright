# Examples

Concrete STL files to try revmesh on, each with what SolidCraft makes of it. Download a file,
open it in SolidCraft with **Plugins › revmesh › stl** (see [Use](/guide/use)), and change a
parameter in the parameter panel: the part rebuilds with the new value.

The pictures show the STL as it is, a mesh of flat triangles (the thin lines are the facets'
edges). After the import the round faces are true circles and every number in the tables below
is a parameter you can edit.

## Everyday parts

These are in [`examples/`](https://github.com/stritti/revmesh/tree/main/examples) in the
repository, tessellated the way a CAD program exports STL.

### Single-board computer mount

![sbc-mount.stl: a 95 x 66 x 3 mm plate with rounded corners and eight holes](/examples/sbc-mount.png)

[`sbc-mount.stl`](https://github.com/stritti/revmesh/raw/main/examples/sbc-mount.stl): a
plate for a credit-card-sized board, with the board's four M2.5 holes on their 58 x 49 mm grid
and four Ø4.5 holes to screw the plate down.

| In SolidCraft | Parameters |
|---|---|
| `Sketch1`, `Extrude1`, `Move1`: the plate | `base_x` 95, `base_y` 66, `base_t` 3 |
| `HoleGrid1` + `HoleGrid1Pattern`: the four Ø4.5 holes | `hole_grid_1_d` 4.5, `hole_grid_1_pitch_x` 83, `hole_grid_1_pitch_y` 54 |
| `HoleGrid2` + `HoleGrid2Pattern`: the board's holes | `hole_grid_2_d` 2.7, `hole_grid_2_pitch_x` 58, `hole_grid_2_pitch_y` 49, corner `hole_grid_2_x` 18.5, `hole_grid_2_y` 8.5 |
| `Fillet1`: the four corners | `fillet_r` 4 |

Try: set `hole_grid_2_pitch_x` and `_pitch_y` to another board's hole spacing.

### NEMA 17 motor plate

![nema17-motor-plate.stl: a 60 x 60 x 5 mm plate with a large bore and two square hole patterns](/examples/nema17-motor-plate.png)

[`nema17-motor-plate.stl`](https://github.com/stritti/revmesh/raw/main/examples/nema17-motor-plate.stl):
the Ø22.5 pilot bore and the four M3 holes on the 31 mm square of a NEMA 17 stepper motor, and
four Ø5.5 holes on a 48 mm square.

| In SolidCraft | Parameters |
|---|---|
| The plate | `base_x` 60, `base_y` 60, `base_t` 5 |
| `HoleGrid1` + pattern: the Ø5.5 holes | `hole_grid_1_d` 5.5, `hole_grid_1_pitch_x` / `_pitch_y` 48 |
| `HoleGrid2` + pattern: the motor's holes | `hole_grid_2_d` 3.4, `hole_grid_2_pitch_x` / `_pitch_y` 31 |
| `Hole1`: the pilot bore | `hole_1_d` 22.5, `hole_1_x` 30, `hole_1_y` 30 |
| `Fillet1` | `fillet_r` 5 |

Try: a NEMA 23 motor has its holes on 47.14 mm and a Ø38.1 pilot.

### Square flange

![square-flange.stl: an 80 x 80 x 8 mm plate with a bore and six holes on a bolt circle](/examples/square-flange.png)

[`square-flange.stl`](https://github.com/stritti/revmesh/raw/main/examples/square-flange.stl):
a Ø30 bore and six M6 clearance holes on a Ø55 bolt circle.

| In SolidCraft | Parameters |
|---|---|
| The plate | `base_x` 80, `base_y` 80, `base_t` 8 |
| `HoleCircle1` + `HoleCircle1Pattern`: the bolt circle | `hole_circle_1_n` 6, `hole_circle_1_d` 6.6, `hole_circle_1_r` 27.5, centre `hole_circle_1_cx` / `_cy` 40, `hole_circle_1_angle` 0 |
| `Hole1`: the bore | `hole_1_d` 30 |
| `Fillet1` | `fillet_r` 8 |

Try: `hole_circle_1_n` 8 puts eight holes on the circle.

### Corner plate

![corner-plate.stl: an L-shaped plate with rounded corners and four holes](/examples/corner-plate.png)

[`corner-plate.stl`](https://github.com/stritti/revmesh/raw/main/examples/corner-plate.stl):
an L to brace a frame, every corner rounded, inside and outside alike. Its outline is a sketch
whose six sides you can dimension; the plate is that sketch extruded.

| In SolidCraft | Parameters |
|---|---|
| `Sketch1` (the L), `Extrude1`, `Move1` | `extrude_len` 4, `extrude_0` 0; the sides in the sketch |
| `Fillet1`: all six vertical corners | `outline_fillet_r` 3 |
| `HoleRow1` + `HoleRow1Pattern`: three Ø5 holes along the long leg | `hole_row_1_n` 3, `hole_row_1_pitch` 22, `hole_row_1_d` 5 |
| `Hole1`: the Ø6 hole in the short leg | `hole_1_d` 6, `hole_1_x` 10, `hole_1_y` 38 |

Holes of one diameter become a pattern only when all of them form it, which is why the hole in
the short leg has a diameter of its own here.

### Chamfered cover

![chamfered-cover.stl: an 80 x 50 x 4 mm cover with chamfered corners, a row of holes and a cable hole](/examples/chamfered-cover.png)

[`chamfered-cover.stl`](https://github.com/stritti/revmesh/raw/main/examples/chamfered-cover.stl):
corners cut off at 45°, a row of four Ø4 holes and a Ø12 cable hole. A chamfered corner is one
more straight side of the outline, so the cover's sketch has eight sides.

| In SolidCraft | Parameters |
|---|---|
| `Sketch1` (eight sides), `Extrude1`, `Move1` | `extrude_len` 4 |
| `HoleRow1` + pattern | `hole_row_1_n` 4, `hole_row_1_pitch` 20, `hole_row_1_d` 4 |
| `Hole1`: the cable hole | `hole_1_d` 12 |

### Spacer bar

![spacer-bar.stl: a 70 mm long bar lying along X with a bore along its length](/examples/spacer-bar.png)

[`spacer-bar.stl`](https://github.com/stritti/revmesh/raw/main/examples/spacer-bar.stl): a
20 x 12 mm bar, 70 mm long along X, with a Ø6.5 bore along it, placed away from the origin as it
sat in an assembly. revmesh sees that it is extruded along X, not Z, and keeps it where it is.

| In SolidCraft | Parameters |
|---|---|
| `Sketch1` on the YZ plane, `Extrude1` along X, `Move1` | `extrude_len` 70, `extrude_0` 15 (where it starts along X) |
| `Hole1` along X | `hole_1_d` 6.5, `hole_1_y` 15, `hole_1_z` 16 |

### Rounded block

![rounded-block.stl: a 60 x 40 x 25 mm block with all twelve edges rounded](/examples/rounded-block.png)

[`rounded-block.stl`](https://github.com/stritti/revmesh/raw/main/examples/rounded-block.stl):
a block with all twelve edges rounded with 4 mm, like a handheld enclosure before it is
shelled.

| In SolidCraft | Parameters |
|---|---|
| The block | `base_x` 60, `base_y` 40, `base_t` 25 |
| `Fillet1`: all twelve edges | `edge_fillet_r` 4 |

Try: a **Shell** in SolidCraft turns it into an enclosure, which the mesh could not do.

## One feature at a time

The plugin's own test files, in
[`plugin/revmesh/tests/`](https://github.com/stritti/revmesh/tree/main/plugin/revmesh/tests),
show each kind of feature on its own. Most of them were modelled and exported in SolidCraft.
[What the import recognises](/guide/recognition) has the full list.

### Counterbores and countersinks

![stepped-holes.stl](/examples/stepped-holes.png)

[`stepped-holes.stl`](https://github.com/stritti/revmesh/raw/main/plugin/revmesh/tests/stepped-holes.stl):
`Hole1` and `Hole2` are counterbored (`hole_1_cbore_d` 10, `hole_1_cbore_depth` 3), `Hole3` is
countersunk (`hole_3_csk_d` 12, `hole_3_csk_angle` 90).

### Blind holes

![blind-holes.stl](/examples/blind-holes.png)

[`blind-holes.stl`](https://github.com/stritti/revmesh/raw/main/plugin/revmesh/tests/blind-holes.stl):
three holes with a flat floor and one through hole; a blind hole has a depth
(`hole_1_depth` 4, `hole_3_depth` 2.5, `hole_4_depth` 6).

### Rounded hole rims

![hole-rims.stl](/examples/hole-rims.png)

[`hole-rims.stl`](https://github.com/stritti/revmesh/raw/main/plugin/revmesh/tests/hole-rims.stl):
a hole rounded on the top face and one on the bottom face, each a `Hole<n>` with a fillet
`Hole<n>Rim` (`hole_1_rim_r` 1.5, `hole_2_bot_rim_r` 1).

### Slots

![slots.stl](/examples/slots.png)

[`slots.stl`](https://github.com/stritti/revmesh/raw/main/plugin/revmesh/tests/slots.stl):
slots along X, along Y and at 30°, each a sketch and a cut (`slot_<n>_len`, `_w`, `_x`, `_y`,
`_angle`).

### Bosses with a round at the foot

![boss-fillet.stl](/examples/boss-fillet.png)

[`boss-fillet.stl`](https://github.com/stritti/revmesh/raw/main/plugin/revmesh/tests/boss-fillet.stl):
`Boss1` (`boss_1_d` 10, `boss_1_h` 8) with `Boss1Fillet` (`boss_1_fillet` 2), and a plain
`Boss2`.

### Drafted and chamfered bosses

![boss-draft-chamfer.stl](/examples/boss-draft-chamfer.png)

[`boss-draft-chamfer.stl`](https://github.com/stritti/revmesh/raw/main/plugin/revmesh/tests/boss-draft-chamfer.stl):
a boss narrowing upwards (`boss_1_draft_deg` 5) and one with a chamfered top edge
(`boss_2_chamfer` 1).

### Pads and pockets

![pockets.stl](/examples/pockets.png)

[`pockets.stl`](https://github.com/stritti/revmesh/raw/main/plugin/revmesh/tests/pockets.stl):
a block standing on the top face (`pad_1_w` 16, `pad_1_l` 12, `pad_1_h` 4) and two rectangular
pockets (`pocket_<n>_w`, `_l`, `_depth`).

### Cut-outs

![cutouts.stl](/examples/cutouts.png)

[`cutouts.stl`](https://github.com/stritti/revmesh/raw/main/plugin/revmesh/tests/cutouts.stl):
openings of straight sides through the plate, drawn in `CutoutsSketch` and cut by `Cutouts`;
their corners are in the sketch.

### Outlines extruded along several axes

![combined.stl](/examples/combined.png)

[`combined.stl`](https://github.com/stritti/revmesh/raw/main/plugin/revmesh/tests/combined.stl):
a part that is the intersection of an outline seen from above and one seen from the side; each
is a sketch and an extrusion (`extrude_z_len`, `extrude_x_len`).

![combined-3.stl](/examples/combined-3.png)

[`combined-3.stl`](https://github.com/stritti/revmesh/raw/main/plugin/revmesh/tests/combined-3.stl):
the same with outlines along X, Y and Z.

### Windows, cuts and pockets in such parts

![combined-windows.stl](/examples/combined-windows.png)

[`combined-windows.stl`](https://github.com/stritti/revmesh/raw/main/plugin/revmesh/tests/combined-windows.stl):
openings right through the part along an axis, drawn as holes in that axis's sketch.

![combined-cuts.stl](/examples/combined-cuts.png)

[`combined-cuts.stl`](https://github.com/stritti/revmesh/raw/main/plugin/revmesh/tests/combined-cuts.stl):
a blind cut and a pocket, each `Cut<n>Sketch` and `Cut<n>` (`cut_<n>_len`, `cut_<n>_0`).

## Real files from Printables

Parts other people designed and shared on [Printables](https://www.printables.com), exported by
their tools (OpenSCAD, FreeCAD, SketchUp, …) rather than generated here. They are test files in
[`plugin/revmesh/tests/printables/`](https://github.com/stritti/revmesh/tree/main/plugin/revmesh/tests/printables),
each under its author's licence (CC0 or CC BY 4.0, listed there). Where the author published
the design's source, the test checks revmesh's parameters against it.

### NEMA 17 plate for aluminium extrusion

![NEMA 17 aluminium extrusion mounting plate by diegotrap](/examples/printables/nema17-extrusion-plate.png)

[NEMA 17 aluminium extrusion mounting plate](https://www.printables.com/model/509515-nema-17-aluminium-extrusion-mounting-plate)
by diegotrap, CC BY 4.0. Every number matches the author's DXF: the 42 x 80 outline with 2 mm
chamfers (`Sketch1`, eight sides, `extrude_len` 4), the motor's holes (`HoleGrid1`,
`hole_grid_1_d` 3.2 on 31 mm), the Ø24 pilot bore (`hole_1_d` 24) and two slots
(`slot_<n>_len` 24, `slot_<n>_w` 5.25, `slot_1_x` −10, `slot_2_x` 10).

### Simple tiny calibration block

![Simple Tiny Calibration Block by Andy](/examples/printables/tiny-calibration-block.png)

[Simple Tiny Calibration Block](https://www.printables.com/model/182018-simple-tiny-calibration-block)
by Andy, CC0. As in the author's OpenSCAD file: `base_x` 30, `base_y` 20, `base_t` 10 and
`Pocket1` (`pocket_1_w` 20, `pocket_1_l` 10, `pocket_1_depth` 7).

### NEMA 17 spacer plate

![2mm NEMA17 Spacer Plate by derfley](/examples/printables/nema17-spacer-plate.png)

[2mm NEMA17 Spacer Plate](https://www.printables.com/model/149549-2mm-nema17-spacer-plate) by
derfley, CC BY 4.0: the 42.3 mm square with chamfered corners, `hole_grid_1_d` 3.5 on the
31 mm square and `hole_1_d` 27.

### Calibration block with a counterbore and a slot

![Calibration Block with Countersunk Hole and Slot by TechnoSwiss](/examples/printables/calibration-cube.png)

[Calibration Block with Countersunk Hole and Slot](https://www.printables.com/model/254589-calibration-block-with-countersunk-hole-and-slot)
by TechnoSwiss, CC BY 4.0. Despite its title, the hole has a flat step 2 mm down, so revmesh
makes it a counterbored `Hole1` (`hole_1_cbore_depth` 2); the slot is `Pocket1`
(5.5 x 2.4, 5 deep).

### Angle bracket

![90 degree angle bracket by Ken S](/examples/printables/angle-bracket.png)

[90 degree angle bracket](https://www.printables.com/model/276330-90-degree-angle-bracket) by
Ken S, CC0. As in the author's OpenSCAD file: the side view (base, upright and gusset) is one
outline extruded 30 mm along X (`extrude_x_len` 30), and the two Ø10 holes are `Hole1` in the
upright, 25 mm up (`hole_1_z` 25), and `Hole2` in the base (`hole_2_x` 15, `hole_2_y` 15).

### Not recognised yet

![Mending Plate by Gwilherm Kerherve](/examples/printables/mending-plate.png)

[Mending Plate](https://www.printables.com/model/1840620-mending-plate) by Gwilherm Kerherve,
CC0: countersinks that taper through the whole plate. Kept as a mesh.

![Heavy Duty Flat Mending Plate by Mateusz Tokarz](/examples/printables/flat-mending-plate-4x20x60.png)

[Heavy Duty Flat Mending Plates](https://www.printables.com/model/1549159-heavy-duty-flat-mending-plates-20x4mm-m4)
by Mateusz Tokarz, CC BY 4.0: its top edges are rounded all round. Kept as a mesh.

## Making your own

The everyday parts are generated, so they are exact and reproducible:

```sh
cargo run -q -p revmesh-core --example examples_stl -- examples   # writes examples/*.stl
pip install numpy pillow
python3 scripts/render-stl.py --out docs/public/examples examples/*.stl
python3 scripts/render-stl.py --out docs/public/examples/printables plugin/revmesh/tests/printables/*.stl
cargo test -p revmesh-core --test examples --test printables      # each imports as described here
```

To see what revmesh makes of any STL without SolidCraft, run
`cargo run -q -p revmesh-core --example params -- part.stl`: it prints every parameter and the
import report.
