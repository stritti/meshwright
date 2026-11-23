//! Writes the STL fixtures of SolidCraft's plugin tests, tessellated as a CAD exporter would,
//! and prints the volume SolidCraft should measure after rebuilding each:
//!
//! - `plate` (default): `plugin/revmesh/tests/plate-holes.stl`, a 70 x 40 x 6 mm plate with
//!   three through holes and 3 mm rounded corners;
//! - `bracket`: `plugin/revmesh/tests/l-bracket.stl`, an L-shaped plate with two holes and
//!   2 mm rounded inside and outside corners, placed away from the origin;
//! - `inside-hole`: `plugin/revmesh/tests/l-inside-hole.stl`, an L with 5 mm rounded corners
//!   and a 0.4 mm hole in the material the inside corner's rounding adds;
//! - `patterns`: `plugin/revmesh/tests/plate-patterns.stl`, a plate with a hole grid, a hole
//!   row and a bolt circle;
//! - `x-plate`: `plugin/revmesh/tests/x-plate.stl`, a 40 x 20 outline extruded 60 mm along X
//!   with a through hole along X, placed away from the origin (a part standing upright).
//!
//! ```sh
//! cargo run -p revmesh-core --example plate_stl > plugin/revmesh/tests/plate-holes.stl
//! cargo run -p revmesh-core --example plate_stl -- bracket > plugin/revmesh/tests/l-bracket.stl
//! cargo run -p revmesh-core --example plate_stl -- inside-hole > plugin/revmesh/tests/l-inside-hole.stl
//! cargo run -p revmesh-core --example plate_stl -- patterns > plugin/revmesh/tests/plate-patterns.stl
//! cargo run -p revmesh-core --example plate_stl -- x-plate > plugin/revmesh/tests/x-plate.stl
//! ```

use std::io::Write as _;

use revmesh_core::mesh::snap_mm;
use revmesh_core::recognize::{Axis, HoleSpec, PlateSpec, ProfileSpec};
use revmesh_core::{stl, synth};

/// The plate in `plate-holes.stl`.
fn plate() -> PlateSpec {
    PlateSpec {
        x: 70.0,
        y: 40.0,
        z: 6.0,
        holes: vec![
            HoleSpec { x: 10.0, y: 10.0, d: 4.5, head: None, blind: None },
            HoleSpec { x: 35.0, y: 20.0, d: 8.0, head: None, blind: None },
            HoleSpec { x: 60.0, y: 30.0, d: 4.5, head: None, blind: None },
        ],
        cutouts: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![],
        slots: vec![],
        outline_fillet_r: Some(3.0),
    }
}

/// The bracket in `l-bracket.stl`.
fn bracket() -> ProfileSpec {
    ProfileSpec {
        corners: vec![[0.0, 0.0], [60.0, 0.0], [60.0, 15.0], [15.0, 15.0], [15.0, 40.0], [0.0, 40.0]],
        z: 5.0,
        holes: vec![
            HoleSpec { x: 7.5, y: 30.0, d: 5.0, head: None, blind: None },
            HoleSpec { x: 45.0, y: 7.5, d: 6.0, head: None, blind: None },
        ],
        cutouts: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![],
        slots: vec![],
        outline_fillet_r: Some(2.0),
    }
}

/// The plate in `plate-patterns.stl`: a 2 x 2 grid of 5 mm holes, a row of three 3 mm holes, a
/// bolt circle of six 4 mm holes round a 10 mm bore, and 4 mm rounded corners.
fn patterns() -> PlateSpec {
    let mut holes = vec![
        HoleSpec { x: 10.0, y: 10.0, d: 5.0, head: None, blind: None },
        HoleSpec { x: 110.0, y: 10.0, d: 5.0, head: None, blind: None },
        HoleSpec { x: 10.0, y: 70.0, d: 5.0, head: None, blind: None },
        HoleSpec { x: 110.0, y: 70.0, d: 5.0, head: None, blind: None },
        HoleSpec { x: 35.0, y: 70.0, d: 3.0, head: None, blind: None },
        HoleSpec { x: 50.0, y: 70.0, d: 3.0, head: None, blind: None },
        HoleSpec { x: 65.0, y: 70.0, d: 3.0, head: None, blind: None },
        HoleSpec { x: 60.0, y: 40.0, d: 10.0, head: None, blind: None },
    ];
    holes.extend((0..6).map(|i| {
        let a = (15.0 + 60.0 * f64::from(i)).to_radians();
        HoleSpec { x: snap_mm(60.0 + 15.0 * a.cos()), y: snap_mm(40.0 + 15.0 * a.sin()), d: 4.0, head: None, blind: None }
    }));
    PlateSpec {
        x: 120.0,
        y: 80.0,
        z: 6.0,
        holes,
        cutouts: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![],
        slots: vec![],
        outline_fillet_r: Some(4.0),
    }
}

/// The L in `l-inside-hole.stl`.
fn inside_hole() -> ProfileSpec {
    ProfileSpec {
        holes: vec![HoleSpec { x: 16.0, y: 16.0, d: 0.4, head: None, blind: None }],
        cutouts: vec![],
        bosses: vec![],
        slots: vec![],
        outline_fillet_r: Some(5.0),
        ..bracket()
    }
}

/// The part in `x-plate.stl`: a 40 x 20 outline extruded 60 mm along X, with a 6 mm hole through
/// along X, placed away from the origin. Written as a Z plate rotated to stand along X, so the
/// import recognises it as a profile extruded along X.
fn x_plate() -> (revmesh_core::mesh::Mesh, String, f64) {
    let spec = PlateSpec {
        x: 40.0,
        y: 20.0,
        z: 60.0,
        holes: vec![HoleSpec { x: 10.0, y: 5.0, d: 6.0, head: None, blind: None }],
        cutouts: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![],
        slots: vec![],
        outline_fillet_r: None,
    };
    let mesh = Axis::X.unrotate_mesh(&synth::plate(&spec, 48, [3.0, 7.0, 11.0]));
    let profile = spec.into_profile();
    (mesh, profile.describe(), profile.volume())
}

fn main() -> std::io::Result<()> {
    let which = std::env::args().nth(1);
    let (mesh, describe, volume) = if which.as_deref() == Some("x-plate") {
        x_plate()
    } else if let Some((spec, offset)) = match which.as_deref() {
        Some("bracket") => Some((bracket(), [20.0, 10.0, 0.0])),
        Some("inside-hole") => Some((inside_hole(), [0.0; 3])),
        _ => None,
    } {
        (synth::profile(&spec, 48, offset), spec.describe(), spec.volume())
    } else {
        let spec = if which.as_deref() == Some("patterns") { patterns() } else { plate() };
        (synth::plate(&spec, 48, [0.0; 3]), spec.describe(), spec.volume())
    };
    std::io::stdout().write_all(&stl::write_binary(&mesh))?;
    eprintln!("{describe}: volume after rebuild {volume:.6} mm³");
    Ok(())
}
