use std::f64::consts::PI;

use crate::mesh::Mesh;
use crate::plate::P2;
use crate::recognize::{BossChamfer, BossSpec, PadSpec, Recognition};
use crate::synth;

use super::{circle, found, reason};

/// A 70 x 40 x 6 plate exported from SolidCraft 0.49 with an Ø10 x 5 and an Ø8 x 3 boss on
/// its top face and an Ø5 hole.
#[test]
fn recognises_bosses() {
    let mesh = crate::stl::parse(include_bytes!("../../../../../plugin/revmesh/tests/bosses.stl")).expect("parses");
    let (plate, _) = found(&mesh);
    let boss = |x: f64, y: f64, d: f64, h: f64| BossSpec { x, y, d, h, fillet: None, draft_deg: None, chamfer: None };
    assert_eq!(plate.bosses, [boss(20.0, 20.0, 10.0, 5.0), boss(50.0, 25.0, 8.0, 3.0)]);
    assert_eq!((plate.z, plate.holes.len()), (6.0, 1));
    // SolidCraft measured 17,225.686 mm³ for the part it exported.
    assert!((plate.volume() - 17_225.686).abs() < 0.001, "{}", plate.volume());
}

/// A 60 x 40 x 6 plate with a solid standing on its top face over the loop `foot` (`n`
/// corners): `walls` gives each corner of the foot its place on the solid's top, at height
/// `h` above the plate.
fn standing(foot: &[P2], top: &dyn Fn(P2) -> P2, h: f64) -> Mesh {
    let outline = [[0.0, 0.0], [60.0, 0.0], [60.0, 40.0], [0.0, 40.0]];
    let at = |p: P2, z: f64| [p[0], p[1], z];
    let mut t = Vec::new();
    for [a, b, c] in synth::triangulate(&outline, &[foot.to_vec()]) {
        t.push([at(a, 6.0), at(b, 6.0), at(c, 6.0)]);
    }
    for [a, b, c] in synth::triangulate(&outline, &[]) {
        t.push([at(a, 0.0), at(c, 0.0), at(b, 0.0)]);
    }
    for i in 0..4 {
        let (a, b) = (outline[i], outline[(i + 1) % 4]);
        t.extend([[at(a, 0.0), at(b, 0.0), at(b, 6.0)], [at(a, 0.0), at(b, 6.0), at(a, 6.0)]]);
    }
    let roof: Vec<P2> = foot.iter().map(|p| top(*p)).collect();
    for [a, b, c] in synth::triangulate(&roof, &[]) {
        t.push([at(a, 6.0 + h), at(b, 6.0 + h), at(c, 6.0 + h)]);
    }
    let n = foot.len();
    for i in 0..n {
        let (a, b) = (foot[i], foot[(i + 1) % n]);
        let (c, d) = (roof[(i + 1) % n], roof[i]);
        t.extend([[at(a, 6.0), at(b, 6.0), at(c, 6.0 + h)], [at(a, 6.0), at(c, 6.0 + h), at(d, 6.0 + h)]]);
    }
    Mesh::new(t)
}

