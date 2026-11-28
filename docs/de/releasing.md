# Release-Prozess

Releases werden von [release-please](https://github.com/googleapis/release-please) aus den
Conventional-Commit-Nachrichten auf `main` erstellt:

1. Jeder Push auf `main` aktualisiert (oder eröffnet) den Release-Pull-Request `chore: release x.y.z`. Er
   erhöht die Version in `Cargo.toml`, `plugin/revmesh/plugin.json` und `CHANGELOG.md`; ein zweiter Job
   bringt `Cargo.lock` in Schritt.
2. Das Mergen dieses Pull Requests taggt `vx.y.z` und erstellt den GitHub-Release.
3. Der `publish-plugin`-Job baut das Plugin und hängt `revmesh-x.y.z.zip` (Englisch),
   `revmesh-x.y.z-de.zip` (Deutsch) und `SHA256SUMS.txt` an den Release an.

Das SolidCraft-Workshop-Listing (`solidcraft-workshop.json` im Root) wird aus diesem Repository
veröffentlicht: `version` und der Package-`asset`-Name müssen bei jedem Release mitgezogen werden,
sodass das Listing immer auf das neueste Release-Zip zeigt.

## Einmalige Einrichtung: das Release-Token

Pull Requests und Commits, die mit dem eigenen `GITHUB_TOKEN` eines Workflows erstellt werden, starten
keine anderen Workflows. Mit diesem Token bekäme der Release-Pull-Request nie seine CI-Prüfungen, und mit
der erforderlichen Prüfung „CI ok" könnte er nie gemerged werden. release-please nutzt daher ein eigenes
Token:

1. Erstelle ein **fine-grained Personal Access Token** (GitHub › Settings › Developer settings ›
   Personal access tokens › Fine-grained tokens):
   - Repository-Zugriff: nur dieses Repository.
   - Berechtigungen: **Contents** lesen und schreiben, **Pull requests** lesen und schreiben.
   - Laufzeit: so lange, wie du damit leben kannst; verlängere es, bevor es abläuft.
2. Füge es dem Repository als Actions-Secret **`RELEASE_PLEASE_TOKEN** hinzu (Settings › Secrets and
   variables › Actions).

Ohne das Secret stoppt der Release-Workflow mit einem Fehler, der hierher zeigt.

Ein Token einer GitHub-App-Installation geht ebenfalls und ist nicht an eine Person gebunden; tausche das
Secret gegen eines, das mit `actions/create-github-app-token` erstellt wurde, wenn das Projekt mehr
Maintainer bekommt.

## Branch-Schutz

Schütze `main` und verlange die Statusprüfung **CI ok**, die nur besteht, wenn jedes Qualitäts-Gate in
`.github/workflows/ci.yml` bestanden hat. Verlange Pull Requests und Squash-Merging, damit der PR-Titel
(von `pr-title.yml` geprüft) der Commit wird, den release-please liest.

## Die Beta-Phase verlassen

Solange `release-please-config.json` `"prerelease": true` mit `"prerelease-type": "beta"` setzt, ist
jeder Release ein Pre-Release (`v0.1.0-beta.N`) und GitHub markiert ihn als solchen. Um stabil zu werden,
setze `"prerelease": false` und pinne die erste stabile Version mit `"release-as": "0.1.0"` in derselben
Änderung: Beim nächsten Lauf auf `main` eröffnet release-please dann `chore: release 0.1.0`, und das Mergen
taggt `v0.1.0` als normalen Release. Entferne `release-as` wieder, sobald `v0.1.0` existiert — bliebe es
in der Config, würde es jeden späteren Release auf diese Version pinnen. `Release-As: 0.1.0` als Footer
auf einem einzelnen Commit auf `main` geht auch, beendet die Beta-Phase aber nicht von allein: Solange
`prerelease` true ist, markiert release-please Releases weiter als Pre-Releases.

Beachte, dass release-please einen Release-PR nur eröffnet, wenn seit dem letzten Release mindestens ein
nutzersichtbarer Commit (`feat`, `fix`, `docs`, …) gelandet ist — eine reine `ci`/`chore`-Änderung löst
keinen aus, auch nicht mit `release-as`. Um den stabilen Release zu schneiden, lande daher nach dem
Abschalten von `prerelease` noch eine kleine nutzersichtbare Änderung (etwa eine Doku-Zeile).
