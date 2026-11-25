use std::f64::consts::PI;

use crate::mesh::{Mesh, cross, sub};
use crate::recognize::{Axis, HoleSpec, PlateSpec, Recognition, recognize};
use crate::synth;

use super::{P2, banded, ell, found, reason, spec, stadium};

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

/// An L-bracket exported from SolidCraft 0.49 (`file.export_stl`, standard quality): walls in
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

/// The per-face deviation report measures a coarse hole's chord middles, which lie several grid
/// cells inside the circle: a centred radius-16 hole in a 100 x 100 plate, faceted into eight,
/// has its chord middles about 1.218 mm inside its circle.
#[test]
fn deviation_of_a_coarse_hole_is_its_sagitta() {
    let s = PlateSpec {
        x: 100.0,
        y: 100.0,
        z: 6.0,
        holes: vec![HoleSpec { x: 50.0, y: 50.0, d: 32.0, head: None, blind: None }],
        cutouts: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![],
        slots: vec![],
        outline_fillet_r: None,
        edge_fillet_r: None,
    };
    let deviations = crate::plate::face_deviations(&synth::plate(&s, 8, [0.0; 3]));
    let sagitta = 16.0 * (1.0 - (PI / 8.0).cos());
    let hole = deviations.iter().find(|(label, _)| label == "hole 1").unwrap_or_else(|| panic!("no hole 1 in {deviations:?}"));
    assert!((hole.1 - sagitta).abs() < 0.05, "hole 1 lies {} mm from its circle, its chord's sagitta is {sagitta} mm", hole.1);
}

/// The deviation report does not depend on where in the file the part lies: `split_faces` keeps
/// its points relative to the mesh's lowest corner, and the wall index is in the same frame.
#[test]
fn deviation_report_ignores_where_the_part_lies() {
    let s = spec(&[(30.0, 20.0, 10.0)], None);
    let at_origin = banded(&synth::plate(&s, 24, [0.0; 3]), 0.2, [40.0, 25.0]);
    let moved = banded(&synth::plate(&s, 24, [4000.0, 4000.0, 100.0]), 0.2, [4040.0, 4025.0]);
    let here = crate::plate::face_deviations(&at_origin);
    let there = crate::plate::face_deviations(&moved);
    // Same labels, same deviations (to single precision's rounding at the far offset).
    assert_eq!(here.len(), there.len());
    for ((label, d), (_, other)) in here.iter().zip(&there) {
        assert!((d - other).abs() < 1e-9, "{label}: {d} vs {other}");
    }
    // The banded walls' middle corners bulge 0.2 mm inward, and the report says so.
    let hole = here.iter().find(|(label, _)| label == "hole 1").expect("the hole is measured");
    assert!(hole.1 > 0.1, "hole 1: {}", hole.1);
}

/// A box rounded all over is measured against its rounded envelope: one "rounded surface" label
/// with the farthest its tessellated surface lies from it (a facet's sagitta, roadmap 5.4).
#[test]
fn deviation_of_a_rounded_box_measures_its_rounded_surface() {
    let mesh = synth::rounded_box(60.0, 40.0, 30.0, 5.0, 16, [0.0; 3]);
    let deviations = crate::plate::face_deviations(&mesh);
    let surface = deviations.iter().find(|(l, _)| l == "rounded surface").unwrap_or_else(|| panic!("no rounded surface in {deviations:?}"));
    // The corner octants' facets bow inward by about the rim radius's sagitta on their chords.
    assert!(surface.1 > 0.05 && surface.1 < 0.2, "rounded surface: {surface:?}");
    assert_eq!(deviations.len(), 1, "one label for the whole rounded surface: {deviations:?}");
}