/// A straight round boss of a synthetic mesh; a drafted one, a twisted one and a block that
/// is no rectangle along X and Y are refused (a square block along them is a pad).
#[test]
fn bosses_must_be_straight_and_round() {
    let foot = circle([20.0, 20.0], 5.0, 32);
    let (plate, _) = found(&standing(&foot, &|p| p, 4.0));
    assert_eq!(plate.bosses, [BossSpec { x: 20.0, y: 20.0, d: 10.0, h: 4.0, fillet: None, draft_deg: None, chamfer: None }]);
    let draft = |p: P2| [20.0 + (p[0] - 20.0) * 0.9, 20.0 + (p[1] - 20.0) * 0.9];
    let (sin, cos) = (PI / 4.0).sin_cos();
    let twist = |p: P2| [20.0 + (p[0] - 20.0) * cos - (p[1] - 20.0) * sin, 20.0 + (p[0] - 20.0) * sin + (p[1] - 20.0) * cos];
    let block = [[15.0, 15.0], [25.0, 15.0], [25.0, 25.0], [15.0, 25.0]];
    let (plate, _) = found(&standing(&block, &|p| p, 4.0));
    assert_eq!(plate.pads, [PadSpec { x: 20.0, y: 20.0, w: 10.0, l: 10.0, h: 4.0 }]);
    let diamond = [[20.0, 14.0], [26.0, 20.0], [20.0, 26.0], [14.0, 20.0]];
    let pad_draft = |p: P2| [20.0 + (p[0] - 20.0) * 0.9, 20.0 + (p[1] - 20.0) * 0.9];
    // A boss narrowing straight up to a top of 0.9 times its radius is drafted, by
    // atan(0.5 / 4) = 7.13°.
    assert_eq!(found(&standing(&foot, &draft, 4.0)).0.bosses[0].draft_deg, Some(7.13));
    reason(&standing(&foot, &twist, 4.0));
    // A diamond standing on the plate is no plate's pad; it is a stack of the plate and the
    // diamond.
    let diamond = standing(&diamond, &|p| p, 4.0);
    assert!(crate::plate::recognize_plate(&diamond, true).is_err());
    let found = crate::recognize::recognize(&diamond);
    let crate::recognize::Recognition::Stack { spec, .. } = found else { panic!("not a stack: {found:?}") };
    assert_eq!(spec.bands.len(), 2);
    // A square pad narrowing straight up is no plate's pad; it is the intersection of two
    // trapezoids extruded along X and Y, a combined extrusion.
    let tapered = standing(&block, &pad_draft, 4.0);
    assert!(crate::plate::recognize_plate(&tapered, true).is_err());
    assert!(matches!(crate::recognize::recognize(&tapered), crate::recognize::Recognition::Combined { .. }));
}

/// A boss whose top is larger than what is left of the plate's top face.
#[test]
fn bosses_larger_than_the_plate_face() {
    let outline = [[0.0, 0.0], [40.0, 0.0], [40.0, 40.0], [0.0, 40.0]];
    let foot = circle([20.0, 20.0], 17.0, 48);
    let mut mesh = standing(&foot, &|p| p, 4.0);
    // `standing` makes a 60 x 40 plate: cut it to 40 x 40 by rebuilding with that outline.
    let at = |p: P2, z: f64| [p[0], p[1], z];
    let mut t: Vec<crate::mesh::Triangle> = mesh.triangles.iter().filter(|t| t.iter().any(|p| p[2] > 6.0 + 1e-9)).copied().collect();
    for [a, b, c] in synth::triangulate(&outline, std::slice::from_ref(&foot)) {
        t.push([at(a, 6.0), at(b, 6.0), at(c, 6.0)]);
    }
    for [a, b, c] in synth::triangulate(&outline, &[]) {
        t.push([at(a, 0.0), at(c, 0.0), at(b, 0.0)]);
    }
    for i in 0..4 {
        let (a, b) = (outline[i], outline[(i + 1) % 4]);
        t.extend([[at(a, 0.0), at(b, 0.0), at(b, 6.0)], [at(a, 0.0), at(b, 6.0), at(a, 6.0)]]);
    }
    mesh = Mesh::new(t);
    let (plate, _) = found(&mesh);
    assert_eq!(plate.bosses, [BossSpec { x: 20.0, y: 20.0, d: 34.0, h: 4.0, fillet: None, draft_deg: None, chamfer: None }]);
}

#[test]
fn split_boss_footprints() {
    let foot = circle([20.0, 20.0], 5.0, 32);
    let n = foot.len();
    let split: Vec<P2> = (0..n)
        .flat_map(|i| [foot[i], [f64::midpoint(foot[i][0], foot[(i + 1) % n][0]), f64::midpoint(foot[i][1], foot[(i + 1) % n][1])]])
        .collect();
    let (plate, _) = found(&standing(&split, &|p| p, 4.0));
    assert_eq!(plate.bosses, [BossSpec { x: 20.0, y: 20.0, d: 10.0, h: 4.0, fillet: None, draft_deg: None, chamfer: None }]);
}

