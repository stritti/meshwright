# Install

1. Download `revmesh-<version>.zip` from the
   [releases](https://github.com/stritti/revmesh/releases).
2. In SolidCraft, open **Preferences › Plugins › Install plugin…** (also *File › Plugins…*) and
   pick the zip.

revmesh is also listed in the community [SolidCraft plugin list](https://tharje.github.io/solidcraft-plugins/),
which offers the latest zip as a download button.

From a script (`solidcraft-cli`), review the package first and install it with the approval hash
it prints — SolidCraft runs plugin code only for the exact package reviewed:

```sh
solidcraft-cli exec plugin.check '{"path": "revmesh-<version>.zip"}'  # prints "approval": "…"
solidcraft-cli exec plugin.install '{"path": "revmesh-<version>.zip", "approval": "…", "approve": {}}'
```

[examples/cli/](https://github.com/stritti/revmesh/tree/main/examples/cli) shows the whole flow
headless (check → install → `file.import` → export), and
[scripts/example-cli.sh](https://github.com/stritti/revmesh/tree/main/scripts/example-cli.sh) runs
it in one command (CI job `example-cli`). Or unpack the zip into the per-user plugins folder:
`~/.local/share/solidcraft/plugins/` (Linux), `%LOCALAPPDATA%\SolidCraft\plugins\` (Windows),
`~/Library/Application Support/SolidCraft/plugins/` (macOS).

revmesh 0.2.x targets SolidCraft 0.51: the plugin interface is vendored unchanged from SolidCraft
v0.51.2 and CI tests against `solidcraft-cli` 0.51.2 (see
[Compatibility](https://github.com/stritti/revmesh#compatibility) in the README). SolidCraft's
plugin support is experimental, so a different SolidCraft version may refuse the package or need a
different revmesh — keep both up to date.

To build the plugin yourself, see [Development](/development) and the
[README](https://github.com/stritti/revmesh#build-from-source).

Next: [use it](/guide/use).
