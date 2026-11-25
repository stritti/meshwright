use crate::mesh::Mesh;
use crate::plate::P2;
use crate::recognize::{HoleSpec, OutlinePocketSpec, PadSpec, PocketSpec};
use crate::synth;

use super::{found, found_profile, reason};

/// A 70 x 40 x 6 plate exported from SolidCraft 0.49 with a 16 x 12 x 4 pad on its top face, a
/// 20 x 14 x 3 pocket in the top face, a 12 x 8 x 2 one in the bottom face and an Ø4 hole.
#[test]
fn recognises_pads_and_pockets() {
    let mesh = crate::stl::parse(include_bytes!("../../../../../plugin/revmesh/tests/pockets.stl")).expect("parses");
    let (plate, _) = found(&mesh);
    assert_eq!(plate.pads, [PadSpec { x: 20.0, y: 20.0, w: 16.0, l: 12.0, h: 4.0 }]);
    let pocket = |x: f64, y: f64, w: f64, l: f64, depth: f64, bottom: bool| PocketSpec { x, y, w, l, depth, bottom };
    assert_eq!(plate.pockets, [pocket(50.0, 15.0, 20.0, 14.0, 3.0, false), pocket(50.0, 30.0, 12.0, 8.0, 2.0, true)]);
    // SolidCraft measured 16,460.602 mm³ for the part it exported.
    assert!((plate.volume() - 16_460.602).abs() < 0.001, "{}", plate.volume());
}

/// Two pockets of one footprint in the top and the bottom face, a web between their floors.
#[test]
fn pockets_facing_each_other() {
    let outline = [[0.0, 0.0], [60.0, 0.0], [60.0, 40.0], [0.0, 40.0]];
    let rect = vec![[20.0, 10.0], [40.0, 10.0], [40.0, 30.0], [20.0, 30.0]];
    let at = |p: P2, z: f64| [p[0], p[1], z];
    let mut t = Vec::new();
    for [a, b, c] in synth::triangulate(&outline, std::slice::from_ref(&rect)) {
        t.push([at(a, 6.0), at(b, 6.0), at(c, 6.0)]);
        t.push([at(a, 0.0), at(c, 0.0), at(b, 0.0)]);
    }
    for i in 0..4 {
        let (a, b) = (outline[i], outline[(i + 1) % 4]);
        t.extend([[at(a, 0.0), at(b, 0.0), at(b, 6.0)], [at(a, 0.0), at(b, 6.0), at(a, 6.0)]]);
    }
    // The top pocket's floor at Z = 4 faces up, the bottom one's at Z = 2 down.
    let [a, b, c, d] = [rect[0], rect[1], rect[2], rect[3]];
    t.extend([[at(a, 4.0), at(b, 4.0), at(c, 4.0)], [at(a, 4.0), at(c, 4.0), at(d, 4.0)]]);
    t.extend([[at(a, 2.0), at(c, 2.0), at(b, 2.0)], [at(a, 2.0), at(d, 2.0), at(c, 2.0)]]);
    for i in 0..4 {
        let (p, q) = (rect[(i + 1) % 4], rect[i]);
        t.extend([[at(p, 4.0), at(q, 4.0), at(q, 6.0)], [at(p, 4.0), at(q, 6.0), at(p, 6.0)]]);
        t.extend([[at(p, 0.0), at(q, 0.0), at(q, 2.0)], [at(p, 0.0), at(q, 2.0), at(p, 2.0)]]);
    }
    let mesh = Mesh::new(t);
    assert!(mesh.volume() > 0.0);
    let (plate, _) = found(&mesh);
    let pocket = |depth: f64, bottom: bool| PocketSpec { x: 30.0, y: 20.0, w: 20.0, l: 20.0, depth, bottom };
    let mut pockets = plate.pockets.clone();
    pockets.sort_by_key(|p| p.bottom);
    assert_eq!(pockets, [pocket(2.0, false), pocket(2.0, true)]);
    assert!((plate.volume() - mesh.volume()).abs() < 1e-6);
}

