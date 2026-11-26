# Development

## What an import produces

```sh
cargo run -q -p revmesh-core --example params -- part.stl                      # parameters and report
cargo run -q -p revmesh-core --example params -- part.stl --design part.json   # and the design
```

`part.json` opens in SolidCraft (File › Open) and can be measured headless with
`solidcraft-cli eval part.json`; `solidcraft-cli eval script.json --in part.json` measures it
after a script of `parameters.change` commands, which is how a new feature type is checked to
rebuild when its parameters change. `solidcraft-cli run --in part.json --out rebuilt.stl` exports
the rebuilt part.

`cargo xtask check` does all of that for every fixture in `plugin/revmesh/tests` and compares the
rebuilt part with the source mesh: `solidcraft-cli eval` for feature errors and volume, the
re-export for the surface deviation (Hausdorff, `check.rs`), gated as mesh2cad gates its scripts
(see `docs/architecture.md`, testing level 3).

## Texts and translations

Every text the user sees (the import dialog's option titles and descriptions in `plugin.json`,
the progress messages, the loss report and the STL error messages) is a message in the catalogs
`crates/revmesh-core/locales/<locale>.json`: flat JSON of message id → text, with `{name}`
placeholders, read through `revmesh_core::Locale` (`text`, `format`, `decimal`). English (`en`)
is the default and the fallback; German (`de`) is the second locale.

SolidCraft shows a plugin's manifest texts as the package has them and does not tell the plugin
the app's language, so the language is fixed per package: `cargo xtask dist` builds one zip per
locale, localizing `plugin.json` with `revmesh_core::i18n::localize_manifest` (ids
`manifest.description` and `manifest.<entry>.<option>.title|description`) and building the
component with `REVMESH_LOCALE=<locale>`. `plugin/revmesh/plugin.json` stays the English source
and must match `en.json` (a test checks it). Option values such as `auto` and `mesh` are never
translated, as the plugin parses them.

To add a text, add its id to `en.json` and every other catalog; the tests fail on a message that
is missing in a locale or whose placeholders differ from English. To add a locale, add its
catalog and a `Locale` variant. Not translated yet: the detailed reasons the recognisers give for
keeping a mesh (the part after the colon in "No parametric features were recognised: …") and the
reason inside an ASCII STL syntax error.

## Comparing with mesh2cad

revmesh follows mesh2cad's recognition and parameter names, so every recogniser is compared
with mesh2cad's output on the same STL files. Install mesh2cad with CadQuery once, for example in
its own virtual environment:

```sh
uv venv ~/.venvs/mesh2cad && VIRTUAL_ENV=~/.venvs/mesh2cad uv pip install "mesh2cad[cadquery]"
export MESH2CAD=~/.venvs/mesh2cad/bin/mesh2cad
```

Then

```sh
scripts/compare-mesh2cad.sh plugin/revmesh/tests/*.stl
```

prints a Markdown table per file: every parameter name either tool writes, mesh2cad's value
(from the CadQuery script it writes) and revmesh's, with `≠` where they differ by more than
0.001. Below each table are mesh2cad's verdict (the script and its deviation from the mesh, or
why it refused) and revmesh's import summary. A pull request that adds or changes a
recogniser includes this table for its test files.

Differences that are expected, and why:

- **Numbering.** The two tools may number holes in a different order; compare the sets.
- **L-brackets.** mesh2cad writes an L with every corner rounded alike as `base_x`, `base_y`,
  `leg_x`, `leg_y`, `fillet_r` and `base_hole_N_*` / `leg_hole_N_*`; revmesh writes it as
  any other outline (`extrude_len`, `outline_fillet_r`, `hole_N_*`).
- **The outline itself.** mesh2cad's `outline` is a list of points; SolidCraft has no list
  parameters, so revmesh's outline lives in its sketch, where each side can be dimensioned.

## Fixtures

STL fixtures are generated, never edited by hand: `crates/revmesh-core/examples/plate_stl.rs`
writes them from a spec (`synth` tessellates the way CAD exporters do), and prints the volume
SolidCraft should measure after rebuilding. Each fixture in `plugin/revmesh/tests` has a
`.json` case that `cargo xtask plugin-test` runs in SolidCraft.

The intentionally broken fixtures (`broken.stl`, `empty.stl`, `nan.stl`, `truncated.stl`,
`ascii-bad.stl`) come from the same example's raw modes, listed in its header. Their `.json`
cases expect an error — `{"error": "invalid_input", "message": "…"}` with a fragment of the
localised reason, as `broken.json` shows.

Real STL files from Printables, under CC0 or CC BY and used unchanged, are in
`plugin/revmesh/tests/printables/` (authors and licences in its README);
`crates/revmesh-core/tests/printables.rs` checks them against the authors' published sources
where there are any, and the `printables-*.json` cases import each in SolidCraft. For any STL,
`cargo run -q -p revmesh-core --example params -- part.stl` ends with the feature count and the
volume a plugin test case expects.

The example parts in `examples/` (shown in the docs' [examples](/guide/examples) gallery) are
generated the same way, by `crates/revmesh-core/examples/examples_stl.rs`;
`crates/revmesh-core/tests/examples.rs` checks that each imports as the gallery describes. The
gallery's pictures in `docs/public/examples/` come from `scripts/render-stl.py` (numpy and
Pillow): regenerate both after changing an example or a fixture it shows:

```sh
cargo run -q -p revmesh-core --example examples_stl -- examples
python3 scripts/render-stl.py --out docs/public/examples examples/*.stl \
  plugin/revmesh/tests/{stepped-holes,blind-holes,hole-rims,slots,boss-fillet,boss-draft-chamfer,pockets,cutouts,combined,combined-3,combined-windows,combined-cuts}.stl
```
