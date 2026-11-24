# Releasing

Releases are made by [release-please](https://github.com/googleapis/release-please) from the
Conventional Commit messages on `main`:

1. Every push to `main` updates (or opens) the release pull request `chore: release x.y.z`. It
   bumps the version in `Cargo.toml`, `plugin/revmesh/plugin.json` and `CHANGELOG.md`; a
   second job brings `Cargo.lock` in step.
2. Merging that pull request tags `vx.y.z` and creates the GitHub release.
3. The `publish-plugin` job builds the plugin and attaches `revmesh-x.y.z.zip` (English),
   `revmesh-x.y.z-de.zip` (German) and `SHA256SUMS.txt` to the release.

## One-time setup: the release token

Pull requests and commits made with a workflow's own `GITHUB_TOKEN` start no other workflows.
With that token the release pull request would never get its CI checks, and with the required
check "CI ok" it could never be merged. release-please therefore uses a token of its own:

1. Create a **fine-grained personal access token** (GitHub › Settings › Developer settings ›
   Personal access tokens › Fine-grained tokens):
   - Repository access: only this repository.
   - Permissions: **Contents** read and write, **Pull requests** read and write.
   - Expiry: as long as you are comfortable with; renew it before it runs out.
2. Add it to the repository as the Actions secret **`RELEASE_PLEASE_TOKEN`** (Settings › Secrets
   and variables › Actions).

Without the secret the release workflow stops with an error that points here.

A GitHub App installation token works as well and is not tied to a person; swap the secret for
one created with `actions/create-github-app-token` if the project grows more maintainers.

## Branch protection

Protect `main` and require the status check **CI ok**, which only passes when every quality
gate in `.github/workflows/ci.yml` passed. Require pull requests and squash merging, so the PR
title (checked by `pr-title.yml`) becomes the commit release-please reads.
