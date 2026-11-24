//! Straight blind cuts into a combined extrusion: pockets and steps.
//!
//! mesh2cad subtracts straight cut extrusions along the axes from the intersection of outlines,
//! a blind pocket that opens on one face among them (`csg.py`, `cut_<n>`). The facets that do not
//! lie on the outlines' intersection's boundary are grouped into pieces that touch. A piece is a
//! cut along an axis when its facets facing along the axis are its ends (those facing up the
//! axis in one plane, its start; those facing down it in another, its end) and the rest are
//! parallel to the axis (its walls). Its outline is the ends' silhouette; an end without facets
//! is where it opens, through the bounding box's face. A rectangular pocket is a cut along each
//! axis; as mesh2cad, the longest wins, and the first of X, Y and Z when they tie.

#![allow(clippy::many_single_char_names)] // points a, b, c, p, q, as in the geometry

use super::silhouette::{self, P2, area2, segment_distance};
use super::{PARALLEL, SIDE_COS};

use crate::mesh::{Vec3, snap_mm};
use crate::recognize::{Axis, Cut};

/// A facet: its corners and its unit normal.
type Facet = ([Vec3; 3], Vec3);

/// At most this many facets are grouped into cuts (each pair is compared).
const MAX_FACETS: usize = 4096;

/// The cuts the facets `inside` the outlines' intersection make, in the order of their axes
/// (X, Y, Z), starts and positions. `Err` when a piece is no straight blind cut, or cuts overlap.
pub(super) fn find_cuts(inside: &[Facet], size: Vec3, tol: f64) -> Result<Vec<Cut>, String> {
    if inside.is_empty() {
        return Ok(Vec::new());
    }
    if inside.len() > MAX_FACETS {
        return Err(format!("it is not a combination of extrusions: more than {MAX_FACETS} faces lie inside its outlines"));
    }
    let mut cuts = Vec::new();
    for piece in pieces(inside, tol) {
        let facets: Vec<&Facet> = piece.iter().map(|&i| &inside[i]).collect();
        // The longest of the axes the piece is a cut along, the first on a tie.
        let best = [Axis::X, Axis::Y, Axis::Z].into_iter().filter_map(|axis| cut_along(&facets, axis, size, tol)).fold(
            None,
            |best: Option<Cut>, cut| if best.as_ref().is_none_or(|b| cut.to - cut.from > b.to - b.from + tol) { Some(cut) } else { best },
        );
        cuts.push(best.ok_or_else(|| super::INSIDE.to_string())?);
    }
    cuts.sort_by(|a, b| {
        a.axis.index().cmp(&b.axis.index()).then(a.from.total_cmp(&b.from)).then(corner(&a.outline).total_cmp(&corner(&b.outline)))
    });
    for (i, a) in cuts.iter().enumerate() {
        for b in &cuts[i + 1..] {
            let (p, q) = (world_box(a), world_box(b));
            if (0..3).all(|k| p.1[k].min(q.1[k]) - p.0[k].max(q.0[k]) > tol) {
                return Err("it is not a combination of extrusions: two of its cuts overlap".into());
            }
        }
    }
    Ok(cuts)
}

/// The cut's lowest corner's coordinates summed, to order cuts on one axis and floor.
fn corner(outline: &[P2]) -> f64 {
    outline.iter().map(|p| p[0] + p[1]).fold(f64::INFINITY, f64::min)
}

/// The cut's bounding box in world coordinates (the part's box's corner at the origin).
fn world_box(cut: &Cut) -> (Vec3, Vec3) {
    let (mut lo, mut hi) = ([0.0; 3], [0.0; 3]);
    for (k, &world) in cut.axis.profile_axes().iter().enumerate() {
        let sign = cut.axis.profile_sign(k);
        let values = cut.outline.iter().map(|p| sign * p[k]);
        lo[world] = values.clone().fold(f64::INFINITY, f64::min);
        hi[world] = values.fold(f64::NEG_INFINITY, f64::max);
    }
    (lo[cut.axis.index()], hi[cut.axis.index()]) = (cut.from, cut.to);
    (lo, hi)
}

