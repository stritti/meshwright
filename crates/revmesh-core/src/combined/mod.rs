//! Combined extrusions: parts that are the intersection of outlines extruded along X, Y and Z.
//!
//! mesh2cad falls back to booleans of straight extrusions when no single prism fits (its `csg`
//! module). The first of them is a part whose every face is flat and parallel to at least one of
//! the axes, and that is the intersection of its silhouettes along X, Y and Z, each extruded right
//! across it: the three views of a drawing, as a plate seen from above cut by a ramp seen from
//! the side. revmesh recognises that part, with openings through its outlines and straight
//! blind cuts into it (see [`cuts`]); holes and unions of partial extrusions follow.
//!
//! The silhouette along an axis is the union of the facets facing along it, projected (see
//! [`silhouette`]); an axis whose silhouette is the box's side adds nothing and is left out. The
//! mesh lies inside the intersection by construction, and it is the intersection when every
//! facet lies on the intersection's boundary (on a side of an outline's extrusion, or on the
//! bounding box) and the volumes agree: no face lies inside, so nothing is missing. The facets
//! that do lie inside must be the floors and walls of cuts, which then count as boundary too.

#![allow(clippy::many_single_char_names)] // points a, b, c, p, q, as in the geometry

mod cuts;
mod silhouette;
#[cfg(test)]
mod tests;
pub(crate) mod volume;

use silhouette::{P2, area2, segment_distance};

use crate::mesh::{Mesh, Vec3, cross, dot, length, snap_mm, sub};
use crate::recognize::{Axis, CombinedSpec, Extrusion};

/// The fit tolerance, as for plates: STL coordinates have about seven significant digits.
const REL_TOLERANCE: f64 = 1e-5;
const MIN_TOLERANCE: f64 = 1e-3;

/// A facet is parallel to an axis when its unit normal's component along it is at most this.
const PARALLEL: f64 = 1e-4;

/// At most this many planes, counted roughly (a plane split by rounding may count twice).
const MAX_PLANES: usize = 256;

/// A facet lies on a side when its normal is within this cosine of the side's outward normal.
const SIDE_COS: f64 = 0.9999;

const TILTED: &str = "it is not a combination of extrusions: a face is tilted to every axis";
const INSIDE: &str = "it is not a combination of extrusions: a face lies inside the outlines' \
                      intersection (a pocket, a step or a hole)";

