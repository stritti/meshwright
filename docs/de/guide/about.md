# Herkunft und Lizenz

## Woher die Erkennung kommt

Die Erkennung folgt [mesh2cad](https://github.com/emirmrkaljevic/mesh2cad) (MIT): ihrem Ansatz,
den Teilen, auf die sie zielt, und ihren Parameternamen. mesh2cad ist in Python auf numpy, scipy,
trimesh und OpenCascade geschrieben, und nichts davon läuft in SolidCrafts WebAssembly-Plugin-Sandbox
(kein WASI, kein Subprozess, kein Netzwerk). revmesh portiert die Erkennung daher nach Rust und
schreibt statt eines B-rep SolidCrafts eigene Feature-Historie, die SolidCraft dann neu aufbaut. Jeder
Erkenner wird vor dem Merge mit mesh2cads Ausgabe verglichen.

## Wie es funktioniert

```text
 STL-Bytes ──► stl::parse ──► Mesh ──► recognize ──► PlateSpec … ──► Design (JSON) ──► SolidCraft
                                            │                               ▲
                                            └── nichts gefunden ──► Mesh-Body ┘   + Verlustbericht
```

| Crate | Was es tut |
|---|---|
| [`revmesh-core`](https://github.com/stritti/revmesh/tree/main/crates/revmesh-core) | STL-Parsing, Erkennung und das SolidCraft-Dokumentenmodell. Reines Rust, ohne Host-Aufrufe, nativ getestet mit `cargo test`. |
| [`revmesh-plugin`](https://github.com/stritti/revmesh/tree/main/crates/revmesh-plugin) | Die WebAssembly-Komponente (`craft:plugin@0.2.0`, Welt `importer-plugin`). Liest die Datei aus SolidCraft, ruft den Core auf, schreibt die Konstruktion zurück. |
| [`xtask`](https://github.com/stritti/revmesh/tree/main/xtask) | Baut die Komponente und die installierbare zip, und führt SolidCrafts Plugin-Tests aus. |
| [`plugin/revmesh`](https://github.com/stritti/revmesh/tree/main/plugin/revmesh) | Das Plugin-Paket: `plugin.json`, die `icons/`-Grafik, die SolidCraft im Plugins-Menü zeigt, und die Testfälle, die SolidCraft ausführt. |

Mehr in [Architektur](/de/architecture) und [Design-Format](/de/design-format).

## Lizenz

revmesh ist unter der [MIT-Lizenz](https://github.com/stritti/revmesh/blob/main/LICENSE) lizenziert.
Code, der aus mesh2cad portiert wurde, behält seinen ursprünglichen MIT-Copyright-Hinweis; siehe
[NOTICE](https://github.com/stritti/revmesh/blob/main/NOTICE). SolidCrafts Plugin-Schnittstelle
(`crates/revmesh-plugin/wit/plugin.wit`) steht unter © der SolidCraft-Autoren, Apache-2.0.

revmesh ist ein unabhängiges Projekt und nicht mit SolidCraft oder mesh2cad verbunden.
