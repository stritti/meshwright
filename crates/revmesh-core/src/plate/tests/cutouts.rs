use std::f64::consts::PI;

use crate::plate::P2;
use crate::synth;

use super::{found_profile, plate_with_cutout, reason, spec, stadium};

/// A D-shaped cut-out (half a circle closed by its diameter) has every corner on a circle;
/// it must not become a round hole.
#[test]
fn d_shaped_cut_out_is_not_a_hole() {
    let outline = [[0.0, 0.0], [80.0, 0.0], [80.0, 50.0], [0.0, 50.0]];
    let half: Vec<P2> = (0..=32).map(|i| f64::from(i) * PI / 32.0).map(|a| [40.0 + 2.0 * a.cos(), 25.0 + 2.0 * a.sin()]).collect();
    assert!(reason(&synth::prism(&outline, &[half], 5.0, [0.0; 3])).contains("not round"));
    // A shaft's D-hole: a full circle with a flat cutting off a 74° arc.
    let flat = (0.8_f64).acos();
    let mut shaft: Vec<P2> = (0..=27)
        .map(|i| -PI + flat + f64::from(i) * (2.0 * PI - 2.0 * flat) / 27.0)
        .map(|a| [40.0 + 2.5 * a.cos(), 25.0 + 2.5 * a.sin()])
        .collect();
    shaft.rotate_left(14);
    assert!(reason(&synth::prism(&outline, &[shaft], 5.0, [0.0; 3])).contains("not round"));
}

/// A hole of six facets is no round hole but a hexagonal cut-out, rebuilt as it is.
#[test]
fn hexagonal_cut_out_is_not_a_hole() {
    let (plate, _) = found_profile(&synth::plate(&spec(&[(40.0, 25.0, 10.0)], None), 6, [0.0; 3]));
    assert_eq!(plate.holes, []);
    assert_eq!(plate.cutouts.len(), 1);
    assert_eq!(plate.cutouts[0].len(), 6);
    assert_eq!(plate.cutouts[0][0], [37.5, 20.67]);
}

/// A 70 x 40 x 6 plate exported from SolidCraft 0.47 with a 16 x 20 rectangular and a
/// hexagonal cut-out (7 mm to its corners) and an Ø5 hole.
#[test]
fn recognises_polygonal_cut_outs() {
    let mesh = crate::stl::parse(include_bytes!("../../../../../plugin/revmesh/tests/cutouts.stl")).expect("parses");
    let (plate, _) = found_profile(&mesh);
    assert_eq!(plate.corners, [[0.0, 0.0], [70.0, 0.0], [70.0, 40.0], [0.0, 40.0]]);
    assert_eq!(plate.cutouts.len(), 2);
    assert_eq!(plate.cutouts[0], [[8.0, 10.0], [24.0, 10.0], [24.0, 30.0], [8.0, 30.0]]);
    assert_eq!(plate.cutouts[1].len(), 6);
    // SolidCraft measured 13,998.356 mm³; the hexagon's corners are rounded to 0.01 mm.
    assert!((plate.volume() - 13_998.356).abs() < 0.05, "{}", plate.volume());
}

/// Cut-outs of straight sides: a rectangle, a triangle, an L-shape, and a slot whose ends
/// are three facets each. A curve of many gentle corners is no polygon.
#[test]
fn polygonal_cut_outs() {
    let rectangle = vec![[10.0, 10.0], [25.0, 10.0], [25.0, 30.0], [10.0, 30.0]];
    let triangle = vec![[40.0, 10.0], [60.0, 12.0], [48.0, 30.0]];
    let ell = vec![[30.0, 35.0], [50.0, 35.0], [50.0, 40.0], [35.0, 40.0], [35.0, 45.0], [30.0, 45.0]];
    let outline = [[0.0, 0.0], [80.0, 0.0], [80.0, 50.0], [0.0, 50.0]];
    let mesh = synth::prism(&outline, &[rectangle.clone(), triangle.clone(), ell.clone()], 6.0, [0.0; 3]);
    let (plate, _) = found_profile(&mesh);
    assert_eq!(plate.cutouts, [rectangle, triangle, ell]);
    assert!((plate.volume() - mesh.volume()).abs() < 1e-6);
    let (plate, _) = found_profile(&plate_with_cutout(stadium([40.0, 25.0], 18.0, 7.0, 0.0, 3)));
    assert_eq!(plate.cutouts[0].len(), 8);
    let gentle: Vec<P2> = (0..30).map(|i| 2.0 * PI * f64::from(i) / 30.0).map(|a| [40.0 + 12.0 * a.cos(), 25.0 + 6.0 * a.sin()]).collect();
    reason(&plate_with_cutout(gentle));
}
