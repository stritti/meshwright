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
| `extrude` | `sketch` (feature id), `profiles: {"type":"all"}`, `extent: {distance, convention:"total_distance", direction:"positive"}`, `operation: "new_body"` | `body_names` lists the body it creates. |
| `hole` | `position` (numbers), `at` (three expressions), `direction`, `diameter`, `hole: {"type":"simple"}` | `at` follows parameters; `position` is the pick SolidCraft starts from. |
| `fillet` | `edge_names`, `edges` (one point per edge), `radius` | See below. |
| `move` | `bodies: ["Body1"]`, `translate: [x, y, z]` (three expressions) | Moves the whole body; later features use world coordinates. The extrusion's face names (`F2:side:l1`) stay valid after it. |
| `mesh_import` | `file`, `meshes: [{name, positions, triangles}]` | Flat `x,y,z` positions and flat corner indices; `body_names` lists the bodies. |

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

In a polygon outline the fillet comes before the holes: an inside corner's rounding adds
material, and a hole drilled before it would drill empty space. `l-inside-hole.stl` (5 mm
corners, a 0.4 mm hole at (16, 16) inside that material) rebuilds in `solidcraft-cli 0.45.0` to
6,267.07 mm³ as computed, with 7 cylindrical faces (six roundings and the hole) and no feature
errors. The rectangle's history keeps its verified order (holes, then fillet): its corners are
all outside corners, which only take material away.

`solidcraft-cli eval design.sc` measures a design file directly (bodies, volume, face types and
each timeline feature's error), and `eval script.json --in design.sc` measures it after a script
such as `{"commands": [{"command": "parameters.change", "params": {"changes": {...}}}]}`.
