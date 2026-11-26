---
layout: home

hero:
  name: revmesh
  text: STL in, editable design out
  tagline: A SolidCraft plugin that turns an STL into a design you can keep editing — every size a named parameter; change it and the part rebuilds.
  image:
    src: /stl-to-design.svg
    alt: 'Animated: an STL file becomes an editable SolidCraft design with a parameter for every size'
  actions:
    - theme: brand
      text: Install
      link: /guide/install
    - theme: alt
      text: Examples
      link: /guide/examples
    - theme: alt
      text: Use
      link: /guide/use
    - theme: alt
      text: GitHub
      link: https://github.com/stritti/revmesh

features:
  - title: Editable, not a dead mesh
    details: The plate, its holes and its rounded corners arrive as named parameters such as base_x or hole_1_d. Change one in SolidCraft's parameter panel and the part rebuilds — you keep designing where the STL ended.
  - title: Knows what the part was made of
    details: Plates of any outline, round holes, slots, bosses, pockets and fillets are worked out from the mesh and rebuilt as editable operations instead of triangles.
  - title: Nothing extra to install
    details: The plugin runs inside SolidCraft itself — no Python, numpy or OpenCascade on your machine.
  - title: Honest about what it can't do
    details: Every import ends with a report. Anything not recognised stays a mesh body, and the report says why — no silent loss.
---

::: warning Status: early
The pipeline works end to end — STL in, editable design out, tested in SolidCraft 0.50. Parts
that are not recognised yet stay a mesh body, with the reason in the import report. What exactly
is recognised is listed under [What auto recognises](/guide/recognition); what is planned is on
the [roadmap](/guide/roadmap).
:::
