# Comparable tools

revmesh turns an STL triangle mesh into a parametric, editable design inside SolidCraft. This
page lists comparable solutions — tools that turn a triangle mesh (STL, OBJ, PLY) or a point
cloud into an editable CAD model — with a link, the license and what each product is special
at. The **Feature history?** column says whether the mesh becomes named, parametric features
automatically, the way revmesh does it, or only a solid/surface that you edit by hand.

Licenses are as stated in each project's own `LICENSE` file; links and prices were checked in
November 2026 and may change. revmesh is an independent project and not affiliated with any
of the listed products.

## Automatic recognition, open source

| Tool | License | Cost | Special features | Feature history? |
|---|---|---|---|---|
| [revmesh (this project)](https://github.com/stritti/revmesh) | MIT | free | SolidCraft plugin that runs inside the WebAssembly sandbox — no Python, numpy or OpenCascade; writes SolidCraft's feature history with mesh2cad's parameter names; anything not recognised stays a mesh body, with the reason in the import report | Yes: plates and outlines extruded along X, Y or Z, through, blind, counterbored and countersunk holes, slots, bosses, pads, pockets, hole rows/grids/bolt circles, fillets |
| [mesh2cad](https://github.com/emirmrkaljevic/mesh2cad) | MIT | free (`pip`) | Python on numpy, trimesh and OpenCascade; CLI plus MCP server; writes an exact B-rep STEP and parametric CadQuery, OpenSCAD or FreeCAD scripts; fidelity report (Hausdorff, RMS, volume error); refuses parts it cannot express | Yes — the part classes and parameter names revmesh ports |
| [FreeCAD Reverse Engineering workbench](https://wiki.freecad.org/Reverse_Engineering_Workbench) | LGPL v2+ (part of [FreeCAD](https://www.freecad.org)) | free | Fits and approximates B-spline surfaces to meshes and point clouds; segmentation, surface reconstruction | Partially — surfaces, not an automatic feature tree |
| [Detessellate](https://github.com/DesignWeaver3D/Detessellate) (FreeCAD add-on) | LGPL-2.1 | free | Algorithm-assisted, sketch-based workflow for STL/OBJ/3MF: mesh cross-sections to aligned, loft-ready splines | Guided, not automatic |
| [MeshRemodel](https://github.com/mwganson/MeshRemodel) (FreeCAD add-on) | LGPL-2.1-or-later | free | Remodelling assistant: mesh vertices become points, then sketches; mesh repair tools | No — aids manual remodelling |
| [CAD Sketcher](https://github.com/hlorus/CAD_Sketcher) (Blender add-on) | GPL-3.0 | free | Constraint-based parametric sketcher (Solvespace solver) inside Blender; project mesh faces into sketches, then extrude/revolve | No automatic recognition of the mesh itself |

## Research prototypes (machine learning)

| Tool | License | Cost | Special features | Feature history? |
|---|---|---|---|---|
| [CAD-Recode](https://cad-recode.github.io/) ([paper](https://arxiv.org/abs/2412.14042), [code](https://github.com/filaPro/cad-recode)) | research code, no license file | free | Point cloud → CadQuery Python code via a lightweight network plus a pre-trained LLM; state of the art on the DeepCAD, Fusion 360 and CC3D benchmarks | Yes, as code — prototype, needs a GPU |
| [Point2CAD](https://www.obukhov.ai/point2cad.html) ([code](https://github.com/prs-eth/point2cad)) | Apache-2.0 | free | Point cloud → complete CAD model including freeform surfaces (implicit neural representation), edges and corners; state of the art on the ABC benchmark | Yes — prototype, needs a GPU, point cloud input |

## Commercial reverse-engineering suites

| Tool | License | Cost | Special features | Feature history? |
|---|---|---|---|---|
| [Geomagic Design X](https://hexagon.com/products/geomagic-design-x) (Hexagon) | proprietary | subscription or perpetual; Go from ≈ USD 1,900, Pro ≈ USD 22,000+ | Industry standard for scan-to-CAD: automatic feature extraction plus a full history-based parametric tree; LiveTransfer pushes the tree into SOLIDWORKS, NX, Creo, Inventor or Solid Edge; Accuracy Analyser shows the deviation to the scan in real time | Yes, full feature tree |
| [ZEISS REVERSE ENGINEERING](https://www.gom.com/en/products/zeiss-quality-suite/zeiss-reverse-engineering) | proprietary | paid | Mesh/point cloud (STL, PLY, ASCII) → CAD, exported as STEP/IGES/SAT; guided workflow with best-fit, extrusion and lofting; one-click handoff from ZEISS INSPECT | Guided, semi-automatic |
| [PolyWorks\|Modeler](https://www.innovmetric.com/products/polyworks-modeler) (InnovMetric) | proprietary | enterprise | NURBS surfacing and parametric sketching directly on polygonal models; CAD add-ins for CATIA, NX, Creo, SolidWorks and Inventor; handles billion-point clouds | Partial — sketches and surfaces are transferred to the CAD system |
| [QUICKSURFACE](https://www.quicksurface.com/) (KVS) | proprietary | Personal/Lite/Pro tiers | Hybrid parametric + freeform modelling on the Parasolid kernel; constrained primitive extraction (planes, cylinders, cones, spheres); automatic surfacing; QSConnect add-in for SOLIDWORKS; STEP/IGES/Parasolid export | Guided |
| [Artec Studio](https://www.artec3d.com/3d-software/artec-studio) | proprietary; Lite free | subscription or lifetime, Pro ≈ USD 1,700/yr | Scanner suite with mesh cleanup, measurement and mesh-to-CAD deviation; basic scan-to-CAD (auto-surfacing, booleans; since v20 also chamfer, fillet and draft-angle recognition); one-click export to SOLIDWORKS | Partial |
| [EXModel](https://www.shining3d.com/reverse-engineering/exmodel) (Shining 3D) | proprietary; Personal tier free | paid EXModel/EXModel Pro from ≈ USD 495 | Mesh → CAD solid in a few steps: primitive extraction, auto-surfacing, constrained 2D sketching (Pro), STEP/IGES export; one-click handoff from EXScan/FreeScan scanners | Guided, parametric in Pro |
| [peel 3.CAD](https://store.peel-3d.com/software/peel-cad/) | proprietary | scanner + software bundles, ≈ USD 12,000 and up | Turnkey scan-to-CAD with peel 3D scanners: feature extraction, advanced sketching, real-time deviation against the scan | Guided |

## Mesh to B-rep inside CAD apps (editable solid, no feature history)

| Tool | License | Cost | Special features | Feature history? |
|---|---|---|---|---|
| [SOLIDWORKS ScanTo3D](https://help.solidworks.com/2025/English/SolidWorks/ScanTo3D/c_Scanto3d_overview.htm) | proprietary add-in of SOLIDWORKS | included in SOLIDWORKS Professional, Premium and Ultimate | Opens STL/OBJ/PLY and point clouds; Mesh Prep, Curve and Surface wizards fit analytic or non-analytic surfaces; sketch directly against the mesh | No — a solid/surface you then edit by hand |
| [Autodesk Fusion 360](https://www.autodesk.com/products/fusion-360) | proprietary | subscription, free for personal use | Convert Mesh with three methods: faceted (one B-rep face per triangle), prismatic (analytic fit, best for mechanical parts) and T-spline (organic shapes); direct conversion is limited to roughly 10,000 triangles | No |
| [Rhino 3D](https://www.rhino3d.com/) | proprietary | one-time, ≈ USD 1,000 | MeshToNURB/ToNURBS converts each polygon face into a NURBS surface — face by face, no smart reconstruction; SubD workflow, paid plug-ins such as RhinoResurf add more | No |

## Around the mesh (no recognition, but part of the workflow)

| Tool | License | Cost | Special features | Feature history? |
|---|---|---|---|---|
| [OpenSCAD](https://openscad.org/) | GPL-2.0 | free | Script-only CSG modeller; `import()` reads an STL as a reference inside the script (mesh2cad can export `.scad`) | No — the model is the script |
| [CadQuery](https://github.com/CadQuery/cadquery) | Apache-2.0 | free (`pip`) | Python parametric CAD on the OpenCascade kernel (OCP); mesh2cad can export CadQuery scripts with matching parameters | No — you script the model |
| [build123d](https://build123d.readthedocs.io/) | Apache-2.0 | free (`pip`) | Modern Python B-rep framework over OpenCascade; builder mode keeps a design history with named dimensions | No — you script the model |
| [Blender](https://www.blender.org/) | GPL-3.0 | free | Mesh editing, repair and sculpting; no B-rep or feature history | No |
| [MeshLab](https://www.meshlab.net/) | GPL-3.0 | free | Mesh cleanup, repair, decimation and format conversion (STL, OBJ, PLY, …); PyMeshLab for batch processing | No |

## Where revmesh fits

The open-source recognisers above all run on a desktop with Python and OpenCascade. revmesh
is the only one that runs inside SolidCraft's WebAssembly plugin sandbox and writes
SolidCraft's own feature history directly, keeping the mesh — with the reason in the import
report — wherever no features are found. If you already work in Fusion 360, SOLIDWORKS or
Rhino, the mesh-to-B-rep conversions there are the pragmatic route to an editable solid,
without a feature history. For 3D scan data with organic shapes, the commercial suites
(Geomagic Design X, ZEISS REVERSE ENGINEERING, PolyWorks\|Modeler, …) remain the most
complete, at enterprise prices.
