use std::f64::consts::PI;

use crate::mesh::{Mesh, Vec3, cross, sub};
use crate::plate::P2;
use crate::plate::geometry::distance;
use crate::recognize::{Blind, HoleHead, HoleSpec, PlateSpec};
use crate::synth;

use super::{found, reason};

/// A 200 x 200 x 5 mm plate with a 2 mm blind hole 2.5 mm deep, its floor a fan of the rim's
/// corners: every corner between the faces lies on the hole's circle, and the floor is a floor.
#[test]
fn fanned_blind_floors_are_floors() {
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
        // The bore, seen from inside the hole, runs clockwise round the rim.
        let (a, b) = (ring[(i + 1) % m], ring[i]);
        t.push([lift(a, floor), lift(b, floor), lift(b, z)]);
        t.push([lift(a, floor), lift(b, z), lift(a, z)]);
    }
    for i in 1..m - 1 {
        t.push([lift(ring[0], floor), lift(ring[i], floor), lift(ring[i + 1], floor)]);
    }
    let mesh = Mesh::new(t);
    assert!(mesh.volume() > 0.0);
    let (plate, _) = found(&mesh);
    assert_eq!(plate.holes[0].blind, Some(Blind { depth: 2.5, bottom: false }));
    // Its floor's fan is no through hole's wall.
    assert!((plate.volume() - mesh.volume()).abs() / mesh.volume() < 1e-5);
}

/// A large 8-sided bore whose bottom rim is turned half a step against its top rim: the
/// wall's bottom edges are chords whose middles lie 3 mm inside the circle, cells away from
/// it, and still follow the wall.
#[test]
fn chords_of_independently_meshed_rims_follow_the_bore() {
    let (s, z, r) = (400.0, 5.0, 40.0);
    let outline = [[0.0, 0.0], [s, 0.0], [s, s], [0.0, s]];
    let ring = |phase: f64| -> Vec<P2> {
        (0..8).map(|i| (f64::from(i) + phase) * PI / 4.0).map(|a| [200.0 + r * a.cos(), 200.0 + r * a.sin()]).collect()
    };
    let (top_ring, bottom_ring) = (ring(0.0), ring(0.5));
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
    for i in 0..8 {
        let (t0, t1, b0) = (top_ring[i], top_ring[(i + 1) % 8], bottom_ring[i]);
        let b_prev = bottom_ring[(i + 7) % 8];
        t.push([lift(t0, z), lift(b0, 0.0), lift(t1, z)]);
        t.push([lift(t0, z), lift(b_prev, 0.0), lift(b0, 0.0)]);
    }
    // Hole walls face the hole's axis, the others away from the plate's middle.
    for tri in &mut t {
        let n = cross(sub(tri[1], tri[0]), sub(tri[2], tri[0]));
        let centre = [
            (tri[0][0] + tri[1][0] + tri[2][0]) / 3.0,
            (tri[0][1] + tri[1][1] + tri[2][1]) / 3.0,
            (tri[0][2] + tri[1][2] + tri[2][2]) / 3.0,
        ];
        let in_bore = tri.iter().all(|p| ((p[0] - 200.0).hypot(p[1] - 200.0) - r).abs() < 1e-6);
        let away = if in_bore { [200.0 - centre[0], 200.0 - centre[1], 0.0] } else { sub(centre, [200.0, 200.0, z / 2.0]) };
        if (in_bore || n[2].abs() < 0.5) && crate::mesh::dot(n, away) < 0.0 {
            tri.swap(1, 2);
        }
    }
    let mesh = Mesh::new(t);
    assert!(mesh.volume() > 0.0);
    let walls = {
        let (plate, _) = found(&mesh);
        plate.holes
    };
    assert_eq!(walls.len(), 1);
}

/// A 60 x 40 x 8 plate exported from SolidCraft 0.48 with blind holes from the top (Ø6 x 4,
/// Ø3 x 6) and the bottom (Ø8 x 2.5) and an Ø5 hole through.
#[test]
fn recognises_blind_holes() {
    let mesh = crate::stl::parse(include_bytes!("../../../../../plugin/revmesh/tests/blind-holes.stl")).expect("parses");
    let (plate, _) = found(&mesh);
    let blind = |depth: f64, bottom: bool| Some(Blind { depth, bottom });
    let holes: Vec<(f64, f64, f64, Option<Blind>)> = plate.holes.iter().map(|h| (h.x, h.y, h.d, h.blind)).collect();
    assert_eq!(
        holes,
        [
            (15.0, 20.0, 6.0, blind(4.0, false)),
            (30.0, 20.0, 5.0, None),
            (45.0, 20.0, 8.0, blind(2.5, true)),
            (45.0, 32.0, 3.0, blind(6.0, false))
        ]
    );
    // SolidCraft measured 18,761.748 mm³ for the part it exported.
    assert!((plate.volume() - 18_761.748).abs() < 0.001, "{}", plate.volume());
}

