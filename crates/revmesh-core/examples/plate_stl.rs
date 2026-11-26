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
//!   with a through hole along X, placed away from the origin (a part standing upright);
//! - `rounded-box`: `plugin/revmesh/tests/rounded-box.stl`, a 60 x 40 x 30 mm box with all
//!   twelve edges rounded with 5 mm (mesh2cad's `edge_fillet_r`);
//! - `stadium`: `plugin/revmesh/tests/stadium.stl`, a 40 x 20 rectangle with a radius-10
//!   semicircle arc side at each end;
//! - `cutout-arc`: `plugin/revmesh/tests/cutout-arc.stl`, a 70 x 40 x 6 mm plate with a
//!   16 x 16 cut-out whose top side is a radius-8 semicircle arc side.
//! - `block-obj`, `block-ply`, `block-ply-bin`: `plugin/revmesh/tests/block-60x40x6.obj`,
//!   `block-60x40x6.ply` (ASCII) and `block-60x40x6-bin.ply` (binary) — the block of
//!   `block-60x40x6.stl` written as OBJ and PLY, so the format cases import the same geometry.
//!
//! ```sh
//! cargo run -p revmesh-core --example plate_stl > plugin/revmesh/tests/plate-holes.stl
//! cargo run -p revmesh-core --example plate_stl -- bracket > plugin/revmesh/tests/l-bracket.stl
//! cargo run -p revmesh-core --example plate_stl -- inside-hole > plugin/revmesh/tests/l-inside-hole.stl
//! cargo run -p revmesh-core --example plate_stl -- patterns > plugin/revmesh/tests/plate-patterns.stl
//! cargo run -p revmesh-core --example plate_stl -- x-plate > plugin/revmesh/tests/x-plate.stl
//! cargo run -p revmesh-core --example plate_stl -- rounded-box > plugin/revmesh/tests/rounded-box.stl
//! cargo run -p revmesh-core --example plate_stl -- stadium > plugin/revmesh/tests/stadium.stl
//! cargo run -p revmesh-core --example plate_stl -- cutout-arc > plugin/revmesh/tests/cutout-arc.stl
//! cargo run -p revmesh-core --example plate_stl -- block-obj > plugin/revmesh/tests/block-60x40x6.obj
//! cargo run -p revmesh-core --example plate_stl -- block-ply > plugin/revmesh/tests/block-60x40x6.ply
//! cargo run -p revmesh-core --example plate_stl -- block-ply-bin > plugin/revmesh/tests/block-60x40x6-bin.ply
//! ```
//!
//! The intentionally broken fixtures for the error cases are written raw by the same example
//! (never edited by hand):
//!
//! - `broken`: `plugin/revmesh/tests/broken.stl`, 28 bytes of plain text;
//! - `empty`: `plugin/revmesh/tests/empty.stl`, an ASCII STL with no facet;
//! - `nan`: `plugin/revmesh/tests/nan.stl`, the ASCII tetrahedron of `tests/data/tetra.stl` with
//!   `nan` in three corners;
//! - `truncated`: `plugin/revmesh/tests/truncated.stl`, a binary STL whose header announces four
//!   triangles but which ends ten bytes early;
//! - `ascii-bad`: `plugin/revmesh/tests/ascii-bad.stl`, an ASCII STL whose fourth line is a
//!   `vertex` with a coordinate that is not a number;
//! - `obj-bad`: `plugin/revmesh/tests/obj-bad.obj`, an OBJ file whose face names vertex 9 of 3;
//! - `ply-bad`: `plugin/revmesh/tests/ply-bad.ply`, a PLY header in `binary_big_endian`.
//!
//! ```sh
//! cargo run -p revmesh-core --example plate_stl -- broken > plugin/revmesh/tests/broken.stl
//! cargo run -p revmesh-core --example plate_stl -- empty > plugin/revmesh/tests/empty.stl
//! cargo run -p revmesh-core --example plate_stl -- nan > plugin/revmesh/tests/nan.stl
//! cargo run -p revmesh-core --example plate_stl -- truncated > plugin/revmesh/tests/truncated.stl
//! cargo run -p revmesh-core --example plate_stl -- ascii-bad > plugin/revmesh/tests/ascii-bad.stl
//! cargo run -p revmesh-core --example plate_stl -- obj-bad > plugin/revmesh/tests/obj-bad.obj
//! cargo run -p revmesh-core --example plate_stl -- ply-bad > plugin/revmesh/tests/ply-bad.ply
//! ```

use std::io::Write as _;

use revmesh_core::mesh::{Mesh, snap_mm};
use revmesh_core::recognize::{ArcSide, Axis, CutoutSpec, HoleSpec, PlateSpec, ProfileSpec};
use revmesh_core::{obj, ply, stl, synth};

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
        edge_fillet_r: None,
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
        outline_pockets: Vec::new(),
        bosses: vec![],
        slots: vec![],
        outline_fillet_r: Some(2.0),
        arcs: Vec::new(),
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
        edge_fillet_r: None,
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
        outline_pockets: Vec::new(),
        arcs: Vec::new(),
        ..bracket()
    }
}

/// The stadium in `stadium.stl`: a 40 x 20 rectangle with a radius-10 semicircular arc side at
/// each end.
fn stadium() -> ProfileSpec {
    ProfileSpec {
        outline_pockets: vec![],
        corners: vec![[0.0, 10.0], [40.0, 10.0], [40.0, 30.0], [0.0, 30.0]],
        z: 4.0,
        holes: vec![],
        slots: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![],
        cutouts: vec![],
        arcs: vec![None, Some(ArcSide { centre: [40.0, 20.0], r: 10.0 }), None, Some(ArcSide { centre: [0.0, 20.0], r: 10.0 })],
        outline_fillet_r: None,
    }
}

