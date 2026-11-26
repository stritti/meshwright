# Roadmap

1. **Gerüst** (erledigt): Plugin-Pipeline, Dokument-Schreiber, Netz-Fallback, Qualitäts-Gates,
   Releases.
2. **Platten** (erledigt): beliebige Kontur aus geraden Seiten mit runden Durchgangsbohrungen und
   verrundeten vertikalen Ecken.
3. **Parität mit mesh2cad**, ein Erkenner pro Pull Request, jeder mit mesh2cads Ausgabe verglichen
   (`scripts/compare-mesh2cad.sh`, siehe [Entwicklung](/de/development)):
   1. (erledigt) Parameternamen, wie mesh2cad sie schreibt (`base_t`, `fillet_r`, `extrude_len`, …),
      und das Teil bleibt an seiner Stelle in der Datei
   2. (erledigt) Bohrungsreihen, -gitter und -lochkreise als SolidCraft-Muster
   3. (erledigt) Bohrungen mit Senkung und mit Kegelsenkung
   4. (erledigt) Sacklochbohrungen
   5. (erledigt) Langlöcher
   6. (erledigt) Zapfen auf der Oberseite
   7. (erledigt) polygonale Ausschnitte
   8. (erledigt) Aufsätze und rechteckige Taschen (Stufen und andere Taschen später)
   9. (erledigt) Extrusionen entlang X und Y
   10. (erledigt) Boxen, deren Kanten alle verrundet sind (`edge_fillet_r`), mit Durchgangsbohrungen
       darin; Zapfen an Seitenflächen noch offen
   11. (erledigt) Fasen, Formschrägen und die Verrundungen an einem Zapfenfuß oder einem Bohrungsrand
   12. kombinierte Extrusionen: (erledigt) der Schnitt von Konturen, die entlang X, Y und Z extrudiert
       werden, mit Öffnungen durch sie, geraden Sackloch-Schnitten hinein und runden Bohrungen, und
       (erledigt) Vereinigungen von Konturen, die entlang einer Achse gestapelt sind; sich kreuzende
       Bohrungen, Dächer und Fasen auf einem Stapel noch offen
4. **Prüfung**: das neu aufgebaute Teil mit dem Netz durch SolidCrafts `evaluate` vergleichen (Volumen
   und Abweichung), wie mesh2cad es mit CadQuery tut.
