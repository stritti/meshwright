//! Writes the example parts in `examples/`: everyday parts as a CAD program would export them
//! to STL, each one a file you can open with revmesh to see what it recognises. The docs'
//! gallery (`docs/guide/examples.md`) shows them; `tests/examples.rs` checks that each imports
//! as the editable design it describes.
//!
//! ```sh
//! cargo run -q -p revmesh-core --example examples_stl -- examples
//! ```
//!
//! Prints one line per part: the file, the import summary it should get and the volume
//! SolidCraft should measure after rebuilding it.

use std::path::{Path, PathBuf};

use revmesh_core::mesh::{Mesh, snap_mm};
use revmesh_core::recognize::{Axis, HoleSpec, PlateSpec, ProfileSpec};
use revmesh_core::{stl, synth};

/// Facets per full circle, as a CAD exporter's fine setting gives them.
const SEGMENTS: usize = 48;

/// A through hole of diameter `d` at `(x, y)`.
fn hole(x: f64, y: f64, d: f64) -> HoleSpec {
    HoleSpec { x, y, d, head: None, blind: None }
}

/// A plate `x` by `y`, `z` thick, with `holes` and rounded vertical corners.
fn plate(x: f64, y: f64, z: f64, holes: Vec<HoleSpec>, fillet: Option<f64>) -> PlateSpec {
    PlateSpec {
        x,
        y,
        z,
        holes,
        cutouts: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![],
        slots: vec![],
        outline_fillet_r: fillet,
        edge_fillet_r: None,
    }
}

/// An outline of straight sides, `z` thick, with `holes` and rounded vertical corners.
fn profile(corners: Vec<[f64; 2]>, z: f64, holes: Vec<HoleSpec>, fillet: Option<f64>) -> ProfileSpec {
    ProfileSpec {
        corners,
        z,
        holes,
        cutouts: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![],
        slots: vec![],
        outline_fillet_r: fillet,
    }
}

/// A mounting plate for a single-board computer: 95 x 66 x 3 mm, 4 mm rounded corners, the
/// board's four M2.5 holes on its 58 x 49 mm grid and four Ø4.5 holes to screw the plate down.
fn sbc_mount() -> PlateSpec {
    let (cx, cy) = (47.5, 33.0);
    let mut holes: Vec<HoleSpec> =
        [(-29.0, -24.5), (29.0, -24.5), (-29.0, 24.5), (29.0, 24.5)].iter().map(|(dx, dy)| hole(cx + dx, cy + dy, 2.7)).collect();
    holes.extend([(6.0, 6.0), (89.0, 6.0), (6.0, 60.0), (89.0, 60.0)].iter().map(|&(x, y)| hole(x, y, 4.5)));
    plate(95.0, 66.0, 3.0, holes, Some(4.0))
}

/// A motor plate for a NEMA 17 stepper: 60 x 60 x 5 mm, 5 mm rounded corners, the Ø22.5 pilot
/// bore, the motor's four M3 holes on the 31 mm square and four Ø5.5 holes on a 48 mm square.
fn nema17_plate() -> PlateSpec {
    let mut holes = vec![hole(30.0, 30.0, 22.5)];
    holes.extend([(-15.5, -15.5), (15.5, -15.5), (-15.5, 15.5), (15.5, 15.5)].iter().map(|(dx, dy)| hole(30.0 + dx, 30.0 + dy, 3.4)));
    holes.extend([(-24.0, -24.0), (24.0, -24.0), (-24.0, 24.0), (24.0, 24.0)].iter().map(|(dx, dy)| hole(30.0 + dx, 30.0 + dy, 5.5)));
    plate(60.0, 60.0, 5.0, holes, Some(5.0))
}

/// A square pipe flange: 80 x 80 x 8 mm, 8 mm rounded corners, a Ø30 bore and six Ø6.6 holes
/// (M6) on a Ø55 bolt circle, the first at 0°.
fn flange() -> PlateSpec {
    let mut holes = vec![hole(40.0, 40.0, 30.0)];
    holes.extend((0..6).map(|i| {
        let a = (60.0 * f64::from(i)).to_radians();
        hole(snap_mm(40.0 + 27.5 * a.cos()), snap_mm(40.0 + 27.5 * a.sin()), 6.6)
    }));
    plate(80.0, 80.0, 8.0, holes, Some(8.0))
}