/// A blind hole's floor turned 0.5 mm off level is no floor.
#[test]
fn a_tilted_blind_floor_is_refused() {
    let mut mesh = crate::stl::parse(include_bytes!("../../../../../plugin/revmesh/tests/blind-holes.stl")).expect("parses");
    // Lower the floor corners of the Ø6 hole (at 15, 20; floor 4 mm down) on one side.
    for t in &mut mesh.triangles {
        for p in t.iter_mut() {
            if (p[2] - 4.0).abs() < 1e-4 && p[0] > 15.0 && (p[0] - 15.0).hypot(p[1] - 20.0) < 3.1 {
                p[2] -= 0.5;
            }
        }
    }
    reason(&mesh);
}

/// A closed 60 x 40 x 8 plate with an Ø6 blind hole of `n` corners at (15, 20) from the top,
/// its wall `depth` deep and its end a flat floor or, with `tip`, a cone that far deeper.
fn blind_plate(depth: f64, tip: f64, n: usize) -> Mesh {
    let ring = synth::rings(&[HoleSpec { x: 15.0, y: 20.0, d: 6.0, head: None, blind: None }], n).remove(0);
    let (z, floor) = (8.0, 8.0 - depth);
    let outline = [[0.0, 0.0], [60.0, 0.0], [60.0, 40.0], [0.0, 40.0]];
    let at = |p: P2, h: f64| [p[0], p[1], h];
    let mut triangles = Vec::new();
    for [a, b, c] in synth::triangulate(&outline, std::slice::from_ref(&ring)) {
        triangles.push([at(a, z), at(b, z), at(c, z)]);
    }
    for [a, b, c] in synth::triangulate(&outline, &[]) {
        triangles.push([at(a, 0.0), at(c, 0.0), at(b, 0.0)]);
    }
    if tip > 0.0 {
        let apex = [15.0, 20.0, floor - tip];
        for i in 0..n {
            triangles.push([at(ring[i], floor), at(ring[(i + 1) % n], floor), apex]);
        }
    } else {
        for [a, b, c] in synth::triangulate(&ring, &[]) {
            triangles.push([at(a, floor), at(b, floor), at(c, floor)]);
        }
    }
    let mut wall = |ring: &[P2], lo: f64, hi: f64| {
        let m = ring.len();
        for i in 0..m {
            let (a, b) = (ring[i], ring[(i + 1) % m]);
            triangles.extend([[at(a, lo), at(b, lo), at(b, hi)], [at(a, lo), at(b, hi), at(a, hi)]]);
        }
    };
    wall(&outline, 0.0, z);
    wall(&ring.iter().rev().copied().collect::<Vec<P2>>(), floor, z);
    Mesh::new(triangles)
}

/// A blind hole of a synthetic mesh, flat-bottomed; one with a drill point is refused.
#[test]
fn blind_holes_need_flat_floors() {
    let (plate, _) = found(&blind_plate(4.0, 0.0, 32));
    assert_eq!(plate.holes, [HoleSpec { x: 15.0, y: 20.0, d: 6.0, head: None, blind: Some(Blind { depth: 4.0, bottom: false }) }]);
    let why = reason(&blind_plate(4.0, 1.8, 32));
    assert!(why.contains("not a flat plate") || why.contains("flat floor"), "{why}");
}

/// A 60 x 40 x 8 plate exported from SolidCraft 0.48 with a counterbore and a countersink on
/// the top face and a counterbore on the bottom face.
#[test]
fn recognises_counterbores_and_countersinks() {
    let mesh = crate::stl::parse(include_bytes!("../../../../../plugin/revmesh/tests/stepped-holes.stl")).expect("parses");
    let (plate, _) = found(&mesh);
    let heads: Vec<(f64, f64, f64, Option<HoleHead>)> = plate.holes.iter().map(|h| (h.x, h.y, h.d, h.head)).collect();
    assert_eq!(
        heads,
        [
            (15.0, 20.0, 5.5, Some(HoleHead::Counterbore { d: 10.0, depth: 3.0, bottom: false })),
            (30.0, 32.0, 4.0, Some(HoleHead::Counterbore { d: 8.0, depth: 2.5, bottom: true })),
            (45.0, 20.0, 6.0, Some(HoleHead::Countersink { d: 12.0, angle: 90.0, bottom: false })),
        ]
    );
    // SolidCraft measured 18,311.518 mm³ for the part it exported.
    assert!((plate.volume() - 18_311.518).abs() < 0.001, "{}", plate.volume());
}