/// A pocket's floor turned off level is no pocket; a triangle sunk into the top face is one
/// of an arbitrary outline (roadmap 5.3).
#[test]
fn pockets_need_flat_floors() {
    let mut mesh = crate::stl::parse(include_bytes!("../../../../../plugin/revmesh/tests/pockets.stl")).expect("parses");
    for t in &mut mesh.triangles {
        for p in t.iter_mut() {
            if (p[2] - 3.0).abs() < 1e-4 && p[0] > 55.0 && p[1] < 22.0 {
                p[2] -= 0.4;
            }
        }
    }
    reason(&mesh);
    let outline = [[0.0, 0.0], [60.0, 0.0], [60.0, 40.0], [0.0, 40.0]];
    let triangle = vec![[20.0, 10.0], [40.0, 10.0], [30.0, 25.0]];
    let at = |p: P2, z: f64| [p[0], p[1], z];
    let mut t = Vec::new();
    for [a, b, c] in synth::triangulate(&outline, std::slice::from_ref(&triangle)) {
        t.push([at(a, 6.0), at(b, 6.0), at(c, 6.0)]);
    }
    for [a, b, c] in synth::triangulate(&outline, &[]) {
        t.push([at(a, 0.0), at(c, 0.0), at(b, 0.0)]);
    }
    for i in 0..4 {
        let (a, b) = (outline[i], outline[(i + 1) % 4]);
        t.extend([[at(a, 0.0), at(b, 0.0), at(b, 6.0)], [at(a, 0.0), at(b, 6.0), at(a, 6.0)]]);
    }
    let [a, b, c] = [triangle[0], triangle[1], triangle[2]];
    t.push([at(a, 3.0), at(b, 3.0), at(c, 3.0)]);
    for i in 0..3 {
        let (p, q) = (triangle[(i + 1) % 3], triangle[i]);
        t.extend([[at(p, 3.0), at(q, 3.0), at(q, 6.0)], [at(p, 3.0), at(q, 6.0), at(p, 6.0)]]);
    }
    // A triangle sunk into the top face: a pocket of any outline of straight sides.
    let sunk = Mesh::new(t);
    let (profile, _) = found_profile(&sunk);
    assert_eq!(profile.pockets, []);
    assert_eq!(profile.outline_pockets, [OutlinePocketSpec { corners: triangle.clone(), depth: 3.0, bottom: false }]);
    assert!((profile.volume() - (60.0 * 40.0 * 6.0 - 150.0 * 3.0)).abs() < 1e-9, "{}", profile.volume());
}

