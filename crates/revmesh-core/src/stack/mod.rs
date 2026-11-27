//! Stacked extrusions: a union of outlines, each extruded over its own span along one axis.
//!
//! mesh2cad tries this when the intersection of full-length outlines misses the mesh: a base, a
//! smaller block on part of it, another on that (`stack.py`, `extrude_z<k>_len`). It finds the
//! shelves (faces square to the axis), takes the section between each two, and writes each band
//! as an extrusion of its own outline. revmesh does the same on a mesh whose axis is Z (the
//! recogniser rotates the others there): every face lies square to the axis or along it, or — on
//! the top band's top edges — slanted as one chamfer (roadmap 3.12: a bevel when small, a roof
//! when the chamfers meet); the levels are the heights of the faces square to the axis (the box's
//! top when the roof has no face there); each band's section is where its walls cross it. The
//! mesh is that stack when every facet lies on a band's wall (with the band's outward normal), on
//! a step between two bands (where one band's section reaches past the other's), or on the top
//! chamfer's plane, and the volumes agree. Holes are not recognised yet.

#![allow(clippy::many_single_char_names)] // points a, b, c, p, q, as in the geometry

#[cfg(test)]
mod tests;

use crate::combined::silhouette::{P2, area2, chain, drop_collinear, segment_distance};
use crate::mesh::{Mesh, Vec3, cross, dot, length, snap_mm, sub};
use crate::recognize::{Band, Chamfer, StackSpec};

/// The fit tolerance, as for plates.
const REL_TOLERANCE: f64 = 1e-5;
const MIN_TOLERANCE: f64 = 1e-3;

/// A facet is square to the axis, or along it, when its unit normal is within this of it.
const PARALLEL: f64 = 1e-4;

/// A facet lies on a wall when its normal is within this cosine of the wall's outward normal.
const SIDE_COS: f64 = 0.9999;

/// At most this many facets (each band's section is cut from all of them).
const MAX_FACETS: usize = 100_000;

const NOT_STACK: &str = "it is no stack of extrusions";

/// A facet: its corners and its unit normal.
type Facet = ([Vec3; 3], Vec3);