/// A counterbore's floor turned 0.5 mm off level is no floor.
#[test]
fn a_tilted_counterbore_floor_is_refused() {
    let mut mesh = crate::stl::parse(include_bytes!("../../../../../plugin/revmesh/tests/stepped-holes.stl")).expect("parses");
    // Lift the floor corners of the top counterbore (at 15, 20; floor 3 mm down) on one side.
    for t in &mut mesh.triangles {
        for p in t.iter_mut() {
            if (p[2] - 5.0).abs() < 1e-4 && p[0] > 15.0 && (p[0] - 15.0).hypot(p[1] - 20.0) < 5.1 {
                p[2] += 0.5;
            }
        }
    }
    reason(&mesh);
}

/// A 1 mm web left in the counterbored bore (between Z = 2 and Z = 3, two blind holes meeting
/// it from both faces) is no through hole, although every corner of its two disks lies on the
/// bore's circle. Its volume is below the 1 % the volume check would notice.
#[test]
fn a_web_across_a_counterbored_bore_is_refused() {
    let mut mesh = crate::stl::parse(include_bytes!("../../../../../plugin/revmesh/tests/stepped-holes.stl")).expect("parses");
    let centre = [15.0, 20.0];
    let mut rim: Vec<P2> = Vec::new();
    for p in mesh.triangles.iter().flatten() {
        let q = [p[0], p[1]];
        if (distance(q, centre) - 2.75).abs() < 1e-3 && !rim.iter().any(|r| distance(*r, q) < 1e-6) {
            rim.push(q);
        }
    }
    rim.sort_by(|a, b| (a[1] - centre[1]).atan2(a[0] - centre[0]).total_cmp(&(b[1] - centre[1]).atan2(b[0] - centre[0])));
    assert!(rim.len() > 8, "{}", rim.len());
    for i in 1..rim.len() - 1 {
        let (a, b, c) = (rim[0], rim[i], rim[i + 1]);
        mesh.triangles.push([[a[0], a[1], 3.0], [b[0], b[1], 3.0], [c[0], c[1], 3.0]]);
        mesh.triangles.push([[a[0], a[1], 2.0], [c[0], c[1], 2.0], [b[0], b[1], 2.0]]);
    }
    reason(&mesh);
}

/// A closed 60 x 60 x 8 plate with one hole at (30, 30): an Ø`bore` bore and an
/// Ø`head` x `depth` counterbore from the top, each a polygon of `n` corners. With `split`,
/// the first facet pair of the bore's and of the counterbore's wall is split where the two
/// meet, as an exporter refining a mesh would: the new corner lies on the chord. `twist` turns
/// the bore's upper rim (at the floor) by that angle about the hole's axis.
fn counterbored(bore: f64, head: f64, depth: f64, n: usize, split: bool, twist: f64) -> Mesh {
    let ring = |d: f64| synth::rings(&[HoleSpec { x: 30.0, y: 30.0, d, head: None, blind: None }], n).remove(0);
    let (bore_ring, head_ring) = (ring(bore), ring(head));
    let (sin, cos) = twist.sin_cos();
    let turned: Vec<P2> = bore_ring
        .iter()
        .map(|p| [30.0 + (p[0] - 30.0) * cos - (p[1] - 30.0) * sin, 30.0 + (p[0] - 30.0) * sin + (p[1] - 30.0) * cos])
        .collect();
    let (z, floor) = (8.0, 8.0 - depth);
    let outline = [[0.0, 0.0], [60.0, 0.0], [60.0, 60.0], [0.0, 60.0]];
    let at = |p: P2, h: f64| [p[0], p[1], h];
    let mut triangles = Vec::new();
    for [a, b, c] in synth::triangulate(&outline, std::slice::from_ref(&head_ring)) {
        triangles.push([at(a, z), at(b, z), at(c, z)]);
    }
    for [a, b, c] in synth::triangulate(&outline, std::slice::from_ref(&bore_ring)) {
        triangles.push([at(a, 0.0), at(c, 0.0), at(b, 0.0)]);
    }
    for [a, b, c] in synth::triangulate(&head_ring, std::slice::from_ref(&turned)) {
        triangles.push([at(a, floor), at(b, floor), at(c, floor)]);
    }
    // A wall from `lower` up to `upper`, two loops of corners that match one for one.
    let mut wall = |lower: &[P2], upper: &[P2], lo: f64, hi: f64, split: bool| {
        let n = lower.len();
        for i in 0..n {
            let (a, b, c, d) = (at(lower[i], lo), at(lower[(i + 1) % n], lo), at(upper[(i + 1) % n], hi), at(upper[i], hi));
            if split && i == 0 {
                let m = [f64::midpoint(a[0], c[0]), f64::midpoint(a[1], c[1]), f64::midpoint(lo, hi)];
                triangles.extend([[a, b, m], [m, b, c], [a, m, d], [m, c, d]]);
            } else {
                triangles.extend([[a, b, c], [a, c, d]]);
            }
        }
    };
    wall(&outline, &outline, 0.0, z, false);
    let reversed = |r: &[P2]| r.iter().rev().copied().collect::<Vec<P2>>();
    wall(&reversed(&bore_ring), &reversed(&turned), 0.0, floor, split);
    wall(&reversed(&head_ring), &reversed(&head_ring), floor, z, split);
    Mesh::new(triangles)
}