/// Each slot's two end arcs are labelled with the slot's number in the design (`slot_<n>`), the
/// two ends counted within it — and each end's deviation is its own slot's sagitta.
#[test]
fn deviation_labels_slot_ends_with_their_slot() {
    let mesh = synth::prism(
        &[[0.0, 0.0], [80.0, 0.0], [80.0, 50.0], [0.0, 50.0]],
        &[stadium([25.0, 15.0], 20.0, 6.0, 0.0, 8), stadium([55.0, 35.0], 16.0, 10.0, 90.0, 8)],
        6.0,
        [0.0; 3],
    );
    let deviations = crate::plate::face_deviations(&mesh);
    let labels: Vec<&str> = deviations.iter().map(|(label, _)| label.as_str()).collect();
    for expected in ["slot 1 end 1", "slot 1 end 2", "slot 2 end 1", "slot 2 end 2"] {
        assert!(labels.contains(&expected), "{expected} in {labels:?}");
    }
    assert!(!labels.iter().any(|l| l.starts_with("slot 3") || l.starts_with("slot 4")), "{labels:?}");
    // Slot 1 (at y = 15) is 6 mm wide, slot 2 (at y = 35) 10 mm; each end is a half circle of
    // eight facets, and its chords bow inward by their sagitta. The straight sides are indexed
    // too (codex finding on #74) and measure zero.
    let sagitta = |w: f64| w / 2.0 * (1.0 - (PI / 16.0).cos());
    for (label, d) in &deviations {
        if label.contains(" side ") {
            assert!(*d < 1e-9, "{label}: {d}");
        } else if label.starts_with("slot 1") {
            assert!((d - sagitta(6.0)).abs() < 0.05, "{label}: {d}");
        } else if label.starts_with("slot 2") {
            assert!((d - sagitta(10.0)).abs() < 0.05, "{label}: {d}");
        }
    }
}

/// A through hole to the right of a blind hole is the design's `hole_2`, not `hole_1`: each
/// circle's wall label carries its hole's index in the sorted hole specifications (codex
/// finding on #74).
#[test]
fn deviation_labels_a_through_hole_with_its_design_number() {
    let (z, floor) = (6.0, 3.0);
    let outline = [[0.0, 0.0], [60.0, 0.0], [60.0, 40.0], [0.0, 40.0]];
    let through_ring = synth::rings(&[HoleSpec { x: 45.0, y: 20.0, d: 8.0, head: None, blind: None }], 16).remove(0);
    let blind_ring = synth::rings(&[HoleSpec { x: 15.0, y: 20.0, d: 6.0, head: None, blind: None }], 16).remove(0);
    let lift = |p: P2, h: f64| [p[0], p[1], h];
    let mut t: Vec<crate::mesh::Triangle> = Vec::new();
    for [a, b, c] in synth::triangulate(&outline, &[through_ring.clone(), blind_ring.clone()]) {
        t.push([lift(a, z), lift(b, z), lift(c, z)]);
    }
    for [a, b, c] in synth::triangulate(&outline, std::slice::from_ref(&through_ring)) {
        t.push([lift(a, 0.0), lift(c, 0.0), lift(b, 0.0)]);
    }
    for i in 0..4 {
        let (a, b) = (outline[i], outline[(i + 1) % 4]);
        t.push([lift(a, 0.0), lift(b, 0.0), lift(b, z)]);
        t.push([lift(a, 0.0), lift(b, z), lift(a, z)]);
    }
    let wall = |ring: &[P2], lo: f64, hi: f64, t: &mut Vec<crate::mesh::Triangle>| {
        let m = ring.len();
        for i in 0..m {
            let (a, b) = (ring[(i + 1) % m], ring[i]);
            t.push([lift(a, lo), lift(b, lo), lift(b, hi)]);
            t.push([lift(a, lo), lift(b, hi), lift(a, hi)]);
        }
    };
    wall(&through_ring, 0.0, z, &mut t);
    wall(&blind_ring, floor, z, &mut t);
    let m = blind_ring.len();
    for i in 1..m - 1 {
        t.push([lift(blind_ring[0], floor), lift(blind_ring[i], floor), lift(blind_ring[i + 1], floor)]);
    }
    let mesh = Mesh::new(t);
    assert!(mesh.volume() > 0.0, "mesh wound inside out");
    let deviations = crate::plate::face_deviations(&mesh);
    let labels: Vec<&str> = deviations.iter().map(|(l, _)| l.as_str()).collect();
    assert!(labels.contains(&"hole 2"), "the through hole is the design's hole 2: {labels:?}");
    assert!(!labels.contains(&"hole 1"), "no through hole mislabelled hole 1: {labels:?}");
}

