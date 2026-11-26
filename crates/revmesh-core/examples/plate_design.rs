//! Prints the design revmesh writes for a 70 x 40 x 6 mm plate with a centred 8 mm hole and
//! 3 mm rounded corners. This is the golden file `tests/data/plate-with-hole.json`:
//!
//! ```sh
//! cargo run -p revmesh-core --example plate_design > crates/revmesh-core/tests/data/plate-with-hole.json
//! ```
//!
//! Open the file in SolidCraft (File › Open) to see the history the importer produces.

use revmesh_core::recognize::{HoleSpec, PlateSpec};

fn main() {
    let spec = PlateSpec {
        x: 70.0,
        y: 40.0,
        z: 6.0,
        holes: vec![HoleSpec { x: 35.0, y: 20.0, d: 8.0, head: None, blind: None }],
        cutouts: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![],
        slots: vec![],
        outline_fillet_rs: vec![3.0; 4],
        edge_fillet_r: None,
    };
    println!("{}", spec.to_design("plate-with-hole").to_json_pretty());
}
