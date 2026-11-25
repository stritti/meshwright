# AGENTS.md

Orientation for agents working on revmesh. Keep this file short: point to the details, don't
duplicate them.

## Agent skills

Eight [Agent Skills](https://agentskills.io) for professional WebAssembly-in-Rust development of
SolidCraft plugins are mirrored into the four locations skills-compatible agents scan:
`.agents/skills/`, `.github/skills/`, `.claude/skills/`, `.vibe/skills/`. Edit `.agents/skills/`
(the canonical copy) and copy the change to the other three. Overview table: the
[README](README.md#agent-skills) "Agent skills" section. The skills cover scaffolding a plugin,
the `craft:plugin` WIT worlds, the wasm build/packaging pipeline, runtime limits and streaming,
host calls and error mapping, the importer workflow, testing, and the recognition know-how
extracted from the literature (`revmesh-recognition-knowhow`).

Every SolidCraft skill opens with the same mandatory conformance gate: before any change that
touches the plugin boundary, check it against SolidCraft's `docs/plugins.md` — [at the pinned tag
`v0.49.1`](https://github.com/tangency-labs/solidcraft/blob/v0.49.1/docs/plugins.md) and [at
`main`](https://github.com/tangency-labs/solidcraft/blob/main/docs/plugins.md) — for compatibility
with the host and for full utilization of the plugin interface.

## Conventions

- Read [CONTRIBUTING.md](CONTRIBUTING.md) first: development setup, the CI gates to run locally
  before a PR, Conventional Commits for commit messages and PR titles, and the contribution
  licence.
- Architecture and rules of thumb (thin plugin crate, mesh fallback, exact-or-nothing
  recognition, limits): [docs/architecture.md](docs/architecture.md).
- Testing levels, fixtures and the mesh2cad comparison: [docs/development.md](docs/development.md).
- The SolidCraft design JSON subset the importer writes: [docs/design-format.md](docs/design-format.md).

## Commands

```sh
cargo test                                                            # core unit tests, native
cargo clippy -p revmesh-plugin --target wasm32-unknown-unknown -- -D warnings
cargo xtask component                                                 # build the wasm component
cargo xtask dist                                                      # installable zip in target/dist/
cargo xtask plugin-test                                               # end-to-end in SolidCraft (set SOLIDCRAFT_CLI)
cargo llvm-cov --workspace --fail-under-lines 80                      # coverage gate >= 80% lines, whole workspace (needs llvm-tools-preview + cargo-llvm-cov)
```

## Hard rules

- Keep the plugin crate thin: all logic lives in `revmesh-core` and is tested natively; the
  plugin crate only moves bytes between SolidCraft and the core.
- No `unsafe` outside the generated `wit_bindgen` bindings (workspace lint denies it); no
  panics/`unwrap` in plugin code — panics abort the wasm component.
- Inside SolidCraft's sandbox there is no WASI, no threads, no network, no system randomness:
  stream host I/O in ≤ 1 MiB chunks (`source::read` / `design::write`), report progress, honour
  `control::cancelled()`, stay deterministic (fixed seeds).
- Never hand-edit golden files (`crates/revmesh-core/tests/data/*.json`), STL fixtures
  (`plugin/revmesh/tests/*.stl`) or example parts (`examples/*.stl`); regenerate them with the
  documented commands.
- Coverage gate: at least 80% line coverage across the whole workspace, no source excluded,
  enforced by the CI `coverage` job (`cargo llvm-cov --workspace --fail-under-lines 80`,
  required via `ci-ok`) — new code comes with tests that keep it above the gate.