/// A 70 x 40 x 6 plate exported from SolidCraft 0.49 with an Ø10 x 8 boss whose foot is rounded
/// with 2 mm, an Ø8 x 5 boss and an Ø4 hole.
#[test]
fn recognises_rounded_boss_feet() {
    let mesh = crate::stl::parse(include_bytes!("../../../../../plugin/revmesh/tests/boss-fillet.stl")).expect("parses");
    let (plate, _) = found(&mesh);
    let boss = |x: f64, y: f64, d: f64, h: f64, fillet: Option<f64>| BossSpec { x, y, d, h, fillet, draft_deg: None, chamfer: None };
    assert_eq!(plate.bosses, [boss(20.0, 20.0, 10.0, 8.0, Some(2.0)), boss(50.0, 22.0, 8.0, 5.0, None)]);
    // SolidCraft measured 17,633.625 mm³ for the part it exported.
    assert!((plate.volume() - 17_633.625).abs() < 0.001, "{}", plate.volume());
}

/// A 60 x 40 x 6 plate with a boss at (20, 20) of `n` facets round, its wall a stack of
/// `rings` (radius, height above the plate) from the footprint up, and a flat top.
fn layered_boss(rings: &[(f64, f64)], n: usize) -> Mesh {
    let outline = [[0.0, 0.0], [60.0, 0.0], [60.0, 40.0], [0.0, 40.0]];
    let at = |p: P2, z: f64| [p[0], p[1], z];
    let ring = |r: f64| circle([20.0, 20.0], r, n);
    let foot = ring(rings[0].0);
    let mut t = Vec::new();
    for [a, b, c] in synth::triangulate(&outline, std::slice::from_ref(&foot)) {
        t.push([at(a, 6.0), at(b, 6.0), at(c, 6.0)]);
    }
    for [a, b, c] in synth::triangulate(&outline, &[]) {
        t.push([at(a, 0.0), at(c, 0.0), at(b, 0.0)]);
    }
    for i in 0..4 {
        let (a, b) = (outline[i], outline[(i + 1) % 4]);
        t.extend([[at(a, 0.0), at(b, 0.0), at(b, 6.0)], [at(a, 0.0), at(b, 6.0), at(a, 6.0)]]);
    }
    for pair in rings.windows(2) {
        let ((r0, z0), (r1, z1)) = (pair[0], pair[1]);
        let (lo, hi) = (ring(r0), ring(r1));
        for i in 0..n {
            let j = (i + 1) % n;
            t.extend([
                [at(lo[i], 6.0 + z0), at(lo[j], 6.0 + z0), at(hi[j], 6.0 + z1)],
                [at(lo[i], 6.0 + z0), at(hi[j], 6.0 + z1), at(hi[i], 6.0 + z1)],
            ]);
        }
    }
    let (r, z) = rings[rings.len() - 1];
    for [a, b, c] in synth::triangulate(&ring(r), &[]) {
        t.push([at(a, 6.0 + z), at(b, 6.0 + z), at(c, 6.0 + z)]);
    }
    Mesh::new(t)
}

/// A boss of 48 facets with a 2 mm round at its foot, the round in 6 bands, is recognised; one
/// with a 2 mm 45° chamfer there instead is refused.
#[test]
fn rounded_feet_and_chamfered_feet() {
    let (r, f) = (5.0, 2.0);
    let mut rings: Vec<(f64, f64)> = (0..=6).map(|k| f64::from(k) * PI / 12.0).map(|a| (r + f - f * a.sin(), f - f * a.cos())).collect();
    rings.push((r, 8.0));
    let (plate, _) = found(&layered_boss(&rings, 48));
    assert_eq!(plate.bosses, [BossSpec { x: 20.0, y: 20.0, d: 10.0, h: 8.0, fillet: Some(2.0), draft_deg: None, chamfer: None }]);
    reason(&layered_boss(&[(r + f, 0.0), (r, f), (r, 8.0)], 48));
}

/// A round at the foot of an Ø20 boss in two bands of 45° each, the coarsest allowed: the middle
/// of each band's edge lies its exact sagitta inside the round.
#[test]
fn rounded_feet_in_two_bands() {
    let (r, f) = (10.0, 2.0);
    let mut rings: Vec<(f64, f64)> = [0.0, PI / 4.0, PI / 2.0].iter().map(|a| (r + f - f * a.sin(), f - f * a.cos())).collect();
    rings.push((r, 8.0));
    let (plate, _) = found(&layered_boss(&rings, 48));
    assert_eq!(plate.bosses, [BossSpec { x: 20.0, y: 20.0, d: 20.0, h: 8.0, fillet: Some(2.0), draft_deg: None, chamfer: None }]);
}

