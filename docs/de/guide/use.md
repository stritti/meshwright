# Anwendung

1. Klicke auf der Ribbon-Registerkarte **Model** auf die Beschriftung der Gruppe **Plugins**, um das
   Plugins-Menü zu öffnen, und wähle **revmesh › stl**. SolidCrafts eigenes *File › Open* nutzt für
   `.stl` weiterhin den eingebauten Netz-Import, daher erreicht man revmesh über das Plugins-Menü.
2. Der Dialog bietet **Importieren als**: `auto` (Features erkennen und das Teil parametrisch neu
   aufbauen — Facetten werden zu idealen Flächen, der Importbericht nennt die Abweichung; das Netz
   bleibt nur dort, wo nichts erkannt wird; Datei bis 16 MiB; die Standardeinstellung) oder `mesh` (das
   Netz exakt so übernehmen, wie es ist; Datei bis 32 MiB).
3. Seine Schaltfläche wählt die Netzdatei (`.stl`, `.obj` oder `.ply`). Sie öffnet sich als neue,
   ungespeicherte Konstruktion.
4. Danach folgt der **Importbericht**: eine Zusammenfassung, eine Zeile pro Art der Änderung und
   alles im Detail unter *Details*. **Plugins › Last import report…** zeigt ihn später erneut an.

Ein erkanntes Teil kommt als Timeline (Skizze, Extrusion, Bohrungen, Verrundung) mit seinen Größen
als Parametern im Parameter-Panel an: Ändere `base_t` oder `hole_1_d`, und SolidCraft baut es neu.
Der Bericht nennt auch die Oberflächenabweichung pro erkannter Fläche — wie weit die Facetten des
Netzes von den idealen Flächen entfernt liegen (bei einer Bohrung wölben sich deren Facetten um ihre
Pfeilhöhe nach innen in den wahren Kreis hinein; eine rundum verrundete Box meldet ihre verrundete
Oberfläche; ein kombiniertes oder gestapftes Teil meldet seine Wände pro Extrusion oder Band,
beschriftet `extrude z side 1`, `band 2 side 3`). Ein Teil, das nicht erkannt wird, kommt als
Mesh-Body an, und der Bericht sagt warum.

::: tip Probiere es aus
Importiere
[`plugin/revmesh/tests/plate-holes.stl`](https://github.com/stritti/revmesh/blob/main/plugin/revmesh/tests/plate-holes.stl)
aus dem Repository oder eines der [Beispiele](/de/guide/examples): alltägliche Teile mit Bildern und
den Parametern, die jedes einzelne bekommt.
:::

Welche Teile erkannt werden und welche Parameter sie bekommen, steht unter
[Was auto erkennt](/de/guide/recognition).

## Grenzen

Netz-Eingabe (STL, OBJ, PLY): bis 16 MiB im Modus `auto`, 32 MiB im Modus `mesh` (`AUTO_MAX_BYTES` /
`MESH_MAX_BYTES` in `crates/revmesh-plugin/src/lib.rs`). Laufzeit: 128 MiB Speicher, eine Deadline
von 120 s und 10¹¹ Fuel, wie in `plugin/revmesh/plugin.json` angefordert. Die Datei wird in
1-MiB-Chunks gestreamt, daher erreicht eine 16-MiB-Datei etwa 90 MiB Spitzenwert im Speicher
(binäres STL; OBJ und PLY halten gemeinsame Vertices, dehnen sich also weniger aus).
Gemessene Werte und die Begründung dahinter: [Architektur](/de/architecture#limits-in-solidcraft).