/// A polygonal cut-out's straight walls are indexed in the deviation report, so its samples are
/// attributed to `cut-out 1, side i` with their real (here zero) deviation, not to distant
/// outline sides (codex finding on #74).
#[test]
fn deviation_of_a_polygonal_cutout_is_measured_on_its_own_walls() {
    let outline = [[0.0, 0.0], [60.0, 0.0], [60.0, 40.0], [0.0, 40.0]];
    let cut = vec![[20.0, 10.0], [40.0, 10.0], [40.0, 30.0], [20.0, 30.0]];
    let mesh = synth::prism(&outline, &[cut], 6.0, [0.0; 3]);
    let deviations = crate::plate::face_deviations(&mesh);
    let labels: Vec<&str> = deviations.iter().map(|(l, _)| l.as_str()).collect();
    assert!(labels.iter().any(|l| l.starts_with("cut-out 1, side")), "the cut-out's walls are indexed: {labels:?}");
    for (l, d) in &deviations {
        if l.starts_with("cut-out") {
            assert!(*d < 1e-6, "{l}: {d}");
        }
    }
}

/// A blind hole's floor and bore samples have no indexed surface: they are excluded from the
/// deviation report instead of being attributed to distant outer walls (codex finding on #74).
#[test]
fn deviation_of_a_blind_holes_floor_is_not_attributed_to_outer_walls() {
    let (z, floor) = (5.0, 2.5);
    let outline = [[0.0, 0.0], [200.0, 0.0], [200.0, 200.0], [0.0, 200.0]];
    let ring = synth::rings(&[HoleSpec { x: 100.0, y: 100.0, d: 2.0, head: None, blind: None }], 16).remove(0);
    let lift = |p: P2, h: f64| [p[0], p[1], h];
    let mut t: Vec<crate::mesh::Triangle> = Vec::new();
    for [a, b, c] in synth::triangulate(&outline, std::slice::from_ref(&ring)) {
        t.push([lift(a, z), lift(b, z), lift(c, z)]);
    }
    for [a, b, c] in synth::triangulate(&outline, &[]) {
        t.push([lift(a, 0.0), lift(c, 0.0), lift(b, 0.0)]);
    }
    for i in 0..4 {
        let (a, b) = (outline[i], outline[(i + 1) % 4]);
        t.push([lift(a, 0.0), lift(b, 0.0), lift(b, z)]);
        t.push([lift(a, 0.0), lift(b, z), lift(a, z)]);
    }
    let m = ring.len();
    for i in 0..m {
        let (a, b) = (ring[(i + 1) % m], ring[i]);
        t.push([lift(a, floor), lift(b, floor), lift(b, z)]);
        t.push([lift(a, floor), lift(b, z), lift(a, z)]);
    }
    for i in 1..m - 1 {
        t.push([lift(ring[0], floor), lift(ring[i], floor), lift(ring[i + 1], floor)]);
    }
    let mesh = Mesh::new(t);
    assert!(mesh.volume() > 0.0, "mesh wound inside out");
    let deviations = crate::plate::face_deviations(&mesh);
    for (label, d) in &deviations {
        if label.starts_with("side") {
            assert!(*d < 0.01, "the blind hole's samples must not inflate {label}: {d}");
        }
    }
}

/// A chord's middle is measured against the wall owning its ends: a hole beside a rounded
/// corner must not steal the corner's chord (codex finding on #74). A 100 x 100 plate with R16
/// corners and a D13.4 hole at (10, 10), eight segments per circle: the corner's chords bow
/// 1.218 mm, the hole's 0.510 mm.
#[test]
fn deviation_of_a_chord_stays_with_its_owning_wall() {
    let s = PlateSpec {
        x: 100.0,
        y: 100.0,
        z: 6.0,
        holes: vec![HoleSpec { x: 10.0, y: 10.0, d: 13.4, head: None, blind: None }],
        cutouts: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![],
        slots: vec![],
        outline_fillet_r: Some(16.0),
        edge_fillet_r: None,
    };
    let deviations = crate::plate::face_deviations(&synth::plate(&s, 8, [0.0; 3]));
    let corner = deviations.iter().find(|(l, _)| l.contains("round")).unwrap_or_else(|| panic!("no corner round in {deviations:?}"));
    let want_corner = 16.0 * (1.0 - (PI / 8.0).cos());
    assert!((corner.1 - want_corner).abs() < 0.05, "the corner keeps its chord's sagitta {want_corner}: {corner:?}");
    let hole = deviations.iter().find(|(l, _)| l == "hole 1").unwrap_or_else(|| panic!("no hole 1 in {deviations:?}"));
    let want_hole = 6.7 * (1.0 - (PI / 8.0).cos());
    assert!((hole.1 - want_hole).abs() < 0.05, "the hole measures its own sagitta {want_hole}, not the corner's: {hole:?}");
}