/// Roadmap 5.3: a pocket whose mouth is any polygon of straight sides, not just an
/// axis-aligned rectangle — here a triangle in the top face and a hexagon in the bottom face
/// of one plate.
#[test]
fn recognises_pockets_of_any_straight_sided_outline() {
    let outline = [[0.0, 0.0], [60.0, 0.0], [60.0, 40.0], [0.0, 40.0]];
    let tri = vec![[15.0, 10.0], [35.0, 10.0], [25.0, 25.0]];
    let hex = vec![[40.0, 22.0], [52.0, 22.0], [55.0, 30.0], [52.0, 38.0], [40.0, 38.0], [37.0, 30.0]];
    let at = |p: P2, z: f64| [p[0], p[1], z];
    let mut t = Vec::new();
    // The top face has the triangle as a hole, the bottom face the hexagon.
    for [a, b, c] in synth::triangulate(&outline, std::slice::from_ref(&tri)) {
        t.push([at(a, 6.0), at(b, 6.0), at(c, 6.0)]);
    }
    for [a, b, c] in synth::triangulate(&outline, std::slice::from_ref(&hex)) {
        t.push([at(a, 0.0), at(c, 0.0), at(b, 0.0)]);
    }
    for i in 0..4 {
        let (a, b) = (outline[i], outline[(i + 1) % 4]);
        t.extend([[at(a, 0.0), at(b, 0.0), at(b, 6.0)], [at(a, 0.0), at(b, 6.0), at(a, 6.0)]]);
    }
    // The triangle's floor at Z = 3 faces up, the hexagon's at Z = 2 faces down; each pocket's
    // walls run between its mouth and its floor.
    for [a, b, c] in synth::triangulate(&tri, &[]) {
        t.push([at(a, 3.0), at(b, 3.0), at(c, 3.0)]);
    }
    for [a, b, c] in synth::triangulate(&hex, &[]) {
        t.push([at(a, 2.0), at(c, 2.0), at(b, 2.0)]);
    }
    let walls = |ring: &[P2], from: f64, to: f64| {
        let mut w = Vec::new();
        for i in 0..ring.len() {
            let (p, q) = (ring[(i + 1) % ring.len()], ring[i]);
            w.extend([[at(p, from), at(q, from), at(q, to)], [at(p, from), at(q, to), at(p, to)]]);
        }
        w
    };
    t.extend(walls(&tri, 3.0, 6.0));
    t.extend(walls(&hex, 0.0, 2.0));
    let mesh = Mesh::new(t);
    assert!(mesh.volume() > 0.0);
    let (profile, _) = found_profile(&mesh);
    assert_eq!(profile.pockets, []);
    assert_eq!(
        profile.outline_pockets,
        [
            OutlinePocketSpec { corners: tri.clone(), depth: 3.0, bottom: false },
            OutlinePocketSpec { corners: hex.clone(), depth: 2.0, bottom: true },
        ]
    );
    assert!((profile.volume() - mesh.volume()).abs() < 1e-6, "{} vs {}", profile.volume(), mesh.volume());
}

/// Walls of a ring's edges from `lo` to `hi`, outward for a counter-clockwise ring.
fn ring_walls(ring: &[P2], lo: f64, hi: f64) -> Vec<[[f64; 3]; 3]> {
    let at = |p: P2, z: f64| [p[0], p[1], z];
    let mut w = Vec::new();
    for i in 0..ring.len() {
        let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
        w.extend([[at(a, lo), at(b, lo), at(b, hi)], [at(a, lo), at(b, hi), at(a, hi)]]);
    }
    w
}

/// A 60 x 40 x 6 plate with a step: the region x >= 30 lies at Z = 3, meshed the way an exporter
/// meshes it (the lower box's walls up to Z = 3, the upper box's from there, the step's level as
/// a cap). The outline is the whole footprint; the step is a pocket cut from the top (5.3).
pub(super) fn stepped_plate(hole: Option<Vec<P2>>) -> Mesh {
    let bottom = [[0.0, 0.0], [30.0, 0.0], [60.0, 0.0], [60.0, 40.0], [30.0, 40.0], [0.0, 40.0]];
    let high = [[0.0, 0.0], [30.0, 0.0], [30.0, 40.0], [0.0, 40.0]];
    let step = [[30.0, 0.0], [60.0, 0.0], [60.0, 40.0], [30.0, 40.0]];
    let at = |p: P2, z: f64| [p[0], p[1], z];
    let holes: &[Vec<P2>] = match &hole {
        Some(h) => std::slice::from_ref(h),
        None => &[],
    };
    let mut t = Vec::new();
    for [a, b, c] in synth::triangulate(&bottom, holes) {
        t.push([at(a, 0.0), at(c, 0.0), at(b, 0.0)]);
    }
    t.extend(ring_walls(&bottom, 0.0, 3.0));
    for [a, b, c] in synth::triangulate(&step, holes) {
        t.push([at(a, 3.0), at(b, 3.0), at(c, 3.0)]);
    }
    t.extend(ring_walls(&high, 3.0, 6.0));
    for [a, b, c] in synth::triangulate(&high, &[]) {
        t.push([at(a, 6.0), at(b, 6.0), at(c, 6.0)]);
    }
    if let Some(ring) = hole {
        // The hole's wall from the bottom face up to the step's level.
        let cw: Vec<P2> = ring.iter().rev().copied().collect();
        for i in 0..cw.len() {
            let (p, q) = (cw[i], cw[(i + 1) % cw.len()]);
            t.extend([[at(p, 0.0), at(q, 0.0), at(q, 3.0)], [at(p, 0.0), at(q, 3.0), at(p, 3.0)]]);
        }
    }
    Mesh::new(t)
}

