//! `cargo xtask <task>`: build automation that needs more than `cargo build`.
//!
//! - `component`: builds `revmesh-plugin` for `wasm32-unknown-unknown` and wraps the module as
//!   a component into `plugin/revmesh/revmesh.wasm` (what `wasm-tools component new` does).
//! - `dist`: builds one package per language and zips each (without its tests), ready for
//!   Preferences › Plugins › Install plugin…: `target/dist/revmesh-<version>.zip` in English (the
//!   default) and `target/dist/revmesh-<version>-<locale>.zip` for every other locale (German:
//!   `-de`). Each has `plugin.json` localized from `crates/revmesh-core/locales/<locale>.json` and
//!   a component built with `REVMESH_LOCALE=<locale>` and `REVMESH_LATEST_VERSION` (the newest
//!   published release, betas included, for the import-time update check), as SolidCraft shows a
//!   plugin's texts as
//!   its package gives them and tells the plugin nothing about the app's language.
//! - `plugin-test`: `component`, then runs `solidcraft-cli plugin test --require-cases` on the
//!   package. The CLI is taken from `$SOLIDCRAFT_CLI`, else from `PATH`, else from the default
//!   Windows install (`C:\Program Files\SolidCraft`, also found from WSL).
//! - `check`: imports every `plugin/revmesh/tests/*.stl`, and checks the rebuild against the
//!   source mesh through SolidCraft (`solidcraft-cli eval` for feature errors and volume,
//!   `run --out` to export the rebuilt part), measuring the volume error and the surface
//!   deviation with mesh2cad's gate. Takes the CLI as `plugin-test` does.

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use revmesh_core::Locale;

const PLUGIN_CRATE: &str = "revmesh-plugin";
const PACKAGE_DIR: &str = "plugin/revmesh";
const TARGET: &str = "wasm32-unknown-unknown";

fn main() -> ExitCode {
    let task = std::env::args().nth(1).unwrap_or_default();
    let root = workspace_root();
    let result = match task.as_str() {
        "component" => component(&root, Locale::En).map(|path| println!("component: {}", path.display())),
        "dist" => dist(&root).map(|paths| paths.iter().for_each(|path| println!("package: {}", path.display()))),
        "plugin-test" => component(&root, Locale::En).and_then(|_| plugin_test(&root)),
        "check" => check(&root),
        _ => {
            eprintln!("usage: cargo xtask <component|dist|plugin-test|check>");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("xtask {task}: {error}");
            ExitCode::FAILURE
        }
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("xtask lives in the workspace root").to_path_buf()
}

/// Builds the plugin module for `locale` and wraps it as a component in the package folder. The
/// build gets `REVMESH_LATEST_VERSION` (the newest release, for the import-time update check the
/// plugin runs; its sandbox has no network) alongside `REVMESH_LOCALE`.
fn component(root: &Path, locale: Locale) -> Result<PathBuf, String> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let status = Command::new(cargo)
        .current_dir(root)
        .env("REVMESH_LOCALE", locale.code())
        .env("REVMESH_LATEST_VERSION", latest_release(root))
        .args(["build", "--release", "--locked", "--target", TARGET, "-p", PLUGIN_CRATE])
        .status()
        .map_err(|e| format!("running cargo: {e}"))?;
    if !status.success() {
        return Err(format!("cargo build -p {PLUGIN_CRATE} --target {TARGET} failed"));
    }
    let module_path = root.join("target").join(TARGET).join("release").join(format!("{}.wasm", PLUGIN_CRATE.replace('-', "_")));
    let module = fs::read(&module_path).map_err(|e| format!("{}: {e}", module_path.display()))?;
    let component = wit_component::ComponentEncoder::default()
        .module(&module)
        .and_then(|encoder| encoder.validate(true).encode())
        .map_err(|e| format!("wrapping the module as a component: {e:#}"))?;
    let out = root.join(PACKAGE_DIR).join(module_file(root)?);
    fs::write(&out, component).map_err(|e| format!("{}: {e}", out.display()))?;
    Ok(out)
}

/// Builds and zips one package per locale (English last, so the package folder is left holding
/// the default component), leaving out tests and hidden files.
fn dist(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut packages = Vec::new();
    for locale in Locale::ALL.into_iter().rev() {
        component(root, locale)?;
        packages.push(zip_package(root, locale)?);
    }
    packages.reverse();
    Ok(packages)
}

