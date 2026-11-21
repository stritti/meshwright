# Contributing to meshwright

Thank you for helping! meshwright is a young project and every contribution counts: a bug
report, an STL file that imports badly, a ported recogniser, a fix in the docs.

## Ways to help

- **Report an import that goes wrong.** Open a [bug report](../../issues/new?template=bug_report.yml)
  and attach the STL (only files you may share) and what you expected to get.
- **Suggest a feature** with the [feature request](../../issues/new?template=feature_request.yml)
  form.
- **Port a recogniser** from [mesh2cad](https://github.com/emirmrkaljevic/mesh2cad). Please open
  an issue first, so two people don't port the same thing.
- **Improve the docs.** Small fixes can go straight into a pull request.

## Development setup

```sh
git clone https://github.com/stritti/meshwright
cd meshwright
cargo test              # rustup installs the pinned toolchain and the wasm target on first use
cargo xtask component   # the WebAssembly component
```

For end-to-end tests install SolidCraft and point `SOLIDCRAFT_CLI` at its `solidcraft-cli`:

```sh
SOLIDCRAFT_CLI=/path/to/solidcraft-cli cargo xtask plugin-test
```

## Before you open a pull request

CI runs these gates; run them locally first:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p meshwright-plugin --target wasm32-unknown-unknown -- -D warnings
cargo test --workspace
cargo deny check            # cargo install cargo-deny
cargo xtask plugin-test     # if you have SolidCraft
```

Guidelines:

- **Keep the plugin crate thin.** Logic belongs in `meshwright-core`, where it can be tested
  natively. The plugin only moves bytes between SolidCraft and the core.
- **Test what you add.** Recognisers get unit tests with small synthetic meshes, and, where it
  matters, a case in `plugin/meshwright/tests` that SolidCraft runs on the real component.
- **Golden files** (`crates/meshwright-core/tests/data/*.json`) are regenerated, never edited by
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
minor version while meshwright is below 1.0.

### Beta phase

meshwright is in its beta phase: releases are pre-releases, versioned `0.1.0-beta.1`,
`0.1.0-beta.2`, … (`versioning: prerelease` with `prerelease-type: beta` in
`release-please-config.json`), and GitHub marks them as pre-releases. To make the first stable
release, either set `"prerelease": false` in `release-please-config.json` or merge a commit whose
message ends with the footer `Release-As: 0.1.0` (any version you choose); a `Release-As` footer
also starts the next beta line, e.g. `Release-As: 0.2.0-beta.1`.

## Licence of contributions

meshwright is licensed under the MIT License. By submitting a contribution you agree that it is
licensed under the same terms, and that you have the right to submit it. Code ported from
mesh2cad keeps its original copyright notice (see `NOTICE`).

## Code of Conduct

Everyone taking part follows our [Code of Conduct](CODE_OF_CONDUCT.md).
