# Contributing to revmesh

Thank you for helping! revmesh is a young project and every contribution counts: a bug
report, an STL file that imports badly, a ported recogniser, a fix in the docs.

## Ways to help

- **Report an import that goes wrong.** Open a [bug report](../../issues/new?template=bug_report.yml)
  and attach the STL (only files you may share) and what you expected to get.
- **Suggest a feature** with the [feature request](../../issues/new?template=feature_request.yml)
  form.
- **Port a recogniser** from [mesh2cad](https://github.com/emirmrkaljevic/mesh2cad). Please open
  an issue first, so two people don't port the same thing. What is already ported, and where we
  deliberately differ: [research/mesh2cad-parity.md](research/mesh2cad-parity.md).
- **Improve the docs.** Small fixes can go straight into a pull request. The site at
  <https://stritti.github.io/revmesh/> is built from `docs/` with VitePress: `cd docs && npm ci && npm run dev`
  previews it.

## Development setup

```sh
git clone https://github.com/stritti/revmesh
cd revmesh
cargo test              # rustup installs the pinned toolchain and the wasm target on first use
cargo xtask component   # the WebAssembly component
```

For end-to-end tests install SolidCraft and point `SOLIDCRAFT_CLI` at its `solidcraft-cli`:

```sh
SOLIDCRAFT_CLI=/path/to/solidcraft-cli cargo xtask plugin-test
```

### Agent skills

`.claude/settings.json` registers [obra/superpowers-marketplace](https://github.com/obra/superpowers-marketplace)
as a known plugin marketplace and enables its `superpowers` plugin (`enabledPlugins`), so Claude Code
installs the skills automatically once the project folder is trusted. More plugins from the marketplace
can be installed manually:

```sh
/plugin install <plugin>@superpowers-marketplace
```

## Before you open a pull request

CI runs these gates; run them locally first:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p revmesh-plugin --target wasm32-unknown-unknown -- -D warnings
cargo test --workspace
cargo llvm-cov --workspace --fail-under-lines 80   # coverage gate >= 80% lines, whole workspace (llvm-tools-preview + cargo-llvm-cov)
cargo deny check            # cargo install cargo-deny
cargo xtask plugin-test     # if you have SolidCraft
```

### Staying current with SolidCraft

`SOLIDCRAFT_VERSION` in `.github/workflows/ci.yml` is the SolidCraft release the plugin is built and
tested against. The `SolidCraft version` workflow (`.github/workflows/solidcraft-version.yml`) checks
it weekly against the latest release of
[tangency-labs/solidcraft](https://github.com/tangency-labs/solidcraft): pull requests that lag behind
fail that check, and when a newer release exists the workflow opens a dependabot-style update PR
(re-vendored `wit/plugin.wit` plus all version references). Dependabot itself cannot track another
repo's release into a vendored file, which is why this is a workflow.

The update job needs the repo secret `SOLIDCRAFT_UPDATE_TOKEN`: a fine-grained PAT or GitHub App token with
`contents: write`, `pull-requests: write` and `workflows: write`. The `workflows` scope is required because the
update branch changes `.github/workflows/ci.yml` — the default `GITHUB_TOKEN` cannot push that (the `workflows`
scope cannot be granted from a workflow file's `permissions` key), and a non-`GITHUB_TOKEN` identity is needed
so the new PR's CI actually runs. Without the secret the scheduled run fails with an explanatory error.

Guidelines:

- When `SOLIDCRAFT_VERSION` moves (weekly check or update PR), re-read `docs/plugins.md` in
  [tangency-labs/solidcraft](https://github.com/tangency-labs/solidcraft) at the new release tag
  (today: [`v0.49.1`](https://github.com/tangency-labs/solidcraft/blob/v0.49.1/docs/plugins.md))
  and diff [`main`](https://github.com/tangency-labs/solidcraft/blob/main/docs/plugins.md):
  confirm the re-vendored plugin is still compatible (capabilities, worlds, limits, manifest
  fields) and uses every interface feature the new host offers that applies. The same two-part
  check is a standing gate in every `solidcraft-*` skill (see `.agents/skills/`).
- **Keep the plugin crate thin.** Logic belongs in `revmesh-core`, where it can be tested
  natively. The plugin only moves bytes between SolidCraft and the core.
- **Test what you add.** Recognisers get unit tests with small synthetic meshes, and, where it
  matters, a case in `plugin/revmesh/tests` that SolidCraft runs on the real component.
- **Golden files** (`crates/revmesh-core/tests/data/*.json`) are regenerated, never edited by
  hand: see the comment in the failing test.
- **No `unsafe`** outside the generated bindings.
- **Ported code** from mesh2cad names the module it comes from in the file header, e.g.
  `//! Ported from mesh2cad `mesh2cad/features/holes.py` (MIT, see NOTICE).`

## Commit messages and releases

We use [Conventional Commits](https://www.conventionalcommits.org/); the pull request title must
follow them too, because PRs are squash-merged and the title becomes the commit:

```text
feat(recognize): detect through holes in plates
fix(stl): accept binary files whose header starts with "solid"
docs: explain the loss report
```

`feat` and `fix` appear in the changelog. [release-please](https://github.com/googleapis/release-please)
turns them into a release pull request; merging it tags the version, and CI attaches the plugin
zip to the GitHub release. Breaking changes (`feat!:` or a `BREAKING CHANGE:` footer) bump the
minor version while revmesh is below 1.0. Maintainers: the setup is in
[docs/releasing.md](docs/releasing.md).

### Beta phase

revmesh is in its beta phase: releases are pre-releases, versioned `0.1.0-beta.1`,
`0.1.0-beta.2`, … (`versioning: prerelease` with `prerelease-type: beta` in
`release-please-config.json`), and GitHub marks them as pre-releases.

To leave the beta phase, set `"prerelease": false` in `release-please-config.json`; this is
required, because while it is `true` release-please marks every 0.x release as a pre-release and
keeps numbering betas. The next release is then the stable version (`0.1.0` after
`0.1.0-beta.N`). To pick a different version, add the footer `Release-As: 0.2.0` to a commit
message as well; on its own, `Release-As` only overrides the next version number, for example to
start a new beta line with `Release-As: 0.2.0-beta.1`.

## Licence of contributions

revmesh is licensed under the MIT License. By submitting a contribution you agree that it is
licensed under the same terms, and that you have the right to submit it. Code ported from
mesh2cad keeps its original copyright notice (see `NOTICE`).

## Code of Conduct

Everyone taking part follows our [Code of Conduct](CODE_OF_CONDUCT.md).