/// Zips `plugin.json`, localized to `locale`, and the component.
fn zip_package(root: &Path, locale: Locale) -> Result<PathBuf, String> {
    let mut manifest = manifest(root)?;
    let version = manifest["version"].as_str().ok_or("plugin.json has no version")?.to_owned();
    let module = module_file(root)?;
    revmesh_core::i18n::localize_manifest(&mut manifest, locale);
    let manifest = serde_json::to_vec_pretty(&manifest).map_err(|e| format!("plugin.json: {e}"))?;
    let dist = root.join("target").join("dist");
    fs::create_dir_all(&dist).map_err(|e| format!("{}: {e}", dist.display()))?;
    let suffix = if locale == Locale::En { String::new() } else { format!("-{}", locale.code()) };
    let out = dist.join(format!("revmesh-{version}{suffix}.zip"));
    let file = fs::File::create(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let component = fs::read(root.join(PACKAGE_DIR).join(&module)).map_err(|e| format!("{module}: {e}"))?;
    for (name, bytes) in [("plugin.json", manifest.as_slice()), (module.as_str(), component.as_slice())] {
        zip.start_file(name, options).map_err(|e| format!("zip: {e}"))?;
        zip.write_all(bytes).map_err(|e| format!("zip: {e}"))?;
    }
    zip.finish().map_err(|e| format!("zip: {e}"))?;
    Ok(out)
}

/// Runs SolidCraft's own plugin test runner on the package folder.
fn plugin_test(root: &Path) -> Result<(), String> {
    note_update(root);
    let cli = cli_path();
    let status = Command::new(&cli)
        .current_dir(root.join("plugin"))
        .args(["plugin", "test", "--require-cases", "revmesh"])
        .status()
        .map_err(|e| format!("running {cli} (set SOLIDCRAFT_CLI to its path): {e}"))?;
    if status.success() { Ok(()) } else { Err("solidcraft-cli plugin test failed".into()) }
}

/// `solidcraft-cli` from `$SOLIDCRAFT_CLI`, else from `PATH`, else from SolidCraft's default
/// Windows install (seen natively or from WSL).
fn cli_path() -> String {
    std::env::var("SOLIDCRAFT_CLI").unwrap_or_else(|_| default_cli())
}

/// `solidcraft-cli` from `PATH`, else from SolidCraft's default Windows install (seen natively
/// or from WSL).
fn default_cli() -> String {
    let on_path = Command::new("solidcraft-cli").arg("--version").output().is_ok_and(|o| o.status.success());
    let installed = ["C:\\Program Files\\SolidCraft\\solidcraft-cli.exe", "/mnt/c/Program Files/SolidCraft/solidcraft-cli.exe"];
    if on_path {
        return "solidcraft-cli".into();
    }
    installed.iter().find_map(|p| Path::new(p).is_file().then(|| (*p).to_owned())).unwrap_or_else(|| "solidcraft-cli".into())
}

/// Checks every plugin test fixture's rebuild against the source mesh, the way mesh2cad checks
/// the scripts it writes:
///
/// 1. the STL is imported in-process ([`revmesh_core::import`]) and the design written to
///    `target/check/<name>.json`;
/// 2. `solidcraft-cli eval` rebuilds the design in SolidCraft: a feature that fails fails the
///    check, and the measured volume goes into the report;
/// 3. `solidcraft-cli run --in <design> --out <rebuilt.stl>` exports the rebuilt part;
/// 4. [`revmesh_core::check::check_rebuild`] measures the volume error and the surface deviation
///    (Hausdorff) between the source STL and the rebuilt STL, and applies mesh2cad's gate.
///
/// Fixtures that do not import (the error cases of `plugin test`) are skipped.
/// Rebuilds the design at `design_path` in SolidCraft after setting `param` to `value + 1 mm`.
/// `Ok(false)` when the rebuild reports a feature error.
fn eval_after_param_change(cli: &str, param: &str, value: f64, design_path: &Path, script_path: &Path) -> Result<bool, String> {
    let mut changes = serde_json::Map::new();
    changes.insert(param.to_string(), serde_json::json!(format!("{} mm", value + 1.0)));
    let script = serde_json::json!({ "commands": [{ "command": "parameters.change", "params": { "changes": changes } }] });
    fs::write(script_path, script.to_string()).map_err(|e| format!("{}: {e}", script_path.display()))?;
    let out = Command::new(cli)
        .args(["eval", &script_path.to_string_lossy(), "--in", &design_path.to_string_lossy(), "--quiet"])
        .output()
        .map_err(|e| format!("running {cli} eval: {e}"))?;
    Ok(out.status.success())
}

/// The user parameters that drive sketch sides — plain `"<n> mm"` lengths named `outline_*`,
/// `cutout_*` or `pocket_*` (a side's length, a cut-out's, a pocket's); if there are none, the
/// first driving extrude/box length (`base_*`/`extrude_*`).
fn driving_lengths(design: &revmesh_core::design::Design) -> Vec<(String, f64)> {
    let plain = |p: &revmesh_core::design::Param| -> Option<(String, f64)> {
        if p.model {
            return None;
        }
        let value: f64 = p.expr.strip_suffix(" mm")?.parse().ok()?;
        Some((p.name.clone(), value))
    };
    let sides: Vec<(String, f64)> = design
        .params
        .iter()
        .filter_map(plain)
        .filter(|(name, _)| name.starts_with("outline_") || name.starts_with("cutout_") || name.starts_with("pocket_"))
        .collect();
    if !sides.is_empty() {
        return sides;
    }
    design.params.iter().filter_map(plain).find(|(name, _)| name.starts_with("base_") || name.starts_with("extrude_")).into_iter().collect()
}

fn check(root: &Path) -> Result<(), String> {
    note_update(root);
    let cli = cli_path();
    let tests = root.join(PACKAGE_DIR).join("tests");
    let out = root.join("target").join("check");
    fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    let mut stls: Vec<PathBuf> = fs::read_dir(&tests)
        .map_err(|e| format!("{}: {e}", tests.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "stl"))
        .collect();
    stls.sort();
    if stls.is_empty() {
        return Err(format!("no STL fixtures in {}", tests.display()));
    }
    let mut failures: Vec<String> = Vec::new();
    let mut checked = 0;
    for stl in &stls {
        let name = stl.file_name().map_or_else(|| "part".into(), |n| n.to_string_lossy().into_owned());
        let stem = stl.file_stem().map_or_else(|| "part".into(), |n| n.to_string_lossy().into_owned());
        let bytes = fs::read(stl).map_err(|e| format!("{}: {e}", stl.display()))?;
        let source = match revmesh_core::stl::parse(&bytes) {
            Ok(mesh) => mesh,
            Err(e) => {
                println!("skip {name}: not an STL ({e})");
                continue;
            }
        };
        let Ok(imported) = revmesh_core::import(&bytes, &name, &revmesh_core::ImportOptions::default()) else {
            println!("skip {name}: the import itself fails (an error case of plugin test)");
            continue;
        };
        checked += 1;
        let design_path = out.join(format!("{stem}.json"));
        fs::write(&design_path, imported.design.to_json_pretty()).map_err(|e| format!("{}: {e}", design_path.display()))?;

        // Rebuild in SolidCraft: eval fails (exit 1) when a timeline feature has an error.
        let eval_path = out.join(format!("{stem}-eval.json"));
        let eval = Command::new(&cli)
            .args(["eval", &design_path.to_string_lossy(), "--json-out", &eval_path.to_string_lossy(), "--quiet"])
            .output()
            .map_err(|e| format!("running {cli} eval (set SOLIDCRAFT_CLI to its path): {e}"))?;
        if !eval.status.success() {
            failures.push(format!("{name}: solidcraft-cli eval failed: {}", String::from_utf8_lossy(&eval.stderr).trim()));
            println!("FAIL {name}: solidcraft-cli eval exited {}", eval.status.code().unwrap_or(-1));
            continue;
        }
        let eval: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&eval_path).map_err(|e| format!("{}: {e}", eval_path.display()))?)
                .map_err(|e| format!("{}: {e}", eval_path.display()))?;
        let eval_volume = eval["total"]["volume_mm3"].as_f64().unwrap_or(f64::NAN);

        // Rebuild after parameter changes too: bump each driving sketch-side length by 1 mm (the
        // first extrude/box length when there are none), each change rebuilt from the original
        // design, so a dimension that is dependent on the others (an over-constrained sketch)
        // fails here as well, not only when the user edits it in SolidCraft.
        for (param, value) in driving_lengths(&imported.design) {
            let script_path = out.join(format!("{stem}-change.json"));
            if !eval_after_param_change(&cli, &param, value, &design_path, &script_path)? {
                failures.push(format!("{name}: rebuild after changing {param} reported a feature error"));
                println!("FAIL {name}: eval after changing {param} failed");
            }
        }

        // Export the rebuilt part and compare it with the source mesh.
        let rebuilt_path = out.join(format!("{stem}-rebuilt.stl"));
        let export = Command::new(&cli)
            .args(["run", "--in", &design_path.to_string_lossy(), "--out", &rebuilt_path.to_string_lossy(), "--quiet"])
            .output()
            .map_err(|e| format!("running {cli} run: {e}"))?;
        if !export.status.success() {
            failures.push(format!("{name}: solidcraft-cli run --out failed: {}", String::from_utf8_lossy(&export.stderr).trim()));
            println!("FAIL {name}: solidcraft-cli run exited {}", export.status.code().unwrap_or(-1));
            continue;
        }
        let rebuilt = revmesh_core::stl::parse(&fs::read(&rebuilt_path).map_err(|e| format!("{}: {e}", rebuilt_path.display()))?)
            .map_err(|e| format!("{}: {e}", rebuilt_path.display()))?;
        let check = revmesh_core::check::check_rebuild(&source, &rebuilt);
        if check.ok() {
            println!(
                "ok   {name}: volume error {:+.3}%, deviation {:.4} mm (limit {:.3} mm), SolidCraft measured {eval_volume:.3} mm3",
                check.volume_error_pct, check.max_deviation, check.deviation_limit
            );
        } else {
            failures.push(format!(
                "{name}: volume error {:+.3}%, deviation {:.4} mm over the limit {:.3} mm",
                check.volume_error_pct, check.max_deviation, check.deviation_limit
            ));
            println!(
                "FAIL {name}: volume error {:+.3}%, deviation {:.4} mm (limit {:.3} mm), SolidCraft measured {eval_volume:.3} mm3",
                check.volume_error_pct, check.max_deviation, check.deviation_limit
            );
        }
    }
    if checked == 0 {
        return Err("no fixture imported".into());
    }
    if failures.is_empty() {
        println!("all {checked} rebuilds within mesh2cad's gate (volume error <= 1%, deviation <= max(4 * 0.1% of the diagonal, 0.1 mm))");
        Ok(())
    } else {
        Err(format!("{} of {checked} rebuilds failed:\n  {}", failures.len(), failures.join("\n  ")))
    }
}

