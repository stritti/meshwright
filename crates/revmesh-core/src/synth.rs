//! Synthetic meshes of known parts, tessellated the way CAD exporters do it (facet corners on the
//! true surfaces). Tests use them to check recognisers against exact answers, and the
//! `plate_stl` example writes the STL fixtures SolidCraft's plugin tests import.

#![allow(clippy::many_single_char_names)] // points a, b, c and radii r, as in the geometry

use std::f64::consts::PI;

use crate::mesh::{Mesh, Triangle, Vec3};
use crate::recognize::{ArcSide, CutoutSpec, HoleSpec, PlateSpec, ProfileSpec};

type P2 = [f64; 2];

/// The plate `spec` as a closed mesh, moved by `offset`. Each hole is a polygon of `segments`
/// corners and each rounded corner a quarter of one (at least one facet).
#[must_use]
pub fn plate(spec: &PlateSpec, segments: usize, offset: Vec3) -> Mesh {
    let corners = [[0.0, 0.0], [spec.x, 0.0], [spec.x, spec.y], [0.0, spec.y]];
    prism(&rounded(&corners, &spec.outline_fillet_rs, segments), &openings(&spec.holes, &spec.cutouts, segments), spec.z, offset)
}

/// The plate `spec` with any outline as a closed mesh, moved by `offset`, faceted as [`plate`].
#[must_use]
pub fn profile(spec: &ProfileSpec, segments: usize, offset: Vec3) -> Mesh {
    let arcs = spec.arc_sides();
    let outline = if arcs.iter().any(Option::is_some) {
        faceted_polygon(&spec.corners, arcs, segments)
    } else {
        rounded(&spec.corners, &spec.outline_fillet_rs, segments)
    };
    prism(&outline, &openings(&spec.holes, &spec.cutouts, segments), spec.z, offset)
}

/// The spec's openings — its holes and its cut-outs — as counter-clockwise rings, each hole a
/// polygon of `segments` corners and each arc side faceted like [`faceted_polygon`].
fn openings(holes: &[HoleSpec], cutouts: &[CutoutSpec], segments: usize) -> Vec<Vec<P2>> {
    let mut rings = rings(holes, segments);
    rings.extend(cutouts.iter().map(|c| faceted_polygon(&c.corners, c.arc_sides(), segments)));
    rings
}

/// The closed polygon through `corners` (counter-clockwise) with each arc side `arcs[i]` faceted
/// into about `segments` facets per quarter circle, as a CAD exporter meshes it.
#[must_use]
pub fn faceted_polygon(corners: &[P2], arcs: &[Option<ArcSide>], segments: usize) -> Vec<P2> {
    expand_arcs(corners, arcs, segments)
}

