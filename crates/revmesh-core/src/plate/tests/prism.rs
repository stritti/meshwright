use std::f64::consts::PI;

use crate::mesh::{Mesh, cross, sub};
use crate::recognize::{Axis, HoleSpec, PlateSpec, Recognition, recognize};
use crate::synth;

use super::{P2, ell, found, reason, spec};

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

/// An L-bracket exported from SolidCraft 0.48 (`file.export_stl`, standard quality): walls in
/// bands, top and bottom faces meshed apart, arcs and rims of uneven facets.
#[test]
fn recognises_solidcraft_exports() {
    let mesh = crate::stl::parse(include_bytes!("../../../../../plugin/revmesh/tests/solidcraft-l-bracket.stl")).expect("parses");
    let Recognition::Profile { profile, offset, .. } = recognize(&mesh) else { panic!("not recognised: {:?}", recognize(&mesh)) };
    assert_eq!(offset, [20.0, 10.0, 0.0]);
    assert_eq!(profile.corners, [[0.0, 0.0], [60.0, 0.0], [60.0, 15.0], [15.0, 15.0], [15.0, 40.0], [0.0, 40.0]]);
    assert_eq!(
        profile.holes,
        [HoleSpec { x: 7.5, y: 30.0, d: 5.0, head: None, blind: None }, HoleSpec { x: 45.0, y: 7.5, d: 6.0, head: None, blind: None }]
    );
    assert_eq!((profile.z, profile.outline_fillet_r), (5.0, Some(2.0)));
}

/// A plate standing along Y (turned by `(x, y, z) -> (x, z, -y)`) is recognised as an outline
/// extruded along Y, as mesh2cad's extrusion module recognises it; seen along Z it is a strip
/// with a hole through its side, no prism over its top face.
#[test]
fn standing_plate_is_recognised_along_y() {
    let mesh = synth::plate(&spec(&[(40.0, 25.0, 10.0)], None), 32, [0.0; 3]);
    let turned = Mesh::new(mesh.triangles.iter().map(|t| t.map(|p| [p[0], p[2], -p[1]])).collect());
    let Recognition::Profile { profile, offset, axis } = recognize(&turned) else { panic!("not recognised: {:?}", recognize(&turned)) };
    assert_eq!(axis, Axis::Y);
    assert_eq!(offset, [0.0; 3]);
    assert_eq!(profile.corners, [[0.0, 0.0], [80.0, 0.0], [80.0, 50.0], [0.0, 50.0]]);
    assert_eq!(profile.z, 5.0);
    assert_eq!(profile.holes, [HoleSpec { x: 40.0, y: 25.0, d: 10.0, head: None, blind: None }]);
}

#[test]
fn mesh_with_a_gap_is_not_recognised() {
    let mut mesh = synth::plate(&spec(&[(40.0, 25.0, 10.0)], Some(3.0)), 32, [0.0; 3]);
    mesh.triangles.remove(0);
    reason(&mesh);
}

/// A box with all twelve edges rounded with one radius (mesh2cad's `edge_fillet_r`): the top
/// and bottom faces are the inner rectangle, and the rim is round, so the plate is the full box
/// with `edge_fillet_r`.
#[test]
fn rounded_box_is_recognised_with_all_edges_filleted() {
    let expected = PlateSpec {
        x: 60.0,
        y: 40.0,
        z: 30.0,
        holes: vec![],
        slots: vec![],
        bosses: vec![],
        pads: vec![],
        pockets: vec![],
        cutouts: vec![],
        outline_fillet_r: None,
        edge_fillet_r: Some(5.0),
    };
    let (plate, offset) = found(&synth::rounded_box(60.0, 40.0, 30.0, 5.0, 32, [0.0; 3]));
    assert_eq!(plate, expected);
    assert_eq!(offset, [0.0; 3]);
}

#[test]
fn rounded_box_survives_single_precision_files() {
    let mesh = synth::rounded_box(60.0, 40.0, 30.0, 5.0, 48, [0.0; 3]);
    let reread = crate::stl::parse(&crate::stl::write_binary(&mesh)).expect("round trip");
    let (plate, _) = found(&reread);
    assert_eq!((plate.x, plate.y, plate.z, plate.edge_fillet_r), (60.0, 40.0, 30.0, Some(5.0)));
}

#[test]
fn rounded_box_moves_an_offset_to_the_origin() {
    let (plate, offset) = found(&synth::rounded_box(60.0, 40.0, 30.0, 4.0, 32, [10.0, 20.0, 3.0]));
    assert_eq!((plate.x, plate.y, plate.z, plate.edge_fillet_r), (60.0, 40.0, 30.0, Some(4.0)));
    assert_eq!(offset, [10.0, 20.0, 3.0]);
}