/// A step down from the top face: the region x >= 30 of a 60 x 40 x 6 plate lies at Z = 3.
#[test]
fn recognises_a_step_in_the_top_face() {
    let mesh = stepped_plate(None);
    assert!(mesh.volume() > 0.0, "mesh wound inside out");
    let step = vec![[30.0, 0.0], [60.0, 0.0], [60.0, 40.0], [30.0, 40.0]];
    let (profile, _) = found_profile(&mesh);
    assert_eq!(profile.outline_pockets, [OutlinePocketSpec { corners: step, depth: 3.0, bottom: false }]);
    assert!((profile.volume() - mesh.volume()).abs() < 1e-6, "{} vs {}", profile.volume(), mesh.volume());
}

/// A through hole in a step's level goes from that level down through the bottom face.
#[test]
fn recognises_a_hole_in_a_step_level() {
    let ring: Vec<P2> = (0..16)
        .map(|i| {
            let a = 2.0 * std::f64::consts::PI * f64::from(i) / 16.0;
            [45.0 + 5.0 * a.cos(), 20.0 + 5.0 * a.sin()]
        })
        .collect();
    let mesh = stepped_plate(Some(ring));
    assert!(mesh.volume() > 0.0, "mesh wound inside out");
    let step = vec![[30.0, 0.0], [60.0, 0.0], [60.0, 40.0], [30.0, 40.0]];
    let (profile, _) = found_profile(&mesh);
    assert_eq!(profile.outline_pockets, [OutlinePocketSpec { corners: step, depth: 3.0, bottom: false }]);
    assert_eq!(profile.holes.len(), 1);
    assert!(
        (profile.holes[0].x - 45.0).abs() < 0.01 && (profile.holes[0].y - 20.0).abs() < 0.01 && (profile.holes[0].d - 10.0).abs() < 0.01,
        "{:?}",
        profile.holes[0]
    );
    assert!(profile.holes[0].blind.is_some(), "blind from the bottom, as deep as the step's level: {:?}", profile.holes[0]);
    // The volume differs only by the hole's facets becoming a true circle.
    assert!((profile.volume() - mesh.volume()).abs() / mesh.volume() < 0.01, "{} vs {}", profile.volume(), mesh.volume());
}

