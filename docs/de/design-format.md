# Das SolidCraft-Design-Format, so wie revmesh es schreibt

SolidCrafts Dokumentenformat ist intern und hat kein veröffentlichtes Schema. Diese Seite hält fest, worauf
revmesh sich verlässt, gelernt aus Konstruktionen, die SolidCraft als JSON gespeichert hat (verifiziert gegen
0.50.0) und aus handgeschriebenen Dateien, die darin geöffnet wurden. Prüfe es mit jedem SolidCraft-Release
erneut.

## Version

`"format": "solidcraft/63"`. SolidCraft 0.50.0 aktualisiert ältere Versionen beim Öffnen einer Datei (es
speichert `solidcraft/68`), daher ist das Schreiben einer festen Version sicher, solange SolidCraft sie noch
liest.

## Obere Ebene

```json
{
  "format": "solidcraft/63", "name": "", "units": "mm",
  "params": [ ... ], "features": [ ... ],
  "next_id": 5, "reserved_body_names": ["Body1"], "next_component_color_index": 0
}
```

Ein leerer `name` nennt die Konstruktion nach der importierten Datei.

## Parameter

```json
{"name": "base_x", "expr": "70 mm", "unit": "mm", "model": false}
{"name": "d1",     "expr": "base_x", "unit": "mm", "model": true}
```

`model: false`-Parameter sind die des Nutzers, angezeigt im Parameter-Panel. `model: true`- ones gehören
zu Features (Skizzen-Bemaßungen) und werden von einem Ausdruck getrieben.

## Features, die revmesh schreibt

| `type` | Schlüsselfelder | Anmerkungen |
|---|---|---|
| `sketch` | `plane: {"type":"origin","name":"XY"}`, `sketch: {points, curves, constraints, counters}` | Punkt 0 ist der feste Ursprung. Kurven und Bedingungen verweisen per **Index** auf Punkte und Kurven. Eine `length`-Bedingung nennt ihren Modell-Parameter in `param`. |
| `extrude` | `sketch` (Feature-ID), `profiles: {"type":"all"}`, `extent: {distance, convention:"total_distance", direction:"positive"}`, `operation: "new_body"` | `body_names` listet den Body auf, den es erzeugt. Ein Schnitt durch alles in beide Richtungen von seiner Skizze: `extent: {distance: "0", convention: "total_distance", direction: "symmetric", through_all: true}`, `operation: "cut"`. |
| `hole` | `position` (Zahlen), `at` (drei Ausdrücke), `direction`, `diameter`, `depth` (einer Sacklochbohrung, von der Fläche; durch alles ohne), `hole: {"type":"simple"}` oder `{"type":"counterbore", cb_diameter, cb_depth}` oder `{"type":"countersink", cs_diameter, cs_angle}` | `at` folgt Parametern; `position` ist die Wahl, von der SolidCraft startet. Eine Bohrung, deren Senkung oder Kegelsenkung auf der Unterseite sitzt, startet dort mit `direction: [0, 0, 1]`. Eine gesenkte Bohrung, die auf der Ebene einer Stufe öffnet (ein Taschenboden nach oben), wird von dieser Ebene gebohrt: Der Achsen-Ausdruck von `at` ist die Fläche der Ebene `<top> - pocket_<n>_depth`, und eine verrundete Kante dort wird an der Startkante der Tasche verrundet (`F<Pocket<n>>:start\|F<hole>:bore:side:0`), geschrieben nach dieser Tasche. `cs_angle` ist der volle Kegelwinkel; ein Parameter in Grad treibt ihn. |
| `fillet` | `edge_names`, `edges` (ein Punkt pro Kante), `radius` | Siehe unten. |
| `box` | `corner` (drei Ausdrücke: die niedrigste Ecke), `length` (X), `width` (Y), `height` (Z), `operation: "join"` oder `"cut"` | SolidCrafts Box-Primitive. |
| `cylinder` | `base` (drei Ausdrücke: Mittelpunkt der Grundfläche), `axis: [0, 0, 1]`, `radius`, `height`, `operation: "join"` | SolidCrafts eigener Zylinder-Primitive speichert `radius` als `"(10 mm) / 2"`, wenn man ihm einen Durchmesser gibt; revmesh schreibt `"boss_<n>_d / 2"`. |
| `move` | `bodies: ["Body1"]`, `translate: [x, y, z]` (drei Ausdrücke) | Verschiebt den ganzen Body; spätere Features nutzen Weltkoordinaten. Die Flächennamen der Extrusion (`F2:side:l1`) bleiben danach gültig. |
| `pattern` | `features: [ids]`, `pattern: {"type": "rectangular", dir1, count1, spacing1, dir2?, count2?, spacing2?}` oder `{"type": "circular", origin: [3 Ausdrücke], axis, count, angle: "360 deg"}` | Anzahlen inklusive des Originals und Ausdrücke; ein als Anzahl genutzter Parameter darf keine Einheit haben (`"unit": ""`), SolidCraft verweigert eine Länge. Richtungen und die Achse sind Zahlen. |
| `mesh_import` | `file`, `meshes: [{name, positions, triangles}]` | Flache `x,y,z`-Positionen und flache Ecken-Indizes; `body_names` listet die Bodies. Eine Position ist eine einfachgenaue Zahl, wenn sie den Roundtrip übersteht (die Präzision des STL selbst, damit die Datei klein bleibt), sonst eine doppeltgenaue — Teile weit vom Ursprung behalten so ihre Präzision. |

