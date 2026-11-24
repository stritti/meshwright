# Comparable tools

revmesh turns an STL triangle mesh into a parametric, editable design inside SolidCraft. This
page lists comparable solutions — tools that turn a triangle mesh (STL, OBJ, PLY) or a point
cloud into an editable CAD model — with a link, the license and what each product is special
at. Below the overview tables, a [feature matrix](#feature-matrix) compares the capabilities
in detail.

Licenses are as stated in each project's own license file; links and prices were checked in
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
| [CAD-Recode](https://cad-recode.github.io/) ([paper](https://arxiv.org/abs/2412.14042), [code](https://github.com/filaPro/cad-recode)) | CC-BY-NC-4.0 (noncommercial only) | free | Point cloud → CadQuery Python code via a lightweight network plus a pre-trained LLM; state of the art on the DeepCAD, Fusion 360 and CC3D benchmarks | Yes, as code — prototype, needs a GPU |
| [Point2CAD](https://www.obukhov.ai/point2cad.html) ([code](https://github.com/prs-eth/point2cad)) | Apache-2.0 | free | Point cloud → complete CAD model including freeform surfaces (implicit neural representation), edges and corners; state of the art on the ABC benchmark | No — reconstructed surfaces, edges and corners, not a construction history; prototype, needs a GPU, point cloud input |

## Commercial reverse-engineering suites

| Tool | License | Cost | Special features | Feature history? |
|---|---|---|---|---|
| [Geomagic Design X](https://hexagon.com/products/geomagic-design-x) (Hexagon) | proprietary | subscription or perpetual; Go from ≈ USD 1,900, Pro ≈ USD 22,000+ | Industry standard for scan-to-CAD: automatic feature extraction plus a full history-based parametric tree; LiveTransfer pushes the tree into SOLIDWORKS, NX, Creo, Inventor or Solid Edge; Accuracy Analyser shows the deviation to the scan in real time | Yes, full feature tree |
| [ZEISS REVERSE ENGINEERING](https://www.gom.com/en/products/zeiss-quality-suite/zeiss-reverse-engineering) | proprietary | paid | Mesh/point cloud (STL, PLY, ASCII) → CAD, exported as STEP/IGES/SAT; guided workflow with best-fit, extrusion and lofting; one-click handoff from ZEISS INSPECT | Guided, semi-automatic |
| [PolyWorks\|Modeler](https://www.innovmetric.com/products/polyworks-modeler) (InnovMetric) | proprietary | enterprise | NURBS surfacing and parametric sketching directly on polygonal models; CAD add-ins for CATIA, NX, Creo, SolidWorks and Inventor; handles billion-point clouds | Partial — sketches and surfaces are transferred to the CAD system |
| [QUICKSURFACE](https://www.quicksurface.com/) (KVS) | proprietary | Personal/Lite/Pro tiers | Hybrid parametric + freeform modelling on the Parasolid kernel; constrained primitive extraction (planes, cylinders, cones, spheres); automatic surfacing; QSConnect add-in for SOLIDWORKS; STEP/IGES/Parasolid export | Guided |
| [Artec Studio](https://www.artec3d.com/3d-software/artec-studio) | proprietary | subscription or lifetime; Lite from ≈ USD 480/yr, Pro ≈ USD 1,700/yr; 30-day Pro trial | Scanner suite with mesh cleanup, measurement and mesh-to-CAD deviation; basic scan-to-CAD (auto-surfacing, booleans; since v20 also chamfer, fillet and draft-angle recognition); one-click export of mesh and fitted primitives to SOLIDWORKS | Partial |
| [EXModel](https://www.shining3d.com/reverse-engineering/exmodel) (Shining 3D) | proprietary | paid licences from ≈ USD 225 (Personal, 1-year) up to EXModel Pro | Mesh → CAD solid in a few steps: primitive extraction, auto-surfacing, constrained 2D sketching (Pro), STEP/IGES export; one-click handoff from EXScan/FreeScan scanners | Guided, parametric in Pro |
| [peel 3.CAD](https://store.peel-3d.com/software/peel-cad/) | proprietary | scanner + software bundles, ≈ USD 12,000 and up | Turnkey scan-to-CAD with peel 3D scanners: feature extraction, advanced sketching, real-time deviation against the scan | Guided |
| [MeshWright](https://www.meshwright.io/) (meshwright.io) | proprietary | Seat USD 99, Pro USD 199 per user/month, Site by agreement; AI usage billed separately by the AI provider | Windows app with Fusion add-in and CLI: your own AI assistant (Claude Code, Codex) builds a native, parametric Fusion feature tree from a scan (STL, OBJ, PLY, 3MF); detected holes, bosses and flats are measured from the mesh; verification compares the rebuilt model with the scan and saves F3D plus STEP; jobs stay local, licence checks go online | Yes — native Fusion feature tree, AI-guided |

> **Name note:** MeshWright (meshwright.io) is another vendor's product, unrelated to revmesh.
> This repository was renamed from *meshwright* to *revmesh* precisely because the name was
> already taken (see [PR #34](https://github.com/stritti/revmesh/pull/34)). meshwright.dev and
> several GitHub projects called *Meshwright* (mesh repair for 3D printing) are further
> unrelated products.

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

## Feature matrix

Legend: ✓ supported · ~ partial, guided or paid tier · – not supported. The ML prototypes
(CAD-Recode, Point2CAD) are omitted here: they take point clouds, not mesh files, and need a
GPU.

### Recognised geometry (automatic tools)

| Feature | revmesh | mesh2cad | MeshWright (meshwright.io) | FreeCAD RE workbench | Detessellate | MeshRemodel | CAD Sketcher |
|---|---|---|---|---|---|---|---|
| Input meshes | STL (≤ 8 MiB in auto, 32 MiB mesh-only) | STL, OBJ, PLY | STL, OBJ, PLY, 3MF | mesh + point cloud | STL, OBJ, 3MF | mesh (STL, OBJ, …) | mesh as sketch reference |
| Plate/outline extruded along Z | ✓ | ✓ | ~ AI-built features | – | ~ cross-sections | ~ manual | ~ projected sketch |
| Extrusion along X or Y | ✓ | ✓ (tries X, Y, Z) | ~ | – | – | – | ~ |
| Through holes | ✓ | ✓ | ✓ fitted from mesh | – | – | – | – |
| Blind holes, counterbore, countersink | ✓ all three | ✓ counterbore + countersink | ~ | – | – | – | – |
| Slots | ✓ | ~ | – | – | – | – | – |
| Bosses, pads, pockets, cut-outs | ✓ | ✓ bosses, ~ rest | ✓ bosses detected | – | – | – | – |
| Hole patterns (row, grid, bolt circle) | ✓ as SolidCraft patterns | ✓ hole_row / hole_grid | – | – | – | – | ✓ array tools |
| Fillets with one radius | ✓ vertical corners, any outline | ✓ vertical corners + all edges (`edge_fillet_r`) | ~ fillet radii as parameters | – | – | – | ✓ constraint-driven |
| Chamfers, drafts | – (roadmap) | ~ | ~ | – | – | – | ✓ |
| Freeform/NURBS surfaces from the mesh | – mesh body fallback | – faceted/hybrid fallback | ~ reference fit, not the feature tree | ✓ B-spline fitting | ~ loft splines | – | – |

### Workflow and platform (automatic tools)

| Feature | revmesh | mesh2cad | MeshWright (meshwright.io) | FreeCAD RE workbench | Detessellate | MeshRemodel | CAD Sketcher |
|---|---|---|---|---|---|---|---|
| Parametric feature history out | ✓ SolidCraft design + named parameters | ✓ CadQuery / OpenSCAD / FreeCAD scripts + exact STEP | ✓ native Fusion feature tree | – | ~ sketches | ~ manual | ✓ Blender constraints |
| Fidelity report vs the mesh | ✓ import report, rebuild check | ✓ Hausdorff, RMS, volume; refuses misses | ✓ verification vs scan, F3D + STEP saved | ~ deviation | – | – | – |
| Mesh fallback when nothing is found | ✓ mesh body + reason | ✓ faceted/hybrid solid | – manual corrections | ✓ (surface fitting) | ✓ | ✓ | n/a |
| Automation | fully automatic, deterministic | fully automatic CLI + MCP server | AI-assistant-guided (Claude Code, Codex) | semi-automatic | guided | manual aid | manual |
| Environment | WebAssembly plugin in SolidCraft | desktop CLI / Python | Windows + Fusion add-in / CLI | FreeCAD desktop | FreeCAD add-on | FreeCAD add-on | Blender add-on |
| Works offline | ✓ | ✓ | – licence check online; the AI app may transmit data | ✓ | ✓ | ✓ | ✓ |
| License, cost | MIT, free | MIT, free | proprietary, USD 99–199/user/mo + AI usage | LGPL, free | LGPL, free | LGPL, free | GPL-3.0, free |

### Commercial suites

| Feature | Geomagic Design X | ZEISS REVERSE ENGINEERING | PolyWorks Modeler | QUICKSURFACE | Artec Studio | EXModel | peel 3.CAD | MeshWright |
|---|---|---|---|---|---|---|---|---|
| Primitive fitting (plane, cylinder, cone, sphere) | ✓ automatic extraction | ✓ | ✓ constrained best-fit | ✓ | ✓ | ✓ | ✓ | ✓ measured from mesh |
| Constrained 2D sketching | ✓ Auto-Sketch | ✓ | ✓ parametric sketches | ✓ | – | ✓ (Pro) | ✓ | ~ via the AI assistant |
| NURBS / freeform surfacing | ✓ Exact Surfacing | ✓ | ✓ | ✓ automatic surfacing | ✓ auto-surfacing | ✓ | – | ~ reference fit |
| Feature tree transferred into CAD | ✓ LiveTransfer (SOLIDWORKS, NX, Creo, Inventor, Solid Edge) | – exports STEP/IGES/SAT | ~ parametric sketches via add-ins (CATIA, NX, Creo, SOLIDWORKS, Inventor); extrude/loft in the CAD app | ✓ QSConnect for SOLIDWORKS | ~ meshes + fitted primitives to SOLIDWORKS — geometry, not a feature tree | ~ parametric STEP | ~ | ✓ native Fusion tree |
| Real-time deviation vs the scan | ✓ Accuracy Analyser | ✓ best-fit | ✓ | ✓ deviation analyser | ✓ compare map | ✓ | ✓ | ✓ verification record |
| AI assistant / MCP integration | – | – | – | – | – | – | – | ✓ Claude Code, Codex |
| Scanner-vendor integration | ✓ plugins (Faro, Hexagon, Aicon) | ✓ one-click from ZEISS INSPECT | – | – | ✓ Artec scanners | ✓ one-click from EXScan | ✓ peel scanners | – |
| Batch / automation | ✓ Pro batch processing | ~ | ✓ macros | – | ✓ workflows | – | – | ✓ Site plan |
| Works offline | ✓ perpetual/offline activation | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | – licence check online |
| Price tier | ≈ USD 1,900–22,000+ | enterprise | enterprise | Lite to Pro | ≈ USD 1,700/yr Pro | from ≈ USD 495 | ≈ USD 12,000+ bundle | USD 99–199/user/mo |

### Mesh to B-rep in CAD apps

| Feature | SOLIDWORKS ScanTo3D | Autodesk Fusion 360 | Rhino 3D |
|---|---|---|---|
| Conversion methods | Surface Wizard (analytic + non-analytic fit), Curve Wizard, Mesh Prep | Convert Mesh: faceted, prismatic, T-spline | MeshToNURB / ToNURBS (one NURBS face per polygon), SubD |
| Result | solid or surface body, then manual features | B-rep solid/surface; timeline entry in parametric mode | polysurface; unusable at high face counts |
| Face-count limit for direct conversion | mesh repair first, then wizard | ≈ 10,000 triangles | impractical beyond low-poly meshes |
| Organic shapes | ~ | ✓ T-spline path via the Form workspace | ~ plug-ins such as RhinoResurf |
| Feature history from the mesh | – | – | – |

## Research and literature

Papers on reconstructing CAD from meshes/point clouds and on evaluating that — grouped by how
close they are to revmesh's approach. The neural methods need a GPU and Python, so they cannot
run in SolidCraft's WebAssembly sandbox; the classical ones (RANSAC shape detection, STL feature
recognition, closed-form extrusion fitting) are the directly portable building blocks, and the
datasets at the end are the standard test corpora (tessellated to STL, they exercise a
recogniser the way revmesh is exercised).

### Surveys and reviews

| Paper | Venue | Links | Relevance |
|---|---|---|---|
| A Survey on Deep Learning in 3D CAD Reconstruction (Lin et al.) | Appl. Sci. 15(12):6681, 2025 | [DOI](https://doi.org/10.3390/app15126681) | Entry point for DL-based CAD reconstruction from scans, meshes, point clouds |
| Integrating Reverse Engineering for Digital Model Reconstruction and Remanufacturing of Mechanical Components: A Systematic Review | Metrology 5(4):66, 2025 | [DOI](https://doi.org/10.3390/metrology5040066) | Systematic review of reverse engineering of mechanical parts from 3D scans, 2005–2025 — the industrial side of this page |
| Machine learning in feature recognition for manufacturing: taxonomy, analytical review, comparisons, trends, challenges, and outlook | Int. J. Manuf. Res., 2026 | [DOI](https://doi.org/10.1080/00207543.2026.2675460) | Taxonomy of (ML-based) feature recognition from CAD models — holes, slots, pockets |

### Mesh / point cloud to parametric CAD

| Paper | Venue | Links | Relevance |
|---|---|---|---|
| Point2Cyl: Reverse Engineering 3D Objects from Point Clouds to Extrusion Cylinders (Uy et al.) | CVPR 2022 | [arXiv](https://arxiv.org/abs/2112.09329), [project](https://point2cyl.github.io/), [code](https://github.com/mikacuy/point2cyl) | Decomposes a point cloud into extrusion cylinders (2D sketch + axis + extent) with closed-form differentiable fitting — the closest learning-based analogue of revmesh's prism recognition |
| ComplexGen: CAD Reconstruction by B-Rep Chain Complex Generation (Guo et al.) | SIGGRAPH 2022, ACM TOG 41(4) | [arXiv](https://arxiv.org/abs/2205.14573), [code](https://github.com/guohaoxiang/ComplexGen) | Reconstructs a full B-rep with explicit topology; its global consistency constraints (closed face loops, cylinder perpendicularity) are ideas for validating recognised parts |
| Point2CAD (Liu et al.) | CVPR 2024 | [arXiv](https://arxiv.org/abs/2312.04962), [code](https://github.com/prs-eth/point2cad) | Listed in the tables above: point cloud → CAD with freeform surfaces, edges, corners |
| CAD-Recode (Rukhovich et al.) | ICCV 2025 | [arXiv](https://arxiv.org/abs/2412.14042), [code](https://github.com/filaPro/cad-recode) | Listed in the tables above: point cloud → CadQuery Python code |

### Primitive and edge detection (building blocks for recognisers)

| Paper | Venue | Links | Relevance |
|---|---|---|---|
| Efficient RANSAC for Point-Cloud Shape Detection (Schnabel, Wahl, Klein) | CGF 26(2):214–226, 2007 | [DOI](https://doi.org/10.1111/j.1467-8659.2007.01016.x) | Detects planes, spheres, cylinders, cones, tori in unorganised points — the classical basis of revmesh's fixed-seed deterministic RANSAC (see [Architecture](/architecture)) |
| SPFN: Supervised Fitting of Geometric Primitives to 3D Point Clouds (Li et al.) | CVPR 2019 | [arXiv](https://arxiv.org/abs/1811.08988), [code](https://github.com/lingxiaoli94/SPFN) | Learned primitive fitting, benchmarked against Efficient RANSAC on mechanical parts |
| CPFN: Cascaded Primitive Fitting Networks for High-Resolution Point Clouds | 2021 | [arXiv](https://arxiv.org/abs/2109.00113) | SPFN cascaded for 100k+ point scans — small features (holes) at high resolution |
| ParSeNet: A Parametric Surface Fitting Network for 3D Point Clouds (Sharma et al.) | CVPR 2021 | [arXiv](https://arxiv.org/abs/2003.12181) | Fits parametric patches including B-splines — freeform side of the recognition |
| HPNet: Deep Primitive Segmentation Using Hybrid Representations (Yan et al.) | ICCV 2021 | [arXiv](https://arxiv.org/abs/2105.10620) | Segments primitives using sharp edges explicitly — relevant for revmesh's wall/face classification |
| PIE-Net: Parametric Inference of Point Cloud Edges (Wang et al.) | NeurIPS 2020 | [arXiv](https://arxiv.org/abs/2007.04883) | Infers feature edges as parametric curves (lines, circles, B-splines) — the analogue of reading hole rims and outlines from a mesh |

### Feature recognition from STL meshes (classical, closest to revmesh's input)

| Paper | Venue | Links | Relevance |
|---|---|---|---|
| Automatic recognition of features from freeform surface CAD models (Sunil, Pande) | Computer-Aided Design 40(4):502–517, 2008 | [record](https://www.semanticscholar.org/paper/Automatic-recognition-of-features-from-freeform-CAD-Sunil-Pande/81a304bfe4043aaa87b1e936942fefe2fe6dcddc) | Segments STL meshes by dihedral angle and curvature, then recognises holes, slots, beads, darts — directly on revmesh's input format |
| A learning-based approach to feature recognition of Engineering shapes (Muraleedharan, Muthuganapathy) | 2021 | [arXiv](https://arxiv.org/abs/2112.07962) | Classifies holes/slots in CAD meshes (random forest on a discrete Gauss map); neural, but the mesh descriptors are portable |

### Datasets and benchmarks

| Dataset | Paper | Links | Relevance |
|---|---|---|---|
| ABC dataset (~1M CAD models, parametric ground truth) | Koch et al., CVPR 2019 | [arXiv](https://arxiv.org/abs/1812.06216), [project](https://deep-geometry.github.io/abc-dataset/) | Benchmark of ParSeNet, HPNet, Point2CAD; tessellated to STL it is a large test corpus for recognisers |
| DeepCAD (178k Onshape models + construction sequences) | Wu et al., ICCV 2021 | [arXiv](https://arxiv.org/abs/2105.09492), [code](https://github.com/rundiwu/DeepCAD) | CAD sketch-extrude sequences as ground truth for reconstruction; used by CAD-Recode and Point2Cyl |
| Fusion 360 Gallery (8,625 human sketch-extrude sequences) | Willis et al., SIGGRAPH 2021, ACM TOG 40(4):54 | [arXiv](https://arxiv.org/abs/2010.02392), [DOI](https://doi.org/10.1145/3450626.3459818), [data](https://github.com/AutodeskAILab/Fusion360GalleryDataset) | Defines "CAD reconstruction" — recovering a CAD program from target geometry — the same task revmesh solves for STL input |

## Where revmesh fits

The open-source recognisers above all run on a desktop with Python and OpenCascade. revmesh
is the only one that runs inside SolidCraft's WebAssembly plugin sandbox and writes
SolidCraft's own feature history directly, keeping the mesh — with the reason in the import
report — wherever no features are found. MeshWright (meshwright.io) is the AI-guided
alternative: an assistant builds a parametric feature tree in Fusion, guided by measurements
taken from the mesh, but it needs Windows, Fusion, an AI account and an online licence check.
If you already work in Fusion 360, SOLIDWORKS or Rhino, the mesh-to-B-rep conversions there
are the pragmatic route to an editable solid, without a feature history. For 3D scan data
with organic shapes, the commercial suites (Geomagic Design X, ZEISS REVERSE ENGINEERING,
PolyWorks\|Modeler, …) remain the most complete, at enterprise prices.

## Concepts from the list, mapped to mesh2cad and revmesh

Which concepts are worth adopting is decided by the sandbox: it has no GPU, no Python and no
network (see [Architecture](/architecture)), so the neural fitters and reconstructors (SPFN,
CPFN, HPNet, ParSeNet, Point2CAD, CAD-Recode, and MeshWright's AI assistant) cannot run there.
What is portable is their *output representation* and the classical, deterministic building
blocks: sketch-extrude sequences, extrusion cylinders, parametric edges, fixed-seed RANSAC
primitives, edge-based mesh segmentation and topology checks.

| Concept | Comes from | mesh2cad | revmesh today | Improvement potential |
|---|---|---|---|---|
| Cross-sections become constrained sketches | Geomagic Auto-Sketch, PolyWorks Modeler, QUICKSURFACE, EXModel (Pro), Detessellate, ZEISS | no — the outline is a point list in the exported script | the outer outline only, as a sketch with horizontal/vertical constraints | slice the mesh at every face level and sketch each level's loops: arbitrary pockets and steps as sketch + extrude instead of boxes, combined extrusions (open [roadmap](/guide/roadmap) items) |
| Sketch-extrude with constraints as the CAD representation | DeepCAD, Fusion 360 Gallery, ABC dataset | geometry as code in a CadQuery/OpenSCAD script, not dimensioned | SolidCraft sketches: points, line/arc curves, constraints (horizontal, vertical, coincident, tangent, equal, midpoint, length, diameter, distance_x/y, angle), with dimensions driven by model parameters | dimension every outline side (length or angle per side, distance_x/y per corner, each with its own parameter), so any outline re-dimensions like the rectangle does today |
| Extrusion cylinders: 2D profile + axis + extent | Point2Cyl | one prism, tried along X, Y and Z | the same single prism along X, Y or Z | decompose a part into several extrusion cylinders, each its own sketch + extrude; closed-form fitting keeps it deterministic |
| Parametric edges read from the mesh | PIE-Net (neural); classically, circle fits on rim loops | circles fitted from facet loops | circles fitted to hole and boss rims; outlines stay polylines | recognise curved sides, round outlines and D-shaped cut-outs as sketch arcs with a radius parameter — today they stay a mesh ([not recognised yet](/guide/recognition#not-recognised-yet)) |
| Segmentation by sharp edges / dihedral angle | Sunil & Pande 2008, Muraleedharan & Muthuganapathy 2021, HPNet's explicit edges | face classification | `faces.rs` / `walls.rs` classify faces and walls; corners are checked against the walls | more feature classes from the same segmentation: chamfers, drafts, beads, blind and slanted slots |
| Primitive detection including tori | Efficient RANSAC (Schnabel et al. 2007) | planes and cylinders | fixed-seed RANSAC; planes, cylinders; a boss foot's round is measured band by band (PR #51) | detect tori analytically, so rounds at hole rims and boss feet become measured surfaces instead of per-band fits |
| Topology as a validity gate | ComplexGen (closed face loops, cylinder perpendicularity); mesh2cad refuses inexact parts | refuses what it cannot express | exact-or-nothing, volume within 1% | add ComplexGen-style checks — recognised faces close into loops, walls meet their face at 90° — to reject twisted or banded fakes before the volume gate |
| Measure the features from the mesh, then build the history | MeshWright (meshwright.io) via an AI assistant; mesh2cad measures, then writes a script | measures, then writes the script | the same, fully deterministic | nothing to adopt from the AI variant — it needs network and an online licence check; measurement-first is already revmesh's approach |
| Deviation against the mesh as a report | Geomagic Accuracy Analyser, PolyWorks, MeshWright verification; mesh2cad's Hausdorff/RMS | fidelity report next to the script | volume within 1% at import; Hausdorff in `cargo xtask check` | report the surface deviation per face in the import report, computed in the sandbox |

### Towards reworkable sketches

SolidCraft's sketches already are what DeepCAD and the Fusion 360 Gallery call a parametric
sketch: points, line and arc curves, and constraints, where `length`, `diameter`, `distance_x`,
`distance_y` and `angle` constraints are driven by model parameters (see
[The SolidCraft design format](/design-format)). revmesh already writes such sketches where it
has the dimensions: the plate rectangle (two length parameters) and the slot (tangent, equal,
midpoint, length, diameter, angle — all parameter-driven). A general outline today is a polygon
of free points with only horizontal/vertical constraints on its axis-aligned sides: its shape
lives in the points, and the parameters cover only thickness and position.

The potential, in the order the roadmap can take it:

1. **Dimension every outline side** — a length or angle constraint per side (or `distance_x`/`distance_y`
   per corner), each with its own model parameter. The outline then re-dimensions in SolidCraft
   the way `base_x` re-dimensions the rectangle today. mesh2cad cannot express this: its
   `outline` is a bare point list.
2. **Sketch arcs instead of only fillet features** — read curved sides, round outlines and
   D-shaped cut-outs as arcs with a radius parameter (PIE-Net's parametric edges; revmesh already
   fits circles to rims), so they become reworkable sketch curves instead of staying a mesh.
3. **One sketch per face level** — slice the mesh where faces change height and sketch each
   loop (the Auto-Sketch concept), giving arbitrary pockets, steps and combined extrusions as
   sketch + extrude pairs (Point2Cyl's extrusion cylinders).
4. **Keep the deterministic gates** — exact-or-nothing, volume within 1%, plus the topology
   checks from ComplexGen. The neural reconstructors stay out of the sandbox; their
   representation is the target, their compute is not.
