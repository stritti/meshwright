mod blocks;
mod bosses;
mod cutouts;
mod geometry;
mod holes;
mod outline;
mod prism;
mod slots;
mod walls;

use std::f64::consts::PI;

use super::P2;
use crate::mesh::{Mesh, Vec3};
use crate::recognize::{HoleSpec, PlateSpec, ProfileSpec, Recognition, recognize};
use crate::synth;

fn spec(holes: &[(f64, f64, f64)], r: Option<f64>) -> PlateSpec {
    PlateSpec {
        x: 80.0,
        y: 50.0,
        z: 5.0,
        holes: holes.iter().map(|&(x, y, d)| HoleSpec { x, y, d, head: None, blind: None }).collect(),
        cutouts: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![],
        slots: vec![],
        outline_fillet_r: r,
        edge_fillet_r: None,
    }
}

/// An L: 60 along X and 15 deep, with a 15 wide leg up to 40, one hole in each arm.
fn ell(r: Option<f64>) -> ProfileSpec {
    ProfileSpec {
        corners: vec![[0.0, 0.0], [60.0, 0.0], [60.0, 15.0], [15.0, 15.0], [15.0, 40.0], [0.0, 40.0]],
        z: 5.0,
        holes: vec![
            HoleSpec { x: 7.5, y: 30.0, d: 5.0, head: None, blind: None },
            HoleSpec { x: 45.0, y: 7.5, d: 6.0, head: None, blind: None },
        ],
        cutouts: vec![],
        pads: vec![],
        pockets: vec![],
        outline_pockets: Vec::new(),
        bosses: vec![],
        slots: vec![],
        outline_fillet_r: r,
        arcs: Vec::new(),
    }
}

fn found(mesh: &Mesh) -> (PlateSpec, Vec3) {
    match recognize(mesh) {
        Recognition::Plate { plate, offset } => (plate, offset),
        other => panic!("not recognised as a rectangular plate: {other:?}"),
    }
}

fn found_profile(mesh: &Mesh) -> (ProfileSpec, Vec3) {
    match recognize(mesh) {
        Recognition::Profile { profile, offset, .. } => (profile, offset),
        other => panic!("not recognised as a profile: {other:?}"),
    }
}

fn reason(mesh: &Mesh) -> String {
    match recognize(mesh) {
        Recognition::Unrecognized { reason } => reason,
        other => panic!("unexpectedly recognised {other:?}"),
    }
}

/// Splits every wall facet at the middle of its vertical edge, moved `inward` mm towards
/// the plate's middle, as an exporter that meshes walls in bands would (with `inward` 0).
#[allow(clippy::float_cmp)] // the generator puts a wall's corners exactly above each other
fn banded(mesh: &Mesh, inward: f64, middle: P2) -> Mesh {
    let mut out = Vec::new();
    for t in &mesh.triangles {
        let vertical_edge = (0..3).find(|&k| {
            let (p, q) = (t[k], t[(k + 1) % 3]);
            p[0] == q[0] && p[1] == q[1] && p[2] != q[2]
        });
        let Some(k) = vertical_edge else {
            out.push(*t);
            continue;
        };
        let (p, q, r) = (t[k], t[(k + 1) % 3], t[(k + 2) % 3]);
        let toward = [middle[0] - p[0], middle[1] - p[1]];
        let len = toward[0].hypot(toward[1]);
        let m = [p[0] + toward[0] / len * inward, p[1] + toward[1] / len * inward, f64::midpoint(p[2], q[2])];
        out.push([p, m, r]);
        out.push([m, q, r]);
    }
    Mesh::new(out)
}

fn circle(centre: P2, r: f64, n: usize) -> Vec<P2> {
    (0..n).map(|i| 2.0 * PI * i as f64 / n as f64).map(|a| [centre[0] + r * a.cos(), centre[1] + r * a.sin()]).collect()
}

/// A slot's loop: `n` facets on each round end, counter-clockwise.
fn stadium(centre: P2, len: f64, w: f64, angle: f64, n: usize) -> Vec<P2> {
    let (sin, cos) = angle.to_radians().sin_cos();
    let mut ring = Vec::new();
    for (end, start) in [(len / 2.0, -PI / 2.0), (-len / 2.0, PI / 2.0)] {
        for i in 0..=n {
            let a = start + PI * i as f64 / n as f64;
            let (s, t) = (end + w / 2.0 * a.cos(), w / 2.0 * a.sin());
            ring.push([centre[0] + s * cos - t * sin, centre[1] + s * sin + t * cos]);
        }
    }
    ring
}

fn plate_with_cutout(cutout: Vec<P2>) -> Mesh {
    synth::prism(&[[0.0, 0.0], [80.0, 0.0], [80.0, 50.0], [0.0, 50.0]], &[cutout], 6.0, [0.0; 3])
}
