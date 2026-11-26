---
layout: home

hero:
  name: revmesh
  text: STL rein, bearbeitbare Konstruktion raus
  tagline: Ein SolidCraft-Plugin, das aus einer STL eine Konstruktion macht, die du weiterbearbeiten kannst — jede Größe ein benannter Parameter; ändere ihn, und das Teil wird neu aufgebaut.
  image:
    src: /stl-to-design.svg
    alt: 'Animiert: Aus einer STL-Datei wird eine bearbeitbare SolidCraft-Konstruktion mit einem Parameter für jede Größe'
  actions:
    - theme: brand
      text: Installation
      link: /de/guide/install
    - theme: alt
      text: Beispiele
      link: /de/guide/examples
    - theme: alt
      text: Anwendung
      link: /de/guide/use
    - theme: alt
      text: GitHub
      link: https://github.com/stritti/revmesh

features:
  - title: Bearbeitbar, kein totes Netz
    details: Die Platte, ihre Bohrungen und ihre verrundeten Ecken kommen als benannte Parameter wie base_x oder hole_1_d an. Ändere einen im Parameter-Panel von SolidCraft, und das Teil wird neu aufgebaut — du machst dort weiter, wo die STL aufgehört hat.
  - title: Erkennt, woraus das Teil besteht
    details: Platten mit beliebiger Kontur, runde Bohrungen, Langlöcher, Zapfen, Taschen und Verrundungen werden aus dem Netz herausgearbeitet und als bearbeitbare Operationen neu aufgebaut statt als Dreiecke.
  - title: Nichts zusätzlich zu installieren
    details: Das Plugin läuft in SolidCraft selbst — kein Python, numpy oder OpenCascade auf deinem Rechner.
  - title: Ehrlich, wenn es etwas nicht kann
    details: Jeder Import endet mit einem Bericht. Was nicht erkannt wird, bleibt ein Mesh-Body, und der Bericht sagt warum — kein stiller Verlust.
---

::: warning Status: frühe Phase
Die Pipeline läuft Ende-zu-Ende — STL rein, bearbeitbare Konstruktion raus, getestet in SolidCraft 0.49.
Teile, die noch nicht erkannt werden, bleiben ein Mesh-Body, mit dem Grund im Importbericht. Was genau
erkannt wird, steht unter [Was auto erkennt](/de/guide/recognition); was geplant ist, auf der
[Roadmap](/de/guide/roadmap).
:::