/// The outline's corners with each arc side faceted into about `segments` facets per quarter
/// circle, as a CAD exporter meshes it.
#[allow(clippy::cast_sign_loss)] // theta is in (0, 2 pi], so the quarter count is at least 1
fn expand_arcs(corners: &[P2], arcs: &[Option<ArcSide>], segments: usize) -> Vec<P2> {
    let n = corners.len();
    let mut out = Vec::new();
    for i in 0..n {
        let a = corners[i];
        out.push(a);
        if let Some(arc) = arcs.get(i).copied().flatten() {
            let b = corners[(i + 1) % n];
            let aa = (a[1] - arc.centre[1]).atan2(a[0] - arc.centre[0]);
            let theta = ((b[1] - arc.centre[1]).atan2(b[0] - arc.centre[0]) - aa).rem_euclid(2.0 * std::f64::consts::PI);
            let steps = (((theta / (std::f64::consts::PI / 2.0)).round() as usize).max(1)) * segments;
            for k in 1..steps {
                let t = aa + theta * k as f64 / steps as f64;
                out.push([arc.centre[0] + arc.r * t.cos(), arc.centre[1] + arc.r * t.sin()]);
            }
        }
    }
    out
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

/// A rectangular box `x` by `y` by `z` with all twelve edges rounded with radius `r` (the
/// Minkowski sum of the inner box and a ball of radius `r`), moved by `offset`, tessellated the
/// way CAD exporters do it: six inset rectangular faces, twelve quarter cylinders and eight
/// sphere octants, each faceted with about `segments` facets per quarter circle.
///
/// # Panics
///
/// If `r` is not smaller than half the smallest side.
#[must_use]
#[allow(clippy::too_many_lines)] // the three patch kinds, read best in one piece
pub fn rounded_box(x: f64, y: f64, z: f64, r: f64, segments: usize, offset: Vec3) -> Mesh {
    assert!(2.0 * r < x.min(y).min(z), "the fillet radius must be under half the smallest side");
    let size = [x, y, z];
    let at = |p: Vec3| [p[0] + offset[0], p[1] + offset[1], p[2] + offset[2]];
    let mut triangles: Vec<Triangle> = Vec::new();

    // One planar face: the rectangle inset by `r` in the plane spanned by axes `a` and `b`, at
    // the fixed coordinate `fixed` on axis `c`, each side faceted into `segments` segments so the
    // ring's edges pair with the quarter cylinders' ends. `flip` turns the winding for the
    // face's outward side.
    let mut face = |a: usize, b: usize, c: usize, fixed: f64, flip: bool| {
        let (lo_a, hi_a) = (r, size[a] - r);
        let (lo_b, hi_b) = (r, size[b] - r);
        let corners = [[lo_a, lo_b], [hi_a, lo_b], [hi_a, hi_b], [lo_a, hi_b]];
        // The ring stays counter-clockwise; `flip` turns the emitted triangles instead.
        let n = segments.max(1);
        let mut ring: Vec<P2> = Vec::with_capacity(4 * n);
        for i in 0..4 {
            let (p, q) = (corners[i], corners[(i + 1) % 4]);
            for j in 0..n {
                #[allow(clippy::cast_precision_loss)]
                let t = j as f64 / n as f64;
                ring.push([p[0] + t * (q[0] - p[0]), p[1] + t * (q[1] - p[1])]);
            }
        }
        let project = |p: P2| {
            let mut q = [0.0; 3];
            q[a] = p[0];
            q[b] = p[1];
            q[c] = fixed;
            at(q)
        };
        for [p, q, s] in triangulate(&ring, &[]) {
            let (p, q, s) = (project(p), project(q), project(s));
            if flip {
                triangles.push([p, s, q]);
            } else {
                triangles.push([p, q, s]);
            }
        }
    };
    // Top (+Z) and bottom (-Z) faces, in the XY plane.
    face(0, 1, 2, z, false);
    face(0, 1, 2, 0.0, true);
    // Front (-Y) and back (+Y), in the XZ plane: CCW in (x, z) faces -Y.
    face(0, 2, 1, 0.0, false);
    face(0, 2, 1, y, true);
    // Left (-X) and right (+X), in the YZ plane: CCW in (y, z) faces +X.
    face(1, 2, 0, x, false);
    face(1, 2, 0, 0.0, true);

    // One quarter cylinder along axis `a` at the edge where axis `b` is on side `sb` and axis
    // `c` on side `sc` (-1: the low face, +1: the high face).
    let mut edge = |a: usize, b: usize, c: usize, sb: f64, sc: f64| {
        let wall_b = if sb < 0.0 { 0.0 } else { size[b] };
        let c2 = if sc < 0.0 { r } else { size[c] - r };
        let lo = r;
        let hi = size[a] - r;
        let point = |t: f64, phi: f64| {
            let mut p = [0.0; 3];
            p[a] = lo + t * (hi - lo);
            p[b] = wall_b - sb * r * (1.0 - phi.cos());
            p[c] = c2 + sc * r * phi.sin();
            at(p)
        };
        let n = segments.max(1);
        for i in 0..n {
            for j in 0..n {
                let (t0, t1) = (i as f64 / n as f64, (i + 1) as f64 / n as f64);
                let (p0, p1) = (j as f64 / n as f64 * PI / 2.0, (j + 1) as f64 / n as f64 * PI / 2.0);
                let (q00, q10, q11, q01) = (point(t0, p0), point(t1, p0), point(t1, p1), point(t0, p1));
                // The winding faces outward when the two sides' signs differ.
                if sb * sc < 0.0 {
                    triangles.push([q00, q10, q11]);
                    triangles.push([q00, q11, q01]);
                } else {
                    triangles.push([q00, q11, q10]);
                    triangles.push([q00, q01, q11]);
                }
            }
        }
    };
    // The four edges along each axis: the two sides of each of the other two axes.
    for a in 0..3 {
        let (b, c) = ((a + 1) % 3, (a + 2) % 3);
        for sb in [-1.0, 1.0] {
            for sc in [-1.0, 1.0] {
                edge(a, b, c, sb, sc);
            }
        }
    }

    // One sphere octant at the corner where each axis is on side `s` (-1: 0, +1: size).
    let mut corner = |sx: f64, sy: f64, sz: f64| {
        let centre =
            [if sx < 0.0 { r } else { size[0] - r }, if sy < 0.0 { r } else { size[1] - r }, if sz < 0.0 { r } else { size[2] - r }];
        let point = |u: f64, v: f64| {
            at([centre[0] + sx * r * u.sin() * v.cos(), centre[1] + sy * r * u.sin() * v.sin(), centre[2] + sz * r * u.cos()])
        };
        let n = segments.max(1);
        for i in 0..n {
            for j in 0..n {
                let (u0, u1) = (i as f64 / n as f64 * PI / 2.0, (i + 1) as f64 / n as f64 * PI / 2.0);
                let (v0, v1) = (j as f64 / n as f64 * PI / 2.0, (j + 1) as f64 / n as f64 * PI / 2.0);
                let (q00, q10, q11, q01) = (point(u0, v0), point(u1, v0), point(u1, v1), point(u0, v1));
                // The winding faces outward when an even number of sides are low.
                if sx * sy * sz > 0.0 {
                    triangles.push([q00, q10, q11]);
                    triangles.push([q00, q11, q01]);
                } else {
                    triangles.push([q00, q11, q10]);
                    triangles.push([q00, q01, q11]);
                }
            }
        }
    };
    for sx in [-1.0, 1.0] {
        for sy in [-1.0, 1.0] {
            for sz in [-1.0, 1.0] {
                corner(sx, sy, sz);
            }
        }
    }

    Mesh::new(triangles)
}

/// Each hole as a counter-clockwise polygon of `segments` corners on its circle.
pub(crate) fn rings(holes: &[HoleSpec], segments: usize) -> Vec<Vec<P2>> {
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

/// The outline through `corners` (counter-clockwise) with corner `i` rounded with radius
/// `radii[i]` (`0` or missing for a sharp corner), each arc faceted like a circle of `segments`
/// facets (at least one facet per corner).
pub(crate) fn rounded(corners: &[P2], radii: &[f64], segments: usize) -> Vec<P2> {
    let n = corners.len();
    let mut out = Vec::new();
    for i in 0..n {
        let r = radii.get(i).copied().unwrap_or(0.0);
        if r <= 0.0 {
            out.push(corners[i]);
            continue;
        }
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
pub(crate) fn triangulate(outline: &[P2], holes: &[Vec<P2>]) -> Vec<[P2; 3]> {
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
