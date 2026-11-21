//! Synthetic meshes of known parts, tessellated the way CAD exporters do it (facet corners on the
//! true surfaces). Tests use them to check recognisers against exact answers, and the
//! `plate_stl` example writes the STL fixtures SolidCraft's plugin tests import.

use std::f64::consts::PI;

use crate::mesh::{Mesh, Triangle, Vec3};
use crate::recognize::PlateSpec;

type P2 = [f64; 2];

/// The plate `spec` as a closed mesh, moved by `offset`. Each hole is a polygon of `segments`
/// corners and each rounded corner a quarter of one (at least one facet).
///
/// # Panics
///
/// If two holes overlap along X: the top face is split into bands between holes to triangulate
/// it, so holes must be separable by vertical lines that stay clear of the rounded corners.
#[must_use]
pub fn plate(spec: &PlateSpec, segments: usize, offset: Vec3) -> Mesh {
    let r = spec.outline_fillet_r.unwrap_or(0.0);
    let mut holes = spec.holes.clone();
    holes.sort_by(|a, b| a.x.total_cmp(&b.x));
    let cuts: Vec<f64> = holes.windows(2).map(|w| (w[0].x + w[0].d / 2.0 + w[1].x - w[1].d / 2.0) / 2.0).collect();
    for (i, cut) in cuts.iter().enumerate() {
        assert!(holes[i].x + holes[i].d / 2.0 < *cut && *cut < holes[i + 1].x - holes[i + 1].d / 2.0, "holes overlap along X");
        assert!(r < *cut && *cut < spec.x - r, "a band boundary runs through a rounded corner");
    }
    let outline = outline(spec.x, spec.y, r, (segments / 4).max(1), &cuts);
    let rings: Vec<Vec<P2>> = holes
        .iter()
        .map(|h| {
            (0..segments)
                .map(|i| 2.0 * PI * i as f64 / segments as f64)
                .map(|a| [h.x + h.d / 2.0 * a.cos(), h.y + h.d / 2.0 * a.sin()])
                .collect()
        })
        .collect();

    let mut cap: Vec<[P2; 3]> = Vec::new();
    let bounds: Vec<f64> = std::iter::once(f64::NEG_INFINITY).chain(cuts.iter().copied()).chain(std::iter::once(f64::INFINITY)).collect();
    for (band, w) in bounds.windows(2).enumerate() {
        let eps = 1e-9 * spec.x;
        let cell: Vec<P2> = outline.iter().copied().filter(|p| p[0] >= w[0] - eps && p[0] <= w[1] + eps).collect();
        match rings.get(band) {
            Some(ring) => zip(&cell, ring, [holes[band].x, holes[band].y], &mut cap),
            None => fan(&cell, &mut cap),
        }
    }

    let at = |p: P2, z: f64| [p[0] + offset[0], p[1] + offset[1], z + offset[2]];
    let mut triangles: Vec<Triangle> = Vec::new();
    for [a, b, c] in &cap {
        triangles.push([at(*a, spec.z), at(*b, spec.z), at(*c, spec.z)]);
        triangles.push([at(*a, 0.0), at(*c, 0.0), at(*b, 0.0)]);
    }
    // Walls: the outline runs counter-clockwise and each hole, seen as the top face's boundary,
    // clockwise; both give outward walls with the same winding.
    let reversed: Vec<Vec<P2>> = rings.iter().map(|ring| ring.iter().rev().copied().collect()).collect();
    for ring in std::iter::once(&outline).chain(&reversed) {
        for i in 0..ring.len() {
            let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
            triangles.push([at(a, 0.0), at(b, 0.0), at(b, spec.z)]);
            triangles.push([at(a, 0.0), at(b, spec.z), at(a, spec.z)]);
        }
    }
    Mesh::new(triangles)
}

