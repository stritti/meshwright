//! `cargo xtask <task>`: build automation that needs more than `cargo build`.
//!
//! - `component`: builds `revmesh-plugin` for `wasm32-unknown-unknown` and wraps the module as
//!   a component into `plugin/revmesh/revmesh.wasm` (what `wasm-tools component new` does).
//! - `dist`: `component`, then zips the package (without its tests) to
//!   `target/dist/revmesh-<version>.zip`, ready for Preferences › Plugins › Install plugin….
//! - `plugin-test`: `component`, then runs `solidcraft-cli plugin test --require-cases` on the
//!   package. The CLI is taken from `$SOLIDCRAFT_CLI`, else from `PATH`, else from the default
//!   Windows install (`C:\Program Files\SolidCraft`, also found from WSL).

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
        _ => {
            eprintln!("usage: cargo xtask <component|dist|plugin-test>");
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
    let cli = std::env::var("SOLIDCRAFT_CLI").unwrap_or_else(|_| default_cli());
    let status = Command::new(&cli)
        .current_dir(root.join("plugin"))
        .args(["plugin", "test", "--require-cases", "revmesh"])
        .status()
        .map_err(|e| format!("running {cli} (set SOLIDCRAFT_CLI to its path): {e}"))?;
    if status.success() { Ok(()) } else { Err("solidcraft-cli plugin test failed".into()) }
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