## Ebenen

Die `plane` einer Skizze nennt eine der Ursprungsebenen. Ihre Achsen, wie SolidCraft sie definiert
(`Plane` in `solidcore-record`): `XY`-Skizzen (x, y) mit Normale +Z, `YZ`-Skizzen (y, z) mit Normale +X und
`XZ`-Skizzen (x, -z) mit Normale +Y. Eine Extrusion mit `direction: "positive"` läuft entlang der Normalen
der Ebene, daher extrudiert eine Skizze auf `YZ` entlang +X und eine auf `XZ` entlang +Y. So schreibt
revmesh ein Teil, das entlang X oder Y steht: Die Koordinaten des erkannten Profils sind exakt die
Skizzen-Koordinaten auf dieser Ebene, und das Teil wird per `move` nach der Extrusion dorthin verschoben, wo
es in der Datei liegt, wie bei einer Z-Platte.

## Kombinierte Extrusionen

Ein Teil, das der Schnitt von Konturen ist, die entlang zweier oder dreier Achsen extrudiert werden, bekommt
eine Skizze pro Kontur auf der Ursprungsebene ihrer Achse (die Ecken der Kontur sind dort die
Skizzen-Koordinaten, die niedrigste Ecke der Bounding Box des Teils am Ursprung, daher auf `XZ` die zweite
Koordinate -Z) und eine Extrusion davon, `extent: {distance: "extrude_<a>_len", direction: "positive"}`, quer
durch das ganze Teil. Jede Kontur-Skizze wird wie die einer Platte bemaßt: eine `Length`-Bedingung pro
treibender Seite, getrieben von `outline_<a>_<i>_len` (`<a>` der Achsen-Buchstabe), und eine `Angle` an ihrer
längsten schrägen Seite, getrieben von `outline_<a>_<i>_angle`, sodass ein kombiniertes Teil in SolidCraft
ebenso Seite für Seite neu bemaßt und neu gewinkelt wird. Die erste Extrusion erzeugt den Body
(`operation: "new_body"`), jede weitere hat `operation: "intersect"` ohne `body_names` und ohne Ziele:
SolidCraft schneidet den bestehenden Body, koinzidente Flächen eingeschlossen, wie `solidcraft-cli 0.50.0`
ihn baut. Ein `move` bringt den Body dann dorthin, wo er ist, seine Translation entlang der Achse jeder
Kontur `extrude_<a>_0`. Ändert man `extrude_x_len` von 60 auf 50 bei `plugin/revmesh/tests/combined.stl`,
baut es auf 32.500 mm³ neu, wie gerechnet, und `extrude_z_0 = 10` hebt das Teil um 10 mm an.

Eine Kontur mit Öffnungen quer durch sie zeichnet diese in ihrer Skizze nach der Kontur (ihre Linien `l1` bis
`l<n>` zuerst, jede Öffnung bemaßt mit `cutout_<n>_<i>_len`/`cutout_<n>_<i>_angle`, über die Konturen in
Design-Reihenfolge nummeriert) und extrudiert `profiles: {"type": "curves", "loops": [["l1", …, "l<n>"]]}`:
das Profil, dessen äußere Schleife die Kontur ist, die Schleifen darin ihre Löcher. Mit `{"type": "all"}`
extrudiert SolidCraft jede Region, die Öffnungen eingeschlossen, und sie kämen wieder gefüllt
(`plugin/revmesh/tests/combined-windows.stl` baut auf 37.000 statt 35.410 mm³ neu).

