//! Synthetic meshes of known parts, tessellated the way CAD exporters do it (facet corners on the
//! true surfaces). Tests use them to check recognisers against exact answers, and the
//! `plate_stl` example writes the STL fixtures SolidCraft's plugin tests import.

#![allow(clippy::many_single_char_names)] // points a, b, c and radii r, as in the geometry

use std::f64::consts::PI;

use crate::mesh::{Mesh, Triangle, Vec3};
use crate::recognize::{HoleSpec, PlateSpec, ProfileSpec};

type P2 = [f64; 2];

/// The plate `spec` as a closed mesh, moved by `offset`. Each hole is a polygon of `segments`
/// corners and each rounded corner a quarter of one (at least one facet).
#[must_use]
pub fn plate(spec: &PlateSpec, segments: usize, offset: Vec3) -> Mesh {
    let corners = [[0.0, 0.0], [spec.x, 0.0], [spec.x, spec.y], [0.0, spec.y]];
    prism(&rounded(&corners, spec.outline_fillet_r, segments), &rings(&spec.holes, segments), spec.z, offset)
}

/// The plate `spec` with any outline as a closed mesh, moved by `offset`, faceted as [`plate`].
#[must_use]
pub fn profile(spec: &ProfileSpec, segments: usize, offset: Vec3) -> Mesh {
    prism(&rounded(&spec.corners, spec.outline_fillet_r, segments), &rings(&spec.holes, segments), spec.z, offset)
}

/// A straight prism of height `z` over the counter-clockwise `outline` with counter-clockwise
/// `holes` in it, moved by `offset`.
///
/// # Panics
///
/// If the outline and holes do not form a simple polygon with holes (the triangulation finds no
/// ear or no bridge to a hole).
#[must_use]
pub fn prism(outline: &[P2], holes: &[Vec<P2>], z: f64, offset: Vec3) -> Mesh {
    let at = |p: P2, h: f64| [p[0] + offset[0], p[1] + offset[1], h + offset[2]];
    let mut triangles: Vec<Triangle> = Vec::new();
    for [a, b, c] in triangulate(outline, holes) {
        triangles.push([at(a, z), at(b, z), at(c, z)]);
        triangles.push([at(a, 0.0), at(c, 0.0), at(b, 0.0)]);
    }
    // Walls: the outline runs counter-clockwise and each hole, seen as the top face's boundary,
    // clockwise; both give outward walls with the same winding.
    let reversed: Vec<Vec<P2>> = holes.iter().map(|ring| ring.iter().rev().copied().collect()).collect();
    for ring in std::iter::once(outline).chain(reversed.iter().map(Vec::as_slice)) {
        for i in 0..ring.len() {
            let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
            triangles.push([at(a, 0.0), at(b, 0.0), at(b, z)]);
            triangles.push([at(a, 0.0), at(b, z), at(a, z)]);
        }
    }
    Mesh::new(triangles)
}

/// Each hole as a counter-clockwise polygon of `segments` corners on its circle.
fn rings(holes: &[HoleSpec], segments: usize) -> Vec<Vec<P2>> {
    holes
        .iter()
        .map(|h| {
            (0..segments)
                .map(|i| 2.0 * PI * i as f64 / segments as f64)
                .map(|a| [h.x + h.d / 2.0 * a.cos(), h.y + h.d / 2.0 * a.sin()])
                .collect()
        })
        .collect()
}

/// The outline through `corners` (counter-clockwise) with every corner rounded with radius `r`,
/// each arc faceted like a circle of `segments` facets (at least one facet per corner).
fn rounded(corners: &[P2], r: Option<f64>, segments: usize) -> Vec<P2> {
    let Some(r) = r else { return corners.to_vec() };
    let n = corners.len();
    let mut out = Vec::new();
    for i in 0..n {
        let (a, c, b) = (corners[(i + n - 1) % n], corners[i], corners[(i + 1) % n]);
        let unit = |p: P2, q: P2| {
            let len = (q[0] - p[0]).hypot(q[1] - p[1]);
            [(q[0] - p[0]) / len, (q[1] - p[1]) / len]
        };
        let (d1, d2) = (unit(a, c), unit(c, b));
        let turn = (d1[0] * d2[1] - d1[1] * d2[0]).atan2(d1[0] * d2[0] + d1[1] * d2[1]);
        let t = r * (turn.abs() / 2.0).tan();
        let start = [c[0] - d1[0] * t, c[1] - d1[1] * t];
        // The centre lies to the left of the incoming side for an outside corner, to its right
        // for an inside one.
        let side = r * turn.signum();
        let centre = [start[0] - d1[1] * side, start[1] + d1[0] * side];
        #[allow(clippy::cast_sign_loss)] // a share of `segments`, never negative
        let facets = ((segments as f64 * turn.abs() / (2.0 * PI)).round() as usize).max(1);
        let (sx, sy) = (start[0] - centre[0], start[1] - centre[1]);
        for j in 0..=facets {
            let angle = turn * j as f64 / facets as f64;
            let (sin, cos) = angle.sin_cos();
            out.push([centre[0] + sx * cos - sy * sin, centre[1] + sx * sin + sy * cos]);
        }
    }
    out
}

