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