/// A box with all twelve edges rounded and through openings along Z clear of the rounds,
/// tessellated the way an exporter does it: the caps are the inner rectangle with the openings'
/// loops, the rim as in [`synth::rounded_box`], the openings' walls vertical. Each opening is a
/// counter-clockwise loop (a hole's circle, a cut-out's polygon).
fn rounded_box_with_openings(x: f64, y: f64, z: f64, r: f64, openings: &[Vec<P2>], segments: usize) -> Mesh {
    let n = segments.max(1);
    let corners = [[r, r], [x - r, r], [x - r, y - r], [r, y - r]];
    let mut ring: Vec<P2> = Vec::with_capacity(4 * n);
    for i in 0..4 {
        let (p, q) = (corners[i], corners[(i + 1) % 4]);
        for j in 0..n {
            let t = j as f64 / n as f64;
            ring.push([p[0] + t * (q[0] - p[0]), p[1] + t * (q[1] - p[1])]);
        }
    }
    let mut mesh = synth::rounded_box(x, y, z, r, segments, [0.0; 3]);
    // The top and bottom caps come first in rounded_box's triangles; replace them with holed ones.
    mesh.triangles.drain(..2 * (4 * n - 2));
    for [a, b, c] in synth::triangulate(&ring, openings) {
        mesh.triangles.push([[a[0], a[1], z], [b[0], b[1], z], [c[0], c[1], z]]);
        mesh.triangles.push([[a[0], a[1], 0.0], [c[0], c[1], 0.0], [b[0], b[1], 0.0]]);
    }
    // Each opening's wall, wound as prism() winds a hole's.
    for opening in openings {
        let rev: Vec<P2> = opening.iter().rev().copied().collect();
        for i in 0..rev.len() {
            let (a, b) = (rev[i], rev[(i + 1) % rev.len()]);
            mesh.triangles.push([[a[0], a[1], 0.0], [b[0], b[1], 0.0], [b[0], b[1], z]]);
            mesh.triangles.push([[a[0], a[1], 0.0], [b[0], b[1], z], [a[0], a[1], z]]);
        }
    }
    mesh
}

/// A round opening's loop: `segments` corners on the circle about `(x, y)` of diameter `d`.
fn circle_loop(x: f64, y: f64, d: f64, segments: usize) -> Vec<P2> {
    (0..segments).map(|i| 2.0 * PI * i as f64 / segments as f64).map(|a| [x + d / 2.0 * a.cos(), y + d / 2.0 * a.sin()]).collect()
}

/// A rounded box with a through hole clear of the rounds is the box with `edge_fillet_r` and the
/// hole, as mesh2cad's `_edge_fillet_with_holes` recognises it.
#[test]
fn rounded_box_with_a_hole_is_recognised() {
    let expected = PlateSpec {
        x: 60.0,
        y: 40.0,
        z: 30.0,
        holes: vec![HoleSpec { x: 30.0, y: 20.0, d: 8.0, head: None, blind: None }],
        slots: vec![],
        bosses: vec![],
        pads: vec![],
        pockets: vec![],
        cutouts: vec![],
        outline_fillet_r: None,
        edge_fillet_r: Some(5.0),
    };
    let (plate, offset) = found(&rounded_box_with_openings(60.0, 40.0, 30.0, 5.0, &[circle_loop(30.0, 20.0, 8.0, 32)], 32));
    assert_eq!(plate, expected);
    assert_eq!(offset, [0.0; 3]);
}

/// A finely tessellated rounded box still reads its rim radius exactly: the near-tangent rim
/// facets that leak into the top cap bulge the outline out by their arcs' radius, which the
/// detection adds back.
#[test]
fn finely_tessellated_rounded_box_is_recognised() {
    let (plate, _) = found(&synth::rounded_box(60.0, 40.0, 30.0, 5.0, 128, [0.0; 3]));
    assert_eq!((plate.x, plate.y, plate.z, plate.outline_fillet_r, plate.edge_fillet_r), (60.0, 40.0, 30.0, None, Some(5.0)));
    let (plate, _) = found(&synth::rounded_box(60.0, 40.0, 30.0, 5.0, 256, [0.0; 3]));
    assert_eq!(plate.edge_fillet_r, Some(5.0));
}

/// A dent whose corner projects onto the inner rectangle must not pass as a rim wall: the inset
/// outline's segments are no walls of the rounded box.
#[test]
fn dent_projecting_onto_the_inner_outline_is_not_recognised() {
    let mut mesh = synth::rounded_box(60.0, 40.0, 30.0, 1.0, 16, [0.0; 3]);
    let t = mesh.triangles.iter_mut().find(|t| t.iter().all(|p| p[0] == 0.0)).expect("a flat side face");
    t[0] = [1.0, 1.0, 15.0];
    reason(&mesh);
}

/// A rounded box with a polygonal cut-out is not recognised: the profile branch has no
/// `edge_fillet_r`, so the rim must not be silently dropped.
#[test]
fn rounded_box_with_a_cutout_is_not_recognised() {
    let cutout = vec![[250.0, 150.0], [350.0, 150.0], [300.0, 250.0]];
    let mesh = rounded_box_with_openings(600.0, 400.0, 30.0, 0.5, std::slice::from_ref(&cutout), 32);
    reason(&mesh);
}

/// A dent in the rounded rim is no rounded box: the corner between the faces lies off the
/// surface at distance `r` from the inner box.
#[test]
fn rounded_box_with_a_dent_is_not_recognised() {
    let mut mesh = synth::rounded_box(60.0, 40.0, 30.0, 5.0, 16, [0.0; 3]);
    let t = mesh.triangles.iter_mut().find(|t| t.iter().all(|p| p[2] > 1.0 && p[2] < 29.0)).expect("a wall facet between the faces");
    let inward = [30.0 - t[0][0], 20.0 - t[0][1]];
    let len = inward[0].hypot(inward[1]);
    t[0][0] += inward[0] / len * 0.2;
    t[0][1] += inward[1] / len * 0.2;
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
    let b = PlateSpec {
        x: 60.0,
        y: 40.0,
        z: 30.0,
        holes: vec![],
        slots: vec![],
        bosses: vec![],
        pads: vec![],
        pockets: vec![],
        cutouts: vec![],
        outline_fillet_r: None,
        edge_fillet_r: Some(5.0),
    };
    let mesh = synth::rounded_box(60.0, 40.0, 30.0, 5.0, 128, [0.0; 3]);
    assert!((mesh.volume() - b.volume()).abs() / b.volume() < 1e-3, "{} vs {}", mesh.volume(), b.volume());
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
