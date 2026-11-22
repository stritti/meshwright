# The SolidCraft design format, as meshwright writes it

SolidCraft's document format is internal and has no published schema. This page records what
meshwright relies on, learnt from designs SolidCraft 0.45.0 saved as JSON and from hand-written
files opened in it. Re-check it with every SolidCraft release.

## Version

`"format": "solidcraft/63"` (SolidCraft 0.45.0). SolidCraft upgrades older versions when it opens
a file, so writing one fixed version is safe as long as SolidCraft still reads it.

## Top level

```json
{
  "format": "solidcraft/63", "name": "", "units": "mm",
  "params": [ ... ], "features": [ ... ],
  "next_id": 5, "reserved_body_names": ["Body1"], "next_component_color_index": 0
}
```

An empty `name` names the design after the imported file.

## Parameters

```json
{"name": "base_x", "expr": "70 mm", "unit": "mm", "model": false}
{"name": "d1",     "expr": "base_x", "unit": "mm", "model": true}
```

`model: false` parameters are the user's, shown in the parameter panel. `model: true` ones belong
to features (sketch dimensions) and are driven by an expression.

## Features meshwright writes

| `type` | Key fields | Notes |
|---|---|---|
| `sketch` | `plane: {"type":"origin","name":"XY"}`, `sketch: {points, curves, constraints, counters}` | Point 0 is the fixed origin. Curves and constraints refer to points and curves by **index**. A `length` constraint names its model parameter in `param`. |
| `extrude` | `sketch` (feature id), `profiles: {"type":"all"}`, `extent: {distance, convention:"total_distance", direction:"positive"}`, `operation: "new_body"` | `body_names` lists the body it creates. A cut through everything both ways from its sketch: `extent: {distance: "0", convention: "total_distance", direction: "symmetric", through_all: true}`, `operation: "cut"`. |
| `hole` | `position` (numbers), `at` (three expressions), `direction`, `diameter`, `depth` (a blind hole's, from the face; through all without it), `hole: {"type":"simple"}`, `{"type":"counterbore", cb_diameter, cb_depth}` or `{"type":"countersink", cs_diameter, cs_angle}` | `at` follows parameters; `position` is the pick SolidCraft starts from. A hole whose counterbore or countersink is on the bottom face starts there with `direction: [0, 0, 1]`. `cs_angle` is the cone's full angle; a parameter in degrees drives it. |
| `fillet` | `edge_names`, `edges` (one point per edge), `radius` | See below. |
| `move` | `bodies: ["Body1"]`, `translate: [x, y, z]` (three expressions) | Moves the whole body; later features use world coordinates. The extrusion's face names (`F2:side:l1`) stay valid after it. |
| `pattern` | `features: [ids]`, `pattern: {"type": "rectangular", dir1, count1, spacing1, dir2?, count2?, spacing2?}` or `{"type": "circular", origin: [3 expressions], axis, count, angle: "360 deg"}` | Counts include the original and are expressions; a parameter used as a count must have no unit (`"unit": ""`), SolidCraft refuses a length. Directions and the axis are numbers. |
| `mesh_import` | `file`, `meshes: [{name, positions, triangles}]` | Flat `x,y,z` positions and flat corner indices; `body_names` lists the bodies. |

## Slot sketches

A slot is drawn as SolidCraft's centre-point slot tool draws it: lines `l1` and `l2`, arcs `a1`
and `a2` (`{"type": "arc", "c", "a", "b"}`, counter-clockwise from `a` to `b` about centre `c`,
whose points are named `a1.center` and `a2.center`), and the construction line `l3` between the
arc centres, with its midpoint `p5`. Its constraints:

- four `tangent` (`a`, `b`: curve indices), `equal` (the two arcs), `midpoint` (`p`, `l`)
- `length` of `l1` and `diameter` (`c`) of `a1`, each with a model parameter
- `coincident` (`p`, `q`) of the midpoint with `p6`, a point placed by the position parameters: `{"id": "p6", "pos": [x, y], "fixed": true, "exprs": {"x": "slot_1_x", "y": "slot_1_y"}}`. A `distance_x` / `distance_y` dimension from the origin would not do: those are unsigned (the side is the one the point is drawn on, a negative `value` fails the sketch with "invalid value: dimension value -75"), so a slot could not be moved across the origin.
- `angle` (`a`, `b`: curve indices, `value` in radians, its model parameter in degrees) between `l3` and the sketch's X axis. The axis is linked in as `l4`: two fixed points and a construction line carrying `"link": "j1"`, and `links: [{"id": "j1", "kind": "project", "source": {"type": "sketch_axis", "name": "x"}}]` on the sketch. Angles of 0 and 90 degrees work.

The sketch's `counters` then include `"a": 2` and `"j": 1`.

`slots.stl` (exported by SolidCraft: an 80 x 50 x 8 plate with a 20 x 6 slot along X, a 15 x 8
one along Y, a 16 x 5 one at 30 degrees and an Ø5 hole) imports to 28,497.522 mm³, the volume
SolidCraft measured for the part it exported. Changing every slot's sizes, moving two of them,
turning one to 45 degrees and setting `base_t = 10` rebuilt without errors to 34,129.978 mm³, as
computed; turning the slots to 20, 0 and 90 degrees kept the volume. The same part moved to
negative coordinates, and moved so that a slot's middle lies on the origin, rebuilt as well, and
a slot moved from X = 0 to X = -5 was there when the rebuilt part was exported and read again.

Fields SolidCraft derives itself and that can be left out: `param_names`, `face_names`,
`edge_refs`.

## Edge names: required for robust fillets

A fillet that has only pick points (`edges`) re-finds its edges by proximity after a rebuild.
In a test, changing `base_x` from 70 to 100 made two of four corner fillets snap to the wrong
edges ("2 edge reference(s) re-found on the nearest edge"). With `edge_names` the same change
rebuilt without warnings. meshwright therefore always writes them:

```text
F<extrude id>:side:<line id>|F<extrude id>:side:<line id>
```

`F2:side:l1` is the side face extrusion 2 made from sketch line `l1`; an edge is the pair of
faces it joins. Other names seen in saved designs: `F2:top`, `F3:bore:side:0` (a hole's bore),
`F4:blend(F2:side:l1|F2:top)` (a fillet face).

## Verified

`crates/meshwright-core/tests/data/plate-with-hole.json` (written by `PlateSpec::to_design`)
opened in SolidCraft 0.45.0 with no errors: volume 16,452.05 mm³ as computed by hand
(70 × 40 × 6 minus an 8 mm hole minus four 3 mm corner fillets). Setting `base_x = 100`,
`hole_1_x = 50` and `hole_1_d = 12` rebuilt to 23,275.06 mm³, again as computed, with no
warnings.

A polygon outline (`ProfileSpec::to_design`: free sketch points, horizontal and vertical
constraints only, a fillet on every corner named `F2:side:l<i>|F2:side:l<i+1>`) works the same
way, inside corners included. The L-bracket of `plugin/meshwright/tests/l-bracket.stl` imports
in `solidcraft-cli 0.45.0` to 6,118.29 mm³ as computed; setting `outline_fillet_r = 4`,
`hole_1_d = 8`, `hole_2_x = 50` and `base_z = 8` rebuilt every feature without errors to
9,461.81 mm³, again as computed.

Since meshwright keeps a part where it is in the file, the sketch and extrusion stay at the
origin and a `move` feature right after the extrusion takes the body there: for a rectangular
plate by `[base_cx - base_x / 2, base_cy - base_y / 2, base_z0]`, for an outline by its corner
and `extrude_0`. Holes and fillet picks are in world coordinates. In `solidcraft-cli 0.45.0`
`plate-holes.stl` with `base_cx = 50`, `base_x = 90`, `base_z0 = 5`, `fillet_r = 5` and
`hole_2_x = 70` rebuilt without errors or warnings to 20,978.79 mm³, and the L-bracket (in the
file at (20, 10)) with `extrude_0 = 4`, `extrude_len = 8` and `outline_fillet_r = 4` to
9,706.85 mm³, both as computed.

A hole pattern is one `hole` feature at the first hole and a `pattern` feature on it. A bolt
circle's first hole is at `[cx + r * cos(angle), cy + r * sin(angle), top]` as expressions and
its pattern's origin is `[cx, cy, 0]`. `plate-patterns.stl` (a 2 x 2 grid, a row of three, a
bolt circle of six round a single bore) imports in `solidcraft-cli 0.45.0` to 55,995.49 mm³ as
computed; with `hole_grid_1_nx = 3`, `hole_grid_1_pitch_x = 50`, `hole_row_1_n = 4`,
`hole_circle_1_n = 8`, `hole_circle_1_r = 18` and `hole_circle_1_angle = 0` it rebuilt without
errors or warnings to 55,566.66 mm³ with five more bores, again as computed.

`stepped-holes.stl` (exported by SolidCraft: a 60 x 40 x 8 plate with an Ø5.5 / Ø10 x 3
counterbore and an Ø6 / Ø12 90° countersink on top and an Ø4 / Ø8 x 2.5 counterbore on the
bottom) imports to 18,311.518 mm³, the volume SolidCraft measured for the part it exported. With
`hole_1_cbore_d = 12`, `hole_1_cbore_depth = 4`, `hole_2_cbore_depth = 3.5`, `hole_3_csk_d = 14`,
`hole_3_csk_angle = 82` and `base_t = 10` it rebuilt without errors or warnings to
22,614.14 mm³, as computed.

`blind-holes.stl` (exported by SolidCraft: a 60 x 40 x 8 plate with Ø6 x 4 and Ø3 x 6 blind
holes from the top, an Ø8 x 2.5 one from the bottom and an Ø5 hole through) imports to
18,761.748 mm³, the volume SolidCraft measured for the part it exported. A blind hole's `depth`
runs to its flat floor. With `hole_1_d = 7`, `hole_1_depth = 6`, `hole_3_x = 40`,
`hole_3_depth = 4`, `hole_4_depth = 2` and `base_t = 10` it rebuilt without errors to
23,357.544 mm³, as computed.

In a polygon outline the fillet comes before the holes: an inside corner's rounding adds
material, and a hole drilled before it would drill empty space. `l-inside-hole.stl` (5 mm
corners, a 0.4 mm hole at (16, 16) inside that material) rebuilds in `solidcraft-cli 0.45.0` to
6,267.07 mm³ as computed, with 7 cylindrical faces (six roundings and the hole) and no feature
errors. The rectangle's history keeps its verified order (holes, then fillet): its corners are
all outside corners, which only take material away.

`solidcraft-cli eval design.sc` measures a design file directly (bodies, volume, face types and
each timeline feature's error), and `eval script.json --in design.sc` measures it after a script
such as `{"commands": [{"command": "parameters.change", "params": {"changes": {...}}}]}`.