/// A rounded boss of only 16 facets round, its top a polygon well inside the circle; a cone
/// from the footprint straight up to a smaller top is no round; and a cylinder wall facet split
/// at its diagonal's middle (the new corner inside the circle) still is the wall.
#[test]
fn rounded_feet_coarse_coned_and_split() {
    let (r, f) = (5.0, 2.0);
    let mut rings: Vec<(f64, f64)> = (0..=6).map(|k| f64::from(k) * PI / 12.0).map(|a| (r + f - f * a.sin(), f - f * a.cos())).collect();
    rings.push((r, 8.0));
    let (plate, _) = found(&layered_boss(&rings, 16));
    assert_eq!(plate.bosses, [BossSpec { x: 20.0, y: 20.0, d: 10.0, h: 8.0, fillet: Some(2.0), draft_deg: None, chamfer: None }]);
    // A cone from the footprint straight up to a smaller top is a drafted boss, no round.
    let (plate, _) = found(&layered_boss(&[(6.0, 0.0), (2.0, 4.1)], 48));
    assert_eq!((plate.bosses[0].draft_deg, plate.bosses[0].fillet), (Some(44.29), None));
    // A 45° chamfer in four bands, on a coarse boss of 8 facets round, is no 1 mm round.
    reason(&layered_boss(&[(6.0, 0.0), (5.75, 0.25), (5.5, 0.5), (5.25, 0.75), (5.0, 1.0), (5.0, 8.0)], 8));
    let mut split = layered_boss(&rings, 48);
    // The wall's first facet above the round: corners at Z = 6 + 2 and 6 + 8 on the cylinder.
    let k = split
        .triangles
        .iter()
        .position(|t| t.iter().all(|p| p[2] >= 8.0 - 1e-9) && t.iter().any(|p| p[2] < 14.0 - 1e-9) && t.iter().any(|p| p[2] > 14.0 - 1e-9))
        .expect("a wall facet");
    let [a, b, c] = split.triangles.swap_remove(k);
    // Split the edge whose ends differ most (the diagonal) at its middle.
    let pairs = [(a, b, c), (b, c, a), (c, a, b)];
    let (p, q, o) = pairs
        .into_iter()
        .max_by(|x, y| crate::mesh::length(crate::mesh::sub(x.0, x.1)).total_cmp(&crate::mesh::length(crate::mesh::sub(y.0, y.1))))
        .expect("edges");
    let m = [f64::midpoint(p[0], q[0]), f64::midpoint(p[1], q[1]), f64::midpoint(p[2], q[2])];
    split.triangles.extend([[p, m, o], [m, q, o]]);
    let (plate, _) = found(&split);
    assert_eq!(plate.bosses[0].fillet, Some(2.0));
}

/// A rounded boss whose top is a fan from its middle, each facet split by a corner at 98% of
/// the way to its rim edge's middle: corners inside the top near its rim are no rim.
#[test]
fn rounded_boss_tops_with_corners_near_the_rim() {
    let (r, f, n) = (5.0, 2.0, 48);
    let mut rings: Vec<(f64, f64)> = (0..=6).map(|k| f64::from(k) * PI / 12.0).map(|a| (r + f - f * a.sin(), f - f * a.cos())).collect();
    rings.push((r, 8.0));
    let mut mesh = layered_boss(&rings, n);
    let top = 14.0;
    mesh.triangles.retain(|t| !t.iter().all(|p| (p[2] - top).abs() < 1e-9));
    let rim = circle([20.0, 20.0], r, n);
    let c = [20.0, 20.0, top];
    for i in 0..n {
        let (a, b) = (rim[i], rim[(i + 1) % n]);
        let q = [20.0 + 0.98 * (f64::midpoint(a[0], b[0]) - 20.0), 20.0 + 0.98 * (f64::midpoint(a[1], b[1]) - 20.0), top];
        let (a, b) = ([a[0], a[1], top], [b[0], b[1], top]);
        mesh.triangles.extend([[c, a, q], [a, b, q], [b, c, q]]);
    }
    let (plate, _) = found(&mesh);
    assert_eq!(plate.bosses, [BossSpec { x: 20.0, y: 20.0, d: 10.0, h: 8.0, fillet: Some(2.0), draft_deg: None, chamfer: None }]);
}

