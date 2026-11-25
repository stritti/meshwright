use std::f64::consts::PI;

use crate::mesh::Mesh;
use crate::plate::P2;
use crate::recognize::SlotSpec;
use crate::synth;

use super::{banded, found, found_profile, plate_with_cutout, reason, stadium};

/// An 80 x 50 x 8 plate exported from SolidCraft 0.49 with three slots (along X, along Y and
/// at 30°) and a hole.
#[test]
fn recognises_slots() {
    let mesh = crate::stl::parse(include_bytes!("../../../../../plugin/revmesh/tests/slots.stl")).expect("parses");
    let (plate, _) = found(&mesh);
    let slot = |x: f64, y: f64, len: f64, w: f64, angle: f64| SlotSpec { x, y, len, w, angle };
    assert_eq!(plate.slots, [slot(25.0, 15.0, 20.0, 6.0, 0.0), slot(62.0, 30.0, 15.0, 8.0, 90.0), slot(30.0, 36.0, 16.0, 5.0, 30.0)]);
    assert_eq!(plate.holes.len(), 1);
    // SolidCraft measured 28,497.522 mm³ for the part it exported.
    assert!((plate.volume() - 28_497.522).abs() < 0.001, "{}", plate.volume());
}

/// Slots of a synthetic mesh at any angle, finely or coarsely faceted.
#[test]
fn synthetic_slots() {
    for (angle, n) in [(0.0, 16), (0.0, 15), (90.0, 8), (90.0, 9), (30.0, 12), (30.0, 13), (137.5, 24), (137.5, 7)] {
        let (plate, _) = found(&plate_with_cutout(stadium([40.0, 25.0], 18.0, 7.0, angle, n)));
        assert_eq!(plate.slots, [SlotSpec { x: 40.0, y: 25.0, len: 18.0, w: 7.0, angle }], "{angle} {n}");
    }
}

/// Cut-outs that look like slots but are not: rounded corners and an ellipse stay a mesh. An
/// unevenly faceted slot and one whose facets run long across to a short tip are no slots either
/// (the ends are not faceted as finely as a round hole's), but they are cut-outs of four arc
/// sides, and those are recognised now (roadmap 5.2).
#[test]
fn near_slots_are_refused() {
    let rounded = synth::rounded(&[[30.0, 22.0], [50.0, 22.0], [50.0, 28.0], [30.0, 28.0]], Some(1.5), 32);
    let ellipse: Vec<P2> = (0..48).map(|i| 2.0 * PI * f64::from(i) / 48.0).map(|a| [40.0 + 12.0 * a.cos(), 25.0 + 4.0 * a.sin()]).collect();
    let mut uneven = stadium([40.0, 25.0], 18.0, 7.0, 0.0, 16);
    for p in &mut uneven[..17] {
        p[1] = 25.0 + (p[1] - 25.0) * 1.2;
    }
    // Codex's: long facets from each side across to a short tip of the end.
    let full = stadium([40.0, 25.0], 18.0, 7.0, 0.0, 16);
    let mut cut: Vec<P2> = Vec::new();
    for (end, sign) in [(&full[..17], -1.0), (&full[17..], 1.0)] {
        for k in [0, 6, 7, 8, 9, 10, 16] {
            let mut p = end[k];
            if k == 0 || k == 16 {
                p[0] += sign * 4.0;
            }
            cut.push(p);
        }
    }
    // (A slot with ends of three facets is an octagonal cut-out, see `polygonal_cut_outs`.)
    for cutout in [rounded, ellipse, uneven] {
        let why = reason(&plate_with_cutout(cutout));
        assert!(why.contains("not round") || why.contains("corners"), "{why}");
    }
    // The slot with long facets across to a short tip is no slot, but its four sides lie on
    // circles: one cut-out of four arc sides.
    let (plate, _) = found_profile(&plate_with_cutout(cut));
    assert_eq!(plate.cutouts.len(), 1);
    assert_eq!(plate.cutouts[0].corners.len(), 4, "{:?}", plate.cutouts[0]);
    assert_eq!(plate.cutouts[0].arc_sides().iter().filter(|a| a.is_some()).count(), 4, "{:?}", plate.cutouts[0]);
}

/// Ends faceted more finely below the axis than above it: the middle comes from the straight
/// sides, not the loop's centroid.
#[test]
fn unevenly_faceted_slot_ends() {
    let full = stadium([40.0, 25.0], 18.0, 7.0, 0.0, 16);
    // Every other corner above the axis left out, the straight sides' ends kept.
    let uneven: Vec<P2> =
        full.iter().enumerate().filter(|(i, p)| p[1] <= 25.0 || p[1] >= 28.5 - 1e-9 || i % 2 == 0).map(|(_, p)| *p).collect();
    assert!(uneven.len() < full.len() - 6);
    let (plate, _) = found(&plate_with_cutout(uneven));
    assert_eq!(plate.slots, [SlotSpec { x: 40.0, y: 25.0, len: 18.0, w: 7.0, angle: 0.0 }]);
}