Ein gerader Sackloch-Schnitt kommt nach den Schnitten und vor dem `move`, im eigenen Frame des Teils:
`Cut<n>Sketch` auf der Ursprungsebene seiner Achse (seine Kontur bemaßt mit
`cut_<n>_<i>_len`/`cut_<n>_<i>_angle`), und `Cut<n>`, eine Extrusion mit `operation: "cut"` und
`extent: {distance: "cut_<n>_len", direction: "positive", start_offset: "cut_<n>_0 - extrude_<a>_0"}`:
`start_offset` (so speichert SolidCraft es) verschiebt den Start entlang der Normalen der Ebene, und
`cut_<n>_0` ist, wo der Schnitt in der Datei beginnt, daher wird die Translation des `move` entlang der Achse
wieder abgezogen (eine Zahl, wo die Achse keine Kontur hat). Eine rechteckige Tasche ist ein Schnitt entlang
jeder Achse, entlang der sie geschlossen oder offen ist; wie bei mesh2cad gewinnt die längste (bei Gleichstand
die erste von X, Y und Z), daher wird die Tasche von `combined-pocket.stl` entlang X von 5 bis 15 geschnitten,
und `cut_1_len = 20` baut auf 36.400 mm³ neu, wie gerechnet. revmesh nummeriert Schnitte nach Achse und Start;
mesh2cad in der Reihenfolge seiner Netz-Regionen, daher können die Nummern bei mehreren Schnitten abweichen.

Eine runde Bohrung ist ein `cylinder` mit `operation: "cut"`, ebenfalls vor dem `move`: `radius:
"hole_<n>_d / 2"`, `base` die zwei Koordinaten der Bohrung im Frame des Teils (`hole_<n>_x - extrude_x_0`,
oder weniger eine Zahl, wo die Achse keine Kontur hat) und, entlang ihrer Achse, `-1` mit
`height: "extrude_<a>_len + 2"` für eine Durchgangsbohrung (1 mm über jedes Ende hinaus), oder 1 mm außerhalb
der Fläche, auf der eine Sacklochbohrung öffnet, mit `height: "hole_<n>_depth + 1"` und `axis` ins Teil hinein.
Ein Zylinder braucht keine Startfläche, daher darf eine Bohrung in eine schräge Seite eintreten. Bei
`plugin/revmesh/tests/combined-holes.stl` (gebaut mit SolidCraft-Zylinder-Schnitten) baut die Konstruktion
auf 35.272.124 mm³ neu, wie SolidCraft die Quelle vermisst, und `hole_3_d = 8`, `hole_2_depth = 5` auf
34.675.221 mm³, wie gerechnet; mesh2cad schreibt dieselben 14 Parameter und Werte für die als `fine`
exportierte Datei.

## Gestapelte Extrusionen

Ein Stapel von Konturen entlang einer Achse ist pro Band eine `Sketch<k>` und eine `Extrude<k>`: die erste
`operation: "new_body"`, die anderen `operation: "join"` mit `extent.start_offset:
"extrude_<a><k>_0 - extrude_<a>1_0"`, dann ein `move`, dessen Translation entlang der Achse `extrude_<a>1_0`
ist. Die Starts der Bänder nach dem ersten sind Nutzer-Parameter mit Ausdrücken
(`extrude_z2_0 = extrude_z1_0 + extrude_z1_len`), daher hebt eine neue Höhe eines Bandes die Bänder darüber an. Die Skizze jedes Bands wird
wie die Kontur einer Platte bemaßt: eine `Length` pro treibender Seite seiner Kontur, getrieben von
`outline_<k>_<i>_len`, und eine `Angle` an ihrer längsten schrägen Seite, getrieben von
`outline_<k>_<i>_angle` (Öffnungen durch das Band mit `cutout_<n>_<i>_len`/`_angle`, über die Bänder
nummeriert). SolidCraft fusioniert Bänder, die sich Fläche an Fläche treffen, zu einem Body
(`solidcraft-cli 0.50.0`); mesh2cad überlappt sie um 0.02 mm für OpenCascades Fuse, was SolidCraft nicht
braucht. `plugin/revmesh/tests/stack.stl` (eine Basis, ein L und ein Rhombus) baut auf 34.296 mm³ entlang Z
und, gedreht, entlang Y neu; `extrude_z1_len = 15` baut auf 46.296 mm³, die Bänder darüber um 5 mm angehoben.

## Ausschnitte