/// A blind hole from the bottom face beneath a top pocket keeps its own measured depth: the
/// pocket's level must not overwrite it (the web between the cavities stays).
#[test]
fn a_blind_hole_beneath_a_pocket_keeps_its_measured_depth() {
    use crate::recognize::Blind;
    let outline = [[0.0, 0.0], [60.0, 0.0], [60.0, 40.0], [0.0, 40.0]];
    let triangle = vec![[20.0, 10.0], [40.0, 10.0], [30.0, 25.0]];
    let hole: Vec<P2> = (0..16)
        .map(|i| {
            let a = 2.0 * std::f64::consts::PI * f64::from(i) / 16.0;
            [30.0 + 4.0 * a.cos(), 17.0 + 4.0 * a.sin()]
        })
        .collect();
    let at = |p: P2, z: f64| [p[0], p[1], z];
    let mut t = Vec::new();
    for [a, b, c] in synth::triangulate(&outline, std::slice::from_ref(&triangle)) {
        t.push([at(a, 6.0), at(b, 6.0), at(c, 6.0)]);
    }
    for [a, b, c] in synth::triangulate(&outline, std::slice::from_ref(&hole)) {
        t.push([at(a, 0.0), at(c, 0.0), at(b, 0.0)]);
    }
    t.extend(ring_walls(&outline, 0.0, 6.0));
    let [a, b, c] = [triangle[0], triangle[1], triangle[2]];
    t.push([at(a, 3.0), at(b, 3.0), at(c, 3.0)]);
    for i in 0..3 {
        let (p, q) = (triangle[(i + 1) % 3], triangle[i]);
        t.extend([[at(p, 3.0), at(q, 3.0), at(q, 6.0)], [at(p, 3.0), at(q, 6.0), at(p, 6.0)]]);
    }
    // The blind hole from the bottom face, 1 mm deep, beneath the pocket.
    let cw: Vec<P2> = hole.iter().rev().copied().collect();
    t.extend(ring_walls(&cw, 0.0, 1.0));
    for [a, b, c] in synth::triangulate(&hole, &[]) {
        t.push([at(a, 1.0), at(c, 1.0), at(b, 1.0)]);
    }
    let mesh = Mesh::new(t);
    assert!(mesh.volume() > 0.0, "mesh wound inside out");
    let (profile, _) = found_profile(&mesh);
    assert_eq!(profile.outline_pockets, [OutlinePocketSpec { corners: triangle, depth: 3.0, bottom: false }]);
    assert_eq!(profile.holes.len(), 1, "{:?}", profile.holes);
    let h = &profile.holes[0];
    assert!((h.x - 30.0).abs() < 0.01 && (h.y - 17.0).abs() < 0.01 && (h.d - 8.0).abs() < 0.01, "{h:?}");
    assert_eq!(h.blind, Some(Blind { depth: 1.0, bottom: true }), "the measured depth, not the pocket's level: {h:?}");
    assert!((profile.volume() - mesh.volume()).abs() / mesh.volume() < 0.01, "{} vs {}", profile.volume(), mesh.volume());
}

/// A 60 x 40 x 8 plate with two steps down from the top face: x in [30, 45) tops at 6 mm and
/// x >= 45 at 4 mm — two step levels deep (5.3).
fn two_level_stepped_plate() -> Mesh {
    let at = |p: [f64; 2], z: f64| [p[0], p[1], z];
    let mut t: Vec<[[f64; 3]; 3]> = Vec::new();
    // Vertical wall from a to b between lo and hi (outward = right of a -> b), split at the
    // step levels below hi so every level's edge is a chord.
    let wall = |t: &mut Vec<[[f64; 3]; 3]>, a: [f64; 2], b: [f64; 2], lo: f64, hi: f64| {
        t.push([at(a, lo), at(b, lo), at(b, hi)]);
        t.push([at(a, lo), at(b, hi), at(a, hi)]);
    };
    let wall_split = |t: &mut Vec<[[f64; 3]; 3]>, a: [f64; 2], b: [f64; 2], h: f64| {
        let levels: [f64; 4] = [0.0, 4.0, 6.0, 8.0];
        let mut lo = 0.0;
        for &hi in &levels[1..] {
            if lo < h {
                wall(t, a, b, lo, hi.min(h));
            }
            lo = hi;
        }
    };
    let cap = |t: &mut Vec<[[f64; 3]; 3]>, ring: &[[f64; 2]], z: f64, up: bool| {
        for [a, b, c] in synth::triangulate(ring, &[]) {
            if up {
                t.push([at(a, z), at(b, z), at(c, z)]);
            } else {
                t.push([at(a, z), at(c, z), at(b, z)]);
            }
        }
    };
    let bottom = [[0.0, 0.0], [60.0, 0.0], [60.0, 40.0], [0.0, 40.0]];
    cap(&mut t, &bottom, 0.0, false);
    let a_region = [[0.0, 0.0], [30.0, 0.0], [30.0, 40.0], [0.0, 40.0]];
    let b_region = [[30.0, 0.0], [45.0, 0.0], [45.0, 40.0], [30.0, 40.0]];
    let c_region = [[45.0, 0.0], [60.0, 0.0], [60.0, 40.0], [45.0, 40.0]];
    cap(&mut t, &a_region, 8.0, true);
    cap(&mut t, &b_region, 6.0, true);
    cap(&mut t, &c_region, 4.0, true);
    // Outer walls, each segment with its region's height.
    wall_split(&mut t, [0.0, 0.0], [30.0, 0.0], 8.0);
    wall_split(&mut t, [30.0, 0.0], [45.0, 0.0], 6.0);
    wall_split(&mut t, [45.0, 0.0], [60.0, 0.0], 4.0);
    wall_split(&mut t, [60.0, 0.0], [60.0, 40.0], 4.0);
    wall_split(&mut t, [60.0, 40.0], [45.0, 40.0], 4.0);
    wall_split(&mut t, [45.0, 40.0], [30.0, 40.0], 6.0);
    wall_split(&mut t, [30.0, 40.0], [0.0, 40.0], 8.0);
    wall_split(&mut t, [0.0, 40.0], [0.0, 0.0], 8.0);
    // The steps' walls: x = 30 exposed from 6 to 8, x = 45 from 4 to 6.
    wall(&mut t, [30.0, 0.0], [30.0, 40.0], 6.0, 8.0);
    wall(&mut t, [45.0, 0.0], [45.0, 40.0], 4.0, 6.0);
    Mesh::new(t)
}