/// A flat corner plate to brace a frame: an L of a 70 mm and a 50 mm leg, each 20 mm wide,
/// 4 mm thick, every corner (inside and outside) rounded with 3 mm, a row of three Ø5 holes
/// along the long leg and a Ø6 hole in the short leg (holes of one diameter form a pattern
/// only when all of them do, so it has a diameter of its own).
fn corner_plate() -> ProfileSpec {
    profile(
        vec![[0.0, 0.0], [70.0, 0.0], [70.0, 20.0], [20.0, 20.0], [20.0, 50.0], [0.0, 50.0]],
        4.0,
        vec![hole(10.0, 10.0, 5.0), hole(32.0, 10.0, 5.0), hole(54.0, 10.0, 5.0), hole(10.0, 38.0, 6.0)],
        Some(3.0),
    )
}

/// A cover with chamfered corners: an 80 x 50 x 4 mm plate whose four corners are cut off at
/// 45° by 8 mm (an octagonal outline), with a row of four Ø4 holes at a 20 mm pitch and a
/// Ø12 cable hole.
fn chamfered_cover() -> ProfileSpec {
    let c = 8.0;
    profile(
        vec![[c, 0.0], [80.0 - c, 0.0], [80.0, c], [80.0, 50.0 - c], [80.0 - c, 50.0], [c, 50.0], [0.0, 50.0 - c], [0.0, c]],
        4.0,
        vec![hole(10.0, 40.0, 4.0), hole(30.0, 40.0, 4.0), hole(50.0, 40.0, 4.0), hole(70.0, 40.0, 4.0), hole(40.0, 18.0, 12.0)],
        None,
    )
}

/// A spacer bar standing along X: a 20 x 12 mm section, 70 mm long, with a Ø6.5 bore along its
/// length, placed away from the origin as in an assembly. Written as a Z plate turned to lie
/// along X, so the import recognises it as an outline extruded along X.
fn spacer_bar() -> (Mesh, String, f64) {
    let spec = plate(20.0, 12.0, 70.0, vec![hole(10.0, 6.0, 6.5)], None);
    let mesh = Axis::X.unrotate_mesh(&synth::plate(&spec, SEGMENTS, [5.0, 10.0, 15.0]));
    let profile = spec.into_profile();
    (mesh, profile.describe(), profile.volume())
}

fn main() -> std::io::Result<()> {
    let dir = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "examples".into()));
    std::fs::create_dir_all(&dir)?;
    let plates = [("sbc-mount", sbc_mount()), ("nema17-motor-plate", nema17_plate()), ("square-flange", flange())];
    for (name, spec) in plates {
        write(&dir, name, &synth::plate(&spec, SEGMENTS, [0.0; 3]), &spec.describe(), spec.volume())?;
    }
    let profiles = [("corner-plate", corner_plate()), ("chamfered-cover", chamfered_cover())];
    for (name, spec) in profiles {
        write(&dir, name, &synth::profile(&spec, SEGMENTS, [0.0; 3]), &spec.describe(), spec.volume())?;
    }
    let (mesh, describe, volume) = spacer_bar();
    write(&dir, "spacer-bar", &mesh, &describe, volume)?;
    let rounded = synth::rounded_box(60.0, 40.0, 25.0, 4.0, 16, [0.0; 3]);
    let spec = PlateSpec { edge_fillet_r: Some(4.0), ..plate(60.0, 40.0, 25.0, vec![], None) };
    write(&dir, "rounded-block", &rounded, &spec.describe(), spec.volume())
}

/// Writes `mesh` as binary STL to `dir/name.stl` and prints what the import should say.
fn write(dir: &Path, name: &str, mesh: &Mesh, describe: &str, volume: f64) -> std::io::Result<()> {
    std::fs::write(dir.join(format!("{name}.stl")), stl::write_binary(mesh))?;
    println!("{name}.stl: {describe}: volume after rebuild {volume:.6} mm³");
    Ok(())
}
