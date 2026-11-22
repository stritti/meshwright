# What `auto` recognises

| In the STL | In SolidCraft | Parameters |
|---|---|---|
| A rectangular plate, thickness along Z | `Sketch1` (rectangle on XY), `Extrude1` and `Move1` (to where the plate is) | `base_x`, `base_y`, `base_t`; the centre `base_cx`, `base_cy` and the bottom `base_z0` |
| A plate with any other outline of straight sides | `Sketch1` (the outline on XY, sides along X or Y kept horizontal or vertical), `Extrude1` and `Move1` | `extrude_len` (thickness), `extrude_0` (bottom) |
| Round vertical through holes (8 or more facets going evenly round the circle) | `Hole1`, `Hole2`, … numbered from left to right | `hole_<n>_x`, `hole_<n>_y` (where the hole is in the file), `hole_<n>_d` |
| A counterbored through hole, its counterbore on the top or the bottom face | `Hole<n>` of type counterbore (drilled from that face) | `hole_<n>_d`, `hole_<n>_cbore_d`, `hole_<n>_cbore_depth` (from that face) |
| A countersunk through hole, on the top or the bottom face | `Hole<n>` of type countersink | `hole_<n>_d`, `hole_<n>_csk_d` (at the face), `hole_<n>_csk_angle` (the cone's full angle) |
| Three or more holes of one diameter on a straight line at one pitch | `HoleRow<n>` and `HoleRow<n>Pattern` (a rectangular pattern along the row) | `hole_row_<n>_n`, `_pitch`, `_d`, and the first hole `_x`, `_y` |
| Holes of one diameter in an axis-aligned grid of at least 2 x 2 | `HoleGrid<n>` and `HoleGrid<n>Pattern` | `hole_grid_<n>_nx`, `_ny`, `_pitch_x`, `_pitch_y`, `_d`, and the corner hole `_x`, `_y` |
| Three or more holes of one diameter equally spaced on a circle (a bolt circle) | `HoleCircle<n>` and `HoleCircle<n>Pattern` (a circular pattern about the circle's centre) | `hole_circle_<n>_n`, `_d`, `_r` (centre to hole), `_cx`, `_cy`, `_angle` (first hole, from +X) |
| A round blind hole with a flat floor, drilled from the top or the bottom face | `Hole<n>` with a depth (drilled from that face) | `hole_<n>_d`, `hole_<n>_depth` (from that face) |
| A straight slot through the plate (two round ends of one width), along X, Y or at any angle | `Slot<n>Sketch` (the slot on XY) and `Slot<n>` (a cut through everything) | `slot_<n>_len` (between the end centres), `_w`, the middle `_x`, `_y`, and `_angle` (of the long axis, from +X) |
| The four vertical corners of a rectangular plate rounded with one radius | `Fillet1` | `fillet_r` |
| Every vertical corner of any other outline rounded with one radius, inside and outside corners alike | `Fillet1` (before the holes, so a hole may lie in an inside corner's rounding) | `outline_fillet_r` |

The parameter names are mesh2cad's, and the part stays where it is in the file. A chamfered
corner is simply one more straight side, so it comes over exactly.

## Checks

The rebuilt part must match the mesh's volume within 1%, otherwise the mesh is kept. The import
report notes how much the volume changed because round facets became true circles.

## Not recognised yet

These are kept as a mesh, with the reason in the report:

- curved sides and round outlines
- corners with different radii, or only some corners rounded
- D-shaped and polygonal cut-outs, blind or slanted slots
- corners rounded almost all the way along their sides
- steps, pockets and bosses
- blind holes with a drill point or a counterbore
- parts standing upright or rotated

The [roadmap](/guide/roadmap) lists the order in which they are coming.