fn orient(a: P2, b: P2, c: P2) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

fn same(p: P2, q: P2) -> bool {
    p[0].to_bits() == q[0].to_bits() && p[1].to_bits() == q[1].to_bits()
}

/// Whether segment `a b` crosses an edge of `ring` other than at a shared corner.
fn crosses(a: P2, b: P2, ring: &[P2]) -> bool {
    (0..ring.len()).any(|i| {
        let (c, d) = (ring[i], ring[(i + 1) % ring.len()]);
        if [c, d].iter().any(|p| same(*p, a) || same(*p, b)) {
            return false;
        }
        let (o1, o2, o3, o4) = (orient(a, b, c), orient(a, b, d), orient(c, d, a), orient(c, d, b));
        o1 * o2 < 0.0 && o3 * o4 < 0.0
    })
}

/// Ear clipping, after bridging each hole into the outline from its rightmost corner.
fn triangulate(outline: &[P2], holes: &[Vec<P2>]) -> Vec<[P2; 3]> {
    let n = outline.len();
    if holes.is_empty() && (0..n).all(|i| orient(outline[(i + n - 1) % n], outline[i], outline[(i + 1) % n]) > 0.0) {
        // A convex outline: a fan from its first corner (and no cubic ear search for test
        // outlines of thousands of corners).
        return (1..n - 1).map(|i| [outline[0], outline[i], outline[i + 1]]).collect();
    }
    let max_x = |ring: &[P2]| ring.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max);
    let mut holes: Vec<Vec<P2>> = holes.iter().map(|h| h.iter().rev().copied().collect()).collect();
    holes.sort_by(|a, b| max_x(b).total_cmp(&max_x(a)));
    let mut poly = outline.to_vec();
    for k in 0..holes.len() {
        let hole = &holes[k];
        let m = (0..hole.len()).max_by(|&a, &b| hole[a][0].total_cmp(&hole[b][0])).expect("a hole has corners");
        let from = hole[m];
        let mut candidates: Vec<usize> = (0..poly.len()).collect();
        let dist = |p: P2| (p[0] - from[0]).powi(2) + (p[1] - from[1]).powi(2);
        candidates.sort_by(|&a, &b| dist(poly[a]).total_cmp(&dist(poly[b])));
        let bridge = candidates
            .into_iter()
            .find(|&p| !crosses(from, poly[p], &poly) && holes[k..].iter().all(|h| !crosses(from, poly[p], h)))
            .expect("every hole can see the outline");
        let mut merged = poly[..=bridge].to_vec();
        merged.extend(hole[m..].iter().chain(&hole[..=m]));
        merged.extend_from_slice(&poly[bridge..]);
        poly = merged;
    }

    let mut out = Vec::new();
    while poly.len() > 3 {
        let n = poly.len();
        let ear = (0..n)
            .find(|&i| {
                let (a, b, c) = (poly[(i + n - 1) % n], poly[i], poly[(i + 1) % n]);
                orient(a, b, c) > 0.0
                    && poly.iter().all(|&p| {
                        same(p, a)
                            || same(p, b)
                            || same(p, c)
                            || !(orient(a, b, p) >= 0.0 && orient(b, c, p) >= 0.0 && orient(c, a, p) >= 0.0)
                    })
            })
            .expect("a simple polygon always has an ear");
        out.push([poly[(ear + n - 1) % n], poly[ear], poly[(ear + 1) % n]]);
        poly.remove(ear);
    }
    out.push([poly[0], poly[1], poly[2]]);
    out
}