/// A round of 60 bands on an Ø10 boss: its first band rises less than the tolerance and lies in
/// the top face, so the footprint is smaller than the round's; the round's radius comes from its
/// corners. A boss whose cylinder is turned 45° above its round is no boss.
#[test]
fn finely_banded_rounds_and_twisted_walls() {
    let (r, f) = (5.0, 2.0);
    let mut rings: Vec<(f64, f64)> = (0..=60).map(|k| f64::from(k) * PI / 120.0).map(|a| (r + f - f * a.sin(), f - f * a.cos())).collect();
    rings.push((r, 8.0));
    let (plate, _) = found(&layered_boss(&rings, 48));
    assert_eq!(plate.bosses, [BossSpec { x: 20.0, y: 20.0, d: 10.0, h: 8.0, fillet: Some(2.0), draft_deg: None, chamfer: None }]);
    let mut rings: Vec<(f64, f64)> = (0..=6).map(|k| f64::from(k) * PI / 12.0).map(|a| (r + f - f * a.sin(), f - f * a.cos())).collect();
    rings.push((r, 8.0));
    let mesh = layered_boss(&rings, 48);
    // Turn the top ring (and the top) by 45° about the boss's axis.
    let (sin, cos) = (PI / 4.0).sin_cos();
    let turned = Mesh::new(
        mesh.triangles
            .iter()
            .map(|t| {
                t.map(|p| {
                    if p[2] > 14.0 - 1e-9 {
                        let (x, y) = (p[0] - 20.0, p[1] - 20.0);
                        [20.0 + x * cos - y * sin, 20.0 + x * sin + y * cos, p[2]]
                    } else {
                        p
                    }
                })
            })
            .collect(),
    );
    reason(&turned);
}

/// A round of six bands on a boss of 192 facets round (its edges along the round travel further
/// out than one facet is wide), and the same round of 30 bands moved 10 m along X and rounded
/// to single precision, as a binary STL stores it.
#[test]
fn rounded_feet_fine_round_and_far_away() {
    let (r, f) = (5.0, 2.0);
    let rings = |bands: u32| -> Vec<(f64, f64)> {
        let mut rings: Vec<(f64, f64)> =
            (0..=bands).map(|k| f64::from(k) * PI / f64::from(2 * bands)).map(|a| (r + f - f * a.sin(), f - f * a.cos())).collect();
        rings.push((r, 8.0));
        rings
    };
    let expected = [BossSpec { x: 20.0, y: 20.0, d: 10.0, h: 8.0, fillet: Some(2.0), draft_deg: None, chamfer: None }];
    assert_eq!(found(&layered_boss(&rings(6), 192)).0.bosses, expected);
    let mesh = layered_boss(&rings(30), 48);
    #[allow(clippy::cast_possible_truncation)] // single precision, as a binary STL stores it
    let far = Mesh::new(
        mesh.triangles
            .iter()
            .map(|t| t.map(|p| [f64::from((p[0] + 10_000.0) as f32), f64::from(p[1] as f32), f64::from(p[2] as f32)]))
            .collect(),
    );
    assert_eq!(found(&far).0.bosses, expected);
}

/// A 0.1 mm round of 30 bands, its first bands within the tolerance of the plate's face; and a
/// rounded boss whose top lacks half its facets, which is no top.
#[test]
fn rounded_feet_tiny_round_and_open_top() {
    let (r, f) = (5.0, 0.1);
    let mut rings: Vec<(f64, f64)> = (0..=30).map(|k| f64::from(k) * PI / 60.0).map(|a| (r + f - f * a.sin(), f - f * a.cos())).collect();
    rings.push((r, 8.0));
    assert_eq!(
        found(&layered_boss(&rings, 48)).0.bosses,
        [BossSpec { x: 20.0, y: 20.0, d: 10.0, h: 8.0, fillet: Some(0.1), draft_deg: None, chamfer: None }]
    );
    let f = 2.0;
    let mut rings: Vec<(f64, f64)> = (0..=6).map(|k| f64::from(k) * PI / 12.0).map(|a| (r + f - f * a.sin(), f - f * a.cos())).collect();
    rings.push((r, 8.0));
    let mut mesh = layered_boss(&rings, 48);
    let mut kept = 0;
    mesh.triangles.retain(|t| {
        if t.iter().all(|p| (p[2] - 14.0).abs() < 1e-9) {
            kept += 1;
            kept <= 23
        } else {
            true
        }
    });
    reason(&mesh);
}

