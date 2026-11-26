# Entwicklung

## Was ein Import produziert

```sh
cargo run -q -p revmesh-core --example params -- part.stl                      # Parameter und Bericht
cargo run -q -p revmesh-core --example params -- part.stl --design part.json   # und die Konstruktion
```

`part.json` öffnet in SolidCraft (File › Open) und lässt sich headless mit `solidcraft-cli eval part.json`
vermessen; `solidcraft-cli eval script.json --in part.json` vermisst sie nach einem Skript aus
`parameters.change`-Befehlen, womit geprüft wird, dass ein neuer Feature-Typ beim Ändern seiner
Parameter neu aufgebaut wird. `solidcraft-cli run --in part.json --out rebuilt.stl` exportiert das neu
aufgebaute Teil.

`cargo xtask check` macht das für jede Fixture in `plugin/revmesh/tests` und vergleicht das neu aufgebaute
Teil mit dem Quell-Netz: `solidcraft-cli eval` für Feature-Fehler und Volumen, der Re-Export für die
Oberflächenabweichung (Hausdorff, `check.rs`), gegated wie mesh2cad seine Skripte gate (siehe
`docs/architecture.md`, Testebene 3).

## Texte und Übersetzungen

Jeder Text, den der Nutzer sieht (die Options-Titel und -Beschreibungen des Import-Dialogs in
`plugin.json`, die Fortschrittsmeldungen, der Verlustbericht und die STL-Fehlermeldungen), ist eine
Nachricht in den Katalogen `crates/revmesh-core/locales/<locale>.json`: flaches JSON von Nachrichten-ID
→ Text, mit `{name}`-Platzhaltern, gelesen über `revmesh_core::Locale` (`text`, `format`, `decimal`).
Englisch (`en`) ist die Standardeinstellung und der Fallback; Deutsch (`de`) ist die zweite Locale.

SolidCraft zeigt die Manifest-Texte eines Plugins so, wie das Paket sie hat, und teilt dem Plugin die
Sprache der App nicht mit, daher ist die Sprache pro Paket festgelegt: `cargo xtask dist` baut eine zip
pro Locale, lokalisiert `plugin.json` mit `revmesh_core::i18n::localize_manifest` (IDs
`manifest.description` und `manifest.<entry>.<option>.title|description`) und baut die Komponente mit
`REVMESH_LOCALE=<locale>`. `plugin/revmesh/plugin.json` bleibt die englische Quelle und muss zu `en.json`
passen (ein Test prüft das). Optionswerte wie `auto` und `mesh` werden nie übersetzt, da das Plugin sie
parst.