Die Ausschnitte aus geraden Seiten sind eine Skizze, `CutoutsSketch`, aus geschlossenen Schleifen in
Weltkoordinaten (Seiten entlang X oder Y bleiben horizontal bzw. vertikal, jede Seite bemaßt mit einer
`Length`-Bedingung, getrieben von `cutout_<n>_<i>_len`, die längste schräge Seite jedes Polygons mit einer
`Angle` von der X-Achse der Skizze, getrieben von `cutout_<n>_<i>_angle`), und ein Schnitt durch alles in
beide Richtungen davon, `Cutouts`. Er kommt nach der Verrundung der Kontur, wie die Bohrungen: Ein Ausschnitt
darf in dem Material liegen, das die Verrundung einer Innenecke hinzufügt (ein L mit 5 mm Verrundungen und
ein Dreieck in seiner Innenverrundung baut auf 6.169.124 mm³ neu, wie gerechnet). In die eigene Skizze der
Kontur gezeichnet, würde ein solcher Ausschnitt von der Verrundung wieder gefüllt. (Wären die Öffnungen dort
gezeichnet, müsste die Extrusion das Profil der Kontur nennen, so wie SolidCraft es speichert:
`"profiles": {"type": "curves", "loops": [["l1", "l2", "l3", "l4"]]}`; mit `{"type": "all"}` füllt sie jede
Region.) `cutouts.stl` (exportiert von SolidCraft: eine 70 x 40 x 6 Platte mit einem ausgeschnittenen
16 x 20 Rechteck und einem Sechseck 7 mm zu dessen Ecken, und einer Ø5-Bohrung) importiert auf 13.998.378 mm³,
0.022 mm³ über dem, was SolidCraft gemessen hat, da die Ecken des Sechsecks auf 0.01 mm verrundet sind; mit
`extrude_len = 9` und `hole_1_d = 6` baut es fehlerfrei auf den für diese verrundeten Ecken gerechneten Wert
neu. Ein Rechteck mit Ausschnitten wird als Kontur geschrieben, wie mesh2cad es tut (`extrude_len`,
`extrude_0`), nicht als `base_*`-Platte.

Sobald ein Ausschnitt eine Bogen-Seite hat, bekommt jeder Ausschnitt seine eigene Skizze `Cutout<n>Sketch` und
einen Schnitt `Cutout<n>` durch alles in beide Richtungen, statt der gemeinsamen `CutoutsSketch`: Die
Bogen-Seite ist eine `arc`-Kurve, deren `Diameter`-Bedingung vom Modell-Parameter hinter
`cutout_<n>_arc_<i>_d` getrieben wird, ihr Mittelpunkt ein freier Skizzenpunkt, den der Löser im Abstand des
treibenden Durchmessers von den beiden Ecken der Seite hält (wie die Bogen-Seiten einer Kontur, siehe die
polygonale Kontur unten), die treibenden geraden Seiten mit `cutout_<n>_<i>_len` bemaßt wie zuvor, und eine
gerade Seite, die tangential an eine Bogen-Seite stößt, mit einer `tangent`-Bedingung gehalten. Wie in einer
Kontur ist die einer Bogen-Seite gegenüberliegende Seite, die tangential an beide Nachbarn stößt, abgeleitet,
ihre Länge ist der Durchmesser des Bogens. Ein Ausschnitt, dessen Ecken alle verrundet sind, ein D-förmiger
Ausschnitt, eine konkave Bogen-Seite und eine Kurve, die kein Kreisbogen ist, bleiben ein Netz.

## Zapfen

Ein Zapfen ist ein `cylinder`, der mit der Platte verbunden ist, stehend auf dem Ausdruck der Oberseite
(`base_z0 + base_t`, oder `extrude_0 + extrude_len` bei einer Kontur), nach den Bohrungen und Langlöchern.
`bosses.stl` (exportiert von SolidCraft: eine 70 x 40 x 6 Platte mit einem Ø10 x 5 und einem Ø8 x 3 Zapfen und
einer Ø5-Bohrung) importiert auf 17.225.686 mm³, das Volumen, das SolidCraft für das exportierte Teil gemessen
hat. Mit `boss_1_d = 12`, `boss_1_h = 8`, `boss_2_x = 40`, `boss_2_h = 4`, `base_t = 9` und `base_z0 = 2` baut es
fehlerfrei auf 26.129.126 mm³ neu, wie gerechnet; erneut exportiert, wird das neu aufgebaute Teil mit diesen
Werten erkannt.

## Verrundete Zapfenfüße

Ein Zapfen, dessen Fuß verrundet ist, ist derselbe `cylinder`, gefolgt von einer `fillet` der Kante zwischen der
Oberseite der Platte und der Seite des Zylinders: `edge_names: ["F<extrude>:top|F<cylinder>:side"]` (SolidCraft
nennt diese Kante `F1:+z|F2:side` bei einem Box-Primitiv), Radius `boss_<n>_fillet`. Im Netz verbreitert die
Verrundung die Standfläche des Zapfens in der Oberseite um ihren Radius; sie muss in Schritten von höchstens
50° ihres Viertelskreises gemeesht sein (in einem Schritt ist sie eine 45°-Fase, die genauso aussieht).
`boss-fillet.stl` (exportiert von SolidCraft 0.50: eine 70 x 40 x 6 Platte mit einem Ø10 x 8 Zapfen, dessen Fuß
mit 2 mm verrundet ist, einem Ø8 x 5 Zapfen und einer Ø4-Bohrung) importiert auf 17.633.625 mm³, das Volumen, das
SolidCraft für das exportierte Teil gemessen hat (die Verrundung addiert `2π (f² (r + f/2) − π f²/4 (r + f) +
f³/3)`); mit `boss_1_fillet = 3`, `boss_1_d = 12`, `boss_1_x = 22` und `base_t = 8` baut es fehlerfrei auf
23.536.520 mm³ neu, wie gerechnet, und erneut exportiert wird es mit diesen Werten erkannt.