/// A bore whose top rim is tessellated more coarsely than its bottom rim has its largest
/// deviation on the top chords, which `split_faces` skips (both endpoints on the top face); the
/// report samples the top boundary loops' chords too (codex finding on #74). A centred radius-1
/// bore in a 100 x 100 x 6 plate: eight top vertices, sixteen bottom ones — the 45-degree top
/// chords bow 0.0761 mm, the 22.5-degree wall edges only 0.0192 mm.
#[test]
fn deviation_samples_the_top_rims_coarse_chords() {
    let (z, r) = (6.0, 1.0);
    let outline = [[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]];
    let bottom_ring: Vec<P2> = (0..16).map(|i| 2.0 * PI * f64::from(i) / 16.0).map(|a| [50.0 + r * a.cos(), 50.0 + r * a.sin()]).collect();
    let top_ring: Vec<P2> = (0..8).map(|j| bottom_ring[2 * j]).collect();
    let lift = |p: P2, h: f64| [p[0], p[1], h];
    let mut t: Vec<crate::mesh::Triangle> = Vec::new();
    for [a, b, c] in synth::triangulate(&outline, std::slice::from_ref(&top_ring)) {
        t.push([lift(a, z), lift(b, z), lift(c, z)]);
    }
    for [a, b, c] in synth::triangulate(&outline, std::slice::from_ref(&bottom_ring)) {
        t.push([lift(a, 0.0), lift(c, 0.0), lift(b, 0.0)]);
    }
    for i in 0..4 {
        let (a, b) = (outline[i], outline[(i + 1) % 4]);
        t.push([lift(a, 0.0), lift(b, 0.0), lift(b, z)]);
        t.push([lift(a, 0.0), lift(b, z), lift(a, z)]);
    }
    // The bore's wall from the 16-gon at the bottom to the 8-gon at the top: each top edge spans
    // two bottom edges.
    for j in 0..8 {
        let (p0, p1, p2) = (bottom_ring[2 * j], bottom_ring[2 * j + 1], bottom_ring[2 * (j + 1) % 16]);
        t.push([lift(p2, 0.0), lift(p1, 0.0), lift(p2, z)]);
        t.push([lift(p2, 0.0), lift(p2, z), lift(p0, z)]);
    }
    let mesh = Mesh::new(t);
    assert!(mesh.volume() > 0.0, "mesh wound inside out");
    let deviations = crate::plate::face_deviations(&mesh);
    let hole = deviations.iter().find(|(l, _)| l == "hole 1").unwrap_or_else(|| panic!("no hole 1 in {deviations:?}"));
    let want = r * (1.0 - (PI / 8.0).cos());
    assert!((hole.1 - want).abs() < 0.01, "hole 1 measures the coarse top chords' sagitta {want}: {hole:?}");
}

/// A box rounded all over with a through hole: the rounded surface is measured against the
/// envelope and the hole against its own circle, so neither is attributed to the other ~30 mm
/// away (codex finding on #74; roadmap 5.4).
#[test]
fn deviation_of_a_rounded_box_with_a_hole_separates_surface_and_hole() {
    let mesh = rounded_box_with_openings(60.0, 40.0, 30.0, 5.0, &[circle_loop(30.0, 20.0, 8.0, 32)], 32);
    let deviations = crate::plate::face_deviations(&mesh);
    let hole = deviations.iter().find(|(l, _)| l == "hole 1").unwrap_or_else(|| panic!("no hole 1 in {deviations:?}"));
    // The 32-gon's chords bow inward by their sagitta.
    let want = 4.0 * (1.0 - (PI / 32.0).cos());
    assert!((hole.1 - want).abs() < 0.01, "hole 1 measures its chords' sagitta {want}: {hole:?}");
    let surface = deviations.iter().find(|(l, _)| l == "rounded surface").unwrap_or_else(|| panic!("no rounded surface in {deviations:?}"));
    assert!(surface.1 < 0.1, "the rounded surface is not inflated by the hole: {surface:?}");
}

/// A step down from the top face is registered before measuring: the step's internal wall chords
/// (here the wall at x = 30) are excluded, not attributed to an outer side 20 mm away (codex
/// finding on #74).
#[test]
fn deviation_of_a_stepped_plates_internal_wall_is_excluded() {
    let deviations = crate::plate::face_deviations(&super::blocks::stepped_plate(None));
    for (label, d) in &deviations {
        if label.starts_with("side") {
            assert!(*d < 0.01, "the step's internal wall must not inflate {label}: {d}");
        }
    }
}