Um einen Text hinzuzufügen, ergänze seine ID in `en.json` und jedem anderen Katalog; die Tests schlagen
fehl bei einer Nachricht, die in einer Locale fehlt oder deren Platzhalter von Englisch abweichen. Um
eine Locale hinzuzufügen, ergänze ihren Katalog und eine `Locale`-Variante. Noch nicht übersetzt: die
detaillierten Gründe, die die Erkenner fürs Behalten eines Netzes nennen (der Teil nach dem Doppelpunkt in
„Es wurden keine parametrischen Features erkannt: …") und der Grund innerhalb eines ASCII-STL-Syntaxfehlers.

## Vergleich mit mesh2cad

revmesh folgt mesh2cads Erkennung und Parameternamen, daher wird jeder Erkenner mit mesh2cads Ausgabe auf
denselben STL-Dateien verglichen. Installiere mesh2cad mit CadQuery einmal, zum Beispiel in einer eigenen
virtuellen Umgebung:

```sh
uv venv ~/.venvs/mesh2cad && VIRTUAL_ENV=~/.venvs/mesh2cad uv pip install "mesh2cad[cadquery]"
export MESH2CAD=~/.venvs/mesh2cad/bin/mesh2cad
```

Dann

```sh
scripts/compare-mesh2cad.sh plugin/revmesh/tests/*.stl
```

gibt pro Datei eine Markdown-Tabelle aus: jeder Parametername, den eines der beiden Werkzeuge schreibt,
mesh2cads Wert (aus dem CadQuery-Skript, das es schreibt) und revmeshs, mit `≠` wo sie um mehr als 0.001
abweichen. Unter jeder Tabelle stehen mesh2cads Verdict (das Skript und seine Abweichung vom Netz, oder
warum es abgelehnt hat) und revmeshs Import-Zusammenfassung. Ein Pull Request, der einen Erkenner ergänzt
oder ändert, enthält diese Tabelle für seine Testdateien. Die Abbildung der mesh2cad-Module auf unsere und die
bewussten Abweichungen stehen in
[research/mesh2cad-parity.md](https://github.com/stritti/revmesh/blob/main/research/mesh2cad-parity.md).

Unterschiede, die erwartet sind, und warum:

- **Nummerierung.** Die beiden Werkzeuge können Bohrungen in anderer Reihenfolge nummerieren; vergleiche
  die Mengen.
- **L-Winkel.** mesh2cad schreibt ein L mit gleich verrundeten Ecken als `base_x`, `base_y`, `leg_x`,
  `leg_y`, `fillet_r` und `base_hole_N_*` / `leg_hole_N_*`; revmesh schreibt es wie jede andere Kontur
  (`extrude_len`, `outline_fillet_r`, `hole_N_*`).
- **Die Kontur selbst.** mesh2cads `outline` ist eine Punktliste; SolidCraft hat keine Listen-Parameter,
  daher liegt revmeshs Kontur in ihrer Skizze, wo jede Seite bemaßt werden kann.

## Fixtures

Netz-Fixtures (STL, OBJ, PLY) werden generiert, nie von Hand bearbeitet: `crates/revmesh-core/examples/plate_stl.rs`
schreibt sie aus einem Spec (`synth` tesselliert so, wie CAD-Exporteure es tun) und gibt das Volumen aus,
das SolidCraft nach dem Neuaufbau messen soll. Jede Fixture in `plugin/revmesh/tests` hat eine
`.json`-Case, die `cargo xtask plugin-test` in SolidCraft ausführt. Die OBJ/PLY-Fixtures sind der
STL-Block im jeweiligen Format (`block-obj`/`block-ply`/`block-ply-bin`).

Die absichtlich kaputten Fixtures (`broken.stl`, `empty.stl`, `nan.stl`, `truncated.stl`,
`ascii-bad.stl`, `obj-bad.obj`, `ply-bad.ply`) kommen aus den Raw-Modi desselben Beispiels (in dessen Header aufgelistet). Ihre
`.json`-Cases erwarten einen Fehler — `{"error": "invalid_input", "message": "…"}` mit einem
Ausschnitt des lokalisierten Grundes, wie `broken.json` zeigt.

Echte STL-Dateien von Printables, unter CC0 oder CC BY und unverändert verwendet, liegen in
`plugin/revmesh/tests/printables/` (Autoren und Lizenzen in dessen README);
`crates/revmesh-core/tests/printables.rs` prüft sie gegen die veröffentlichten Quellen der Autoren, wo es
sie gibt, und die `printables-*.json`-Cases importieren jede in SolidCraft. Für jede Netzdatei endet
`cargo run -q -p revmesh-core --example params -- part.obj` mit der Feature-Anzahl und dem Volumen, das
eine Plugin-Test-Case erwartet.

Die Beispielteile in `examples/` (in der [Beispiel-Galerie](/de/guide/examples) der Doku gezeigt) werden
genauso erzeugt, von `crates/revmesh-core/examples/examples_stl.rs`; `crates/revmesh-core/tests/examples.rs`
prüft, dass jede so importiert, wie die Galerie es beschreibt. Die Bilder der Galerie in
`docs/public/examples/` kommen von `scripts/render-stl.py` (numpy und Pillow): regeneriere beides nach dem
Ändern eines Beispiels oder einer Fixture, die es zeigt:

```sh
cargo run -q -p revmesh-core --example examples_stl -- examples
python3 scripts/render-stl.py --out docs/public/examples examples/*.stl \
  plugin/revmesh/tests/{stepped-holes,blind-holes,hole-rims,slots,boss-fillet,boss-draft-chamfer,pockets,cutouts,combined,combined-3,combined-windows,combined-cuts}.stl
```
