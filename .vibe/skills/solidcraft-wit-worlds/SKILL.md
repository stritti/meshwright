---
name: solidcraft-wit-worlds
description: >-
  Work with SolidCraft's plugin interface (craft:plugin@0.2.0, vendored at crates/revmesh-plugin/wit/plugin.wit): the worlds (command-plugin, importer-plugin, exporter-plugin, dialog variants), their imported/exported interfaces, manifest capabilities, the plugin-error variants and WIT versioning. Use when choosing a world, reading or updating plugin.wit, adding a capability, or mapping errors to plugin-error.
license: MIT
---

# The craft:plugin WIT interface

The interface is vendored at `crates/revmesh-plugin/wit/plugin.wit` (package `craft:plugin@0.2.0`,
from SolidCraft 0.49.1 — the release pinned in `SOLIDCRAFT_VERSION` in `.github/workflows/ci.yml`).
SolidCraft links no WASI, so the plugin is a **component** whose imports/exports are these worlds.

## Plugin-interface conformance — always check this

This skill touches SolidCraft's plugin boundary, so before committing any change here, check it
against the host's own documentation: `docs/plugins.md` in
[tangency-labs/solidcraft](https://github.com/tangency-labs/solidcraft) at the pinned release
[tag `v0.49.1`](https://github.com/tangency-labs/solidcraft/blob/v0.49.1/docs/plugins.md)
(`SOLIDCRAFT_VERSION` in `.github/workflows/ci.yml`, matching the vendored
`crates/revmesh-plugin/wit/plugin.wit`) and at
[`main`](https://github.com/tangency-labs/solidcraft/blob/main/docs/plugins.md) for what is
coming next. Both questions are mandatory:

1. **Compatible?** The host at `SOLIDCRAFT_VERSION` accepts what you declare and build: known
   capabilities only (unknown or newer required capabilities are refused at install, and
   `min_host_version` is checked), entry fields valid for that version, limits under the host
   ceilings (fuel ≤ 10,000,000,000,000 approved, memory ≤ the 128 MiB ceiling unless the user
   approves more, deadline ≤ 10 min, zip ≤ 256 entries and ≤ 16 MiB per file, importer component
   ≤ 64 MiB), and a component whose imports the declared capabilities cover. Prove it with
   `plugin.check`, a dry-run `plugin.install` and `cargo xtask plugin-test`.
2. **Fully utilised?** Walk the plugins.md feature list and for each feature that applies to this
   plugin either use it or record why not: capabilities beyond `formats@0.2` (`evaluate@0.2`
   self-check, `design@0.1` with `get-token`/`expected_design` and the
   `design.changed`/`design.links_changed` listeners, `storage@0.1`, `forms@0.1`, `dialog@0.1`,
   `ui@0.1` panels), `inspect.classify_points` through `commands.run` on `read` entries, manifest
   conveniences (menu/ribbon slots, `x-solidcraft-field-order`, `$defs`, `oneOf` titles,
   `x-solidcraft-visible-when`, importer `file_first`/`dialog_hook`, `updates.github`,
   `min_host_version`), the headless grant flow (`--keep-plugin-approvals`), and the complete
   `plugin-error` mapping.
   When `SOLIDCRAFT_VERSION` moves, re-run both parts against the new tag's plugins.md, diff
   `main` for upcoming interface changes, and state the outcome in the PR.

For this skill:

- Every interface of the chosen world must be coverable by the manifest's `capabilities` — the
  WIT header lists which capability covers which interface (e.g. `importer-plugin` imports
  `evaluate`, which needs `evaluate@0.2` alongside `formats@0.2`); an import the host will not
  satisfy fails at check/install or run time, so verify against `plugin.check` / dry-run output.
- When re-vendoring a newer WIT, diff the old against the new `plugin.wit`, and re-read
  plugins.md at the new tag plus `main` for world and interface changes (while 0.x, a minor may
  break the one before it; the host still runs 0.1 components).

## Worlds

| World | Exports | Imports | Use for |
|---|---|---|---|
| `importer-plugin` | `importer` | `log`, `control`, `clock`, `source`, `design`, `evaluate` | file-format import slots (this repo) |
| `exporter-plugin` | `formats-provider` | `commands`, `log`, `control`, `clock` | file-format export slots |
| `command-plugin` | `entry` | `forms`, `commands`, `design-state`, `log`, `control`, `files`, `clock`, `storage` | app commands / menu entries |
| `dialog-plugin` | `dialog-provider` | `commands`, `log`, `control`, `files`, `clock` | dialogs (additive `dialog@0.1`) |
| `command-dialog-plugin` | `dialog-provider` | `include command-plugin` | commands + dialogs |

## Interfaces

| Interface | Key functions | Notes |
|---|---|---|
| `types` | — | `json` is just `string`; `plugin-error` variant (below); `command-failure`, `field-failure` |
| `importer` | `import-file(entry, file: file-info, options: json) -> result<json, plugin-error>` | the design goes to `design.write`; the return value is the loss report JSON |
| `formats-provider` | `export-file(slot, options) -> result<list<u8>, plugin-error>` | returns file bytes, at most 256 MiB |
| `entry` | `execute(entry, params: json) -> result<json, plugin-error>` | manifest entries called by name |
| `source` | `read(path, offset, len)`, `size(path)`, `resolve(path)`; `file-info { name, size }` | `path ""` is the imported file; any other path needs the `input_folder` permission; reads up to 1 MiB per call |
| `design` | `write(bytes)` | appends the design file, at most 1 MiB per call, in order |
| `evaluate` | `stage(bytes)`, `run(command, args)`, `reset()` | self-check a staged design in a private session; `stage` ≤ 1 MiB, 256 MiB total; allowed commands: `document.inspect`, `inspect.measure`, `model.edges`, `model.faces`, `timeline.roll_to` |
| `control` | `progress(fraction: f32, message)`, `cancelled() -> bool` | call in loops; return `cancelled` when set |
| `log` | `log(level, message)` | `level`: debug, info, warn, error |
| `clock` | `now() -> u64` | milliseconds |
| `files` | `read-file(grant, path)`, `write-file(grant, path, data)` | host files the user granted |
| `storage` | `put-blob(name, data, cache, append)`, `get-blob(plugin, name, offset, len)`, `get-document(plugin, offset, len)` | the plugin's own files inside the open design; reads ≤ 1 MiB |
| `commands` | `run(command, args: json) -> result<json, plugin-error>` | the app's command surface (same as its MCP clients); read-only commands must be listed in the manifest |
| `forms` / `design-state` / `dialog-provider` | `open(entry, params)`, `get-token()`, `update-fields(entry, values)` | additive capabilities `forms@0.1`, `design@0.1`, `dialog@0.1` |

## plugin-error: which variant when

- `invalid-input(field-failure)` — the input is **broken** (unparseable file, bad options JSON):
  name the `field` and the `reason`.
- `unsupported(string)` — input is valid but this plugin does not handle it (no solids, newer
  file version, unconverted feature, option value not offered). Added in 0.2; text in the user's words.
- `trap(string)` — an internal failure (e.g. an `io::Error` while writing the design; it cannot
  cross the boundary, so keep it in a side channel and convert).
- `cancelled` — the user cancelled: check `control::cancelled()` and return this.
- `limit-exceeded(string)` — a size/deadline/fuel limit (e.g. input file over your ceiling).
- `permission-denied`, `command-failed` — propagate host failures from `files`/`commands`.

## Capabilities ↔ imports

The manifest's `capabilities` list names the interface sets the host will satisfy:
`commands@0.2`, `formats@0.2` (import/export slots), `evaluate@0.2` (with `formats`), `storage@0.1`
(with `commands`), `design@0.1`, `forms@0.1`, `dialog@0.1`. The install review shows the slots
before any plugin code runs.

## Versioning

While the package is 0.x, a minor version may break the one before it. The host reads the version
from the component's imports/exports and still runs components built against 0.1 (kept at
`../wit-0.1/plugin.wit` upstream). When `solidcraft-version.yml` re-vendors a newer WIT, update
the `world:` in `wit_bindgen::generate!`, the `capabilities` in `plugin.json`, and
`SOLIDCRAFT_VERSION` in `.github/workflows/ci.yml` in one PR.

## Binding with wit-bindgen

```rust
wit_bindgen::generate!({ path: "wit", world: "importer-plugin" });
// Guest trait: exports::craft::plugin::importer::Guest
// Host calls:  craft::plugin::{source, design, control, log, ...}
// Error type:  craft::plugin::types::PluginError
export!(MyPlugin);
```

Everything crossing the boundary is owned (`String`, `Vec<u8>`/`list<u8>`); no lifetimes, no
borrows. `json` values are strings — parse them yourself and report malformed ones as
`invalid-input`.

## References in this repo

- `crates/revmesh-plugin/wit/plugin.wit` (the vendored interface, heavily commented)
- `crates/revmesh-plugin/src/lib.rs` (bindings + host-call usage)
- `plugin/revmesh/plugin.json` (capabilities, entries, limits)
- `docs/architecture.md` ("Limits inside SolidCraft")