/// The stack `mesh` is along Z, its bounding box's lowest corner at the origin, and where that
/// corner was. `Err` carries the reason in the user's words.
pub(crate) fn recognize_stack(mesh: &Mesh) -> Result<(StackSpec, Vec3), String> {
    let bounds = mesh.bounds().ok_or("the mesh is empty")?;
    if mesh.triangles.len() > MAX_FACETS {
        return Err(format!("{NOT_STACK}: it has more than {MAX_FACETS} facets"));
    }
    let size = bounds.size().map(snap_mm);
    let tol = (size.iter().copied().fold(0.0, f64::max) * REL_TOLERANCE).max(MIN_TOLERANCE);
    let facets: Vec<Facet> = mesh
        .triangles
        .iter()
        .filter_map(|t| {
            let t = t.map(|p| sub(p, bounds.min));
            let n = cross(sub(t[1], t[0]), sub(t[2], t[0]));
            let len = length(n);
            (len > tol * tol).then(|| (t, n.map(|c| c / len)))
        })
        .collect();
    let level = |n: &Vec3| n[2].abs() >= 1.0 - PARALLEL;
    // Slanted faces are allowed only as the top band's chamfer (checked once the bands are built).
    let slanted: Vec<Facet> = facets.iter().filter(|(_, n)| !level(n) && n[2].abs() > PARALLEL).copied().collect();
    if slanted.iter().any(|(t, _)| t.iter().all(|p| p[2] < size[2] - tol)) {
        return Err(format!("{NOT_STACK}: a slanted face lies below its top"));
    }
    // The levels: the heights of the faces square to the axis.
    let mut levels: Vec<f64> = Vec::new();
    for (t, n) in &facets {
        if level(n) {
            let z = t[0][2];
            if t.iter().any(|p| (p[2] - z).abs() > tol) {
                return Err(format!("{NOT_STACK}: a face square to its axis is not flat"));
            }
            levels.push(z);
        }
    }
    levels.sort_by(f64::total_cmp);
    levels.dedup_by(|a, b| *a - *b <= tol);
    let mut levels: Vec<f64> = levels.into_iter().map(snap_mm).collect();
    if levels[0] > tol {
        return Err(format!("{NOT_STACK}: it does not step along its axis"));
    }
    // A roofed top has no face at the box's top; the slanted faces reach it there instead.
    if levels[levels.len() - 1] < size[2] - tol {
        if slanted.is_empty() {
            return Err(format!("{NOT_STACK}: it does not step along its axis"));
        }
        levels.push(snap_mm(size[2]));
    }
    if levels.len() < 3 {
        return Err(format!("{NOT_STACK}: it does not step along its axis"));
    }
    // Each band's section, bands of one section merged. A chamfered top band's walls end below
    // its top, so the section is cut where the walls of the window are.
    let mut bands: Vec<Band> = Vec::new();
    for w in levels.windows(2) {
        let (from, to) = (w[0], w[1]);
        let wall_top = facets
            .iter()
            .filter(|(_, n)| n[2].abs() <= PARALLEL)
            .filter(|(t, _)| t.iter().any(|p| p[2] > from + tol) && t.iter().any(|p| p[2] < to - tol))
            .map(|(t, _)| t.iter().map(|p| p[2]).fold(f64::NEG_INFINITY, f64::max))
            .fold(from, f64::max)
            .min(to);
        let rings = section(&facets, f64::midpoint(from, wall_top), tol)?;
        let (mut outer, openings): (Vec<Vec<P2>>, Vec<Vec<P2>>) = rings.into_iter().partition(|r| area2(r) > 0.0);
        if outer.len() != 1 {
            return Err(format!("{NOT_STACK}: a section of it is not one outline"));
        }
        let snap = |ring: Vec<P2>| -> Vec<P2> { ring.into_iter().map(|p| p.map(snap_mm)).collect() };
        let outline = snap(outer.remove(0));
        let cutouts: Vec<Vec<P2>> = openings
            .into_iter()
            .map(|mut r| {
                r.reverse();
                snap(r)
            })
            .collect();
        match bands.last_mut() {
            Some(last)
                if same_ring(&last.outline, &outline)
                    && last.cutouts.len() == cutouts.len()
                    && last.cutouts.iter().zip(&cutouts).all(|(a, b)| same_ring(a, b)) =>
            {
                last.to = to;
            }
            _ => bands.push(Band { outline, cutouts, from, to, chamfer: None }),
        }
    }
    if bands.len() < 2 {
        return Err(format!("{NOT_STACK}: it is one extrusion"));
    }
    // Roadmap 3.12: the top band's top edges may all carry one chamfer — a bevel, or a roof when
    // the chamfers meet. Every slanted face must lie in one edge's chamfer plane.
    if !slanted.is_empty() {
        recognize_chamfer(&slanted, bands.last_mut().expect("at least two bands"), size[2], tol)?;
    }
    let spec = StackSpec { size, bands };
    if !facets.iter().all(|(t, n)| on_stack(&spec, t, *n, tol)) {
        return Err(format!("{NOT_STACK}: a face lies on no band's wall or step"));
    }
    let area: f64 = mesh.triangles.iter().map(|t| length(cross(sub(t[1], t[0]), sub(t[2], t[0]))) / 2.0).sum();
    let (found, measured) = (spec.volume(), mesh.volume());
    if (found - measured).abs() > tol * area || (found - measured).abs() > crate::plate::MAX_VOLUME_ERROR * measured.abs() {
        return Err(format!("{NOT_STACK}: its bands hold {found:.1} mm³, the mesh {measured:.1} mm³"));
    }
    Ok((spec, bounds.min.map(snap_mm)))
}

/// The chamfer plane of the top edge from `a` to `b` at height `to`, for the chamfer `c`: it runs
/// from the edge at `to - c.h` up-inward to the top face, `c.w` in from the edge at `to`. Returns
/// a point on the plane and its outward unit normal (the wall's outward normal `m` tilted by the
/// chamfer: `h·m + w·k`).
pub(crate) fn chamfer_plane(a: P2, b: P2, to: f64, c: Chamfer) -> (Vec3, Vec3) {
    let d = [b[0] - a[0], b[1] - a[1]];
    let len = d[0].hypot(d[1]);
    let m = [d[1] / len, -d[0] / len]; // the wall's outward normal (the outline is counter-clockwise)
    let q0 = [a[0], a[1], to - c.h];
    let n = [c.h * m[0], c.h * m[1], c.w];
    let nl = length(n);
    (q0, [n[0] / nl, n[1] / nl, n[2] / nl])
}