/// A slot's wall meshed in bands with one corner of a band folded into the slot is no wall:
/// the inner half of an end's circle lies inside the slot.
#[test]
#[allow(clippy::float_cmp)] // corners compared as the generator wrote them
fn a_slot_wall_folded_inward_is_refused() {
    let mesh = plate_with_cutout(stadium([40.0, 25.0], 18.0, 7.0, 0.0, 16));
    // Split every wall facet at Z = 3, then reflect the band corner of the right end's
    // second rim corner across X = 49 (the end's centre).
    let ring = stadium([40.0, 25.0], 18.0, 7.0, 0.0, 16);
    let target = ring[1];
    let banded = banded(&mesh, 0.0, [40.0, 25.0]);
    let mut moved = 0;
    let triangles = banded
        .triangles
        .iter()
        .map(|t| {
            t.map(|p| {
                if p[2] == 3.0 && (p[0] - target[0]).abs() < 1e-6 && (p[1] - target[1]).abs() < 1e-6 {
                    moved += 1;
                    [2.0 * 49.0 - p[0], p[1], p[2]]
                } else {
                    p
                }
            })
        })
        .collect();
    assert!(moved > 0);
    reason(&Mesh::new(triangles));
}

/// Top and bottom rims meshed apart, with eight facets per end on top and seven below: a
/// wall facet's edge across an end's tip still follows the end.
#[test]
fn slot_rims_meshed_apart() {
    let outline = [[0.0, 0.0], [80.0, 0.0], [80.0, 50.0], [0.0, 50.0]];
    let (top, bottom) = (stadium([40.0, 25.0], 18.0, 7.0, 0.0, 8), stadium([40.0, 25.0], 18.0, 7.0, 0.0, 7));
    let mut t: Vec<crate::mesh::Triangle> = Vec::new();
    let at = |p: P2, h: f64| [p[0], p[1], h];
    for [a, b, c] in synth::triangulate(&outline, std::slice::from_ref(&top)) {
        t.push([at(a, 6.0), at(b, 6.0), at(c, 6.0)]);
    }
    for [a, b, c] in synth::triangulate(&outline, std::slice::from_ref(&bottom)) {
        t.push([at(a, 0.0), at(c, 0.0), at(b, 0.0)]);
    }
    for i in 0..4 {
        let (a, b) = (outline[i], outline[(i + 1) % 4]);
        t.extend([[at(a, 0.0), at(b, 0.0), at(b, 6.0)], [at(a, 0.0), at(b, 6.0), at(a, 6.0)]]);
    }
    // The slot's wall: walk both rims clockwise (as the top face's boundary runs), ordered
    // by their angle about the middle, joining them by triangles as they advance.
    let clockwise = |ring: &[P2]| -> Vec<P2> {
        let turn = |p: &P2| (-(p[1] - 25.0).atan2(p[0] - 40.0)).rem_euclid(2.0 * PI);
        let mut out: Vec<P2> = ring.to_vec();
        out.sort_by(|a, b| turn(a).total_cmp(&turn(b)));
        out
    };
    let (lower, upper) = (clockwise(&bottom), clockwise(&top));
    let turn = |p: P2| (-(p[1] - 25.0).atan2(p[0] - 40.0)).rem_euclid(2.0 * PI);
    let (nl, nu) = (lower.len(), upper.len());
    let (mut i, mut j) = (0, 0);
    while i < nl || j < nu {
        let (l0, l1) = (lower[i % nl], lower[(i + 1) % nl]);
        let (u0, u1) = (upper[j % nu], upper[(j + 1) % nu]);
        let next_low = if i + 1 >= nl { 2.0 * PI + turn(lower[0]) } else { turn(l1) };
        let next_up = if j + 1 >= nu { 2.0 * PI + turn(upper[0]) } else { turn(u1) };
        if j >= nu || (i < nl && next_low <= next_up) {
            t.push([at(l0, 0.0), at(l1, 0.0), at(u0, 6.0)]);
            i += 1;
        } else {
            t.push([at(l0, 0.0), at(u1, 6.0), at(u0, 6.0)]);
            j += 1;
        }
    }
    let mesh = Mesh::new(t);
    assert!(mesh.volume() > 0.0);
    let (plate, _) = found(&mesh);
    assert_eq!(plate.slots, [SlotSpec { x: 40.0, y: 25.0, len: 18.0, w: 7.0, angle: 0.0 }]);
}

/// Every facet of a slot's rim split in the middle: the new corners lie on the facets'
/// chords, not the circles, and the slot is the same.
#[test]
fn split_slot_facets() {
    let ring = stadium([40.0, 25.0], 18.0, 7.0, 0.0, 16);
    let n = ring.len();
    let split: Vec<P2> = (0..n)
        .flat_map(|i| [ring[i], [f64::midpoint(ring[i][0], ring[(i + 1) % n][0]), f64::midpoint(ring[i][1], ring[(i + 1) % n][1])]])
        .collect();
    let (plate, _) = found(&plate_with_cutout(split));
    assert_eq!(plate.slots, [SlotSpec { x: 40.0, y: 25.0, len: 18.0, w: 7.0, angle: 0.0 }]);
}

/// A slot through the top face that does not reach the bottom face is no through slot.
#[test]
fn a_slot_must_go_through() {
    let mut mesh = plate_with_cutout(stadium([40.0, 25.0], 18.0, 7.0, 0.0, 16));
    // Widen the slot's bottom loop by a tenth: still a slot, but the walls lean.
    for t in &mut mesh.triangles {
        for p in t.iter_mut() {
            if p[2] == 0.0 && (p[1] - 25.0).abs() < 3.6 && (p[0] - 40.0).abs() < 12.6 {
                p[0] = 40.0 + (p[0] - 40.0) * 1.1;
                p[1] = 25.0 + (p[1] - 25.0) * 1.1;
            }
        }
    }
    let why = reason(&mesh);
    assert!(why.contains("slot does not go straight through") || why.contains("faces differ"), "{why}");
}
