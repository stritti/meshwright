# The SolidCraft design format, as revmesh writes it

SolidCraft's document format is internal and has no published schema. This page records what
revmesh relies on, learnt from designs SolidCraft saved as JSON (verified against 0.48.1) and
from hand-written files opened in it. Re-check it with every SolidCraft release.

## Version

`"format": "solidcraft/63"`. SolidCraft 0.48.1 upgrades older versions when it opens a file (it
saves `solidcraft/66`), so writing one fixed version is safe as long as SolidCraft still reads it.

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

## Features revmesh writes

| `type` | Key fields | Notes |
|---|---|---|
| `sketch` | `plane: {"type":"origin","name":"XY"}`, `sketch: {points, curves, constraints, counters}` | Point 0 is the fixed origin. Curves and constraints refer to points and curves by **index**. A `length` constraint names its model parameter in `param`. |
| `extrude` | `sketch` (feature id), `profiles: {"type":"all"}`, `extent: {distance, convention:"total_distance", direction:"positive"}`, `operation: "new_body"` | `body_names` lists the body it creates. A cut through everything both ways from its sketch: `extent: {distance: "0", convention: "total_distance", direction: "symmetric", through_all: true}`, `operation: "cut"`. |
| `hole` | `position` (numbers), `at` (three expressions), `direction`, `diameter`, `depth` (a blind hole's, from the face; through all without it), `hole: {"type":"simple"}`, `{"type":"counterbore", cb_diameter, cb_depth}` or `{"type":"countersink", cs_diameter, cs_angle}` | `at` follows parameters; `position` is the pick SolidCraft starts from. A hole whose counterbore or countersink is on the bottom face starts there with `direction: [0, 0, 1]`. `cs_angle` is the cone's full angle; a parameter in degrees drives it. |
| `fillet` | `edge_names`, `edges` (one point per edge), `radius` | See below. |
| `box` | `corner` (three expressions: the lowest corner), `length` (X), `width` (Y), `height` (Z), `operation: "join"` or `"cut"` | SolidCraft's box primitive. |
| `cylinder` | `base` (three expressions: the bottom face's centre), `axis: [0, 0, 1]`, `radius`, `height`, `operation: "join"` | SolidCraft's own cylinder primitive saves `radius` as `"(10 mm) / 2"` when given a diameter; revmesh writes `"boss_<n>_d / 2"`. |
| `move` | `bodies: ["Body1"]`, `translate: [x, y, z]` (three expressions) | Moves the whole body; later features use world coordinates. The extrusion's face names (`F2:side:l1`) stay valid after it. |
| `pattern` | `features: [ids]`, `pattern: {"type": "rectangular", dir1, count1, spacing1, dir2?, count2?, spacing2?}` or `{"type": "circular", origin: [3 expressions], axis, count, angle: "360 deg"}` | Counts include the original and are expressions; a parameter used as a count must have no unit (`"unit": ""`), SolidCraft refuses a length. Directions and the axis are numbers. |
| `mesh_import` | `file`, `meshes: [{name, positions, triangles}]` | Flat `x,y,z` positions and flat corner indices; `body_names` lists the bodies. |

## Planes

A sketch's `plane` names one of the origin planes. Their axes, as SolidCraft defines them
(`solidcore-record`'s `Plane`): `XY` sketches (x, y) with normal +Z, `YZ` sketches (y, z) with
normal +X, and `XZ` sketches (x, -z) with normal +Y. An extrusion with `direction: "positive"`
runs along the plane normal, so a sketch on `YZ` extrudes along +X and one on `XZ` along +Y.
revmesh writes a part standing along X or Y this way: the recognised profile's coordinates are
exactly the sketch coordinates on that plane, and the part is moved to where it is in the file by
the `move` after the extrusion, as for a Z plate.

## Combined extrusions

A part that is the intersection of outlines extruded along two or three axes gets one sketch per
outline on its axis's origin plane (the outline's corners are the sketch coordinates there, the
part's bounding box's lowest corner at the origin, so on `XZ` the second coordinate is -Z) and
one extrusion of it, `extent: {distance: "extrude_<a>_len", direction: "positive"}`, right across
the part. The first extrusion makes the body (`operation: "new_body"`), every further one has
`operation: "intersect"` with no `body_names` and no targets: SolidCraft intersects the existing
body, coincident faces included, as `solidcraft-cli 0.48.1` builds it. A `move` then takes the
body to where it is, its translation along each outline's axis `extrude_<a>_0`. Changing
`extrude_x_len` from 60 to 50 on `plugin/revmesh/tests/combined.stl` rebuilds to 32,500 mm³, as
computed, and `extrude_z_0 = 10` lifts the part by 10 mm.

An outline with openings right through it draws them in its sketch after the outline (its lines
`l1` to `l<n>` first) and extrudes `profiles: {"type": "curves", "loops": [["l1", …, "l<n>"]]}`:
the profile whose outer loop is the outline, the loops inside it its holes. With `{"type":
"all"}` SolidCraft extrudes every region, the openings included, and they come back filled
(`plugin/revmesh/tests/combined-windows.stl` rebuilt to 37,000 instead of 35,410 mm³).

A straight blind cut comes after the intersections and before the `move`, in the part's own
frame: `Cut<n>Sketch` on its axis's origin plane, and `Cut<n>`, an extrusion with `operation:
"cut"` and `extent: {distance: "cut_<n>_len", direction: "positive", start_offset: "cut_<n>_0 -
extrude_<a>_0"}`: `start_offset` (as SolidCraft saves it) moves the start along the plane's
normal, and `cut_<n>_0` is where the cut starts in the file, so the move's translation along the
axis is taken off again (a number where the axis has no outline). A rectangular pocket is a cut
along each axis it is closed or open along; as mesh2cad, the longest wins (the first of X, Y
and Z on a tie), so the pocket of `combined-pocket.stl` is cut along X from 5 to 15, and
`cut_1_len = 20` rebuilds to 36,400 mm³, as computed. revmesh numbers cuts by axis and start;
mesh2cad in the order of its mesh regions, so with several cuts the numbers may differ.

A round hole is a `cylinder` with `operation: "cut"`, also before the `move`: `radius:
"hole_<n>_d / 2"`, `base` the hole's two coordinates in the part's frame (`hole_<n>_x -
extrude_x_0`, or less a number where the axis has no outline) and, along its axis, `-1` with
`height: "extrude_<a>_len + 2"` for a through hole (1 mm beyond each end), or 1 mm outside the
face a blind hole opens on with `height: "hole_<n>_depth + 1"` and `axis` pointing into the
part. A cylinder needs no face to start from, so a hole may enter a slanted side. On
`plugin/revmesh/tests/combined-holes.stl` (built with SolidCraft cylinder cuts) the design
rebuilds to 35,272.124 mm³ as SolidCraft measures the source, and `hole_3_d = 8`,
`hole_2_depth = 5` to 34,675.221 mm³, as computed; mesh2cad writes the same 14 parameters and
values for the file exported `fine`.

## Stacked extrusions

A stack of outlines along one axis is one `Sketch<k>` and one `Extrude<k>` per band: the first
`operation: "new_body"`, the others `operation: "join"` with `extent.start_offset:
"extrude_<a><k>_0 - extrude_<a>1_0"`, then a `move` whose translation along the axis is
`extrude_<a>1_0`. The bands' starts after the first are user parameters with expressions
(`extrude_z2_0 = extrude_z1_0 + extrude_z1_len`), so a band's height moves the bands above it.
SolidCraft fuses bands that meet face to face into one body (`solidcraft-cli 0.48.1`); mesh2cad
overlaps them by 0.02 mm for OpenCascade's fuse, which SolidCraft does not need.
`plugin/revmesh/tests/stack.stl` (a base, an L and a diamond) rebuilds to 34,296 mm³ along Z and,
turned, along Y; `extrude_z1_len = 15` rebuilds to 46,296 mm³, the bands above lifted by 5 mm.

## Cut-outs

The cut-outs of straight sides are one sketch, `CutoutsSketch`, of closed loops in world
coordinates (corners free, sides along X or Y held horizontal or vertical), and a cut through
everything both ways from it, `Cutouts`. It comes after the outline's fillet, as the holes do: a
cut-out may lie in the material an inside corner's rounding adds (an L with 5 mm rounds and a
triangle in its inside rounding rebuilds to 6,169.124 mm³, as computed). Drawn into the outline's
own sketch instead, such a cut-out would be filled again by the fillet. (Had the openings been
drawn there, the extrusion would have to name the outline's profile, as SolidCraft saves it:
`"profiles": {"type": "curves", "loops": [["l1", "l2", "l3", "l4"]]}`; with `{"type": "all"}` it
fills every region.) `cutouts.stl` (exported by SolidCraft: a
70 x 40 x 6 plate with a 16 x 20 rectangle and a hexagon 7 mm to its corners cut out, and an Ø5
hole) imports to 13,998.378 mm³, 0.022 mm³ over what SolidCraft measured, as the hexagon's corners
are rounded to 0.01 mm; with `extrude_len = 9` and `hole_1_d = 6` it rebuilt without errors to the
value computed for those rounded corners. A rectangle with cut-outs is written as an outline,
as mesh2cad writes it (`extrude_len`, `extrude_0`), not as a `base_*` plate.

## Bosses

A boss is a `cylinder` joined to the plate, standing on the top face's expression
(`base_z0 + base_t`, or `extrude_0 + extrude_len` for an outline), after the holes and slots.
`bosses.stl` (exported by SolidCraft: a 70 x 40 x 6 plate with an Ø10 x 5 and an Ø8 x 3 boss and
an Ø5 hole) imports to 17,225.686 mm³, the volume SolidCraft measured for the part it exported.
With `boss_1_d = 12`, `boss_1_h = 8`, `boss_2_x = 40`, `boss_2_h = 4`, `base_t = 9` and
`base_z0 = 2` it rebuilt without errors to 26,129.126 mm³, as computed; exported again, the
rebuilt part is recognised with those parameters.

## Rounded boss feet

A boss whose foot is rounded is the same `cylinder`, followed by a `fillet` of the edge between
the plate's top face and the cylinder's side: `edge_names: ["F<extrude>:top|F<cylinder>:side"]`
(SolidCraft names that edge `F1:+z|F2:side` on a box primitive), radius `boss_<n>_fillet`. In
the mesh the round widens the boss's footprint in the top face by its radius; it must be meshed
in steps of at most 50° of its quarter circle (in one step it is a 45° chamfer, which looks the
same). `boss-fillet.stl` (exported by SolidCraft 0.48: a 70 x 40 x 6 plate with an Ø10 x 8 boss
rounded at its foot with 2 mm, an Ø8 x 5 boss and an Ø4 hole) imports to 17,633.625 mm³, the
volume SolidCraft measured for the part it exported (the round adds `2π (f² (r + f/2) −
π f²/4 (r + f) + f³/3)`); with `boss_1_fillet = 3`, `boss_1_d = 12`, `boss_1_x = 22` and
`base_t = 8` it rebuilt without errors to 23,536.520 mm³, as computed, and exported again it is
recognised with those values.

## Rounded hole rims

A through hole whose rim is rounded is a plain `hole` followed by a `fillet` of the edge between
the face and its bore: `edge_names: ["F<extrude>:top|F<hole>:bore:side:0"]` on the top face,
`"F<extrude>:start|F<hole>:bore:side:0"` on the bottom face, radius `hole_<n>_rim_r` or
`hole_<n>_bot_rim_r` as mesh2cad names them. In the mesh the mouth is wider than the bore by the
round's radius and exactly as deep, like a 90° countersink; it is a round when its corners between
the face and the bore lie on the quarter circle (a round in one step stays the countersink it looks
like). `hole-rims.stl` (exported by SolidCraft 0.48: a 70 x 40 x 8 plate with an Ø8 hole rounded
1.5 mm at its top rim and an Ø6 hole rounded 1 mm at its bottom rim) imports to 21,754.183 mm³,
the volume SolidCraft measured for the part it exported; with `hole_1_rim_r = 2`, `hole_1_d = 9`,
`hole_2_bot_rim_r = 1.5`, `hole_2_x = 52` and `base_t = 10` it rebuilt without errors to
27,044.286 mm³, as computed, and exported again it is recognised with those values.

## Drafted and chamfered bosses

A drafted boss is a `cone` (`base`, `axis`, `radius`, `top_radius`, `height`, `operation:
"join"`) whose `top_radius` is `boss_<n>_d / 2 - boss_<n>_h * tan(boss_<n>_draft_deg)`
(SolidCraft's expressions know `tan` of an angle parameter). A boss with a chamfered top edge is
the `cylinder`, followed by a `chamfer` (`edge_names: ["F<cylinder>:side|F<cylinder>:top"]`,
`edges`, `distance`, and `distance2` when the two differ). The first distance runs along the
face facing up most, the top: `boss_<n>_chamfer_w` in from the side, `distance2` the
`boss_<n>_chamfer_h` down it; a 45° chamfer is one `boss_<n>_chamfer`, as mesh2cad writes it.
In the mesh such a boss's top is narrower than its footprint; its wall is checked against the
profile of each kind in turn (a round at the foot, a straight cone, a cylinder and then the
chamfer), every corner and edge middle on it round the axis. `boss-draft-chamfer.stl` (exported
by SolidCraft 0.48: a 70 x 40 x 6 plate with an Ø12 x 8 boss drafted by 5°, an Ø10 x 6 boss
chamfered by 1 mm at its top and an Ø4 hole) imports to 17,984.519 mm³, the volume SolidCraft
measured for the part it exported; with `boss_1_draft_deg = 8`, `boss_1_h = 7`,
`boss_2_chamfer = 1.5`, `boss_2_d = 11` and `base_t = 8` it rebuilt without errors to
23,503.294 mm³, as computed, and exported again it is recognised with those values. A 1.5 x 0.5
chamfer (`distance: 1.5 mm, distance2: 0.5 mm`) comes back as `boss_1_chamfer_w = 1.5`,
`boss_1_chamfer_h = 0.5` and rebuilds to the volume SolidCraft measured.

## Pads and pockets

A pad is a `box` joined to the plate, its corner at `[pad_<n>_x - pad_<n>_w / 2, pad_<n>_y -
pad_<n>_l / 2, base_z0 + base_t]`; a pocket a `box` cut out of it, from `base_z0 + base_t -
pocket_<n>_depth` up for a pocket in the top face, or from `base_z0` for one in the bottom face.
`pockets.stl` (exported by SolidCraft: a 70 x 40 x 6 plate with a 16 x 12 x 4 pad, a 20 x 14 x 3
pocket in the top face, a 12 x 8 x 2 one in the bottom face and an Ø4 hole) imports to
16,460.602 mm³, the volume SolidCraft measured for the part it exported. With `pad_1_w = 20`,
`pad_1_h = 6`, `pocket_1_depth = 4`, `pocket_1_l = 10`, `pocket_2_x = 45`, `pocket_2_depth = 1`,
`base_t = 8` and `base_z0 = 3` it rebuilt without errors to 22,843.469 mm³, as computed; exported
again, it is recognised with those sizes.

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
rebuilt without warnings. revmesh therefore always writes them:

```text
F<extrude id>:side:<line id>|F<extrude id>:side:<line id>
```

`F2:side:l1` is the side face extrusion 2 made from sketch line `l1`; an edge is the pair of
faces it joins. The extrude's caps are `F2:top` (at the far end of a positive extrusion) and
`F2:start` (on the sketch plane), so a box rounded on all twelve edges names
`F2:side:l<i>|F2:top` and `F2:side:l<i>|F2:start` too. Other names seen in saved designs:
`F2:top`, `F3:bore:side:0` (a hole's bore), `F4:blend(F2:side:l1|F2:top)` (a fillet face).

## Verified

`crates/revmesh-core/tests/data/plate-with-hole.json` (written by `PlateSpec::to_design`)
opened in SolidCraft 0.48.1 with no errors: volume 16,452.05 mm³ as computed by hand
(70 × 40 × 6 minus an 8 mm hole minus four 3 mm corner fillets). Setting `base_x = 100`,
`hole_1_x = 50` and `hole_1_d = 12` rebuilt to 23,275.06 mm³, again as computed, with no
warnings.

A polygon outline (`ProfileSpec::to_design`: free sketch points, horizontal and vertical
constraints only, a fillet on every corner named `F2:side:l<i>|F2:side:l<i+1>`) works the same
way, inside corners included. The L-bracket of `plugin/revmesh/tests/l-bracket.stl` imports
in `solidcraft-cli 0.48.1` to 6,118.29 mm³ as computed; setting `outline_fillet_r = 4`,
`hole_1_d = 8`, `hole_2_x = 50` and `base_z = 8` rebuilt every feature without errors to
9,461.81 mm³, again as computed.

Since revmesh keeps a part where it is in the file, the sketch and extrusion stay at the
origin and a `move` feature right after the extrusion takes the body there: for a rectangular
plate by `[base_cx - base_x / 2, base_cy - base_y / 2, base_z0]`, for an outline by its corner
and `extrude_0`. Holes and fillet picks are in world coordinates. In `solidcraft-cli 0.48.1`
`plate-holes.stl` with `base_cx = 50`, `base_x = 90`, `base_z0 = 5`, `fillet_r = 5` and
`hole_2_x = 70` rebuilt without errors or warnings to 20,978.79 mm³, and the L-bracket (in the
file at (20, 10)) with `extrude_0 = 4`, `extrude_len = 8` and `outline_fillet_r = 4` to
9,706.85 mm³, both as computed.

A hole pattern is one `hole` feature at the first hole and a `pattern` feature on it. A bolt
circle's first hole is at `[cx + r * cos(angle), cy + r * sin(angle), top]` as expressions and
its pattern's origin is `[cx, cy, 0]`. `plate-patterns.stl` (a 2 x 2 grid, a row of three, a
bolt circle of six round a single bore) imports in `solidcraft-cli 0.48.1` to 55,995.49 mm³ as
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
corners, a 0.4 mm hole at (16, 16) inside that material) rebuilds in `solidcraft-cli 0.48.1` to
6,267.07 mm³ as computed, with 7 cylindrical faces (six roundings and the hole) and no feature
errors. The rectangle's history keeps its verified order (holes, then fillet): its corners are
all outside corners, which only take material away.

`solidcraft-cli eval design.sc` measures a design file directly (bodies, volume, face types and
each timeline feature's error), and `eval script.json --in design.sc` measures it after a script
such as `{"commands": [{"command": "parameters.change", "params": {"changes": {...}}}]}`.