## Verrundete Bohrungsränder

Eine Durchgangsbohrung, deren Rand verrundet ist, ist eine schlichte `hole`, gefolgt von einer `fillet` der Kante
zwischen der Fläche und ihrer Bohrung: `edge_names: ["F<extrude>:top|F<hole>:bore:side:0"]` auf der Oberseite,
`"F<extrude>:start|F<hole>:bore:side:0"` auf der Unterseite, Radius `hole_<n>_rim_r` bzw.
`hole_<n>_bot_rim_r`, wie mesh2cad sie nennt. Im Netz ist die Mündung um den Radius der Verrundung breiter als
die Bohrung und genauso tief, wie eine 90°-Kegelsenkung; sie ist eine Verrundung, wenn ihre Ecken zwischen Fläche
und Bohrung auf dem Viertelskreis liegen (eine Verrundung in einem Schritt bleibt die Kegelsenkung, als die sie
aussieht). `hole-rims.stl` (exportiert von SolidCraft 0.50: eine 70 x 40 x 8 Platte mit einer Ø8-Bohrung, deren
oberer Rand mit 1.5 mm verrundet ist, und einer Ø6-Bohrung, deren unterer Rand mit 1 mm verrundet ist)
importiert auf 21.754.183 mm³, das Volumen, das SolidCraft für das exportierte Teil gemessen hat; mit
`hole_1_rim_r = 2`, `hole_1_d = 9`, `hole_2_bot_rim_r = 1.5`, `hole_2_x = 52` und `base_t = 10` baut es
fehlerfrei auf 27.044.286 mm³ neu, wie gerechnet, und erneut exportiert wird es mit diesen Werten erkannt.

## Zapfen mit Formschräge und Fase

Ein Zapfen mit Formschräge ist ein `cone` (`base`, `axis`, `radius`, `top_radius`, `height`,
`operation: "join"`), dessen `top_radius` `boss_<n>_d / 2 - boss_<n>_h * tan(boss_<n>_draft_deg)` ist
(SolidCrafts Ausdrücke kennen `tan` eines Winkel-Parameters). Ein Zapfen mit gefaster Oberkante ist der
`cylinder`, gefolgt von einer `chamfer` (`edge_names: ["F<cylinder>:side|F<cylinder>:top"]`, `edges`, `distance`
und `distance2`, wenn beide verschieden sind). Die erste Distanz läuft entlang der am weitesten oben
liegenden Fläche, der Oberseite: `boss_<n>_chamfer_w` von der Seite hinein, `distance2` das `boss_<n>_chamfer_h`
an ihr hinab; eine 45°-Fase ist eine einzige `boss_<n>_chamfer`, wie mesh2cad sie schreibt. Im Netz ist die
Oberseite eines solchen Zapfens schmaler als seine Standfläche; seine Wand wird gegen das Profil jeder Art der
Reihe nach geprüft (eine Verrundung am Fuß, ein gerader Kegel, ein Zylinder und dann die Fase), jede Ecke und jede
Kantenmitte auf ihr rund um die Achse. `boss-draft-chamfer.stl` (exportiert von SolidCraft 0.50: eine
70 x 40 x 6 Platte mit einem Ø12 x 8 Zapfen mit 5° Formschräge, einem Ø10 x 6 Zapfen, dessen Oberkante mit
1 mm gefast ist, und einer Ø4-Bohrung) importiert auf 17.984.519 mm³, das Volumen, das SolidCraft für das
exportierte Teil gemessen hat; mit `boss_1_draft_deg = 8`, `boss_1_h = 7`, `boss_2_chamfer = 1.5`, `boss_2_d = 11`
und `base_t = 8` baut es fehlerfrei auf 23.503.294 mm³ neu, wie gerechnet, und erneut exportiert wird es mit
diesen Werten erkannt. Eine 1.5 x 0.5 Fase (`distance: 1.5 mm, distance2: 0.5 mm`) kommt als
`boss_1_chamfer_w = 1.5`, `boss_1_chamfer_h = 0.5` zurück und baut auf das Volumen neu, das SolidCraft gemessen
hat.

## Aufsätze und Taschen