/// Recognise the slanted `facets` as one chamfer of all the top band's top edges (roadmap 3.12: a
/// bevel, or a roof when the chamfers of opposite edges meet), storing it on `top`. Every slanted
/// facet must lie in one top edge's chamfer plane, with one shared `w` and `h`, and every top edge
/// must carry it; the outline must be convex for the chamfer's volume.
fn recognize_chamfer(slanted: &[Facet], top: &mut Band, to: f64, tol: f64) -> Result<(), String> {
    let outline = top.outline.clone();
    let n = outline.len();
    let mut chamfer: Option<Chamfer> = None;
    let mut covered = vec![false; n];
    for (t, nrm) in slanted {
        let zmin = t.iter().map(|p| p[2]).fold(f64::INFINITY, f64::min);
        let zmax = t.iter().map(|p| p[2]).fold(f64::NEG_INFINITY, f64::max);
        if (zmax - to).abs() > tol || zmin >= to - tol {
            return Err(format!("{NOT_STACK}: a slanted face is no chamfer of the top"));
        }
        let bottoms: Vec<P2> = t.iter().filter(|p| p[2] < to - tol).map(|p| [p[0], p[1]]).collect();
        let tops: Vec<P2> = t.iter().filter(|p| p[2] >= to - tol).map(|p| [p[0], p[1]]).collect();
        if bottoms.is_empty() || tops.is_empty() {
            return Err(format!("{NOT_STACK}: a slanted face is no chamfer of the top"));
        }
        // The top edge the facet chamfers: its bottom corners lie on that edge's wall, and its
        // top corners lie on a line parallel to it (the inset line on the top face) — at a shared
        // corner both neighbours' walls hold the bottom corner, so the parallel top edge decides.
        let mut best: Option<(usize, f64)> = None;
        for i in 0..n {
            let (a, b) = (outline[i], outline[(i + 1) % n]);
            let d = [b[0] - a[0], b[1] - a[1]];
            let len = d[0].hypot(d[1]);
            let line_dist = |p: P2| ((p[0] - a[0]) * d[1] - (p[1] - a[1]) * d[0]).abs() / len;
            let on_wall = bottoms.iter().map(|p| segment_distance(*p, a, b)).fold(0.0, f64::max);
            let dists: Vec<f64> = tops.iter().map(|p| line_dist(*p)).collect();
            let (lo, hi) = dists.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &dd| (lo.min(dd), hi.max(dd)));
            let parallel = hi - lo <= tol; // the tops lie on one line parallel to the side
            if on_wall <= tol && parallel && best.is_none_or(|(_, bd)| on_wall < bd) {
                best = Some((i, on_wall));
            }
        }
        let Some((i, _)) = best else {
            return Err(format!("{NOT_STACK}: a slanted face lies on no top edge"));
        };
        let (a, b) = (outline[i], outline[(i + 1) % n]);
        let d = [b[0] - a[0], b[1] - a[1]];
        let len = d[0].hypot(d[1]);
        let line_dist = |p: P2| ((p[0] - a[0]) * d[1] - (p[1] - a[1]) * d[0]).abs() / len;
        let w = tops.iter().map(|p| line_dist(*p)).fold(0.0, f64::max);
        let h = to - zmin;
        if w <= tol || h <= tol {
            return Err(format!("{NOT_STACK}: the top's chamfer is too small to recognise"));
        }
        match chamfer {
            None => chamfer = Some(Chamfer { w: snap_mm(w), h: snap_mm(h) }),
            Some(c) if (c.w - w).abs() <= tol && (c.h - h).abs() <= tol => {}
            _ => return Err(format!("{NOT_STACK}: the top's chamfers differ")),
        }
        // The facet lies in that edge's chamfer plane and faces out of it.
        let c = chamfer.expect("set or matched above");
        let (q0, nch) = chamfer_plane(a, b, to, c);
        if t.iter().any(|p| dot(sub(*p, q0), nch).abs() > tol) || dot(*nrm, nch) < SIDE_COS {
            return Err(format!("{NOT_STACK}: a slanted face is no chamfer of the top"));
        }
        covered[i] = true;
    }
    if covered.iter().any(|c| !*c) {
        return Err(format!("{NOT_STACK}: the top's chamfer leaves an edge sharp"));
    }
    // The chamfer's volume comes from Steiner's offset formula, which needs a convex outline.
    let convex = (0..n).all(|i| {
        let (a, c, b) = (outline[(i + n - 1) % n], outline[i], outline[(i + 1) % n]);
        (c[0] - a[0]) * (b[1] - c[1]) - (c[1] - a[1]) * (b[0] - c[0]) > 0.0
    });
    if !convex {
        return Err(format!("{NOT_STACK}: the top's outline is not convex"));
    }
    top.chamfer = chamfer;
    Ok(())
}

/// Whether two rings (of corners rounded to micrometres) have the same corners, whichever corner
/// they start at.
fn same_ring(a: &[P2], b: &[P2]) -> bool {
    let same = |p: &P2, q: &P2| (p[0] - q[0]).abs() < 1e-9 && (p[1] - q[1]).abs() < 1e-9;
    a.len() == b.len() && (0..b.len()).any(|k| a.iter().enumerate().all(|(i, p)| same(p, &b[(i + k) % b.len()])))
}

