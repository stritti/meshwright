//! `cargo xtask <task>`: build automation that needs more than `cargo build`.
//!
//! - `component`: builds `revmesh-plugin` for `wasm32-unknown-unknown` and wraps the module as
//!   a component into `plugin/revmesh/revmesh.wasm` (what `wasm-tools component new` does).
//! - `dist`: `component`, then zips the package (without its tests) to
//!   `target/dist/revmesh-<version>.zip`, ready for Preferences › Plugins › Install plugin….
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

const PLUGIN_CRATE: &str = "revmesh-plugin";
const PACKAGE_DIR: &str = "plugin/revmesh";
const TARGET: &str = "wasm32-unknown-unknown";

fn main() -> ExitCode {
    let task = std::env::args().nth(1).unwrap_or_default();
    let root = workspace_root();
    let result = match task.as_str() {
        "component" => component(&root).map(|path| println!("component: {}", path.display())),
        "dist" => component(&root).and_then(|_| dist(&root)).map(|path| println!("package: {}", path.display())),
        "plugin-test" => component(&root).and_then(|_| plugin_test(&root)),
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

/// Builds the plugin module and wraps it as a component in the package folder.
fn component(root: &Path) -> Result<PathBuf, String> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let status = Command::new(cargo)
        .current_dir(root)
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

/// Zips `plugin.json` and the component, leaving out tests and hidden files.
fn dist(root: &Path) -> Result<PathBuf, String> {
    let manifest = manifest(root)?;
    let version = manifest["version"].as_str().ok_or("plugin.json has no version")?;
    let module = module_file(root)?;
    let dist = root.join("target").join("dist");
    fs::create_dir_all(&dist).map_err(|e| format!("{}: {e}", dist.display()))?;
    let out = dist.join(format!("revmesh-{version}.zip"));
    let file = fs::File::create(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for name in ["plugin.json", module.as_str()] {
        let bytes = fs::read(root.join(PACKAGE_DIR).join(name)).map_err(|e| format!("{name}: {e}"))?;
        zip.start_file(name, options).map_err(|e| format!("zip: {e}"))?;
        zip.write_all(&bytes).map_err(|e| format!("zip: {e}"))?;
    }
    zip.finish().map_err(|e| format!("zip: {e}"))?;
    Ok(out)
}

/// Runs SolidCraft's own plugin test runner on the package folder.
fn plugin_test(root: &Path) -> Result<(), String> {
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
fn check(root: &Path) -> Result<(), String> {
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
