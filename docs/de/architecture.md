# Architektur

## Die Bedingung, die alles prägt

SolidCraft führt Plugins als WebAssembly-Komponenten aus, gebaut für `wasm32-unknown-unknown`: kein
WASI, keine Threads, kein Netzwerk, keine Subprozesse, keine System-Zufälligkeit. mesh2cad selbst
(Python mit numpy, scipy, trimesh und OpenCascade) kann dort nicht laufen, daher portiert revmesh seine
Erkennung nach Rust.

Es portiert **nicht** mesh2cads OpenCascade-Teil, der erkannte Flächen zu einem Volumenkörper näht.
Der Importer schreibt stattdessen eine *Feature-Historie* (Skizze, Extrusion, Bohrungen, Verrundungen)
und lässt SolidCrafts eigenen Kernel den Volumenkörper bauen. Das ist sowohl weniger Code als auch das,
was das Ergebnis bearbeitbar macht.

## Aufbau

```text
crates/
  revmesh-core/     host-unabhängig: STL/OBJ/PLY → Mesh → Erkennung → Design + LossReport
    src/format.rs      das Format (an Name oder ersten Bytes) und der gepufferte MeshReader, gemeinsame Fehler
    src/stl.rs         binäres und ASCII-STL
    src/obj.rs         Wavefront OBJ
    src/ply.rs         PLY, ASCII und binary little-endian
    src/mesh.rs        Dreieckssuppe, Bounds, Volumen, Fläche
    src/recognize/     was die Erkennung findet (`Recognition`), die Extrusionsachse und die
                       Rotationen, die sie auf Z ausrichten (axis.rs), die Specs (spec.rs) und
                       PlateSpec/ProfileSpec → Design (to_design.rs)
    src/plate/         der Platten-Erkenner, eine Datei pro Concern: mod.rs (Einstieg, gemeinsame
                       Konstanten), geometry.rs (2D-Messungen), faces.rs und walls.rs (Flächen-
                       und Wand-Klassifikation), outline.rs (Kontur, verrundete Ecken), holes.rs
                       (runde Bohrungen, Langlöcher, polygonale Ausschnitte), bosses.rs, prism.rs
                       (Prismen-Prüfung, Flächenschleifen, Bohrungs-Paarung, der verrundete Rand
                       einer rundum verrundeten Box), tests/ (die Tests, eine Datei pro Concern)
    src/synth.rs       synthetische Netze bekannter Teile (Platten, Profile, rundum verrundete
                       Boxen), für Tests und STL-Fixtures
    src/design/        das SolidCraft-Dokumentenmodell (die Teilmenge, die revmesh schreibt):
                       mod.rs (Design, Features, DesignBuilder), sketch.rs (Skizzen und ihre
                       Konstruktoren)
    src/report.rs      der Verlustbericht, den SolidCraft nach einem Import zeigt
    src/import.rs      die ganze Pipeline und die Importoptionen
    tests/data/        Fixtures und Golden Designs
  revmesh-plugin/   die Komponente: craft:plugin@0.2.0, Welt importer-plugin
    wit/plugin.wit     vendored aus SolidCraft v0.50.0
plugin/revmesh/     das Paket: plugin.json, icons/ (das SVG, das SolidCraft im Plugins-Menü zeigt), die gebaute .wasm, SolidCraft-Testfälle
xtask/                 Component-Wrapping, Packaging, Plugin-Tests
```

## Faustregeln

- **Alles Testbare liegt im Core.** Das Plugin-Crate kompiliert auf nativen Targets zu nichts; es liest
  nur die Quelle in 1-MiB-Chunks, ruft `revmesh_core::import` auf und schreibt die Konstruktion in
  1-MiB-Chunks.
- **Die Ausgabeseite steht vor den Erkenner fest.** `PlateSpec::to_design` schreibt bereits die Historie,
  die eine erkannte Platte wird, und ihre Golden-Datei wurde in SolidCraft geöffnet und neu aufgebaut. Die
  Aufgabe eines Erkennens ist nur, eine `PlateSpec` (und ihre Nachfolger) auszufüllen.
- **Verliere nie das Teil des Nutzers.** Was nicht erkannt wird, wird als Mesh-Body importiert, mit einem
  Eintrag im Verlustbericht.
- **Genau erkennen oder gar nicht.** Ein Erkenner misst das Netz so, wie ein CAD-Exporteur es geschrieben
  hat (Facetten-Ecken auf den wahren Flächen), innerhalb 1 µm oder 10⁻⁵ der Teilgröße, je nachdem, was
  größer ist, und das neu aufgebaute Teil muss dem Netz volumenmäßig innerhalb von 1 % entsprechen. Alles
  Unsicherere bleibt ein Netz, mit dem Grund im Verlustbericht.
- **Nimm die Exporteure, wie sie sind.** Exporteure unterscheiden sich darin, wie sie netzen: SolidCrafts
  eigener STL-Export mesht hohe Wände in Bändern (Ecken zwischen Ober- und Unterseite, Facetten, die an
  einer gekrümmten Wand leicht anlehnen), mesht Ober- und Unterseite getrennt und verteilt die Ecken von
  Bögen und Rändern ungleichmäßig. Daher wird eine Ecke zwischen den Flächen gegen die Wände selbst geprüft
  (sie muss auf dem Kreis einer Bohrung, einer Seite oder dem Bogen einer Ecke liegen), nicht gegen die
  Normale ihrer Facette, und Bögen und Bohrungen werden dort gelesen, wo ihre Ecken liegen, nicht anhand
  gleicher Facetten. `plugin/revmesh/tests/solidcraft-l-bracket.stl` ist ein solcher Export.