fn counterbore_of(mesh: &Mesh) -> Option<HoleHead> {
    let (plate, _) = found(mesh);
    assert_eq!(plate.holes.len(), 1);
    plate.holes[0].head
}

/// The floor of a counterbore only 0.1 mm wider than its 16-cornered bore reaches inside the
/// bore's circle, between the corners of its loop.
#[test]
fn narrow_counterbores_over_coarse_bores() {
    assert_eq!(
        counterbore_of(&counterbored(10.0, 10.2, 3.0, 16, false, 0.0)),
        Some(HoleHead::Counterbore { d: 10.2, depth: 3.0, bottom: false })
    );
}

/// Corners an exporter adds within the facets of a counterbored hole's walls lie on the walls'
/// chords, not on their circles, as with a plain hole.
#[test]
fn subdivided_walls_of_counterbored_holes() {
    let expected = Some(HoleHead::Counterbore { d: 11.0, depth: 3.0, bottom: false });
    assert_eq!(counterbore_of(&counterbored(10.0, 11.0, 3.0, 16, false, 0.0)), expected);
    let split = counterbored(10.0, 11.0, 3.0, 16, true, 0.0);
    assert!((split.volume() - counterbored(10.0, 11.0, 3.0, 16, false, 0.0).volume()).abs() < 1e-9);
    assert_eq!(counterbore_of(&split), expected);
}

/// A counterbored bore whose upper rim is turned 60° against its lower one before the wall
/// joins them narrows in the middle: its facets cut across the bore, although every corner
/// lies on the bore's circle. Turned half a step it is still a bore, as exporters mesh it.
#[test]
fn a_twisted_counterbored_bore_is_refused() {
    let why = reason(&counterbored(5.5, 10.0, 3.0, 32, false, 60_f64.to_radians()));
    assert!(why.contains("not vertical"), "{why}");
    let half_step = Some(HoleHead::Counterbore { d: 10.0, depth: 3.0, bottom: false });
    assert_eq!(counterbore_of(&counterbored(5.5, 10.0, 3.0, 32, false, PI / 32.0)), half_step);
}

