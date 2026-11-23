use crate::mesh::{Mesh, cross, sub};
use crate::recognize::{HoleSpec, Recognition, recognize};
use crate::synth;

use super::{ell, found, reason, spec};

#[test]
fn recognises_holes_and_rounded_corners() {
    let expected = spec(&[(10.0, 10.0, 5.0), (40.0, 25.0, 12.0), (70.0, 38.5, 6.6)], Some(4.0));
    let (plate, offset) = found(&synth::plate(&expected, 32, [0.0; 3]));
    assert_eq!(plate, expected);
    assert_eq!(offset, [0.0; 3]);
}

#[test]
fn recognises_sharp_corners_and_coarse_holes() {
    let expected = spec(&[(20.0, 25.0, 8.0), (60.0, 25.0, 8.0)], None);
    assert_eq!(found(&synth::plate(&expected, 12, [0.0; 3])).0, expected);
}

#[test]
fn survives_single_precision_files() {
    let expected = spec(&[(12.5, 30.0, 3.4), (55.0, 20.0, 9.0)], Some(2.5));
    let mesh = synth::plate(&expected, 48, [0.0; 3]);
    let reread = crate::stl::parse(&crate::stl::write_binary(&mesh)).expect("round trip");
    assert_eq!(found(&reread).0, expected);
}

#[test]
fn moves_an_offset_plate_to_the_origin() {
    let expected = spec(&[(40.0, 25.0, 10.0)], Some(5.0));
    let (plate, offset) = found(&synth::plate(&expected, 32, [-40.0, -25.0, 3.0]));
    assert_eq!(plate, expected);
    assert_eq!(offset, [-40.0, -25.0, 3.0]);
}

/// Far from the origin single precision leaves hole corners about 0.2 µm off their circle;
/// rounding the hole's numbers must not add to that when the walls are checked.
#[test]
fn plates_far_from_the_origin() {
    let expected = spec(&[(20.0, 25.0, 8.0), (60.0, 25.0, 5.0)], Some(3.0));
    let mesh = synth::plate(&expected, 48, [4000.0, 4000.0, 0.0]);
    let reread = crate::stl::parse(&crate::stl::write_binary(&mesh)).expect("round trip");
    let (plate, offset) = found(&reread);
    assert_eq!(offset, [4000.0, 4000.0, 0.0]);
    assert_eq!(plate.holes.len(), 2);
}

/// An L-bracket exported from SolidCraft 0.45 (`file.export_stl`, standard quality): walls in
/// bands, top and bottom faces meshed apart, arcs and rims of uneven facets.
#[test]
fn recognises_solidcraft_exports() {
    let mesh = crate::stl::parse(include_bytes!("../../../../../plugin/revmesh/tests/solidcraft-l-bracket.stl")).expect("parses");
    let Recognition::Profile { profile, offset } = recognize(&mesh) else { panic!("not recognised: {:?}", recognize(&mesh)) };
    assert_eq!(offset, [20.0, 10.0, 0.0]);
    assert_eq!(profile.corners, [[0.0, 0.0], [60.0, 0.0], [60.0, 15.0], [15.0, 15.0], [15.0, 40.0], [0.0, 40.0]]);
    assert_eq!(
        profile.holes,
        [HoleSpec { x: 7.5, y: 30.0, d: 5.0, head: None, blind: None }, HoleSpec { x: 45.0, y: 7.5, d: 6.0, head: None, blind: None }]
    );
    assert_eq!((profile.z, profile.outline_fillet_r), (5.0, Some(2.0)));
}

#[test]
fn standing_plate_is_not_recognised() {
    let mesh = synth::plate(&spec(&[(40.0, 25.0, 10.0)], None), 32, [0.0; 3]);
    let turned = Mesh::new(mesh.triangles.iter().map(|t| t.map(|p| [p[0], p[2], -p[1]])).collect());
    // Seen along Z it is a strip with a hole through its side: no prism over its top face.
    let why = reason(&turned);
    assert!(why.contains("not a flat plate") || why.contains("faces differ"), "{why}");
}

#[test]
fn mesh_with_a_gap_is_not_recognised() {
    let mut mesh = synth::plate(&spec(&[(40.0, 25.0, 10.0)], Some(3.0)), 32, [0.0; 3]);
    mesh.triangles.remove(0);
    reason(&mesh);
}

#[test]
fn synthetic_volumes_match_the_specs() {
    let s = spec(&[(40.0, 25.0, 10.0)], Some(5.0));
    let mesh = synth::plate(&s, 128, [0.0; 3]);
    assert!((mesh.volume() - s.volume()).abs() / s.volume() < 1e-3);
    let l = ell(Some(2.0));
    let mesh = synth::profile(&l, 128, [0.0; 3]);
    assert!((mesh.volume() - l.volume()).abs() / l.volume() < 1e-3);
}

#[test]
#[allow(clippy::float_cmp)] // the generator puts every top corner at exactly the same height
fn synthetic_top_faces_are_not_folded() {
    let plate = synth::plate(&spec(&[(10.0, 10.0, 5.0), (40.0, 25.0, 12.0), (70.0, 38.5, 6.6)], Some(4.0)), 32, [0.0; 3]);
    let profile = synth::profile(&ell(Some(2.0)), 32, [0.0; 3]);
    for mesh in [plate, profile] {
        for [a, b, c] in &mesh.triangles {
            let n = cross(sub(*b, *a), sub(*c, *a));
            if a[2] == b[2] && b[2] == c[2] {
                assert!(n[2].abs() > 1e-9, "degenerate cap triangle");
                assert_eq!(n[2] > 0.0, a[2] > 0.0, "cap triangle facing the wrong way");
            }
        }
    }
}
