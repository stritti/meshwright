# What `auto` recognises

| In the STL | In SolidCraft | Parameters |
|---|---|---|
| A rectangular plate, thickness along Z | `Sketch1` (rectangle on XY), `Extrude1` and `Move1` (to where the plate is) | `base_x`, `base_y`, `base_t`; the centre `base_cx`, `base_cy` and the bottom `base_z0` |
| A plate with any other outline of straight sides | `Sketch1` (the outline on XY, sides along X or Y kept horizontal or vertical), `Extrude1` and `Move1` | `extrude_len` (thickness), `extrude_0` (bottom) |
| Round vertical through holes (8 or more facets going evenly round the circle) | `Hole1`, `Hole2`, … numbered from left to right | `hole_<n>_x`, `hole_<n>_y` (where the hole is in the file), `hole_<n>_d` |
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
- slots, D-shaped and polygonal cut-outs
- corners rounded almost all the way along their sides
- steps, pockets, bosses and blind holes
- parts standing upright or rotated

The [roadmap](/guide/roadmap) lists the order in which they are coming.