/// Two step levels down from the top face are both recognised, each as an outline pocket cut
/// from the top down to its level.
#[test]
fn recognises_two_step_levels() {
    let mesh = two_level_stepped_plate();
    assert!(mesh.volume() > 0.0, "mesh wound inside out");
    let (profile, _) = found_profile(&mesh);
    let mut pockets = profile.outline_pockets.clone();
    pockets.sort_by(|a, b| a.depth.total_cmp(&b.depth));
    assert_eq!(
        pockets,
        [
            OutlinePocketSpec { corners: vec![[30.0, 0.0], [45.0, 0.0], [45.0, 40.0], [30.0, 40.0]], depth: 2.0, bottom: false },
            OutlinePocketSpec { corners: vec![[45.0, 0.0], [60.0, 0.0], [60.0, 40.0], [45.0, 40.0]], depth: 4.0, bottom: false },
        ],
        "{profile:?}"
    );
    assert!((profile.volume() - mesh.volume()).abs() / mesh.volume() < 0.01, "{} vs {}", profile.volume(), mesh.volume());
}

/// A 60 x 40 x 6 plate with the step x >= 30 at Z = 3 and a counterbored hole at (45, 20): an
/// Ø`bore` bore and an Ø`head` mouth whose floor lies at Z = `floor`. With `head_on_level` the
/// mouth opens on the step's level, else on the bottom face.
fn stepped_plate_with_counterbore(bore: f64, head: f64, floor: f64, head_on_level: bool) -> Mesh {
    let bottom = [[0.0, 0.0], [30.0, 0.0], [60.0, 0.0], [60.0, 40.0], [30.0, 40.0], [0.0, 40.0]];
    let high = [[0.0, 0.0], [30.0, 0.0], [30.0, 40.0], [0.0, 40.0]];
    let step = [[30.0, 0.0], [60.0, 0.0], [60.0, 40.0], [30.0, 40.0]];
    let at = |p: P2, z: f64| [p[0], p[1], z];
    let ring = |d: f64| synth::rings(&[HoleSpec { x: 45.0, y: 20.0, d, head: None, blind: None }], 16).remove(0);
    let (bore_ring, head_ring) = (ring(bore), ring(head));
    let (level_hole, bottom_hole) = if head_on_level { (&head_ring, &bore_ring) } else { (&bore_ring, &head_ring) };
    let mut t = Vec::new();
    for [a, b, c] in synth::triangulate(&bottom, std::slice::from_ref(bottom_hole)) {
        t.push([at(a, 0.0), at(c, 0.0), at(b, 0.0)]);
    }
    t.extend(ring_walls(&bottom, 0.0, 3.0));
    for [a, b, c] in synth::triangulate(&step, std::slice::from_ref(level_hole)) {
        t.push([at(a, 3.0), at(b, 3.0), at(c, 3.0)]);
    }
    t.extend(ring_walls(&high, 3.0, 6.0));
    for [a, b, c] in synth::triangulate(&high, &[]) {
        t.push([at(a, 6.0), at(b, 6.0), at(c, 6.0)]);
    }
    // The counterbore's floor between the two rings, facing the mouth's face; the mouth's wall
    // from that face to the floor, the bore's from the floor to the other face.
    for [a, b, c] in synth::triangulate(&head_ring, std::slice::from_ref(&bore_ring)) {
        if head_on_level {
            t.push([at(a, floor), at(b, floor), at(c, floor)]);
        } else {
            t.push([at(a, floor), at(c, floor), at(b, floor)]);
        }
    }
    let cw = |r: &[P2]| r.iter().rev().copied().collect::<Vec<P2>>();
    if head_on_level {
        t.extend(ring_walls(&cw(&bore_ring), 0.0, floor));
        t.extend(ring_walls(&cw(&head_ring), floor, 3.0));
    } else {
        t.extend(ring_walls(&cw(&head_ring), 0.0, floor));
        t.extend(ring_walls(&cw(&bore_ring), floor, 3.0));
    }
    Mesh::new(t)
}

