# What `auto` recognises

| In the STL | In SolidCraft | Parameters |
|---|---|---|
| A rectangular plate, thickness along Z | `Sketch1` (rectangle on XY), `Extrude1` and `Move1` (to where the plate is) | `base_x`, `base_y`, `base_t`; the centre `base_cx`, `base_cy` and the bottom `base_z0` |
| A plate with any other outline of straight sides | `Sketch1` (the outline on XY, sides along X or Y kept horizontal or vertical, every side a `Length` dimension), `Extrude1` and `Move1` | `extrude_len` (thickness), `extrude_0` (bottom), `outline_<i>_len` per side |
| Round vertical through holes (8 or more facets going evenly round the circle) | `Hole1`, `Hole2`, … numbered from left to right | `hole_<n>_x`, `hole_<n>_y` (where the hole is in the file), `hole_<n>_d` |
| A counterbored through hole, its counterbore on the top or the bottom face | `Hole<n>` of type counterbore (drilled from that face) | `hole_<n>_d`, `hole_<n>_cbore_d`, `hole_<n>_cbore_depth` (from that face) |
| A countersunk through hole, on the top or the bottom face | `Hole<n>` of type countersink | `hole_<n>_d`, `hole_<n>_csk_d` (at the face), `hole_<n>_csk_angle` (the cone's full angle) |
| A through hole whose rim is rounded, on the top or the bottom face | `Hole<n>` and `Hole<n>Rim` (a fillet of the edge between the face and the bore) | `hole_<n>_d`, `hole_<n>_rim_r` (top) or `hole_<n>_bot_rim_r` (bottom) |
| Three or more holes of one diameter on a straight line at one pitch | `HoleRow<n>` and `HoleRow<n>Pattern` (a rectangular pattern along the row) | `hole_row_<n>_n`, `_pitch`, `_d`, and the first hole `_x`, `_y` |
| Holes of one diameter in an axis-aligned grid of at least 2 x 2 | `HoleGrid<n>` and `HoleGrid<n>Pattern` | `hole_grid_<n>_nx`, `_ny`, `_pitch_x`, `_pitch_y`, `_d`, and the corner hole `_x`, `_y` |
| Three or more holes of one diameter equally spaced on a circle (a bolt circle) | `HoleCircle<n>` and `HoleCircle<n>Pattern` (a circular pattern about the circle's centre) | `hole_circle_<n>_n`, `_d`, `_r` (centre to hole), `_cx`, `_cy`, `_angle` (first hole, from +X) |
| A round blind hole with a flat floor, drilled from the top or the bottom face | `Hole<n>` with a depth (drilled from that face) | `hole_<n>_d`, `hole_<n>_depth` (from that face) |
| A straight slot through the plate (two round ends of one width), along X, Y or at any angle | `Slot<n>Sketch` (the slot on XY) and `Slot<n>` (a cut through everything) | `slot_<n>_len` (between the end centres), `_w`, the middle `_x`, `_y`, and `_angle` (of the long axis, from +X) |
| A round boss standing on the top face, straight, drafted (narrowing upwards) or with its top edge chamfered | `Boss<n>` (a cylinder joined to the plate, on its top face) | `boss_<n>_d`, `boss_<n>_h` (above the top face), `boss_<n>_fillet` (a round at its foot, if it has one; `Boss<n>Fillet`), `boss_<n>_draft_deg` (a drafted boss is a cone), `boss_<n>_chamfer` (45°) or `boss_<n>_chamfer_h` and `_w` (`Boss<n>Chamfer`), the centre `boss_<n>_x`, `boss_<n>_y` |
| A cut-out of straight sides through the plate (3 to 24 corners, each a real turn) | `CutoutsSketch` (every cut-out, on XY, every side a `Length` dimension) and `Cutouts` (a cut through everything, after the outline's fillet) | `cutout_<n>_<i>_len` per side (mesh2cad writes the corners as the point list `cutout_<n>`) |
| A rectangular block (pad) along X and Y standing on the top face | `Pad<n>` (a box joined to the plate) | `pad_<n>_w` (along X), `pad_<n>_l` (along Y), `pad_<n>_h`, the middle `pad_<n>_x`, `pad_<n>_y` |
| A rectangular blind pocket along X and Y with a flat floor, in the top or the bottom face | `Pocket<n>` (a box cut out of the plate) | `pocket_<n>_w`, `pocket_<n>_l`, `pocket_<n>_depth` (from that face), the middle `pocket_<n>_x`, `pocket_<n>_y` |
| A blind pocket of any other outline of straight sides (3 to 24 corners, each a real turn) with a flat floor, in the top or the bottom face | `Pocket<n>Sketch` (the mouth's outline on XY, every side a `Length` dimension) and `Pocket<n>` (a cut from that face down by `pocket_<n>_depth`), numbered after the rectangular pockets | `pocket_<n>_depth`, `pocket_<n>_<i>_len` per side |
| A step down from the top face along Z (a lower level whose region shares an edge with the outline) | written like a pocket of that region's outline: `Pocket<n>Sketch` and `Pocket<n>` cut from the top down to the level | `pocket_<n>_depth` (the step's height), `pocket_<n>_<i>_len` per side |
| The four vertical corners of a rectangular plate rounded with one radius | `Fillet1` | `fillet_r` |
| Every vertical corner of any other outline rounded with one radius, inside and outside corners alike | `Fillet1` (before the holes, so a hole may lie in an inside corner's rounding) | `outline_fillet_r` |
| A rectangular box with all twelve edges rounded with one radius, with through holes | `Fillet1` on all twelve edges of the extrude (the four `side|side`, the four `side|top`, the four `side|start`), before the holes | `edge_fillet_r` |
| The intersection of outlines of straight sides extruded right across the part along two or three of X, Y and Z (a plate seen from above cut by a ramp seen from the side; every face flat and parallel to at least one axis) | `Sketch<k>` (each outline on its axis's plane) and `Extrude<k>` (the first a new body, each further one `intersect`), `Move1` | `extrude_<a>_len` (the length along axis `a`), `extrude_<a>_0` (where it starts along it) for each outline's axis; the outlines' corners as in the file (mesh2cad's point lists `outline_<a>`); openings of straight sides right through an outline (a window through the part along that axis) are drawn in its sketch and are the extruded profile's holes (mesh2cad's `outline_<a>_cutout_<i>`); straight blind cuts and pockets of straight sides along X, Y or Z are `Cut<n>Sketch` and `Cut<n>` (a cut extrusion from its start), `cut_<n>_len`, `cut_<n>_0` (where it starts in the file), the longest of the axes a rectangular pocket fits; round holes along X, Y or Z, through it or blind from a face, are `Hole<n>` (a cylinder cut out of it), `hole_<n>_d`, the hole's two coordinates across its axis (`hole_<n>_x`, `hole_<n>_y` along Z), `hole_<n>_depth` for a blind one |
| Outlines stacked along X, Y or Z, each extruded over its own span (a base, a smaller block on part of it, another on that), when no intersection of outlines fits: every face square to the axis or along it | per band `Sketch<k>` (its section on the axis's plane) and `Extrude<k>` from where the band below ends (the first a new body, the others joined), `Move1` | `extrude_<a><k>_len` and `extrude_<a><k>_0` (mesh2cad's names; a band's start is `extrude_<a><k-1>_0 + extrude_<a><k-1>_len`, so a band's new height moves the bands above); of the axes it stacks along, the one with the fewest bands |

The [examples](/guide/examples) show these on concrete STL files. The parameter names are
mesh2cad's, and the part stays where it is in the file. A chamfered
corner is simply one more straight side, so it comes over exactly.

## Checks

The rebuilt part must match the mesh's volume within 1%, otherwise the mesh is kept. The import
report notes how much the volume changed because round facets became true circles.

## Not recognised yet

These are kept as a mesh, with the reason in the report:

- curved sides and round outlines
- corners with different radii, or only some corners rounded
- D-shaped and other curved cut-outs, blind or slanted slots
- corners rounded almost all the way along their sides
- steps along X or Y, pockets with curved sides, other shapes on the top face, and bosses with a draft, a round or a hole
- blind holes with a drill point or a counterbore
- parts rotated off the axes
- combined extrusions with holes that cross each other or a cut, round holes ending in a
  slanted face (they stay openings of their facets), counterbored or chamfered holes, cuts that
  touch each other or have islands; stacks with slanted roofs, bevels or holes

The [roadmap](/guide/roadmap) lists the order in which they are coming.