/// The counter-clockwise outline of an `x` by `y` rectangle with corners of radius `r` (each
/// `quarter` facets), with extra corners on the bottom and top sides at each of `cuts`.
#[allow(clippy::many_single_char_names)] // x, y, r and cos, sin as in the geometry
fn outline(x: f64, y: f64, r: f64, quarter: usize, cuts: &[f64]) -> Vec<P2> {
    let mut points = Vec::new();
    let corner = |points: &mut Vec<P2>, centre: P2, start: usize| {
        if r == 0.0 {
            points.push(centre);
            return;
        }
        for j in 0..=quarter {
            let [c, s] = unit(start * quarter + j, 4 * quarter);
            points.push([centre[0] + r * c, centre[1] + r * s]);
        }
    };
    corner(&mut points, [x - r, r], 3);
    corner(&mut points, [x - r, y - r], 0);
    points.extend(cuts.iter().rev().map(|c| [*c, y]));
    corner(&mut points, [r, y - r], 1);
    corner(&mut points, [r, r], 2);
    points.extend(cuts.iter().map(|c| [*c, 0.0]));
    points
}

/// The unit vector at `step` of `steps` around the circle, exact at the quarter turns.
fn unit(step: usize, steps: usize) -> P2 {
    let step = step % steps;
    if (step * 4).is_multiple_of(steps) {
        return [[1.0, 0.0], [0.0, 1.0], [-1.0, 0.0], [0.0, -1.0]][step * 4 / steps];
    }
    let a = 2.0 * PI * step as f64 / steps as f64;
    [a.cos(), a.sin()]
}

/// Triangulates a convex polygon from its centroid.
fn fan(cell: &[P2], out: &mut Vec<[P2; 3]>) {
    let n = cell.len() as f64;
    let centre = [cell.iter().map(|p| p[0]).sum::<f64>() / n, cell.iter().map(|p| p[1]).sum::<f64>() / n];
    for i in 0..cell.len() {
        out.push([centre, cell[i], cell[(i + 1) % cell.len()]]);
    }
}

/// Triangulates the ring between a convex polygon and a counter-clockwise hole polygon inside
/// it, walking both by their angle around the hole's `centre`.
fn zip(cell: &[P2], hole: &[P2], centre: P2, out: &mut Vec<[P2; 3]>) {
    let angle = |p: &P2| (p[1] - centre[1]).atan2(p[0] - centre[0]).rem_euclid(2.0 * PI);
    let sorted = |ring: &[P2]| {
        let first = (0..ring.len()).min_by(|&a, &b| angle(&ring[a]).total_cmp(&angle(&ring[b]))).unwrap_or(0);
        let mut ring: Vec<(P2, f64)> = (0..ring.len()).map(|i| ring[(first + i) % ring.len()]).map(|p| (p, angle(&p))).collect();
        // Unwrap the angles so that they increase, and close the ring with its first corner.
        for i in 1..ring.len() {
            while ring[i].1 < ring[i - 1].1 {
                ring[i].1 += 2.0 * PI;
            }
        }
        ring.push((ring[0].0, ring[0].1 + 2.0 * PI));
        ring
    };
    let (outer, inner) = (sorted(cell), sorted(hole));
    let ccw = |t: &[P2; 3]| (t[1][0] - t[0][0]) * (t[2][1] - t[0][1]) - (t[1][1] - t[0][1]) * (t[2][0] - t[0][0]) > 0.0;
    let (mut i, mut j) = (0, 0);
    while i + 1 < outer.len() || j + 1 < inner.len() {
        let along_outline = (i + 1 < outer.len()).then(|| [outer[i].0, outer[i + 1].0, inner[j].0]);
        let along_hole = (j + 1 < inner.len()).then(|| [outer[i].0, inner[j + 1].0, inner[j].0]);
        // Follow the angles; where that would fold a triangle over (a long side facing the
        // hole), take the other step.
        let by_angle = j + 1 == inner.len() || (i + 1 < outer.len() && outer[i + 1].1 <= inner[j + 1].1);
        let outline_step = match (along_outline, along_hole) {
            (Some(a), Some(h)) => {
                if by_angle {
                    ccw(&a) || !ccw(&h)
                } else {
                    ccw(&a) && !ccw(&h)
                }
            }
            (a, _) => a.is_some(),
        };
        if outline_step {
            out.extend(along_outline);
            i += 1;
        } else {
            out.extend(along_hole);
            j += 1;
        }
    }
}