/// A blind hole drilled from the bottom face: its mouth's bottom-rim points are excluded like the
/// other sample kinds, not attributed to outer sides ~99 mm away (codex finding on #74). Same
/// mesh as the top blind hole above, mirrored through z.
#[test]
fn deviation_of_a_bottom_blind_holes_mouth_is_not_attributed_to_outer_walls() {
    let (z, floor) = (5.0, 2.5);
    let outline = [[0.0, 0.0], [200.0, 0.0], [200.0, 200.0], [0.0, 200.0]];
    let ring = synth::rings(&[HoleSpec { x: 100.0, y: 100.0, d: 2.0, head: None, blind: None }], 16).remove(0);
    let lift = |p: P2, h: f64| [p[0], p[1], h];
    let mut t: Vec<crate::mesh::Triangle> = Vec::new();
    for [a, b, c] in synth::triangulate(&outline, std::slice::from_ref(&ring)) {
        t.push([lift(a, z), lift(b, z), lift(c, z)]);
    }
    for [a, b, c] in synth::triangulate(&outline, &[]) {
        t.push([lift(a, 0.0), lift(c, 0.0), lift(b, 0.0)]);
    }
    for i in 0..4 {
        let (a, b) = (outline[i], outline[(i + 1) % 4]);
        t.push([lift(a, 0.0), lift(b, 0.0), lift(b, z)]);
        t.push([lift(a, 0.0), lift(b, z), lift(a, z)]);
    }
    let m = ring.len();
    for i in 0..m {
        let (a, b) = (ring[(i + 1) % m], ring[i]);
        t.push([lift(a, floor), lift(b, floor), lift(b, z)]);
        t.push([lift(a, floor), lift(b, z), lift(a, z)]);
    }
    for i in 1..m - 1 {
        t.push([lift(ring[0], floor), lift(ring[i], floor), lift(ring[i + 1], floor)]);
    }
    // Mirror the top blind hole through z (reversing each triangle to keep the winding): the
    // mouth moves to the bottom face.
    let mesh = Mesh::new(
        t.into_iter()
            .map(|[a, b, c]| {
                let m = |p: [f64; 3]| [p[0], p[1], z - p[2]];
                [m(c), m(b), m(a)]
            })
            .collect(),
    );
    assert!(mesh.volume() > 0.0, "mesh wound inside out");
    let deviations = crate::plate::face_deviations(&mesh);
    for (label, d) in &deviations {
        if label.starts_with("side") {
            assert!(*d < 0.01, "the bottom blind hole's mouth must not inflate {label}: {d}");
        }
    }
}

/// A clockwise chord (its endpoints sorted lexicographically) still measures against its owning
/// arc: with the hole at (10, 90) the hole reports its own 0.510 mm sagitta, not 0.965 mm, and
/// the corner keeps 1.218 mm (codex finding on #74).
#[test]
fn deviation_of_a_clockwise_chord_stays_with_its_own_wall() {
    let s = PlateSpec {
        x: 100.0,
        y: 100.0,
        z: 6.0,
        holes: vec![HoleSpec { x: 10.0, y: 90.0, d: 13.4, head: None, blind: None }],
        cutouts: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![],
        slots: vec![],
        outline_fillet_r: Some(16.0),
        edge_fillet_r: None,
    };
    let deviations = crate::plate::face_deviations(&synth::plate(&s, 8, [0.0; 3]));
    let hole = deviations.iter().find(|(l, _)| l == "hole 1").unwrap_or_else(|| panic!("no hole 1 in {deviations:?}"));
    let want_hole = 6.7 * (1.0 - (PI / 8.0).cos());
    assert!((hole.1 - want_hole).abs() < 0.05, "the hole measures its own sagitta {want_hole}, clockwise chord or not: {hole:?}");
    let corner = deviations.iter().find(|(l, _)| l.contains("round")).unwrap_or_else(|| panic!("no corner round in {deviations:?}"));
    let want_corner = 16.0 * (1.0 - (PI / 8.0).cos());
    assert!((corner.1 - want_corner).abs() < 0.05, "the corner keeps its chord's sagitta {want_corner}: {corner:?}");
}
