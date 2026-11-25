# Use

1. On the **Model** ribbon tab, click the label of the **Plugins** group to open the Plugins
   menu, and choose **revmesh › stl**. SolidCraft's own *File › Open* keeps using its built-in
   mesh import for `.stl`, so revmesh is reached through the Plugins menu.
2. The dialog offers **Import as**: `auto` (recognise features, keep the mesh where none are
   found; the default) or `mesh` (keep the mesh as it is).
3. Its button chooses the STL file. It opens as a new, unsaved design.
4. The **Import report** follows: a summary, one line per kind of change, and everything in
   detail under *Details*. **Plugins › Last import report…** shows it again later.

A recognised part arrives as a timeline (sketch, extrusion, holes, fillet) with its sizes as
parameters in the parameter panel: change `base_t` or `hole_1_d` and SolidCraft rebuilds it. The
report also lists the surface deviation per recognised face — how far the mesh's facets lie from
the ideal faces (a hole's facets bow inside its true circle by their sagitta). A part that is not
recognised arrives as a mesh body, and the report says why.

::: tip Try it
Import
[`plugin/revmesh/tests/plate-holes.stl`](https://github.com/stritti/revmesh/blob/main/plugin/revmesh/tests/plate-holes.stl)
from the repository, or one of the [examples](/guide/examples): everyday parts with pictures
and the parameters each one gets.
:::

Which parts are recognised, and which parameters they get, is on
[What auto recognises](/guide/recognition).