/// The plate in `cutout-arc.stl`: a 70 x 40 x 6 mm plate with a 16 x 16 cut-out whose top side
/// bulges out as a radius-8 semicircle.
fn cutout_arc() -> ProfileSpec {
    ProfileSpec {
        corners: vec![[0.0, 0.0], [70.0, 0.0], [70.0, 40.0], [0.0, 40.0]],
        z: 6.0,
        holes: vec![],
        slots: vec![],
        pads: vec![],
        pockets: vec![],
        outline_pockets: Vec::new(),
        bosses: vec![],
        cutouts: vec![CutoutSpec {
            corners: vec![[10.0, 10.0], [26.0, 10.0], [26.0, 26.0], [10.0, 26.0]],
            arcs: vec![None, None, Some(ArcSide { centre: [18.0, 26.0], r: 8.0 }), None],
        }],
        arcs: Vec::new(),
        outline_fillet_r: None,
    }
}

/// The box in `rounded-box.stl`: 60 x 40 x 30 mm with all twelve edges rounded with 5 mm.
fn rounded_box() -> (PlateSpec, revmesh_core::mesh::Vec3) {
    (
        PlateSpec {
            x: 60.0,
            y: 40.0,
            z: 30.0,
            holes: vec![],
            cutouts: vec![],
            pads: vec![],
            pockets: vec![],
            bosses: vec![],
            slots: vec![],
            outline_fillet_r: None,
            edge_fillet_r: Some(5.0),
        },
        [0.0; 3],
    )
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
        edge_fillet_r: None,
    };
    let mesh = Axis::X.unrotate_mesh(&synth::plate(&spec, 48, [3.0, 7.0, 11.0]));
    let profile = spec.into_profile();
    (mesh, profile.describe(), profile.volume())
}

/// Intentionally broken STL fixtures for the plugin tests' error cases, written raw — the only
/// way to change them is here, never by hand.
fn broken(which: Option<&str>) -> Option<Vec<u8>> {
    const TETRA: &str = include_str!("../tests/data/tetra.stl");
    match which? {
        "broken" => Some(b"hello, I am not an STL file\n".to_vec()),
        "empty" => Some(b"solid empty\nendsolid empty\n".to_vec()),
        "nan" => Some(TETRA.replace("vertex 0 10 0", "vertex 0 nan 0").into_bytes()),
        "truncated" => {
            let tetra = Mesh {
                triangles: vec![
                    [[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [0.0, 10.0, 0.0]],
                    [[0.0, 0.0, 0.0], [0.0, 10.0, 0.0], [0.0, 0.0, 10.0]],
                    [[0.0, 0.0, 0.0], [0.0, 0.0, 10.0], [10.0, 0.0, 0.0]],
                    [[10.0, 0.0, 0.0], [0.0, 10.0, 0.0], [0.0, 0.0, 10.0]],
                ],
            };
            let mut bytes = stl::write_binary(&tetra);
            bytes.truncate(bytes.len() - 10);
            Some(bytes)
        }
        "ascii-bad" => Some(b"solid t\nfacet normal 0 0 1\nouter loop\nvertex 0 x 0\nendloop\nendfacet\nendsolid t\n".to_vec()),
        "obj-bad" => Some(b"v 0 0 0\nv 10 0 0\nv 10 10 0\nf 1 2 9\n".to_vec()),
        "ply-bad" => Some(b"ply\nformat binary_big_endian 1.0\nend_header\n".to_vec()),
        _ => None,
    }
}

/// The block of `block-60x40x6.stl` written as OBJ or PLY (ASCII or binary), for the format
/// cases of the plugin tests: the same geometry in every format the entry accepts.
fn block_format(which: Option<&str>) -> Option<Vec<u8>> {
    const BLOCK: &[u8] = include_bytes!("../../../plugin/revmesh/tests/block-60x40x6.stl");
    let mesh = stl::parse(BLOCK).ok()?;
    match which? {
        "block-obj" => Some(obj::write(&mesh)),
        "block-ply" => Some(ply::write_ascii(&mesh)),
        "block-ply-bin" => Some(ply::write_binary(&mesh)),
        _ => None,
    }
}

fn main() -> std::io::Result<()> {
    let which = std::env::args().nth(1);
    if let Some(bytes) = broken(which.as_deref()).or_else(|| block_format(which.as_deref())) {
        std::io::stdout().write_all(&bytes)?;
        return Ok(());
    }
    let (mesh, describe, volume) = if which.as_deref() == Some("x-plate") {
        x_plate()
    } else if which.as_deref() == Some("rounded-box") {
        let (spec, offset) = rounded_box();
        (
            synth::rounded_box(spec.x, spec.y, spec.z, spec.edge_fillet_r.expect("the rounded box"), 48, offset),
            spec.describe(),
            spec.volume(),
        )
    } else if let Some((spec, offset)) = match which.as_deref() {
        Some("bracket") => Some((bracket(), [20.0, 10.0, 0.0])),
        Some("inside-hole") => Some((inside_hole(), [0.0; 3])),
        Some("stadium") => Some((stadium(), [0.0; 3])),
        Some("cutout-arc") => Some((cutout_arc(), [0.0; 3])),
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
