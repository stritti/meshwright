use std::f64::consts::PI;

use crate::mesh::Mesh;
use crate::plate::P2;
use crate::recognize::{BossSpec, PadSpec};
use crate::synth;

use super::{circle, found, reason};

/// A 70 x 40 x 6 plate exported from SolidCraft 0.47 with an Ø10 x 5 and an Ø8 x 3 boss on
/// its top face and an Ø5 hole.
#[test]
fn recognises_bosses() {
    let mesh = crate::stl::parse(include_bytes!("../../../../../plugin/revmesh/tests/bosses.stl")).expect("parses");
    let (plate, _) = found(&mesh);
    let boss = |x: f64, y: f64, d: f64, h: f64| BossSpec { x, y, d, h };
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
    assert_eq!(plate.bosses, [BossSpec { x: 20.0, y: 20.0, d: 10.0, h: 4.0 }]);
    let draft = |p: P2| [20.0 + (p[0] - 20.0) * 0.9, 20.0 + (p[1] - 20.0) * 0.9];
    let (sin, cos) = (PI / 4.0).sin_cos();
    let twist = |p: P2| [20.0 + (p[0] - 20.0) * cos - (p[1] - 20.0) * sin, 20.0 + (p[0] - 20.0) * sin + (p[1] - 20.0) * cos];
    let block = [[15.0, 15.0], [25.0, 15.0], [25.0, 25.0], [15.0, 25.0]];
    let (plate, _) = found(&standing(&block, &|p| p, 4.0));
    assert_eq!(plate.pads, [PadSpec { x: 20.0, y: 20.0, w: 10.0, l: 10.0, h: 4.0 }]);
    let diamond = [[20.0, 14.0], [26.0, 20.0], [20.0, 26.0], [14.0, 20.0]];
    let pad_draft = |p: P2| [20.0 + (p[0] - 20.0) * 0.9, 20.0 + (p[1] - 20.0) * 0.9];
    for mesh in
        [standing(&foot, &draft, 4.0), standing(&foot, &twist, 4.0), standing(&diamond, &|p| p, 4.0), standing(&block, &pad_draft, 4.0)]
    {
        reason(&mesh);
    }
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
    assert_eq!(plate.bosses, [BossSpec { x: 20.0, y: 20.0, d: 34.0, h: 4.0 }]);
}

#[test]
fn split_boss_footprints() {
    let foot = circle([20.0, 20.0], 5.0, 32);
    let n = foot.len();
    let split: Vec<P2> = (0..n)
        .flat_map(|i| [foot[i], [f64::midpoint(foot[i][0], foot[(i + 1) % n][0]), f64::midpoint(foot[i][1], foot[(i + 1) % n][1])]])
        .collect();
    let (plate, _) = found(&standing(&split, &|p| p, 4.0));
    assert_eq!(plate.bosses, [BossSpec { x: 20.0, y: 20.0, d: 10.0, h: 4.0 }]);
}
