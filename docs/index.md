---
layout: home

hero:
  name: revmesh
  text: STL in, editable design out
  tagline: A SolidCraft importer plugin that turns triangle meshes into a parametric feature history you can keep designing on.
  image:
    src: /logo.svg
    alt: revmesh
  actions:
    - theme: brand
      text: Install
      link: /guide/install
    - theme: alt
      text: What it recognises
      link: /guide/recognition
    - theme: alt
      text: GitHub
      link: https://github.com/stritti/revmesh

features:
  - title: Editable, not a dead mesh
    details: Plates, holes and fillets arrive as a sketch, an extrusion, holes and a fillet, driven by named parameters such as base_x or hole_1_d. Change one and SolidCraft rebuilds the part.
  - title: Follows mesh2cad
    details: The recognition follows mesh2cad (MIT), its approach, the parts it targets and its parameter names, ported to Rust.
  - title: Runs in the sandbox
    details: A WebAssembly component for SolidCraft's plugin host, with no Python, numpy or OpenCascade to install.
  - title: Honest about losses
    details: Every import ends with a report. Anything not recognised stays a mesh body, and the report says why.
---

::: warning Status: early
The plugin pipeline works end to end: STL in, design out, tested in SolidCraft 0.45.
Recognition finds plates: any outline of straight sides extruded along Z, its vertical corners
sharp or all rounded with one radius, with round through holes. Everything else is imported as
a mesh body. See the [roadmap](/guide/roadmap).
:::