/// Prints a note when a newer revmesh release than this tree is out. The import flows this xtask
/// drives (`plugin-test`, `check`) run the locally built component, whose sandbox cannot ask for
/// newer releases itself — here, where the network exists, the fresh check happens.
fn note_update(root: &Path) {
    let current = manifest(root).ok().and_then(|m| m["version"].as_str().map(str::to_owned)).unwrap_or_default();
    let latest = latest_release(root);
    if let (Some(current), Some(latest)) = (revmesh_core::version::Version::parse(&current), revmesh_core::version::Version::parse(&latest))
        && latest > current
    {
        println!("note: revmesh {latest} is out, this tree is {current}; the imports below run with {current}");
    }
}

/// The newest published revmesh release's version, for `REVMESH_LATEST_VERSION`: the newest tag
/// among the GitHub releases — drafts excluded, pre-releases included, since revmesh releases
/// betas and `/releases/latest` skips those — when `gh` can ask for it, else the package's own
/// version, which makes the baked-in update check find nothing newer (offline or `gh`-less
/// builds still package fine).
fn latest_release(root: &Path) -> String {
    static LATEST: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    LATEST
        .get_or_init(|| {
            let fallback = || manifest(root).ok().and_then(|m| m["version"].as_str().map(str::to_owned)).unwrap_or_default();
            let output = Command::new("gh")
                .args([
                    "api",
                    &format!("repos/{}/releases?per_page=100", repository(root)),
                    "--jq",
                    ".[] | select(.draft | not) | .tag_name",
                ])
                .output();
            output
                .ok()
                .filter(|o| o.status.success())
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .and_then(|tags| {
                    tags.lines().filter_map(|tag| revmesh_core::version::Version::parse(tag.trim())).max().map(|v| v.to_string())
                })
                .unwrap_or_else(fallback)
        })
        .clone()
}

/// The `owner/repo` the workspace's `repository` in the root `Cargo.toml` names.
fn repository(root: &Path) -> String {
    let fallback = "stritti/revmesh".to_owned();
    let Ok(toml) = fs::read_to_string(root.join("Cargo.toml")) else { return fallback };
    toml.lines()
        .find_map(|line| {
            line.trim()
                .strip_prefix("repository = ")
                .and_then(|value| value.trim_matches('"').strip_prefix("https://github.com/"))
                .map(str::to_owned)
        })
        .unwrap_or(fallback)
}

fn manifest(root: &Path) -> Result<serde_json::Value, String> {
    let path = root.join(PACKAGE_DIR).join("plugin.json");
    let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

fn module_file(root: &Path) -> Result<String, String> {
    manifest(root)?["runtime"]["module"]
        .as_str()
        .filter(|m| Path::new(m).extension().is_some_and(|e| e == "wasm") && !m.contains(['/', '\\']))
        .map(String::from)
        .ok_or_else(|| "plugin.json names no runtime.module".into())
}