- **Deterministische Numerik.** Keine Threads und keine System-Zufälligkeit in der Sandbox: Algorithmen wie
  RANSAC nutzen einen festen Seed. Die Literatur hinter dem Erkennungsansatz — RANSAC-Formerkennung,
  Feature-Erkennung aus STL-Netzen, Extrusionszylinder-Zerlegung — ist unter
  [Forschung und Literatur](/de/guide/comparison#research-and-literature) gelistet.
- **Eine Pipeline, drei Achsen.** Wie mesh2cad, das ein Prisma entlang X, Y und Z probiert, probiert
  `recognize` jede Achse der Reihe nach: Das Netz wird per eigentlicher Rotation (damit Volumen und
  Windungen erhalten bleiben) in ein Frame gedreht, in dem die Achse +Z ist, die Z-Pipeline läuft
  unverändert, und das Ergebnis wird zurückgedreht. Z wird zuerst probiert, damit eine Box die
  `base_*`-Platte behält. Die Koordinaten des Profils in diesem Frame sind exakt die Skizzen-Koordinaten
  auf SolidCrafts passender Ursprungsebene (YZ: y, z; XZ: x, -z; XY: x, y), daher muss der Design-Schreiber
  nur die Ebene wählen und die Richtungen abbilden.

## Limits in SolidCraft

128 MiB Speicher standardmäßig (bis 4 GiB, wenn der Nutzer es beim Installieren genehmigt), ein
Import-Deadline von bis zu 10 Minuten, eine 64 MiB Komponente und eine 256 MiB Konstruktion. `plugin.json`
bittet um 128 MiB und 2 Minuten, das meiste, was ein Plugin ohne Genehmigung des Nutzers bekommt.

Der Speicher entscheidet, wie groß eine Datei sein darf. Das Plugin hält daher nichts doppelt: Die STL
wird chunkweise geparst, während sie gelesen wird (`stl::StlReader`), das Netz wandert ohne Kopie in die
Konstruktion, und die Konstruktion wird direkt in 1-MiB-Schreibvorgänge an SolidCraft serialisiert, mit
Koordinaten in einfacher Genauigkeit. Gemessen mit SolidCraft 0.50: eine STL mit 1,5 Millionen Dreiecken
(75 MB) importiert als Netz in etwa 8 s; 2 Millionen Dreiecke überschreiten 128 MiB (vor dem Streaming
schon 600.000).

## Selbstprüfung in der Sandbox der App

Im auto-Modus wird die serialisierte Konstruktion in SolidCrafts privater Sandbox gestaget (Capability
`evaluate@0.2`) und dort neu aufgebaut (`document.inspect`), bevor sie in die Konstruktionsdatei
 geschrieben wird. Ein Befehlsfehler oder ein Timeline-Feature mit einem `error` in der
`document.inspect`-Antwort fällt auf den Mesh-Body zurück, mit dem Grund im Verlustbericht (Code
`revmesh.selfcheck`); ein Aufruf-Fehler (eine Konstruktion, die die App nicht öffnen kann, eine Antwort über
1 MiB) ist ein Fehler nur für diesen Aufruf, daher wird die Konstruktion ungeprüft geschrieben. Das Parsen
der Antwort und die Fallback-Richtlinie liegen in `revmesh_core::import` (`SelfCheck`, `apply_self_check`),
nativ getestet; das Plugin stagt und führt nur aus. Mesh-Modus-Importe werden nicht geprüft: Die
Konstruktion ist das Netz, und das doppelte Halten ihres JSON würde das Speicherbudget oben sprengen.

## Testebenen

1. `cargo test`: Unit-Tests des Core mit synthetischen Netzen und Golden Designs.
2. `cargo xtask plugin-test`: `solidcraft-cli plugin test` installiert die echte Komponente in einem
   Scratch-Ordner und importiert jede Datei in `plugin/revmesh/tests` über SolidCrafts `file.import`, dann
   werden Bodies, Features, der Verlustbericht und das gemessene Volumen geprüft.
3. `cargo xtask check`: importiert jede Fixture, baut die Konstruktion in SolidCraft neu auf
   (`solidcraft-cli eval`: ein Feature-Fehler schlägt die Prüfung fehl) und exportiert sie erneut
   (`solidcraft-cli run --out`), dann vergleicht sie das neu aufgebaute Netz mit der Quell-STL in
   `check.rs`: der Volumenfehler und die Oberflächenabweichung (Hausdorff der beiden Triangulierungen),
   gegated wie mesh2cad seine Skripte gate (Abweichung höchstens `max(4 * 0.1% der Diagonale, 0.1 mm)`,
   Volumenfehler höchstens 1%).
4. Von Hand: eine Golden-Konstruktion (`cargo run -p revmesh-core --example plate_design`) in SolidCraft
   öffnen, Parameter ändern und zusehen, wie sie neu aufgebaut wird.
5. `scripts/example-cli.sh` (CI-Job `example-cli`): installiert das `cargo xtask dist`-Zip in einem
   Scratch-Plugins-Verzeichnis und importiert eine Beispiel-STL über `file.import` der
   `solidcraft-cli` (Eintrag `revmesh.stl`), dann wird das Volumen der neu exportierten STL gegen die
   Quelle innerhalb von 1 % geprüft. Die Skripte liegen in
   [examples/cli/](https://github.com/stritti/revmesh/tree/main/examples/cli).
