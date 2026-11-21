//! Writes the STL fixtures of SolidCraft's plugin tests, tessellated as a CAD exporter would,
//! and prints the volume SolidCraft should measure after rebuilding each:
//!
//! - `plate` (default): `plugin/meshwright/tests/plate-holes.stl`, a 70 x 40 x 6 mm plate with
//!   three through holes and 3 mm rounded corners;
//! - `bracket`: `plugin/meshwright/tests/l-bracket.stl`, an L-shaped plate with two holes and
//!   2 mm rounded inside and outside corners, placed away from the origin;
//! - `inside-hole`: `plugin/meshwright/tests/l-inside-hole.stl`, an L with 5 mm rounded corners
//!   and a 0.4 mm hole in the material the inside corner's rounding adds.
//!
//! ```sh
//! cargo run -p meshwright-core --example plate_stl > plugin/meshwright/tests/plate-holes.stl
//! cargo run -p meshwright-core --example plate_stl -- bracket > plugin/meshwright/tests/l-bracket.stl
//! cargo run -p meshwright-core --example plate_stl -- inside-hole > plugin/meshwright/tests/l-inside-hole.stl
//! ```

use std::io::Write as _;

use meshwright_core::recognize::{HoleSpec, PlateSpec, ProfileSpec};
use meshwright_core::{stl, synth};

/// The plate in `plate-holes.stl`.
fn plate() -> PlateSpec {
    PlateSpec {
        x: 70.0,
        y: 40.0,
        z: 6.0,
        holes: vec![HoleSpec { x: 10.0, y: 10.0, d: 4.5 }, HoleSpec { x: 35.0, y: 20.0, d: 8.0 }, HoleSpec { x: 60.0, y: 30.0, d: 4.5 }],
        outline_fillet_r: Some(3.0),
    }
}

/// The bracket in `l-bracket.stl`.
fn bracket() -> ProfileSpec {
    ProfileSpec {
        corners: vec![[0.0, 0.0], [60.0, 0.0], [60.0, 15.0], [15.0, 15.0], [15.0, 40.0], [0.0, 40.0]],
        z: 5.0,
        holes: vec![HoleSpec { x: 7.5, y: 30.0, d: 5.0 }, HoleSpec { x: 45.0, y: 7.5, d: 6.0 }],
        outline_fillet_r: Some(2.0),
    }
}

/// The L in `l-inside-hole.stl`.
fn inside_hole() -> ProfileSpec {
    ProfileSpec { holes: vec![HoleSpec { x: 16.0, y: 16.0, d: 0.4 }], outline_fillet_r: Some(5.0), ..bracket() }
}

fn main() -> std::io::Result<()> {
    let which = std::env::args().nth(1);
    let profile = match which.as_deref() {
        Some("bracket") => Some((bracket(), [20.0, 10.0, 0.0])),
        Some("inside-hole") => Some((inside_hole(), [0.0; 3])),
        _ => None,
    };
    let (mesh, describe, volume) = if let Some((spec, offset)) = profile {
        (synth::profile(&spec, 48, offset), spec.describe(), spec.volume())
    } else {
        let spec = plate();
        (synth::plate(&spec, 48, [0.0; 3]), spec.describe(), spec.volume())
    };
    std::io::stdout().write_all(&stl::write_binary(&mesh))?;
    eprintln!("{describe}: volume after rebuild {volume:.6} mm³");
    Ok(())
}