5. **Nacharbeitbare Skizzen** — jenseits der Parität: jede Skizze vollständig bemaßt und bestimmt, sodass
   sie in SolidCraft nachgearbeitet werden kann. Die Konzepte dahinter und ihre Quellen (Produkte und
   Paper) sind unter [Vergleichbare Werkzeuge → Konzepte aus der Liste, auf revmesh angewendet](/de/guide/comparison#concepts-from-the-list-applied-to-revmesh)
   kartiert:
   1. (erledigt für Plattenkonturen und Ausschnitte sowie für die Skizzen von Stapel und Schnitt: eine
      Kontur des Stapel-Bands `k` als `outline_<k>_<i>_len`/`_angle`, eine kombinierte Extrusion entlang
      Achse `a` als `outline_<a>_<i>_len`/`_angle`, ihre Öffnungen als `cutout_<n>_<i>_len`/`_angle` und
      die Kontur eines kombinierten Schnitts als `cut_<n>_<i>_len`/`_angle`) eine
      `Length`-Bedingung pro Seite mit eigenem Parameter (`outline_<i>_len`,
      `cutout_<n>_<i>_len`) und eine `Angle` von der X-Achse der Skizze an der längsten schrägen Seite
      mit eigenem Parameter (`outline_<i>_angle`, `cutout_<n>_<i>_angle`) in
      `polygon_sketch_dimensioned`/`polygons_sketch_dimensioned`
      (`crates/revmesh-core/src/design/sketch.rs`; parallele schräge Seiten teilen ihren Winkel und die
      Knicke eines Rings summieren sich zu 360°, daher ist ein Winkel pro Polygon der unabhängige — und
      die Länge dieser Seite ist dann abgeleitet, beide zu treiben würde die Skizze überbestimmen;
      ebenso, wenn die übrigen Längen und Orientierungen die Richtung dieser Seite bereits festlegen,
      wie die Katheten eines rechtwinkligen Dreiecks, horizontal und vertikal gehalten, es für die
      Diagonale tun, ist auch die Länge der längsten achsparallelen Seite abgeleitet
      (`dimensioned_sides_for_angle`), sodass der Winkel-Parameter bearbeitbar bleibt)
   2. (erledigt für Konturen und Ausschnitte) Bogen-Seiten in Konturen und Ausschnitten als
      `SketchCurve::Arc` mit einer `Diameter`-Bedingung, getrieben von `arc_<i>_d` (einer Kontur) bzw.
      `cutout_<n>_arc_<i>_d` (eines Ausschnitts), der Mittelpunkt ein freier Skizzenpunkt, den der
      Löser im Abstand des treibenden Durchmessers von den beiden Ecken der Seite hält (wie die
      Bogen-Mittelpunkte eines Langlochs)
      (`polygon_sketch_arcs`, `crates/revmesh-core/src/design/sketch.rs`; erkannt als
      `ProfileSpec::arcs` in `plate/outline.rs::outline_shape` und als `CutoutSpec::arcs` in
      `plate/holes.rs::cutout_shape`, das einen Ausschnitt-Ring wie eine Kontur liest und ihn nur mit
      Bogen-Seiten und ohne verrundete Ecken annimmt) — runde Konturen, D-förmige Ausschnitte,
      Ausschnitte mit verrundeten Ecken und konkave Bogen-Seiten bleiben ein Netz
   3. (erledigt für Taschen beliebiger geradliniger Kontur, für Stufen von der Oberseite mehrere
      Ebenen tief entlang Z, für gesenkte Bohrungen, die auf der Ebene einer Stufe öffnen — von der
      Ebene gemessen und von ihr gebohrt, eine verrundete Kante auf der Startkante der Tasche verrundet
      — und für Stufen entlang X oder Y, die der Stapel-Erkenner als entlang dieser Achse gestapelte
      Konturen liest) eine Skizze pro Flächenebene: `split_faces` über Unter- und Oberseite hinaus
      verallgemeinern — beliebige Taschen und Stufen, beide als `Pocket<n>Sketch` +
      `Pocket<n>`-Schnitt mit `pocket_<n>_<i>_len` pro Seite geschrieben
      (`recognize/to_design.rs::block_features`; die Ebenen in `plate/prism.rs::step_levels`,
      jede Oberseiten-Schleife in `face_outlines` mit ihrer Ebene versehen, Köpfe von ihrer Mund-Fläche
      aus gemessen in `stepped_hole`), und mit 3.12 kombinierten Extrusionen
   4. (erledigt für jede erkannte Art) Oberflächenabweichung pro Fläche im Importbericht, in der Sandbox
      beim Import berechnet: jede erkannte Fläche mit ihrem Label und der weitesten Entfernung ihrer
      Netz-Punkte von der idealen Fläche (`recognize::deviations`: `plate::face_deviations` für Prismen,
      eine rundum verrundete Box meldet ihre verrundete Hülle als "rounded surface",
      `plate::walls_deviations` pro Extrusion und pro Band bei kombinierten und gestapelten Teilen,
      beschriftet `extrude <a> side <i>` / `band <k> side <i>`; `Walls::deviation`; das Volumen
      innerhalb von 1 % bleibt das Gate, Hausdorff in `cargo xtask check`)
6. **Mehr Teile, robust eingepasst** — aus dem Code-Review der Erkenner gegen die Referenzen (das
   extrahierte Know-how ist im Agent-Skill `revmesh-recognition-knowhow` paketiert; die Konzept → Code
   Kartierung mit den Quellen ist [Vergleichbare Werkzeuge → Konzepte aus der Liste, auf revmesh
   angewendet](/de/guide/comparison#concepts-from-the-list-applied-to-revmesh)):
   1. eine beliebige Prismen-Achse: die zwei dominanten parallelen Ebenen mit fixed-seed RANSAC
      erkennen (Schnabel et al. 2007) und das Netz so drehen, dass ihre Normale +Z ist, statt nur X, Y
      und Z zu probieren (`recognize/axis.rs`) — heute bleibt ein in der Datei gedrehtes Teil ein Netz.
      Nur auf Erkennungsseite zunächst: SolidCraft 0.49.1s Design-Format hat Skizzen auf
      Ursprungsebenen und translate-only-Verschiebungen, daher kann ein schräges Prisma in der
      gepinnten Version nicht parametrisch geschrieben werden (braucht Konstruktionsebenen oder einen
      neueren Pin)
   2. (erledigt: ein fixed-seed RANSAC-Kreis in `plate/geometry.rs::ransac_circle`, der den algebraischen
      `fit_circle` ergänzt — 3-Punkt-Minimalproben, Inlier innerhalb der Toleranz, die Iterationszahl an
      das Inlier-Verhältnis angepasst — hinter `round_hole` in `plate/holes.rs`: ein Rand-Ring mit
      wenigen Ecken neben dem Kreis wird auf seinen Inliern eingepasst, wenn mindestens
      `MIN_RING_SEGMENTS` (6) von ihnen eine strikte Mehrheit des Rings sind, ihre Schritte gleichmäßig
      bleiben und die ausgeschlossenen Ecken innerhalb einer Facetten-Pfeilhöhe des Kreises liegen,
      während die eigenen Kanten des Rings das ursprüngliche 50°-Gate behalten, sodass ein ungleichmäßiger
      Exporteur-Rand als runde Bohrung gelesen wird, eine echte Fläche oder Lappung aber abgelehnt
      bleibt: in den eigenen Schleifen einer Fläche werden weniger als `MIN_HOLE_SEGMENTS` (8) Ecken
      zuerst als polygonaler Ausschnitt gelesen, daher bleibt ein Sechseck dort ein Ausschnitt, und die
      flache Seite eines D und der lange Schritt einer ausgelassenen Facette sind weiter ein langer
      Schritt) robuste Kreis-Einpassungen auf Rand-Ringen
   3. (erledigt: ein Torus um die bekannte Achse ist ein Kreis in der (radialen, axialen)
      Profilebene, daher passt `plate/geometry.rs::fit_torus` ihn als einen an — kleinstes-Quadrate-Kreis
      durch die Ecken des Verrundungs-Bands, abgelehnt, wenn sie auf keinem Kreis liegen — in
      `rounded_rim` für Bohrungsränder (`plate/prism.rs`, mit Tangente an die Bohrungswand und an die
      Fläche geprüft) und in `round_foot` für Zapfenfüße (`plate/bosses.rs`), statt des
      Mittelwerts pro Ecke) analytische Torus-Erkennung für die Verrundungen an Bohrungsrändern und
      Zapfenfüßen
   4. pro-Ecken-Radien in Konturen (`plate/outline.rs` passt heute schon einen Kreis pro verrundeter
      Ecke mit `CircleSums` an; heute müssen alle Ecken einen Radius teilen)
   5. Bohrungsreihen in beliebigem Winkel (die Richtung eines Musters ist in SolidCraft kein Ausdruck —
      als einzelne Bohrungen oder auf einer gedrehten Skizze schreiben)
   6. ComplexGen-artige Topologie-Gates neben exact-or-nothing und der Volumen-Prüfung: erkannte
      Flächen schließen zu Schleifen, Wände treffen ihre Fläche rechtwinklig
   plus was in 3.10 und 3.12 noch offen ist: Zapfen an Seitenflächen, Dächer und Fasen auf Stapeln
7. **Import und Konvertierung**:
   1. (erledigt: das Manifest fordert `evaluate@0.2`; im auto-Modus wird die serialisierte Konstruktion
      in der privaten Sandbox der App gestaget und dort neu aufgebaut (`document.inspect`), bevor sie
      geschrieben wird, und eine Konstruktion, die die App nicht öffnen kann, fällt mit dem Grund im
      Verlustbericht auf den Mesh-Body zurück, Code `revmesh.selfcheck`; mesh-Modus-Importe werden nicht
      geprüft — `self_check` in `crates/revmesh-plugin/src/lib.rs`, schließt Punkt 4 zum Importzeitpunkt)
      eine Selbstprüfung in der App-Sandbox: die Capability `evaluate@0.2` anfordern, die erzeugte
      Konstruktion stagen und neu aufbauen, und bei Feature-Fehlern mit dem Grund im Bericht auf den
      Mesh-Body zurückfallen
   2. OBJ- und PLY-Eingabe neben STL, mit demselben chunked Streaming und denselben Größenlimits wie
      `StlReader` (`plugin.json` akzeptiert heute nur `stl`)
   3. doppeltgenaue Koordinaten in Mesh-Bodies jenseits des einfachgenauen Bereichs (die Konstruktion
      wird heute mit `f32`-Koordinaten geschrieben, `design/mod.rs`) — Teile weit vom Ursprung verlieren
      im Netz-Fallback an Präzision
   4. eine höhere `auto`-Eingabegrenze (heute 8 MiB gegen 32 MiB im mesh-Modus, `revmesh-plugin`), indem
      die Indexe, die die Erkennung über Flächen und Wände baut, schlanker werden
   5. lokalisierte ASCII-STL-Syntaxgründe (sie bleiben heute Englisch, `stl.rs`) und eine optionale
      `units`-Importoption (STL ist einheitenlos und wird als mm angenommen)

Beiträge sind sehr willkommen: Bug-Reports, STL-Dateien, die schlecht importieren, Erkenner, Doku.
Fang an mit [CONTRIBUTING.md](https://github.com/stritti/revmesh/blob/main/CONTRIBUTING.md).