Ein Aufsatz ist eine `box`, mit der Platte verbunden, ihre Ecke bei `[pad_<n>_x - pad_<n>_w / 2, pad_<n>_y -
pad_<n>_l / 2, base_z0 + base_t]`; eine rechteckige Tasche eine `box`, aus der Platte geschnitten, von
`base_z0 + base_t - pocket_<n>_depth` aufwärts bei einer Tasche in der Oberseite, oder von `base_z0` bei einer
in der Unterseite. Eine Tasche mit beliebiger anderer Kontur aus geraden Seiten ist stattdessen eine `sketch`
(die Mund-Kontur auf der Ursprungsebene, jede Seite eine `Length`-Bedingung, getrieben vom Modell-Parameter
hinter `pocket_<n>_<i>_len`, nach den rechteckigen Taschen nummeriert) und eine `extrude` cut von
`pocket_<n>_depth`, startend bei `base_z0 + base_t - pocket_<n>_depth` bei einer Oberseiten-Tasche oder bei
`base_z0` bei einer Unterseiten-Tasche. `pockets.stl` (exportiert von SolidCraft: eine 70 x 40 x 6 Platte mit
einem 16 x 12 x 4 Aufsatz, einer 20 x 14 x 3 Tasche in der Oberseite, einer 12 x 8 x 2 in der Unterseite und
einer Ø4-Bohrung) importiert auf 16.460.602 mm³, das Volumen, das SolidCraft für das exportierte Teil gemessen
hat. Mit `pad_1_w = 20`, `pad_1_h = 6`, `pocket_1_depth = 4`, `pocket_1_l = 10`, `pocket_2_x = 45`,
`pocket_2_depth = 1`, `base_t = 8` und `base_z0 = 3` baut es fehlerfrei auf 22.843.469 mm³ neu, wie gerechnet;
erneut exportiert, wird es mit diesen Größen erkannt.

## Langloch-Skizzen

Ein Langloch wird gezeichnet, wie SolidCrafts Mittelpunkt-Langloch-Werkzeug es zeichnet: Linien `l1` und `l2`,
Bögen `a1` und `a2` (`{"type": "arc", "c", "a", "b"}`, gegen den Uhrzeigersinn von `a` nach `b` um den
Mittelpunkt `c`, dessen Punkte `a1.center` und `a2.center` heißen), und die Konstruktionslinie `l3` zwischen den
Bogen-Mittelpunkten, mit ihrem Mittelpunkt `p5`. Ihre Bedingungen:

- vier `tangent` (`a`, `b`: Kurven-Indizes), `equal` (die zwei Bögen), `midpoint` (`p`, `l`)
- `length` von `l1` und `diameter` (`c`) von `a1`, jede mit einem Modell-Parameter
- `coincident` (`p`, `q`) des Mittelpunkts mit `p6`, einem Punkt, der von den Positions-Parametern gesetzt wird:
  `{"id": "p6", "pos": [x, y], "fixed": true, "exprs": {"x": "slot_1_x", "y": "slot_1_y"}}`. Eine
  `distance_x`-/`distance_y`-Bemaßung vom Ursprung ginge nicht: Diese sind vorzeichenlos (die Seite ist die, auf
  der der Punkt gezeichnet ist, ein negativer `value` schlägt die Skizze mit „invalid value: dimension value -75"
  fehl), daher könnte ein Langloch nicht über den Ursprung hinweg verschoben werden.
- `angle` (`a`, `b`: Kurven-Indizes, `value` in Radiant, sein Modell-Parameter in Grad) zwischen `l3` und der
  X-Achse der Skizze. Die Achse wird als `l4` eingebunden: zwei feste Punkte und eine Konstruktionslinie mit
  `"link": "j1"`, und `links: [{"id": "j1", "kind": "project", "source": {"type": "sketch_axis", "name": "x"}}]`
  auf der Skizze. Winkel von 0 und 90 Grad funktionieren.

Die `counters` der Skizze enthalten dann `"a": 2` und `"j": 1`.

`slots.stl` (exportiert von SolidCraft: eine 80 x 50 x 8 Platte mit einem 20 x 6 Langloch entlang X, einem
15 x 8 entlang Y, einem 16 x 5 bei 30 Grad und einer Ø5-Bohrung) importiert auf 28.497.522 mm³, das Volumen,
das SolidCraft für das exportierte Teil gemessen hat. Alle Langloch-Größen ändern, zwei davon verschieben, eines
auf 45 Grad drehen und `base_t = 10` setzen, baut fehlerfrei auf 34.129.978 mm³ neu, wie gerechnet; die
Langlöcher auf 20, 0 und 90 Grad zu drehen hielt das Volumen. Dasselbe Teil an negative Koordinaten verschoben,
und so verschoben, dass die Mitte eines Langlochs auf dem Ursprung liegt, baute ebenfalls neu, und ein Langloch,
von X = 0 nach X = -5 verschoben, war beim erneuten Export und Einlesen des neu aufgebauten Teils da.