/// A facet of a round split into three at its centroid: the new corner lies on the facet, a
/// little off the round, and the boss is the same.
#[test]
fn split_round_facets() {
    let (r, f) = (5.0, 2.0);
    let mut rings: Vec<(f64, f64)> = (0..=6).map(|k| f64::from(k) * PI / 12.0).map(|a| (r + f - f * a.sin(), f - f * a.cos())).collect();
    rings.push((r, 8.0));
    let mut mesh = layered_boss(&rings, 48);
    // A facet of the band between 30° and 45° along the round.
    let (lo, hi) = (6.0 + rings[2].1, 6.0 + rings[3].1);
    let k = mesh
        .triangles
        .iter()
        .position(|t| t.iter().all(|p| p[2] >= lo - 1e-9 && p[2] <= hi + 1e-9) && t.iter().any(|p| p[2] < hi - 1e-9))
        .expect("a facet of the band");
    let [a, b, c] = mesh.triangles.swap_remove(k);
    let m = [(a[0] + b[0] + c[0]) / 3.0, (a[1] + b[1] + c[1]) / 3.0, (a[2] + b[2] + c[2]) / 3.0];
    mesh.triangles.extend([[a, b, m], [b, c, m], [c, a, m]]);
    assert_eq!(found(&mesh).0.bosses, [BossSpec { x: 20.0, y: 20.0, d: 10.0, h: 8.0, fillet: Some(2.0), draft_deg: None, chamfer: None }]);
}

/// A 70 x 40 x 6 plate exported from SolidCraft 0.49 with an Ø12 x 8 boss drafted by 5°, an
/// Ø10 x 6 boss whose top edge is chamfered by 1 mm, and an Ø4 hole.
#[test]
fn recognises_drafted_and_chamfered_bosses() {
    let mesh = crate::stl::parse(include_bytes!("../../../../../plugin/revmesh/tests/boss-draft-chamfer.stl")).expect("parses");
    let (plate, _) = found(&mesh);
    let boss = |x: f64, y: f64, d: f64, h: f64| BossSpec { x, y, d, h, fillet: None, draft_deg: None, chamfer: None };
    assert_eq!(
        plate.bosses,
        [
            BossSpec { draft_deg: Some(5.0), ..boss(20.0, 20.0, 12.0, 8.0) },
            BossSpec { chamfer: Some(BossChamfer { w: 1.0, h: 1.0 }), ..boss(50.0, 22.0, 10.0, 6.0) },
        ]
    );
    // SolidCraft measured 17,984.519 mm³ for the part it exported.
    assert!((plate.volume() - 17_984.519).abs() < 0.001, "{}", plate.volume());
}

/// Chamfers of a synthetic boss's top edge, 45° and not, are found; a wall that bends twice on
/// its way up is neither a draft nor a chamfer.
#[test]
fn chamfered_boss_tops() {
    let chamfer = |rings: &[(f64, f64)]| found(&layered_boss(rings, 48)).0.bosses[0].chamfer;
    assert_eq!(chamfer(&[(5.0, 0.0), (5.0, 7.0), (4.0, 8.0)]), Some(BossChamfer { w: 1.0, h: 1.0 }));
    assert_eq!(chamfer(&[(5.0, 0.0), (5.0, 7.5), (3.5, 8.0)]), Some(BossChamfer { w: 1.5, h: 0.5 }));
    reason(&layered_boss(&[(5.0, 0.0), (5.0, 6.0), (4.6, 7.0), (4.0, 8.0)], 48));
}

/// A boss whose foot is faceted into only six corners is a hexagonal block, not a round one:
/// the RANSAC fit keeps the original 50° gate on the ring's own edges, so no corner is dropped
/// and the foot never becomes a round boss (roadmap 6.2) — a hexagon in a face's loop stays a
/// cut-out too. The block comes over as a stack's hexagonal band instead.
#[test]
fn a_six_faceted_boss_foot_is_not_round() {
    let foot = circle([20.0, 20.0], 5.0, 6);
    match crate::recognize::recognize(&standing(&foot, &|p| p, 4.0)) {
        Recognition::Stack { spec, .. } => assert_eq!(spec.bands[1].outline.len(), 6),
        other => panic!("expected a stack of the plate and a hexagonal band, got {other:?}"),
    }
}
