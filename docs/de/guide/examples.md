# Beispiele

Konkrete STL-Dateien, um revmesh auszuprobieren, jede mit dem, was SolidCraft daraus macht. Lade eine
Datei herunter, öffne sie in SolidCraft mit **Plugins › revmesh › stl** (siehe [Anwendung](/de/guide/use))
und ändere einen Parameter im Parameter-Panel: Das Teil wird mit dem neuen Wert neu aufgebaut.

Die Bilder zeigen die STL so, wie sie ist, ein Netz aus flachen Dreiecken (die dünnen Linien sind die
Kanten der Facetten). Nach dem Import sind die runden Flächen echte Kreise und jede Zahl in den
Tabellen unten ein Parameter, den du bearbeiten kannst.

## Alltägliche Teile

Diese liegen in [`examples/`](https://github.com/stritti/revmesh/tree/main/examples) im Repository,
tesselliert so, wie ein CAD-Programm STL exportiert.

### Halterung für einen Einplatinencomputer

![sbc-mount.stl: eine 95 x 66 x 3 mm Platte mit verrundeten Ecken und acht Bohrungen](/examples/sbc-mount.png)

[`sbc-mount.stl`](https://github.com/stritti/revmesh/raw/main/examples/sbc-mount.stl): eine Platte
für eine kreditkartengroße Platine, mit den vier M2.5-Bohrungen der Platine auf ihrem 58 x 49 mm
Gitter und vier Ø4.5-Bohrungen, um die Platte anzuschrauben.

| In SolidCraft | Parameter |
|---|---|
| `Sketch1`, `Extrude1`, `Move1`: die Platte | `base_x` 95, `base_y` 66, `base_t` 3 |
| `HoleGrid1` + `HoleGrid1Pattern`: die vier Ø4.5-Bohrungen | `hole_grid_1_d` 4.5, `hole_grid_1_pitch_x` 83, `hole_grid_1_pitch_y` 54 |
| `HoleGrid2` + `HoleGrid2Pattern`: die Bohrungen der Platine | `hole_grid_2_d` 2.7, `hole_grid_2_pitch_x` 58, `hole_grid_2_pitch_y` 49, Ecke `hole_grid_2_x` 18.5, `hole_grid_2_y` 8.5 |
| `Fillet1`: die vier Ecken | `fillet_r` 4 |

Probiere: setze `hole_grid_2_pitch_x` und `_pitch_y` auf den Lochabstand einer anderen Platine.

### NEMA-17-Motorplatte

![nema17-motor-plate.stl: eine 60 x 60 x 5 mm Platte mit einer großen Bohrung und zwei quadratischen Lochmustern](/examples/nema17-motor-plate.png)

[`nema17-motor-plate.stl`](https://github.com/stritti/revmesh/raw/main/examples/nema17-motor-plate.stl):
die Ø22.5-Passbohrung und die vier M3-Bohrungen auf dem 31-mm-Quadrat eines NEMA-17-Schrittmotors, und
vier Ø5.5-Bohrungen auf einem 48-mm-Quadrat.

| In SolidCraft | Parameter |
|---|---|
| Die Platte | `base_x` 60, `base_y` 60, `base_t` 5 |
| `HoleGrid1` + Muster: die Ø5.5-Bohrungen | `hole_grid_1_d` 5.5, `hole_grid_1_pitch_x` / `_pitch_y` 48 |
| `HoleGrid2` + Muster: die Bohrungen des Motors | `hole_grid_2_d` 3.4, `hole_grid_2_pitch_x` / `_pitch_y` 31 |
| `Hole1`: die Passbohrung | `hole_1_d` 22.5, `hole_1_x` 30, `hole_1_y` 30 |
| `Fillet1` | `fillet_r` 5 |

Probiere: ein NEMA-23-Motor hat seine Bohrungen auf 47.14 mm und eine Ø38.1-Passung.

### Quadratischer Flansch

![square-flange.stl: eine 80 x 80 x 8 mm Platte mit einer Bohrung und sechs Bohrungen auf einem Lochkreis](/examples/square-flange.png)

[`square-flange.stl`](https://github.com/stritti/revmesh/raw/main/examples/square-flange.stl):
eine Ø30-Bohrung und sechs M6-Löcher auf einem Ø55-Lochkreis.

| In SolidCraft | Parameter |
|---|---|
| Die Platte | `base_x` 80, `base_y` 80, `base_t` 8 |
| `HoleCircle1` + `HoleCircle1Pattern`: der Lochkreis | `hole_circle_1_n` 6, `hole_circle_1_d` 6.6, `hole_circle_1_r` 27.5, Mittelpunkt `hole_circle_1_cx` / `_cy` 40, `hole_circle_1_angle` 0 |
| `Hole1`: die Bohrung | `hole_1_d` 30 |
| `Fillet1` | `fillet_r` 8 |

Probiere: `hole_circle_1_n` 8 setzt acht Bohrungen auf den Kreis.

### Eckplatte

![corner-plate.stl: eine L-förmige Platte mit verrundeten Ecken und vier Bohrungen](/examples/corner-plate.png)

[`corner-plate.stl`](https://github.com/stritti/revmesh/raw/main/examples/corner-plate.stl):
ein L, um einen Rahmen zu versteifen, jede Ecke verrundet, innen und außen gleichermaßen. Ihre Kontur
ist eine Skizze, deren sechs Seiten du bemaßen kannst; die Platte ist diese Skizze extrudiert.

| In SolidCraft | Parameter |
|---|---|
| `Sketch1` (das L), `Extrude1`, `Move1` | `extrude_len` 4, `extrude_0` 0; die Seiten in der Skizze |
| `Fillet1`: alle sechs vertikalen Ecken | `outline_fillet_r` 3 |
| `HoleRow1` + `HoleRow1Pattern`: drei Ø5-Bohrungen entlang des langen Schenkels | `hole_row_1_n` 3, `hole_row_1_pitch` 22, `hole_row_1_d` 5 |
| `Hole1`: die Ø6-Bohrung im kurzen Schenkel | `hole_1_d` 6, `hole_1_x` 10, `hole_1_y` 38 |

Bohrungen eines Durchmessers werden nur dann zu einem Muster, wenn alle sie bilden — daher hat die
Bohrung im kurzen Schenkel hier einen eigenen Durchmesser.

### Gefaste Abdeckung

![chamfered-cover.stl: eine 80 x 50 x 4 mm Abdeckung mit gefasten Ecken, einer Bohrungsreihe und einem Kabelloch](/examples/chamfered-cover.png)

[`chamfered-cover.stl`](https://github.com/stritti/revmesh/raw/main/examples/chamfered-cover.stl):
Ecken bei 45° abgeschnitten, eine Reihe aus vier Ø4-Bohrungen und ein Ø12-Kabelloch. Eine gefaste Ecke
ist eine weitere gerade Seite der Kontur, daher hat die Skizze der Abdeckung acht Seiten.

| In SolidCraft | Parameter |
|---|---|
| `Sketch1` (acht Seiten), `Extrude1`, `Move1` | `extrude_len` 4 |
| `HoleRow1` + Muster | `hole_row_1_n` 4, `hole_row_1_pitch` 20, `hole_row_1_d` 4 |
| `Hole1`: das Kabelloch | `hole_1_d` 12 |

### Distanzstab

![spacer-bar.stl: ein 70 mm langer Stab entlang X mit einer Bohrung entlang seiner Länge](/examples/spacer-bar.png)

[`spacer-bar.stl`](https://github.com/stritti/revmesh/raw/main/examples/spacer-bar.stl): ein
20 x 12 mm Stab, 70 mm lang entlang X, mit einer Ø6.5-Bohrung entlang seiner Länge, abseits des
Ursprungs platziert, wie er in einer Baugruppe saß. revmesh sieht, dass er entlang X und nicht Z
extrudiert ist, und lässt ihn, wo er ist.

| In SolidCraft | Parameter |
|---|---|
| `Sketch1` auf der YZ-Ebene, `Extrude1` entlang X, `Move1` | `extrude_len` 70, `extrude_0` 15 (wo er entlang X beginnt) |
| `Hole1` entlang X | `hole_1_d` 6.5, `hole_1_y` 15, `hole_1_z` 16 |

### Verrundeter Block

![rounded-block.stl: ein 60 x 40 x 25 mm Block mit allen zwölf Kanten verrundet](/examples/rounded-block.png)

[`rounded-block.stl`](https://github.com/stritti/revmesh/raw/main/examples/rounded-block.stl):
ein Block, dessen zwölf Kanten alle mit 4 mm verrundet sind, wie ein Handgehäuse, bevor es ausgeschält wird.

| In SolidCraft | Parameter |
|---|---|
| Der Block | `base_x` 60, `base_y` 40, `base_t` 25 |
| `Fillet1`: alle zwölf Kanten | `edge_fillet_r` 4 |

Probiere: ein **Shell** in SolidCraft macht daraus ein Gehäuse, was das Netz nicht könnte.

## Ein Feature nach dem anderen

Die eigenen Testdateien des Plugins in
[`plugin/revmesh/tests/`](https://github.com/stritti/revmesh/tree/main/plugin/revmesh/tests)
zeigen jede Feature-Art einzeln. Die meisten wurden in SolidCraft modelliert und exportiert.
[Was `auto` erkennt](/de/guide/recognition) hat die vollständige Liste.

### Senkungen und Kegelsenkungen

![stepped-holes.stl](/examples/stepped-holes.png)

[`stepped-holes.stl`](https://github.com/stritti/revmesh/raw/main/plugin/revmesh/tests/stepped-holes.stl):
`Hole1` und `Hole2` haben eine Senkung (`hole_1_cbore_d` 10, `hole_1_cbore_depth` 3), `Hole3` ist eine
Kegelsenkung (`hole_3_csk_d` 12, `hole_3_csk_angle` 90).

### Sacklochbohrungen

![blind-holes.stl](/examples/blind-holes.png)

[`blind-holes.stl`](https://github.com/stritti/revmesh/raw/main/plugin/revmesh/tests/blind-holes.stl):
drei Bohrungen mit flachem Boden und eine Durchgangsbohrung; eine Sacklochbohrung hat eine Tiefe
(`hole_1_depth` 4, `hole_3_depth` 2.5, `hole_4_depth` 6).

### Verrundete Bohrungsränder

![hole-rims.stl](/examples/hole-rims.png)

[`hole-rims.stl`](https://github.com/stritti/revmesh/raw/main/plugin/revmesh/tests/hole-rims.stl):
eine Bohrung oben verrundet und eine unten, jede ein `Hole<n>` mit einer Verrundung
`Hole<n>Rim` (`hole_1_rim_r` 1.5, `hole_2_bot_rim_r` 1).

### Langlöcher

![slots.stl](/examples/slots.png)

[`slots.stl`](https://github.com/stritti/revmesh/raw/main/plugin/revmesh/tests/slots.stl):
Langlöcher entlang X, entlang Y und bei 30°, jedes eine Skizze und ein Schnitt (`slot_<n>_len`, `_w`, `_x`, `_y`,
`_angle`).

### Zapfen mit Verrundung am Fuß

![boss-fillet.stl](/examples/boss-fillet.png)

[`boss-fillet.stl`](https://github.com/stritti/revmesh/raw/main/plugin/revmesh/tests/boss-fillet.stl):
`Boss1` (`boss_1_d` 10, `boss_1_h` 8) mit `Boss1Fillet` (`boss_1_fillet` 2), und ein schlichter
`Boss2`.

### Zapfen mit Formschräge und Fase

![boss-draft-chamfer.stl](/examples/boss-draft-chamfer.png)

[`boss-draft-chamfer.stl`](https://github.com/stritti/revmesh/raw/main/plugin/revmesh/tests/boss-draft-chamfer.stl):
ein Zapfen, der nach oben schmaler wird (`boss_1_draft_deg` 5), und einer mit gefaster Oberkante
(`boss_2_chamfer` 1).

### Aufsätze und Taschen

![pockets.stl](/examples/pockets.png)

[`pockets.stl`](https://github.com/stritti/revmesh/raw/main/plugin/revmesh/tests/pockets.stl):
ein Block auf der Oberseite (`pad_1_w` 16, `pad_1_l` 12, `pad_1_h` 4) und zwei rechteckige
Taschen (`pocket_<n>_w`, `_l`, `_depth`).

### Ausschnitte

![cutouts.stl](/examples/cutouts.png)

[`cutouts.stl`](https://github.com/stritti/revmesh/raw/main/plugin/revmesh/tests/cutouts.stl):
Öffnungen aus geraden Seiten durch die Platte, gezeichnet in `CutoutsSketch` und geschnitten von
`Cutouts`; ihre Ecken liegen in der Skizze.

### Konturen entlang mehrerer Achsen extrudiert

![combined.stl](/examples/combined.png)

[`combined.stl`](https://github.com/stritti/revmesh/raw/main/plugin/revmesh/tests/combined.stl):
ein Teil, das der Schnitt einer von oben gesehenen Kontur und einer von der Seite gesehenen ist; jede
ist eine Skizze und eine Extrusion (`extrude_z_len`, `extrude_x_len`).

![combined-3.stl](/examples/combined-3.png)

[`combined-3.stl`](https://github.com/stritti/revmesh/raw/main/plugin/revmesh/tests/combined-3.stl):
dasselbe mit Konturen entlang X, Y und Z.

### Fenster, Schnitte und Taschen in solchen Teilen

![combined-windows.stl](/examples/combined-windows.png)

[`combined-windows.stl`](https://github.com/stritti/revmesh/raw/main/plugin/revmesh/tests/combined-windows.stl):
Öffnungen quer durch das Teil entlang einer Achse, als Löcher in der Skizze dieser Achse gezeichnet.

![combined-cuts.stl](/examples/combined-cuts.png)

[`combined-cuts.stl`](https://github.com/stritti/revmesh/raw/main/plugin/revmesh/tests/combined-cuts.stl):
ein Sackloch-Schnitt und eine Tasche, jeder `Cut<n>Sketch` und `Cut<n>` (`cut_<n>_len`, `cut_<n>_0`).

## Echte Dateien von Printables

Teile, die andere entworfen und auf [Printables](https://www.printables.com) geteilt haben, exportiert
mit ihren Werkzeugen (OpenSCAD, FreeCAD, SketchUp, …) statt hier erzeugt. Sie sind Testdateien in
[`plugin/revmesh/tests/printables/`](https://github.com/stritti/revmesh/tree/main/plugin/revmesh/tests/printables),
je unter der Lizenz ihres Autors (CC0 oder CC BY 4.0, dort gelistet). Wo der Autor den Quellcode des
Teils veröffentlicht hat, prüft der Test revmeshs Parameter dagegen.

### NEMA-17-Platte für Aluminiumprofil

![NEMA 17 aluminium extrusion mounting plate von diegotrap](/examples/printables/nema17-extrusion-plate.png)

[NEMA 17 aluminium extrusion mounting plate](https://www.printables.com/model/509515-nema-17-aluminium-extrusion-mounting-plate)
von diegotrap, CC BY 4.0. Jede Zahl stimmt mit der DXF des Autors überein: die 42 x 80 Kontur mit 2 mm
Fasen (`Sketch1`, acht Seiten, `extrude_len` 4), die Bohrungen des Motors (`HoleGrid1`,
`hole_grid_1_d` 3.2 auf 31 mm), die Ø24-Passbohrung (`hole_1_d` 24) und zwei Langlöcher
(`slot_<n>_len` 24, `slot_<n>_w` 5.25, `slot_1_x` −10, `slot_2_x` 10).

### Einfacher winziger Kalibrierblock

![Simple Tiny Calibration Block von Andy](/examples/printables/tiny-calibration-block.png)

[Simple Tiny Calibration Block](https://www.printables.com/model/182018-simple-tiny-calibration-block)
von Andy, CC0. Wie in der OpenSCAD-Datei des Autors: `base_x` 30, `base_y` 20, `base_t` 10 und
`Pocket1` (`pocket_1_w` 20, `pocket_1_l` 10, `pocket_1_depth` 7).

### NEMA-17-Distanzplatte

![2mm NEMA17 Spacer Plate von derfley](/examples/printables/nema17-spacer-plate.png)

[2mm NEMA17 Spacer Plate](https://www.printables.com/model/149549-2mm-nema17-spacer-plate) von
derfley, CC BY 4.0: das 42.3 mm Quadrat mit gefasten Ecken, `hole_grid_1_d` 3.5 auf dem
31-mm-Quadrat und `hole_1_d` 27.

### Kalibrierblock mit Senkung und Langloch

![Calibration Block with Countersunk Hole and Slot von TechnoSwiss](/examples/printables/calibration-cube.png)

[Calibration Block with Countersunk Hole and Slot](https://www.printables.com/model/254589-calibration-block-with-countersunk-hole-and-slot)
von TechnoSwiss, CC BY 4.0. Trotz seines Titels hat die Bohrung 2 mm tief eine flache Stufe, daher macht
revmesh daraus eine `Hole1` mit Senkung (`hole_1_cbore_depth` 2); das Langloch ist `Pocket1`
(5.5 x 2.4, 5 tief).

### Winkel

![90 degree angle bracket von Ken S](/examples/printables/angle-bracket.png)

[90 degree angle bracket](https://www.printables.com/model/276330-90-degree-angle-bracket) von
Ken S, CC0. Wie in der OpenSCAD-Datei des Autors: die Seitenansicht (Basis, Stütze und Strebe) ist eine
Kontur, 30 mm entlang X extrudiert (`extrude_x_len` 30), und die zwei Ø10-Bohrungen sind `Hole1` in der
Stütze, 25 mm hoch (`hole_1_z` 25), und `Hole2` in der Basis (`hole_2_x` 15, `hole_2_y` 15).

### Noch nicht erkannt

![Mending Plate von Gwilherm Kerherve](/examples/printables/mending-plate.png)

[Mending Plate](https://www.printables.com/model/1840620-mending-plate) von Gwilherm Kerherve,
CC0: Kegelsenkungen, die sich durch die ganze Platte ziehen. Bleibt ein Netz.

![Heavy Duty Flat Mending Plate von Mateusz Tokarz](/examples/printables/flat-mending-plate-4x20x60.png)

[Heavy Duty Flat Mending Plates](https://www.printables.com/model/1549159-heavy-duty-flat-mending-plates-20x4mm-m4)
von Mateusz Tokarz, CC BY 4.0: ihre Oberkanten sind rundum verrundet. Bleibt ein Netz.

## Selber machen

Die alltäglichen Teile werden erzeugt, sind also exakt und reproduzierbar:

```sh
cargo run -q -p revmesh-core --example examples_stl -- examples   # schreibt examples/*.stl
pip install numpy pillow
python3 scripts/render-stl.py --out docs/public/examples examples/*.stl
python3 scripts/render-stl.py --out docs/public/examples/printables plugin/revmesh/tests/printables/*.stl
cargo test -p revmesh-core --test examples --test printables      # jeder importiert, wie hier beschrieben
```

Um zu sehen, was revmesh aus einer beliebigen STL macht, ohne SolidCraft zu öffnen, führe
`cargo run -q -p revmesh-core --example params -- part.stl` aus: Es gibt jeden Parameter und den
Importbericht aus.
