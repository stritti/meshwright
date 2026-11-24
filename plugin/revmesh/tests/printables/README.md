# Real STL files from Printables

Parts other people designed and shared on [Printables](https://www.printables.com), used
unchanged (only renamed) as test files: the plugin tests `../printables-*.json` import each in
SolidCraft (`cargo xtask plugin-test`), `crates/revmesh-core/tests/printables.rs` checks the
recognised parameters natively, and the docs'
[examples](https://stritti.github.io/revmesh/guide/examples#real-files-from-printables) show them. They are
not covered by revmesh's MIT licence: each keeps its author's licence below. Only files under
CC0 or CC BY are taken, so they may be redistributed here.

They were chosen because they can be checked: where the author published the design's source
(a DXF or an OpenSCAD file), the test expects that source's numbers.

| File | Model | Author | Licence | Checked against |
|---|---|---|---|---|
| `nema17-extrusion-plate.stl` (`nema17_mount_plate.stl`) | [NEMA 17 aluminium extrusion mounting plate](https://www.printables.com/model/509515-nema-17-aluminium-extrusion-mounting-plate) | [diegotrap](https://www.printables.com/@diegotrap) | [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/) | the author's DXF |
| `nema17-spacer-plate.stl` (`motorplate_20180506-67-stga7e.stl`) | [2mm NEMA17 Spacer Plate](https://www.printables.com/model/149549-2mm-nema17-spacer-plate) | [derfley](https://www.printables.com/@derfley_230933) | [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/) | the NEMA 17 sizes (31 mm hole square) |
| `tiny-calibration-block.stl` (`simple-tiny-calibration-block.stl`) | [Simple Tiny Calibration Block](https://www.printables.com/model/182018-simple-tiny-calibration-block) | [Andy](https://www.printables.com/@Andy_259057) | [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/) | the author's OpenSCAD file |
| `calibration-cube.stl` (`Calibration_Cube.stl`) | [Calibration Block with Countersunk Hole and Slot](https://www.printables.com/model/254589-calibration-block-with-countersunk-hole-and-slot) | [TechnoSwiss](https://www.printables.com/@TechnoSwiss) | [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/) | the mesh's own vertices |
| `angle-bracket.stl` (`angle bracket v1.stl`) | [90 degree angle bracket](https://www.printables.com/model/276330-90-degree-angle-bracket) | [Ken S](https://www.printables.com/@KenS) | [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/) | the author's OpenSCAD file |
| `mending-plate.stl` (`Mending-Plate.stl`) | [Mending Plate](https://www.printables.com/model/1840620-mending-plate) | [Gwilherm Kerherve](https://www.printables.com/@GwilhermKerhe_474511) | [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/) | not recognised yet |
| `flat-mending-plate-4x20x60.stl` (`4x20x60 3.stl`) | [Heavy Duty Flat Mending Plates, 20x4mm (M4)](https://www.printables.com/model/1549159-heavy-duty-flat-mending-plates-20x4mm-m4) | [Mateusz Tokarz](https://www.printables.com/@MateuszTokarz) | [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/) | not recognised yet |

The files are stored as downloaded; never edit them. To add one, take a model under CC0 or
CC BY (not NC, ND or SA), add a row here with the original file name, a test in
`crates/revmesh-core/tests/printables.rs`, and a case `../printables-<name>.json`: its
`features`, `summary` and `volume_mm3` are what
`cargo run -q -p revmesh-core --example params -- <file>` prints on its last lines.