Felder, die SolidCraft selbst ableitet und die weggelassen werden können: `param_names`, `face_names`,
`edge_refs`.

## Kantennamen: nötig für robuste Verrundungen

Eine Verrundung, die nur Pick-Punkte (`edges`) hat, findet ihre Kanten nach einem Neuaufbau per Nähe wieder. In
einem Test ließ das Ändern von `base_x` von 70 auf 100 zwei von vier Ecken-Verrundungen auf die falschen Kanten
springen („2 edge reference(s) re-found on the nearest edge“). Mit `edge_names` baute dieselbe Änderung ohne
Warnungen neu. revmesh schreibt sie daher immer:

```text
F<extrude id>:side:<line id>|F<extrude id>:side:<line id>
```

`F2:side:l1` ist die Seitenfläche, die Extrusion 2 aus der Skizzen-Linie `l1` gemacht hat; eine Kante ist das
Paar der Flächen, die sie verbindet. Die Kappen der Extrusion sind `F2:top` (am fernen Ende einer positiven
Extrusion) und `F2:start` (auf der Skizzen-Ebene), daher nennt eine rundum verrundete Box
`F2:side:l<i>|F2:top` und `F2:side:l<i>|F2:start` ebenfalls. Weitere Namen aus gespeicherten Konstruktionen:
`F2:top`, `F3:bore:side:0` (die Bohrung einer Hole), `F4:blend(F2:side:l1|F2:top)` (eine Verrundungsfläche).

## Verifiziert

`crates/revmesh-core/tests/data/plate-with-hole.json` (geschrieben von `PlateSpec::to_design`) öffnete in
SolidCraft 0.50.0 fehlerfrei: Volumen 16.452.05 mm³, von Hand gerechnet (70 × 40 × 6 abzüglich einer 8 mm
Bohrung abzüglich vier 3 mm Ecken-Verrundungen). `base_x = 100`, `hole_1_x = 50` und `hole_1_d = 12` bauen
auf 23.275.06 mm³ neu, ebenfalls wie gerechnet, ohne Warnungen.

Eine polygonale Kontur (`ProfileSpec::to_design`: horizontale und vertikale Bedingungen auf den achsparallelen
Seiten plus eine `Length`-Bedingung pro treibender Seite, getrieben vom Modell-Parameter hinter
`outline_<i>_len`, eine `Angle`-Bedingung an der längsten schrägen Seite, getrieben von `outline_<i>_angle` —
die Länge der Winkelseite ist abgeleitet, und wenn die übrigen Seiten in ihren Orientierungen die Richtung
bereits festlegen, wie die Katheten eines rechtwinkligen Dreiecks, horizontal und vertikal gehalten, es für die
Diagonale tun, ist auch die Länge der längsten achsparallelen Seite abgeleitet (siehe
`dimensioned_sides_for_angle` in `crates/revmesh-core/src/design/sketch.rs`), eine Verrundung auf jeder Ecke
namens `F2:side:l<i>|F2:side:l<i+1>`) funktioniert genauso, Innenecken eingeschlossen. Eine Kontur mit Bogen-Seiten
(`polygon_sketch_arcs`) zeichnet jede Bogen-Seite als `arc`-Kurve, deren `Diameter`-Bedingung vom Modell-Parameter
hinter `arc_<i>_d` getrieben wird; der Mittelpunkt des Bogens ist ein freier Skizzenpunkt (wie die
Langloch-Bogen-Mittelpunkte), den der Löser im Abstand des treibenden Durchmessers von den beiden Ecken der Seite
hält, sodass das Ändern eines `arc_<i>_d` die Skizze lösbar lässt. Eine gerade Seite, die an einer sanft knickenden
Ecke an eine Bogen-Seite stößt, wird tangential zu ihr gehalten, wie die Seiten eines Langlochs, und zwei
Bogen-Seiten, die durch solche tangentialen geraden Seiten verbunden sind (die Enden eines Stadions), teilen über
eine `equal`-Bedingung einen Radius, nur die erste treibt ihren `Diameter`. Die geraden Seiten behalten ihre
`Length`-Bedingungen — außer der einer Bogen-Seite gegenüberliegenden Seite, die tangential an beide Nachbarn in
einem vierseitigen Ring stößt (eine halbrunde Seite an einem Rechteck): Die Tangenten machen ihre Länge zum
Durchmesser des Bogens, daher ist sie abgeleitet. Das L-Winkelstück aus `plugin/revmesh/tests/l-bracket.stl`
importiert in `solidcraft-cli 0.50.0` auf 6.118.29 mm³, wie gerechnet; `outline_fillet_r = 4`, `hole_1_d = 8`,
`hole_2_x = 50` und `base_z = 8` bauen jede Feature fehlerfrei auf 9.461.81 mm³ neu, ebenfalls wie gerechnet.

