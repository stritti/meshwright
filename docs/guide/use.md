# Use

1. On the **Insert** ribbon tab, the plugin has its own **revmesh** group with the **stl** button
   (the same entry is in the Plugins menu: **Model** tab › **Plugins** group label › **revmesh ›
   stl**). SolidCraft's own *File › Open* keeps using its built-in mesh import for `.stl`, so
   revmesh is reached through the Insert tab or the Plugins menu.
2. The dialog only asks for **Units**: `mm` (the default — mesh files are unitless and read as
   millimetres) or `inch`, which is converted to millimetres (× 25.4) and noted in the report.
   What the import makes of the file it decides itself: features are recognised and the part is
   rebuilt parametrically where possible; where nothing is recognised — or the file is too large
   for recognition — the mesh is kept as it is, and the report says why.
3. Its button chooses the mesh file (`.stl`, `.obj` or `.ply`). It opens as a new, unsaved
   design.
4. The **Import report** follows: a summary, one line per kind of change, and everything in
   detail under *Details*. **Plugins › Last import report…** shows it again later.

A recognised part arrives as a timeline (sketch, extrusion, holes, fillet) with its sizes as
parameters in the parameter panel: change `base_t` or `hole_1_d` and SolidCraft rebuilds it. The
report also lists the surface deviation per recognised face — how far the mesh's facets lie from
the ideal faces (a hole's facets bow inside its true circle by their sagitta; a box rounded all
over reports its rounded surface; a combined or stacked part reports its walls per extrusion or
band, labelled `extrude z side 1`, `band 2 side 3`). A part that is not
recognised arrives as a mesh body, and the report says why.

::: tip Try it
Import
[`plugin/revmesh/tests/plate-holes.stl`](https://github.com/stritti/revmesh/blob/main/plugin/revmesh/tests/plate-holes.stl)
from the repository, or one of the [examples](/guide/examples): everyday parts with pictures
and the parameters each one gets.
:::

Which parts are recognised, and which parameters they get, is on
[What the import recognises](/guide/recognition).

## Limits

Mesh input (STL, OBJ, PLY): up to 32 MiB (`MAX_BYTES` in `crates/revmesh-plugin/src/lib.rs`).
Recognition runs up to ≈ 335k triangles — about 16 MiB of binary STL
(`MAX_RECOGNITION_TRIANGLES` in `crates/revmesh-core/src/import.rs`); a larger file is kept as a
mesh. Runtime: 128 MiB memory, a 120 s deadline and 10¹⁹ fuel, as requested in
`plugin/revmesh/plugin.json`. The file is streamed in 1 MiB chunks, so recognition of a 16 MiB
file peaks at ≈ 90 MiB in memory (binary STL; OBJ and PLY hold shared vertices, so they expand
less). Measured numbers and the reasoning behind them:
[Architecture](/architecture#limits-inside-solidcraft).
