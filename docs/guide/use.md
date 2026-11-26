# Use

1. On the **Insert** ribbon tab, the plugin has its own **revmesh** group with the **stl** button
   (the same entry is in the Plugins menu: **Model** tab › **Plugins** group label › **revmesh ›
   stl**). SolidCraft's own *File › Open* keeps using its built-in mesh import for `.stl`, so
   revmesh is reached through the Insert tab or the Plugins menu.
2. The dialog offers **Import as**: `auto` (recognise features and rebuild the part
   parametrically — facets become ideal faces, the import report notes the deviation; the mesh is
   kept only where nothing is found; STL up to 16 MiB; the default) or `mesh` (keep the mesh
   exactly as it is; STL up to 32 MiB), and **Units**: `mm` (the default — STL is unitless and read
   as millimetres) or `inch`, which is converted to millimetres (× 25.4) and noted in the
   report.
3. Its button chooses the STL file. It opens as a new, unsaved design.
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
[What auto recognises](/guide/recognition).
