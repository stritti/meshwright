//! Imports a mesh file (STL, OBJ or PLY) as the plugin would and prints what came out: one line per user
//! parameter (`name value`), then the import report's summary and items on standard error, and
//! the volume SolidCraft should measure after rebuilding the design (the exact volume of the
//! recognised part, or the mesh's own where it is kept as a mesh): the `volume_mm3` a plugin test
//! case in `plugin/revmesh/tests` expects.
//!
//! ```sh
//! cargo run -q -p revmesh-core --example params -- part.stl
//! cargo run -q -p revmesh-core --example params -- part.obj --design part.json
//! ```
//!
//! `--design FILE` also writes the design, which SolidCraft opens (File › Open) or measures
//! (`solidcraft-cli eval FILE`). `scripts/compare-mesh2cad.sh` uses this to set revmesh's
//! parameters beside mesh2cad's.

use std::process::ExitCode;

use revmesh_core::recognize::{Recognition, recognize};
use revmesh_core::{ImportOptions, format, import};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(path) = args.first() else {
        eprintln!("usage: params <mesh-file> [--design <design.json>]");
        return ExitCode::from(2);
    };
    let design_out = args.iter().position(|a| a == "--design").and_then(|i| args.get(i + 1));
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("{path}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let name = std::path::Path::new(path).file_name().map_or_else(|| path.clone(), |n| n.to_string_lossy().into_owned());
    let result = match import(&bytes, &name, &ImportOptions::default()) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("{path}: {e}");
            return ExitCode::FAILURE;
        }
    };
    for param in result.design.params.iter().filter(|p| !p.model) {
        let value = param.expr.strip_suffix(" mm").or_else(|| param.expr.strip_suffix(" deg")).unwrap_or(&param.expr);
        println!("{} {value}", param.name);
    }
    eprintln!("{}", result.report.summary);
    for item in &result.report.items {
        eprintln!("  {}: {}", item.title, item.message);
    }
    if let Ok(mesh) = format::parse(&bytes, &name) {
        let volume = match recognize(&mesh) {
            Recognition::Plate { plate, .. } => plate.volume(),
            Recognition::Profile { profile, .. } => profile.volume(),
            Recognition::Combined { spec, .. } => spec.volume(),
            Recognition::Stack { spec, .. } => spec.volume(),
            Recognition::Unrecognized { .. } => mesh.volume(),
            Recognition::Slanted { prism, .. } => match &*prism {
                Recognition::Plate { plate, .. } => plate.volume(),
                Recognition::Profile { profile, .. } => profile.volume(),
                _ => mesh.volume(),
            },
        };
        eprintln!("features {}, volume after rebuild {volume:.6} mm³", result.design.features.len());
    }
    if let Some(out) = design_out
        && let Err(e) = std::fs::write(out, result.design.to_json_pretty())
    {
        eprintln!("{out}: {e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