/// The facets grouped into pieces that touch: sharing a corner, or one's corner on the other's
/// side (an exporter splits a wall where the floor's corners are).
fn pieces(facets: &[Facet], tol: f64) -> Vec<Vec<usize>> {
    let n = facets.len();
    let mut parent: Vec<usize> = (0..n).collect();
    let near = |p: &Vec3, a: &Vec3, b: &Vec3| {
        let d = crate::mesh::sub(*b, *a);
        let len2 = crate::mesh::dot(d, d);
        let t = if len2 > 0.0 { (crate::mesh::dot(crate::mesh::sub(*p, *a), d) / len2).clamp(0.0, 1.0) } else { 0.0 };
        crate::mesh::length(crate::mesh::sub(*p, [a[0] + t * d[0], a[1] + t * d[1], a[2] + t * d[2]])) <= tol
    };
    let touch = |s: &[Vec3; 3], t: &[Vec3; 3]| s.iter().any(|p| (0..3).any(|i| near(p, &t[i], &t[(i + 1) % 3])));
    for i in 0..n {
        for j in i + 1..n {
            if touch(&facets[i].0, &facets[j].0) || touch(&facets[j].0, &facets[i].0) {
                let (a, b) = (root(&mut parent, i), root(&mut parent, j));
                parent[a] = b;
            }
        }
    }
    let mut groups: std::collections::BTreeMap<usize, Vec<usize>> = std::collections::BTreeMap::new();
    for i in 0..n {
        let r = root(&mut parent, i);
        groups.entry(r).or_default().push(i);
    }
    groups.into_values().collect()
}

/// The root of `i`'s set in the union-find forest `parent`, its path halved on the way.
fn root(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

/// The piece as a cut along `axis`, if it is one: its facets facing up the axis all in one plane
/// (its start) and those facing down it in another (its end), the others parallel to the axis,
/// the ends' silhouette one outline.
fn cut_along(facets: &[&Facet], axis: Axis, size: Vec3, tol: f64) -> Option<Cut> {
    let b = axis.index();
    let (ends, walls): (Vec<&Facet>, Vec<&Facet>) = facets.iter().copied().partition(|(_, n)| n[b].abs() >= 1.0 - PARALLEL);
    if ends.is_empty() || walls.iter().any(|(_, n)| n[b].abs() > PARALLEL) {
        return None;
    }
    // The plane the facets facing one way lie in, or the box's face where there are none.
    let plane = |up: bool, face: f64| -> Option<f64> {
        let mut at = ends.iter().filter(|(_, n)| (n[b] > 0.0) == up).flat_map(|(t, _)| t.iter().map(|p| p[b]));
        let Some(first) = at.next() else { return Some(face) };
        at.all(|v| (v - first).abs() <= tol).then_some(snap_mm(first))
    };
    let (from, to) = (plane(true, 0.0)?, plane(false, size[b])?);
    let projected: Vec<[P2; 3]> = ends
        .iter()
        .map(|(t, _)| {
            t.map(|p| {
                let q = axis.rotate(p);
                [q[0], q[1]]
            })
        })
        .collect();
    let rings = silhouette::loops(&projected, tol).ok()?;
    let [outline] = <[Vec<P2>; 1]>::try_from(rings).ok()?;
    if area2(&outline) <= 0.0 {
        return None;
    }
    let outline = outline.into_iter().map(|p| p.map(snap_mm)).collect();
    (to - from > tol).then_some(Cut { axis, outline, from, to })
}

/// Whether the facet's `points` (its normal `n`) lie on the cut's boundary: on an end facing into
/// it, or on a side of its outline facing into it, within its span.
pub(super) fn on_cut(cut: &Cut, points: &[Vec3], n: Vec3, tol: f64) -> bool {
    let m = cut.axis.rotate(n);
    let q: Vec<Vec3> = points.iter().map(|p| cut.axis.rotate(*p)).collect();
    let end = |at: f64| q.iter().all(|p| (p[2] - at).abs() <= tol && inside_or_on(&cut.outline, [p[0], p[1]], tol));
    if m[2] >= 1.0 - PARALLEL {
        return end(cut.from);
    }
    if m[2] <= -1.0 + PARALLEL {
        return end(cut.to);
    }
    if m[2].abs() > PARALLEL || q.iter().any(|p| p[2] < cut.from - tol || p[2] > cut.to + tol) {
        return false;
    }
    let k = cut.outline.len();
    (0..k).any(|i| {
        let (a, c) = (cut.outline[i], cut.outline[(i + 1) % k]);
        let d = [c[0] - a[0], c[1] - a[1]];
        let len = d[0].hypot(d[1]);
        // The material lies outside the cut: the facet faces into it, to the side's left.
        let into = [-d[1] / len, d[0] / len];
        into[0] * m[0] + into[1] * m[1] >= SIDE_COS && q.iter().all(|p| segment_distance([p[0], p[1]], a, c) <= tol)
    })
}

/// Whether `p` lies inside the ring or within `tol` of its sides.
fn inside_or_on(ring: &[P2], p: P2, tol: f64) -> bool {
    let n = ring.len();
    let mut inside = false;
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        if segment_distance(p, a, b) <= tol {
            return true;
        }
        if (a[1] > p[1]) != (b[1] > p[1]) && p[0] < a[0] + (p[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1]) {
            inside = !inside;
        }
    }
    inside
}