/// The section at height `z`: where the walls cross it, each piece turned so that the material
/// lies on its left, chained into rings (outlines counter-clockwise, openings clockwise).
fn section(facets: &[Facet], z: f64, tol: f64) -> Result<Vec<Vec<P2>>, String> {
    let mut pieces = Vec::new();
    for (t, n) in facets {
        if n[2].abs() > PARALLEL {
            continue;
        }
        // Where the facet's sides cross the height.
        let mut cut: Vec<P2> = Vec::new();
        for i in 0..3 {
            let (p, q) = (t[i], t[(i + 1) % 3]);
            if (p[2] < z) != (q[2] < z) {
                let s = (z - p[2]) / (q[2] - p[2]);
                cut.push([p[0] + s * (q[0] - p[0]), p[1] + s * (q[1] - p[1])]);
            }
        }
        if let [a, b] = cut[..] {
            // The outward normal lies to the right of the direction the material keeps on its left.
            let d = [b[0] - a[0], b[1] - a[1]];
            let (a, b) = if d[0] * n[1] - d[1] * n[0] < 0.0 { (a, b) } else { (b, a) };
            if (a[0] - b[0]).hypot(a[1] - b[1]) > tol / 16.0 {
                pieces.push((a, b));
            }
        }
    }
    let rings = chain(pieces, tol).map_err(|_| format!("{NOT_STACK}: a section of it does not close"))?;
    Ok(rings.into_iter().map(|r| drop_collinear(r, tol)).filter(|r| r.len() >= 3).collect())
}

/// Whether the facet `t` (its normal `n`) lies on the stack's boundary: on a band's wall with the
/// wall's outward normal, on a step: a face facing up where the band below reaches past the
/// one above, facing down where the band above reaches past the one below, or — slanted — on the
/// top band's chamfer plane of one of its top edges (roadmap 3.12).
fn on_stack(spec: &StackSpec, t: &[Vec3; 3], n: Vec3, tol: f64) -> bool {
    let centre = [0, 1, 2].map(|i| (t[0][i] + t[1][i] + t[2][i]) / 3.0);
    let flat = |p: &Vec3| [p[0], p[1]];
    if n[2].abs() >= 1.0 - PARALLEL {
        let z = centre[2];
        let below = spec.bands.iter().find(|b| (b.to - z).abs() <= tol);
        let above = spec.bands.iter().find(|b| (b.from - z).abs() <= tol);
        let (inside, outside) = if n[2] > 0.0 { (below, above) } else { (above, below) };
        let Some(inside) = inside else { return false };
        return t.iter().all(|p| inside.holds(flat(p), tol)) && outside.is_none_or(|o| !o.strictly_holds(flat(&centre), tol));
    }
    if n[2].abs() > PARALLEL {
        // A slanted facet: on the top band's chamfer plane of one of its top edges.
        let Some(top) = spec.bands.last() else { return false };
        let Some(c) = top.chamfer else { return false };
        let m = top.outline.len();
        return (0..m).any(|i| {
            let (a, b) = (top.outline[i], top.outline[(i + 1) % m]);
            let (q0, nch) = chamfer_plane(a, b, top.to, c);
            dot(n, nch) >= SIDE_COS && t.iter().all(|p| dot(sub(*p, q0), nch).abs() <= tol)
        });
    }
    // A wall: each of its corners and its centre on a side of a band reaching it, facing out.
    let mut points = t.to_vec();
    points.push(centre);
    points.iter().all(|p| {
        spec.bands.iter().filter(|b| p[2] >= b.from - tol && p[2] <= b.to + tol).any(|b| {
            b.rings().iter().any(|r| {
                (0..r.len()).any(|i| {
                    let (a, c) = (r[i], r[(i + 1) % r.len()]);
                    let d = [c[0] - a[0], c[1] - a[1]];
                    let len = d[0].hypot(d[1]);
                    (d[1] * n[0] - d[0] * n[1]) / len >= SIDE_COS && segment_distance(flat(p), a, c) <= tol
                })
            })
        })
    })
}

impl Band {
    /// Whether `p` lies in the band's section or within `tol` of its sides.
    pub(crate) fn holds(&self, p: P2, tol: f64) -> bool {
        self.rings().iter().any(|r| (0..r.len()).any(|i| segment_distance(p, r[i], r[(i + 1) % r.len()]) <= tol))
            || self.strictly_holds(p, tol)
    }

    /// Whether `p` lies in the band's section, farther than `tol` from its sides.
    pub(crate) fn strictly_holds(&self, p: P2, tol: f64) -> bool {
        let rings = self.rings();
        if rings.iter().any(|r| (0..r.len()).any(|i| segment_distance(p, r[i], r[(i + 1) % r.len()]) <= tol)) {
            return false;
        }
        let mut inside = false;
        for r in &rings {
            for i in 0..r.len() {
                let (a, b) = (r[i], r[(i + 1) % r.len()]);
                if (a[1] > p[1]) != (b[1] > p[1]) && p[0] < a[0] + (p[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1]) {
                    inside = !inside;
                }
            }
        }
        inside
    }
}
