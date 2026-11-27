# Installation

1. Lade `revmesh-<version>.zip` aus den [Releases](https://github.com/stritti/revmesh/releases)
   herunter (es gibt auch `revmesh-<version>-de.zip` mit deutschen Plugin-Texten — beide sind
   dasselbe Plugin mit derselben ID).
2. Öffne in SolidCraft **Preferences › Plugins › Install plugin…** (auch *File › Plugins…*) und
   wähle die zip-Datei.

revmesh ist außerdem in der Community-[Liste der SolidCraft-Plugins](https://tharje.github.io/solidcraft-plugins/)
geführt, die die neueste zip als Download-Button anbietet.

Per Skript (`solidcraft-cli`) zuerst das Paket prüfen und mit dem ausgegebenen Approval-Hash
installieren — SolidCraft führt Plugin-Code nur für genau das geprüfte Paket aus:

```sh
solidcraft-cli exec plugin.check '{"path": "revmesh-<version>.zip"}'  # gibt "approval": "…" aus
solidcraft-cli exec plugin.install '{"path": "revmesh-<version>.zip", "approval": "…", "approve": {}}'
```

[examples/cli/](https://github.com/stritti/revmesh/tree/main/examples/cli) zeigt den kompletten
Ablauf headless (check → install → `file.import` → export), und
[scripts/example-cli.sh](https://github.com/stritti/revmesh/tree/main/scripts/example-cli.sh) führt
ihn in einem Kommando aus (CI-Job `example-cli`). Oder entpacke die zip selbst in den
Plugin-Ordner des Nutzers: `~/.local/share/solidcraft/plugins/` (Linux),
`%LOCALAPPDATA%\SolidCraft\plugins\` (Windows), `~/Library/Application Support/SolidCraft/plugins/`
(macOS).

revmesh 0.2.x ist auf SolidCraft 0.50 ausgerichtet: Die Plugin-Schnittstelle ist unverändert aus
SolidCraft v0.50.0 übernommen und CI testet gegen `solidcraft-cli` 0.50.0 (siehe
[Kompatibilität](https://github.com/stritti/revmesh#compatibility) im README). Die
Plugin-Unterstützung von SolidCraft ist experimentell, daher kann eine andere SolidCraft-Version
das Paket ablehnen oder ein anderes revmesh brauchen — halte beides aktuell.

Um das Plugin selbst zu bauen, siehe [Entwicklung](/de/development) und das
[README](https://github.com/stritti/revmesh#build-from-source).

Weiter: [Anwendung](/de/guide/use).