/// A counterbore opening on a step's level is drilled from that level, not from the plate's top
/// face: it is recognised exactly, its bore blind from the bottom as deep as the level lies
/// above it (roadmap 5.3).
#[test]
fn a_counterbore_opening_on_a_step_level_is_recognised() {
    use crate::recognize::{Blind, HoleHead};
    let mesh = stepped_plate_with_counterbore(6.0, 10.0, 2.0, true);
    assert!(mesh.volume() > 0.0, "mesh wound inside out");
    let step = vec![[30.0, 0.0], [60.0, 0.0], [60.0, 40.0], [30.0, 40.0]];
    let (profile, _) = found_profile(&mesh);
    assert_eq!(profile.outline_pockets, [OutlinePocketSpec { corners: step, depth: 3.0, bottom: false }]);
    assert_eq!(
        profile.holes,
        [HoleSpec {
            x: 45.0,
            y: 20.0,
            d: 6.0,
            head: Some(HoleHead::Counterbore { d: 10.0, depth: 1.0, bottom: false }),
            blind: Some(Blind { depth: 3.0, bottom: true }),
        }]
    );
    assert!((profile.volume() - mesh.volume()).abs() / mesh.volume() < 0.01, "{} vs {}", profile.volume(), mesh.volume());
}

/// A counterbore opening on the bottom face of a hole in a step's level is drilled from the
/// bottom face, as deep as the level lies above it: it is recognised exactly.
#[test]
fn a_counterbore_on_the_bottom_in_a_step_level_is_recognised() {
    use crate::recognize::{Blind, HoleHead};
    let mesh = stepped_plate_with_counterbore(6.0, 10.0, 2.0, false);
    assert!(mesh.volume() > 0.0, "mesh wound inside out");
    let step = vec![[30.0, 0.0], [60.0, 0.0], [60.0, 40.0], [30.0, 40.0]];
    let (profile, _) = found_profile(&mesh);
    assert_eq!(profile.outline_pockets, [OutlinePocketSpec { corners: step, depth: 3.0, bottom: false }]);
    assert_eq!(
        profile.holes,
        [HoleSpec {
            x: 45.0,
            y: 20.0,
            d: 6.0,
            head: Some(HoleHead::Counterbore { d: 10.0, depth: 2.0, bottom: true }),
            blind: Some(Blind { depth: 3.0, bottom: true }),
        }]
    );
    assert!((profile.volume() - mesh.volume()).abs() / mesh.volume() < 0.01, "{} vs {}", profile.volume(), mesh.volume());
}
