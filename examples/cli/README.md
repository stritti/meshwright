# Headless import, from a script

Import an example STL without opening SolidCraft, through `solidcraft-cli`: install the built
plugin zip, then `file.import` with the importer entry `revmesh.stl`, and export the rebuilt
design as STL. The importer's loss report comes back in the run's results.

Needs a built package and the CLI (ships with SolidCraft; CI downloads it from the
[tangency-labs/solidcraft releases](https://github.com/tangency-labs/solidcraft/releases)):

```sh
cargo xtask dist        # target/dist/revmesh-<version>.zip
```

1. **Review the package and copy its approval hash** (SolidCraft runs plugin code only after the
   package it reviewed is the package installed — the hash covers manifest and component):

   ```sh
   solidcraft-cli exec plugin.check '{"path": "target/dist/revmesh-<version>.zip"}'
   # or, as a run script: solidcraft-cli run examples/cli/check-package.json --script-relative
   ```

2. **Fill the hash into the import script** `import-corner-plate.json` (and the version in the
   zip name), then run it with `--script-relative` so the script's own paths resolve from
   `examples/cli/`:

   ```sh
   solidcraft-cli --plugins-dir /tmp/revmesh-plugins \
     run examples/cli/import-corner-plate.json --script-relative
   ```

   `--plugins-dir` keeps a scratch plugin store, apart from the user's; approvals given in a run
   last for that run only (`--keep-plugin-approvals` shares the desktop app's). The rebuilt part
   is written to `target/cli-example/rebuilt-corner-plate.stl`.

The one-command version — check, install, import, export, and compare the rebuilt STL's volume
with the source within 1 % — is [`scripts/example-cli.sh`](../../scripts/example-cli.sh); CI
runs it as the `example-cli` job.