/// The combined extrusion `mesh` is, its bounding box's lowest corner at the origin, and where
/// that corner was. `Err` carries the reason in the user's words.
pub(crate) fn recognize_combined(mesh: &Mesh) -> Result<(CombinedSpec, Vec3), String> {
    let bounds = mesh.bounds().ok_or("the mesh is empty")?;
    let size = bounds.size().map(snap_mm);
    let tol = (size.iter().copied().fold(0.0, f64::max) * REL_TOLERANCE).max(MIN_TOLERANCE);
    // The facets with their unit normals, the box's corner moved to the origin.
    let facets: Vec<([Vec3; 3], Vec3)> = mesh
        .triangles
        .iter()
        .filter_map(|t| {
            let t = t.map(|p| sub(p, bounds.min));
            let n = cross(sub(t[1], t[0]), sub(t[2], t[0]));
            let len = length(n);
            (len > tol * tol).then(|| (t, n.map(|c| c / len)))
        })
        .collect();
    if facets.iter().any(|(_, n)| n.iter().all(|c| c.abs() > PARALLEL)) {
        return Err(TILTED.into());
    }
    // An outline of straight sides has few planes; a faceted curve has one per facet.
    let planes: std::collections::HashSet<[i64; 4]> = facets
        .iter()
        .map(|(t, n)| {
            let q = |v: f64, step: f64| (v / step).round() as i64;
            [q(n[0], 1e-3), q(n[1], 1e-3), q(n[2], 1e-3), q(dot(*n, t[0]), 16.0 * tol)]
        })
        .collect();
    if planes.len() > MAX_PLANES {
        return Err(format!(
            "it is not a combination of extrusions: it has more than {MAX_PLANES} flat faces (curved sides are not recognised yet)"
        ));
    }
    let mut extrusions = Vec::new();
    for axis in Axis::ALL {
        // The facets facing along the axis, in its profile coordinates.
        let front: Vec<[P2; 3]> = facets
            .iter()
            .filter(|(_, n)| axis.rotate(*n)[2] > PARALLEL)
            .map(|(t, _)| {
                t.map(|p| {
                    let q = axis.rotate(p);
                    [q[0], q[1]]
                })
            })
            .collect();
        // One boundary (counter-clockwise) and the openings through it (clockwise).
        let rings = silhouette::loops(&front, tol)?;
        let (mut outer, openings): (Vec<Vec<P2>>, Vec<Vec<P2>>) = rings.into_iter().partition(|r| area2(r) > 0.0);
        if outer.len() != 1 {
            return Err(format!("it is not a combination of extrusions: its silhouette along {} is not one outline", axis.letter()));
        }
        let outline = outer.remove(0);
        let snap = |ring: Vec<P2>| -> Vec<P2> { ring.into_iter().map(|p| p.map(snap_mm)).collect() };
        let outline = snap(outline);
        let cutouts: Vec<Vec<P2>> = openings
            .into_iter()
            .map(|mut r| {
                r.reverse();
                snap(r)
            })
            .collect();
        let [u, v] = axis.profile_axes();
        let box_side = size[u] * size[v];
        if cutouts.is_empty() && outline.len() == 4 && (area2(&outline) / 2.0 - box_side).abs() <= tol * (size[u] + size[v]) {
            continue;
        }
        extrusions.push(Extrusion { axis, outline, cutouts });
    }
    // What lies inside the outlines' intersection must be cuts into it.
    let base = CombinedSpec { size, extrusions: extrusions.clone(), cuts: Vec::new() };
    let inside: Vec<([Vec3; 3], Vec3)> = facets.iter().filter(|(t, n)| !on_boundary(&base, t, *n, tol)).copied().collect();
    let cuts = cuts::find_cuts(&inside, size, tol)?;
    if extrusions.len() < 2 && cuts.is_empty() {
        return Err("it is not a combination of extrusions: it is not bounded by outlines along two axes".into());
    }
    if extrusions.is_empty() {
        // A box with cuts into it: the box is the outline along Z.
        extrusions.push(Extrusion {
            axis: Axis::Z,
            outline: vec![[0.0, 0.0], [size[0], 0.0], [size[0], size[1]], [0.0, size[1]]],
            cutouts: Vec::new(),
        });
    }
    let spec = CombinedSpec { size, extrusions, cuts };
    if !facets.iter().all(|(t, n)| on_boundary(&spec, t, *n, tol)) {
        return Err(INSIDE.into());
    }
    let area: f64 = mesh.triangles.iter().map(|t| length(cross(sub(t[1], t[0]), sub(t[2], t[0]))) / 2.0).sum();
    let (found, measured) = (spec.volume(), mesh.volume());
    // Within a skin of the tolerance over the surface, and, for a thin part whose skin is much of
    // it, within the 1% every recognised part's volume keeps.
    if (found - measured).abs() > tol * area || (found - measured).abs() > crate::plate::MAX_VOLUME_ERROR * measured.abs() {
        return Err(format!(
            "it is not a combination of extrusions: the outlines' intersection and its cuts hold {found:.1} mm³, the mesh \
             {measured:.1} mm³"
        ));
    }
    Ok((spec, bounds.min.map(snap_mm)))
}

/// Whether the facet `t` (normal `n`) lies on the intersection's boundary: on the bounding box,
/// or, corners and centre, on a side of an outline whose outward normal it shares.
fn on_boundary(spec: &CombinedSpec, t: &[Vec3; 3], n: Vec3, tol: f64) -> bool {
    let centre = [0, 1, 2].map(|i| (t[0][i] + t[1][i] + t[2][i]) / 3.0);
    let mut points = t.to_vec();
    points.push(centre);
    let on_box = (0..3).any(|i| {
        let flush = |at: f64| points.iter().all(|p| (p[i] - at).abs() <= tol);
        (n[i] <= -1.0 + PARALLEL && flush(0.0)) || (n[i] >= 1.0 - PARALLEL && flush(spec.size[i]))
    });
    on_box
        || spec.cuts.iter().any(|c| cuts::on_cut(c, &points, n, tol))
        || spec.extrusions.iter().any(|e| {
            let m = e.axis.rotate(n);
            if m[2].abs() > PARALLEL {
                return false;
            }
            // Each side with the material on its left: the outline's and the openings' (wound the
            // other way).
            e.rings().iter().flat_map(|r| (0..r.len()).map(move |i| (r[i], r[(i + 1) % r.len()]))).any(|(a, b)| {
                let d = [b[0] - a[0], b[1] - a[1]];
                let len = d[0].hypot(d[1]);
                let outward = [d[1] / len, -d[0] / len];
                dot([outward[0], outward[1], 0.0], m) >= SIDE_COS
                    && points.iter().all(|p| {
                        let q = e.axis.rotate(*p);
                        segment_distance([q[0], q[1]], a, b) <= tol
                    })
            })
        })
}
