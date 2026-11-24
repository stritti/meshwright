//! Stacked extrusions: a union of outlines, each extruded over its own span along one axis.
//!
//! mesh2cad tries this when the intersection of full-length outlines misses the mesh: a base, a
//! smaller block on part of it, another on that (`stack.py`, `extrude_z<k>_len`). It finds the
//! shelves (faces square to the axis), takes the section between each two, and writes each band
//! as an extrusion of its own outline. revmesh does the same on a mesh whose axis is Z (the
//! recogniser rotates the others there): every face lies square to the axis or along it; the
//! levels are the heights of the faces square to it; each band's section is where its walls cross
//! its middle height. The mesh is that stack when every facet lies on a band's wall (with the
//! band's outward normal) or on a step between two bands (where one band's section reaches past
//! the other's), and the volumes agree. Roofs, bevels and holes are not recognised yet.

#![allow(clippy::many_single_char_names)] // points a, b, c, p, q, as in the geometry

#[cfg(test)]
mod tests;

use crate::combined::silhouette::{P2, area2, chain, drop_collinear, segment_distance};
use crate::mesh::{Mesh, Vec3, cross, length, snap_mm, sub};
use crate::recognize::{Band, StackSpec};

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
    if facets.iter().any(|(_, n)| !level(n) && n[2].abs() > PARALLEL) {
        return Err(format!("{NOT_STACK}: a face is neither square to its axis nor along it"));
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
    let levels: Vec<f64> = levels.into_iter().map(snap_mm).collect();
    if levels.len() < 3 || levels[0] > tol || levels[levels.len() - 1] < size[2] - tol {
        return Err(format!("{NOT_STACK}: it does not step along its axis"));
    }
    // Each band's section, bands of one section merged.
    let mut bands: Vec<Band> = Vec::new();
    for w in levels.windows(2) {
        let (from, to) = (w[0], w[1]);
        let rings = section(&facets, f64::midpoint(from, to), tol)?;
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
            _ => bands.push(Band { outline, cutouts, from, to }),
        }
    }
    if bands.len() < 2 {
        return Err(format!("{NOT_STACK}: it is one extrusion"));
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
/// wall's outward normal, or on a step: a face facing up where the band below reaches past the
/// one above, facing down where the band above reaches past the one below.
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
