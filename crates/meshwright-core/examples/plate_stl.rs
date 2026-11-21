//! Writes the STL fixture of SolidCraft's plugin test `plugin/meshwright/tests/plate-holes.json`:
//! a 70 x 40 x 6 mm plate with three through holes and 3 mm rounded corners, tessellated as a CAD
//! exporter would, and prints the volume SolidCraft should measure after rebuilding it.
//!
//! ```sh
//! cargo run -p meshwright-core --example plate_stl > plugin/meshwright/tests/plate-holes.stl
//! ```

use std::io::Write as _;

use meshwright_core::recognize::{HoleSpec, PlateSpec};
use meshwright_core::{stl, synth};

/// The plate in the fixture.
fn spec() -> PlateSpec {
    PlateSpec {
        x: 70.0,
        y: 40.0,
        z: 6.0,
        holes: vec![HoleSpec { x: 10.0, y: 10.0, d: 4.5 }, HoleSpec { x: 35.0, y: 20.0, d: 8.0 }, HoleSpec { x: 60.0, y: 30.0, d: 4.5 }],
        outline_fillet_r: Some(3.0),
    }
}

fn main() -> std::io::Result<()> {
    let spec = spec();
    std::io::stdout().write_all(&stl::write_binary(&synth::plate(&spec, 48, [0.0; 3])))?;
    eprintln!("{}: volume after rebuild {:.6} mm³", spec.describe(), spec.volume());
    Ok(())
}
