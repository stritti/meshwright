---
name: roadmap-maintenance
description: >-
  Keep the roadmap evaluated and up to date at every occasion it changes:
  docs/guide/roadmap.md (EN) and docs/de/guide/roadmap.md (DE) stay in sync, done items carry the
  release they shipped in (`vX.Y.Z`, or "unreleased" until the next `chore: release` lands), and
  the open items are listed in priority order — update all of it whenever a change lands on a
  roadmap item (reference `roadmap X.Y` in the commit message, as the git history does), when a
  release PR merges, and before picking the next item to work on; also sync the README "Roadmap"
  summary. Use before merging any PR that implements, extends or invalidates a roadmap item, and
  on every `chore: release` merge.
license: MIT
---

# Roadmap maintenance

The roadmap is the project's plan of record: `docs/guide/roadmap.md` (canonical, English) and
`docs/de/guide/roadmap.md` (German mirror — same structure, same facts), summarised in
`README.md` → "Roadmap". `CHANGELOG.md` and the version files are release-please's — never
hand-edit them, only read release tags from them.

## When to run this skill (every occasion)

1. **A change lands on a roadmap item.** Commit messages reference the item (`(roadmap 3.12)` —
   see the git history). In the same pull request, update the roadmap: mark the item (or
   sub-item) done, keeping the existing scope note.
2. **A release is cut.** When a `chore: release x.y.z` PR merges, stamp every `unreleased` done
   item with the new tag `vX.Y.Z` (read the tag from `CHANGELOG.md` / `.release-please-manifest.json`,
   never guess it), and re-check the prioritised open list.
3. **Picking the next item to work on.** Read "Open points, prioritised" in
   `docs/guide/roadmap.md` first, and keep that priority order honest — one line of rationale per
   open point.
4. **Docs drift.** When the README "Roadmap" summary's done/open statements no longer match the
   full roadmap, sync it (keep it short — it links to the roadmap for details).

## How to update

- Done items carry their release inline: `(done in v0.1.0-beta.2)`, a range when the item spread
  over releases (`(done in v0.1.0-beta.6–v0.2.0)`), and `(done, unreleased)` for work merged after
  the latest release — re-stamped when the next release is cut. Done items are never deleted or
  renumbered: the numbered structure is stable so commits, issues and docs can reference
  `roadmap X.Y`.
- Open items live in the "Open points, prioritised" section at the top, in priority order with a
  one-line rationale each; sub-remainders inside a partly done item (e.g. 5.2's "round outlines,
  D-shaped cut-outs … stay a mesh") are referenced from there.
- Keep EN and DE identical in structure and facts: same items, same done/unreleased marks, same
  release tags, same priority order — only the language differs.
- Release tags come from the `chore: release` commits / `CHANGELOG.md`; while pre-1.0 with `feat`
  commits since the last tag the next one is the minor bump (today: v0.2.0 → v0.3.0) — confirm
  against `release-please-config.json` and the manifest, don't assume.

## Boundary note

Roadmap and README changes are docs-only and never touch the plugin boundary, so the SolidCraft
`docs/plugins.md` conformance gate of the `solidcraft-*` skills does not apply to this skill; if
a roadmap-driven change does touch the plugin crate, that change itself goes through the gate.