Da revmesh ein Teil dort lässt, wo es in der Datei ist, bleiben Skizze und Extrusion am Ursprung und ein
`move`-Feature direkt nach der Extrusion bringt den Body dorthin: bei einer rechteckigen Platte um
`[base_cx - base_x / 2, base_cy - base_y / 2, base_z0]`, bei einer Kontur um ihre Ecke und `extrude_0`.
Bohrungen und Verrundungs-Picks sind in Weltkoordinaten. In `solidcraft-cli 0.50.0` baut `plate-holes.stl` mit
`base_cx = 50`, `base_x = 90`, `base_z0 = 5`, `fillet_r = 5` und `hole_2_x = 70` fehlerfrei und ohne Warnungen auf
20.978.79 mm³ neu, und das L-Winkelstück (in der Datei bei (20, 10)) mit `extrude_0 = 4`, `extrude_len = 8` und
`outline_fillet_r = 4` auf 9.706.85 mm³, beides wie gerechnet.

Ein Bohrungsmuster ist ein `hole`-Feature an der ersten Bohrung und ein `pattern`-Feature darauf. Die erste
Bohrung eines Lochkreises liegt bei `[cx + r * cos(angle), cy + r * sin(angle), top]` als Ausdrücke und der
Ursprung ihres Musters bei `[cx, cy, 0]`. `plate-patterns.stl` (ein 2 x 2 Gitter, eine Reihe aus drei, ein
Lochkreis aus sechs um eine einzelne Bohrung) importiert in `solidcraft-cli 0.50.0` auf 55.995.49 mm³, wie
gerechnet; mit `hole_grid_1_nx = 3`, `hole_grid_1_pitch_x = 50`, `hole_row_1_n = 4`, `hole_circle_1_n = 8`,
`hole_circle_1_r = 18` und `hole_circle_1_angle = 0` baut es fehlerfrei und ohne Warnungen auf 55.566.66 mm³ mit
fünf weiteren Bohrungen neu, ebenfalls wie gerechnet.

`stepped-holes.stl` (exportiert von SolidCraft: eine 60 x 40 x 8 Platte mit einer Ø5.5 / Ø10 x 3 Senkung und
einer Ø6 / Ø12 90° Kegelsenkung oben und einer Ø4 / Ø8 x 2.5 Senkung unten) importiert auf 18.311.518 mm³, das
Volumen, das SolidCraft für das exportierte Teil gemessen hat. Mit `hole_1_cbore_d = 12`, `hole_1_cbore_depth = 4`,
`hole_2_cbore_depth = 3.5`, `hole_3_csk_d = 14`, `hole_3_csk_angle = 82` und `base_t = 10` baut es fehlerfrei und
ohne Warnungen auf 22.614.14 mm³ neu, wie gerechnet.

`blind-holes.stl` (exportiert von SolidCraft: eine 60 x 40 x 8 Platte mit Ø6 x 4 und Ø3 x 6 Sacklochbohrungen
von oben, einer Ø8 x 2.5 von unten und einer Ø5 Durchgangsbohrung) importiert auf 18.761.748 mm³, das Volumen,
das SolidCraft für das exportierte Teil gemessen hat. Die `depth` einer Sacklochbohrung reicht bis zu ihrem
flachem Boden. Mit `hole_1_d = 7`, `hole_1_depth = 6`, `hole_3_x = 40`, `hole_3_depth = 4`, `hole_4_depth = 2` und
`base_t = 10` baut es fehlerfrei auf 23.357.544 mm³ neu, wie gerechnet.

In einer polygonalen Kontur kommt die Verrundung vor die Bohrungen: Die Verrundung einer Innenecke fügt Material
hinzu, und eine davor gebohrte Bohrung würde Leere bohren. `l-inside-hole.stl` (5 mm Ecken, eine 0.4 mm Bohrung
bei (16, 16) in diesem Material) baut in `solidcraft-cli 0.50.0` auf 6.267.07 mm³ neu, wie gerechnet, mit
7 zylindrischen Flächen (sechs Verrundungen und die Bohrung) und ohne Feature-Fehler. Die Historie des Rechtecks
behält ihre verifizierte Reihenfolge (Bohrungen, dann Verrundung): Seine Ecken sind alles Außenecken, die nur
Material wegnehmen.

`solidcraft-cli eval design.sc` vermisst eine Konstruktionsdatei direkt (Bodies, Volumen, Flächentypen und der
Fehler jedes Timeline-Features), und `eval script.json --in design.sc` vermisst sie nach einem Skript wie
`{"commands": [{"command": "parameters.change", "params": {"changes": {...}}}]}`.