/// A closed 60 x 60 x 8 plate with an Ø5.5 bore at (30, 30) under an Ø10 90° countersink, each
/// loop of `n` corners, the bore (both its rims) turned by `twist` against the mouth.
fn countersunk(n: usize, twist: f64) -> Mesh {
    let ring = |d: f64, turn: f64| -> Vec<P2> {
        (0..n).map(|i| 2.0 * PI * i as f64 / n as f64 + turn).map(|a| [30.0 + d / 2.0 * a.cos(), 30.0 + d / 2.0 * a.sin()]).collect()
    };
    let (bore, mouth) = (ring(5.5, twist), ring(10.0, 0.0));
    let (z, cone) = (8.0, 8.0 - 2.25);
    let outline = [[0.0, 0.0], [60.0, 0.0], [60.0, 60.0], [0.0, 60.0]];
    let at = |p: P2, h: f64| [p[0], p[1], h];
    let mut triangles = Vec::new();
    for [a, b, c] in synth::triangulate(&outline, std::slice::from_ref(&mouth)) {
        triangles.push([at(a, z), at(b, z), at(c, z)]);
    }
    for [a, b, c] in synth::triangulate(&outline, std::slice::from_ref(&bore)) {
        triangles.push([at(a, 0.0), at(c, 0.0), at(b, 0.0)]);
    }
    let mut wall = |lower: &[P2], upper: &[P2], lo: f64, hi: f64| {
        let m = lower.len();
        for i in 0..m {
            let (a, b, c, d) = (at(lower[i], lo), at(lower[(i + 1) % m], lo), at(upper[(i + 1) % m], hi), at(upper[i], hi));
            triangles.extend([[a, b, c], [a, c, d]]);
        }
    };
    wall(&outline, &outline, 0.0, z);
    let reversed = |r: &[P2]| r.iter().rev().copied().collect::<Vec<P2>>();
    wall(&reversed(&bore), &reversed(&bore), 0.0, cone);
    wall(&reversed(&bore), &reversed(&mouth), cone, z);
    Mesh::new(triangles)
}

/// A countersink's cone joining a mouth and a bore turned a quarter turn apart narrows the
/// hole: its facets cut inside the cone, although their corners lie on it.
#[test]
fn a_twisted_countersink_is_refused() {
    let head = Some(HoleHead::Countersink { d: 10.0, angle: 90.0, bottom: false });
    assert_eq!(counterbore_of(&countersunk(32, 0.0)), head);
    assert_eq!(counterbore_of(&countersunk(32, PI / 32.0)), head);
    reason(&countersunk(32, PI / 2.0));
}

/// A facet of a countersink's cone split into three at its centroid leaves a corner on the
/// facet, inside the cone.
#[test]
fn split_countersink_facets() {
    let mut mesh = crate::stl::parse(include_bytes!("../../../../../plugin/revmesh/tests/stepped-holes.stl")).expect("parses");
    let on_cone = |t: &[Vec3; 3]| {
        t.iter().all(|p| (2.9..6.1).contains(&(p[0] - 45.0).hypot(p[1] - 20.0)) && p[2] > 4.9) && t.iter().any(|p| (p[2] - 8.0).abs() > 0.1)
    };
    let k = mesh.triangles.iter().position(|t| on_cone(t) && t.iter().any(|p| (p[2] - 8.0).abs() < 1e-4)).expect("a cone facet");
    let [a, b, c] = mesh.triangles.swap_remove(k);
    let m = [(a[0] + b[0] + c[0]) / 3.0, (a[1] + b[1] + c[1]) / 3.0, (a[2] + b[2] + c[2]) / 3.0];
    mesh.triangles.extend([[a, b, m], [b, c, m], [c, a, m]]);
    assert_eq!(found(&mesh).0.holes.len(), 3);
}

/// A rim edge of a hole's bottom loop split in the middle (both facets on it) leaves a corner
/// on the edge, inside the circle: the rim stays round.
#[test]
#[allow(clippy::float_cmp)] // the edge corners are the generator output, bit for bit
fn split_hole_rims_stay_round() {
    let expected = PlateSpec {
        x: 60.0,
        y: 60.0,
        z: 8.0,
        holes: vec![HoleSpec { x: 30.0, y: 30.0, d: 10.0, head: None, blind: None }],
        cutouts: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![],
        slots: vec![],
        outline_fillet_r: None,
        edge_fillet_r: None,
    };
    let mut mesh = synth::plate(&expected, 16, [0.0; 3]);
    let ring = synth::rings(&expected.holes, 16).remove(0);
    let (p, q) = ([ring[0][0], ring[0][1], 0.0], [ring[1][0], ring[1][1], 0.0]);
    let m = [f64::midpoint(p[0], q[0]), f64::midpoint(p[1], q[1]), 0.0];
    let mut split = 0;
    mesh.triangles = mesh
        .triangles
        .iter()
        .flat_map(|t| {
            // The facet's corners from the edge's start round to the edge's end, if it has it.
            (0..3).find(|&k| (t[k] == p && t[(k + 1) % 3] == q) || (t[k] == q && t[(k + 1) % 3] == p)).map_or_else(
                || vec![*t],
                |k| {
                    split += 1;
                    let (a, b, c) = (t[k], t[(k + 1) % 3], t[(k + 2) % 3]);
                    vec![[a, m, c], [m, b, c]]
                },
            )
        })
        .collect();
    assert_eq!(split, 2);
    assert_eq!(found(&mesh).0, expected);
}
