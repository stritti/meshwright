//! Recognising a plate: an outline of straight sides, optionally with one radius on every
//! vertical corner, extruded along Z, with round vertical through holes.
//!
//! Written independently from mesh2cad's published description (a Z extrusion with one radius on
//! every vertical corner of the outline, through holes, the parameter names `base_x`,
//! `hole_<n>_d` and `outline_fillet_r`); no mesh2cad code was used for it.
//!
//! The mesh is taken as exported by a CAD tool: corners of the true part's facets lie on its
//! surfaces, so a round hole is a polygon whose corners lie on a circle. The steps are:
//!
//! 1. Every corner lies on the bottom or the top face, every side wall is vertical and the top
//!    and bottom faces have the same area as the volume demands: the mesh is a straight prism.
//! 2. The boundary of the top face is split into closed loops. The one loop running
//!    counter-clockwise is the outline, the clockwise ones are holes.
//! 3. The outline must be made of straight sides, its corners all sharp or all rounded with the
//!    same radius (a rounded corner is a run of facet corners on one circle that meets the sides
//!    on either side of it tangentially); each hole must be round. A rectangular outline becomes
//!    a [`PlateSpec`], any other a [`ProfileSpec`].
//! 4. The plate rebuilt from those numbers must have the mesh's volume within 1%, the check
//!    mesh2cad documents for its scripts; otherwise the mesh is kept.

#![allow(clippy::many_single_char_names)] // points a, b, c, sizes x, y, z and radii r, as in the geometry

use std::collections::{BTreeMap, HashMap};
use std::f64::consts::PI;

use crate::mesh::{Mesh, Vec3, cross, length, sub};
use crate::recognize::{HoleSpec, PlateSpec, ProfileSpec, Recognition};

/// Largest relative difference between the rebuilt plate's volume and the mesh's.
pub const MAX_VOLUME_ERROR: f64 = 0.01;

/// Fewest corners a polygon needs to be taken for a round hole. Six would turn hexagonal nut
/// traps into round holes.
pub const MIN_HOLE_SEGMENTS: usize = 8;

/// Recognised sizes are rounded to micrometres (this many steps per millimetre): STL stores
/// single-precision floats, so a 35 mm hole position comes back as 35.0000019…, which nobody
/// wants in the parameter panel.
const SNAP_PER_MM: f64 = 1000.0;

/// How far a corner may lie from where the recogniser expects it, in millimetres, for a part
/// whose largest size is `scale`. STL coordinates have about seven significant digits.
fn tolerance(scale: f64) -> f64 {
    (scale * 1e-5).max(1e-3)
}

fn snap(value: f64) -> f64 {
    let snapped = (value * SNAP_PER_MM).round() / SNAP_PER_MM;
    if snapped == 0.0 { 0.0 } else { snapped }
}

type P2 = [f64; 2];

/// Looks for a plate in `mesh`. `Err` carries the reason in the user's words.
pub(crate) fn recognize_plate(mesh: &Mesh) -> Result<Recognition, String> {
    let bounds = mesh.bounds().ok_or("the mesh has no triangles")?;
    let size = bounds.size();
    let scale = size[0].max(size[1]).max(size[2]);
    let tol = tolerance(scale);
    if size.iter().any(|s| *s <= tol) {
        return Err("the mesh is flat".into());
    }
    let mesh_volume = mesh.volume();
    if mesh_volume <= 0.0 {
        return Err("the mesh is not closed or its faces point inwards".into());
    }

    let (top, bottom_area) = split_faces(mesh, bounds.min, bounds.max[2], tol)?;
    let top_area: f64 = top.iter().map(|[a, b, c]| signed_area(&[*a, *b, *c])).sum();
    let close = |a: f64, b: f64| (a - b).abs() <= 1e-5 * a.abs().max(b.abs());
    if !close(top_area, bottom_area) || !close(top_area * size[2], mesh_volume) {
        return Err("the mesh is not closed, or its top and bottom faces differ".into());
    }

    let loops = boundary_loops(&top, tol)?;
    let mut outlines = loops.iter().filter(|l| signed_area(l) > 0.0);
    let outline = outlines.next().ok_or("the top face has no outline")?;
    if outlines.next().is_some() {
        return Err("the top face has more than one outline (several separate parts are not recognised yet)".into());
    }
    let (corners, radius) = outline_shape(outline, tol)?;
    let mut holes = loops.iter().filter(|l| signed_area(l) < 0.0).map(|l| round_hole(l, tol)).collect::<Result<Vec<_>, _>>()?;
    holes.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));

    let offset = bounds.min.map(|c| if c.abs() <= tol { 0.0 } else { snap(c) });
    let (z, outline_fillet_r) = (snap(size[2]), radius.map(snap));
    let box_corners = [[0.0, 0.0], [size[0], 0.0], [size[0], size[1]], [0.0, size[1]]];
    let rectangle =
        corners.len() == 4 && box_corners.iter().all(|q| corners.iter().any(|c| (c[0] - q[0]).abs() <= tol && (c[1] - q[1]).abs() <= tol));
    let (found, volume) = if rectangle {
        let plate = PlateSpec { x: snap(size[0]), y: snap(size[1]), z, holes, outline_fillet_r };
        let volume = plate.volume();
        (Recognition::Plate { plate, offset }, volume)
    } else {
        let mut corners: Vec<P2> = corners.iter().map(|c| c.map(snap)).collect();
        // Start at the lowest corner, the leftmost of those, so that line l1 is predictable.
        let first =
            (0..corners.len()).min_by(|&a, &b| corners[a][1].total_cmp(&corners[b][1]).then(corners[a][0].total_cmp(&corners[b][0])));
        corners.rotate_left(first.unwrap_or(0));
        let profile = ProfileSpec { corners, z, holes, outline_fillet_r };
        let volume = profile.volume();
        (Recognition::Profile { profile, offset }, volume)
    };
    let error = (volume - mesh_volume) / mesh_volume;
    if error.abs() > MAX_VOLUME_ERROR {
        return Err(format!("the rebuilt plate would differ from the mesh by {:.1}% in volume", error.abs() * 100.0));
    }
    Ok(found)
}

/// Sorts the triangles into the top face (returned in 2D, relative to `min`), the bottom face
/// (only its area is needed) and vertical walls, and refuses anything else.
fn split_faces(mesh: &Mesh, min: Vec3, top_z: f64, tol: f64) -> Result<(Vec<[P2; 3]>, f64), String> {
    let mut top = Vec::new();
    let mut bottom_area = 0.0;
    for triangle in &mesh.triangles {
        let mut on_top = 0;
        for p in triangle {
            if (p[2] - top_z).abs() <= tol {
                on_top += 1;
            } else if (p[2] - min[2]).abs() > tol {
                return Err("it is not a flat plate: some corners lie between its top and bottom faces \
                            (steps, pockets, bosses and blind holes are not recognised yet)"
                    .into());
            }
        }
        let [a, b, c] = *triangle;
        let normal = cross(sub(b, a), sub(c, a));
        let len = length(normal);
        if len == 0.0 {
            continue;
        }
        let nz = normal[2] / len;
        match on_top {
            3 if nz > 0.0 => top.push(triangle.map(|p| [p[0] - min[0], p[1] - min[1]])),
            0 if nz < 0.0 => bottom_area += len / 2.0,
            0 | 3 => return Err("some faces point inwards".into()),
            _ if nz.abs() > 1e-3 => {
                return Err("some side walls are not vertical (drafts and chamfers are not recognised yet)".into());
            }
            _ => {}
        }
    }
    Ok((top, bottom_area))
}

/// The closed boundary loops of a set of counter-clockwise triangles: outlines run
/// counter-clockwise, holes clockwise.
fn boundary_loops(triangles: &[[P2; 3]], tol: f64) -> Result<Vec<Vec<P2>>, String> {
    // Corners that are the same point in the file can differ in the last bit after the offset
    // was subtracted, so they are welded on a grid well below the tolerance.
    let grid = tol / 16.0;
    let mut ids: HashMap<(i64, i64), usize> = HashMap::new();
    let mut points: Vec<P2> = Vec::new();
    let mut id = |p: P2| {
        let key = ((p[0] / grid).round() as i64, (p[1] / grid).round() as i64);
        *ids.entry(key).or_insert_with(|| {
            points.push(p);
            points.len() - 1
        })
    };
    let mut edges: BTreeMap<(usize, usize), usize> = BTreeMap::new();
    for t in triangles {
        let [a, b, c] = t.map(&mut id);
        if a == b || b == c || c == a {
            continue;
        }
        for edge in [(a, b), (b, c), (c, a)] {
            *edges.entry(edge).or_default() += 1;
        }
    }
    let mut next: BTreeMap<usize, usize> = BTreeMap::new();
    for (&(a, b), &count) in &edges {
        if count > 1 {
            return Err("the top face overlaps itself".into());
        }
        if !edges.contains_key(&(b, a)) && next.insert(a, b).is_some() {
            return Err("the top face touches itself at a corner".into());
        }
    }
    let mut loops = Vec::new();
    while let Some((&start, _)) = next.first_key_value() {
        let mut current = start;
        let mut ring = Vec::new();
        while let Some(to) = next.remove(&current) {
            ring.push(points[current]);
            current = to;
        }
        if current != start || ring.len() < 3 {
            return Err("the top face has an open edge (the mesh has a gap)".into());
        }
        loops.push(ring);
    }
    Ok(loops)
}

/// The shoelace area of a closed polygon, positive when counter-clockwise.
fn signed_area(ring: &[P2]) -> f64 {
    let n = ring.len();
    (0..n)
        .map(|i| {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            a[0] * b[1] - b[0] * a[1]
        })
        .sum::<f64>()
        / 2.0
}

/// Reads `outline` (counter-clockwise) as a polygon of straight sides whose corners are all
/// sharp or all rounded with one radius. Returns the corners as if they were sharp, and the
/// radius.
fn outline_shape(outline: &[P2], tol: f64) -> Result<(Vec<P2>, Option<f64>), String> {
    let v = merge_collinear(outline);
    let n = v.len();
    if n < 3 {
        return Err("the outline is degenerate".into());
    }
    let at = |i: usize| v[i % n];

    // A rounded corner starts where a side touches the circle through the next three facet
    // corners, runs on that circle and ends where a side leaves it tangentially again.
    let mut arcs: Vec<(usize, usize, f64)> = Vec::new(); // first and last facet corner, radius
    let mut i = 0;
    while i < n {
        let found = circumcircle(at(i), at(i + 1), at(i + 2)).and_then(|(centre, rho)| {
            if !tangent(at(i + n - 1), at(i), at(i + 1), centre) {
                return None;
            }
            let mut j = i + 2;
            while j < i + n - 1 && !tangent(at(j + 1), at(j), at(j - 1), centre) {
                if (distance(at(j + 1), centre) - rho).abs() > tol {
                    return None;
                }
                j += 1;
            }
            tangent(at(j + 1), at(j), at(j - 1), centre).then_some((centre, j))
        });
        match found {
            Some((_, j)) => {
                let points: Vec<P2> = (i..=j).map(at).collect();
                let (_, r) = fit_circle(&points).ok_or("a rounded corner could not be measured")?;
                arcs.push((i, j, r));
                i = j;
            }
            None => i += 1,
        }
    }
    let mut on_arc = vec![false; n];
    for &(first, last, _) in &arcs {
        for k in first..=last {
            on_arc[k % n] = true;
        }
    }

    // Short facets on one circle that are not a rounded corner make a curved side.
    for k in 0..n {
        if (0..4).all(|d| !on_arc[(k + d) % n]) {
            let (one, two) = (circumcircle(at(k), at(k + 1), at(k + 2)), circumcircle(at(k + 1), at(k + 2), at(k + 3)));
            if let (Some((c1, r1)), Some((c2, r2))) = (one, two)
                && distance(c1, c2) <= tol
                && (r1 - r2).abs() <= tol
                && distance(at(k + 1), at(k + 2)) < 0.5 * r1
            {
                return Err("the outline has curved sides (only straight sides and rounded corners are recognised so far)".into());
            }
        }
    }

    // A rounded corner becomes the point where its two sides meet; sharp corners stay.
    let mut corners = Vec::new();
    for k in 0..n {
        if let Some(&(first, last, _)) = arcs.iter().find(|a| a.0 == k) {
            let corner = intersect(at(first + n - 1), at(first), at(last), at(last + 1))
                .ok_or("a corner is rounded all the way round (slot-shaped ends are not recognised yet)")?;
            corners.push(corner);
        } else if !on_arc[k] {
            corners.push(v[k]);
        }
    }
    if corners.len() < 3 {
        return Err("the outline is degenerate".into());
    }
    if arcs.is_empty() {
        return Ok((corners, None));
    }
    if arcs.len() != corners.len() {
        return Err("only some corners of the outline are rounded (mixed corners are not recognised yet)".into());
    }
    let (low, high) = arcs.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), a| (lo.min(a.2), hi.max(a.2)));
    if high - low > tol {
        return Err("the corners are rounded with different radii (mixed radii are not recognised yet)".into());
    }
    Ok((corners, Some(arcs.iter().map(|a| a.2).sum::<f64>() / arcs.len() as f64)))
}

/// Drops facet corners that lie on a straight line between their neighbours.
fn merge_collinear(ring: &[P2]) -> Vec<P2> {
    let n = ring.len();
    (0..n)
        .filter(|&i| {
            let (a, b, c) = (ring[(i + n - 1) % n], ring[i], ring[(i + 1) % n]);
            let (u, w) = ([b[0] - a[0], b[1] - a[1]], [c[0] - b[0], c[1] - b[1]]);
            let sin = (u[0] * w[1] - u[1] * w[0]) / (u[0].hypot(u[1]) * w[0].hypot(w[1]));
            !(sin.abs() <= 1e-4 && u[0] * w[0] + u[1] * w[1] > 0.0)
        })
        .map(|i| ring[i])
        .collect()
}

fn distance(a: P2, b: P2) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

/// Whether the side from `from` to `at` touches the circle around `centre` at `at`, where
/// `next` is the neighbouring facet corner on the circle. The allowance shrinks with the facets,
/// so that a finely faceted arc's own corners are not taken for its ends.
fn tangent(from: P2, at: P2, next: P2, centre: P2) -> bool {
    let side = [at[0] - from[0], at[1] - from[1]];
    let radius = [at[0] - centre[0], at[1] - centre[1]];
    let (side_len, radius_len) = (side[0].hypot(side[1]), radius[0].hypot(radius[1]));
    let cos = (side[0] * radius[0] + side[1] * radius[1]) / (side_len * radius_len);
    cos.abs() <= 2e-3_f64.min(0.25 * distance(at, next) / (2.0 * radius_len))
}

/// The circle through three points, as centre and radius; `None` when they are in a line.
fn circumcircle(a: P2, b: P2, c: P2) -> Option<(P2, f64)> {
    let d = 2.0 * (a[0] * (b[1] - c[1]) + b[0] * (c[1] - a[1]) + c[0] * (a[1] - b[1]));
    let scale = distance(a, b).max(distance(b, c)).max(distance(c, a));
    if d.abs() <= 1e-12 * scale * scale {
        return None;
    }
    let (a2, b2, c2) = (a[0] * a[0] + a[1] * a[1], b[0] * b[0] + b[1] * b[1], c[0] * c[0] + c[1] * c[1]);
    let x = (a2 * (b[1] - c[1]) + b2 * (c[1] - a[1]) + c2 * (a[1] - b[1])) / d;
    let y = (a2 * (c[0] - b[0]) + b2 * (a[0] - c[0]) + c2 * (b[0] - a[0])) / d;
    Some(([x, y], distance([x, y], a)))
}

/// Where the line through `a` and `b` meets the line through `c` and `d`.
fn intersect(a: P2, b: P2, c: P2, d: P2) -> Option<P2> {
    let (u, w) = ([b[0] - a[0], b[1] - a[1]], [d[0] - c[0], d[1] - c[1]]);
    let den = u[0] * w[1] - u[1] * w[0];
    if den.abs() <= 1e-9 * u[0].hypot(u[1]) * w[0].hypot(w[1]) {
        return None;
    }
    let t = ((c[0] - a[0]) * w[1] - (c[1] - a[1]) * w[0]) / den;
    Some([a[0] + t * u[0], a[1] + t * u[1]])
}

/// Fits a circle to a hole's corners and checks that they all lie on it.
fn round_hole(ring: &[P2], tol: f64) -> Result<HoleSpec, String> {
    if ring.len() < MIN_HOLE_SEGMENTS {
        return Err(format!(
            "a cut-out has only {} corners, so it is taken for a polygon, not a round hole (polygonal cut-outs are not recognised yet)",
            ring.len()
        ));
    }
    let ([cx, cy], r) = fit_circle(ring).ok_or("a cut-out is not round")?;
    if ring.iter().any(|p| ((p[0] - cx).hypot(p[1] - cy) - r).abs() > tol) {
        return Err("a cut-out is not round (slots and other cut-outs are not recognised yet)".into());
    }
    Ok(HoleSpec { x: snap(cx), y: snap(cy), d: snap(2.0 * r) })
}

/// The least-squares circle through `points` (Kåsa's algebraic fit), as centre and radius.
#[allow(clippy::many_single_char_names)] // the textbook symbols
fn fit_circle(points: &[P2]) -> Option<(P2, f64)> {
    let n = points.len() as f64;
    let mean = [points.iter().map(|p| p[0]).sum::<f64>() / n, points.iter().map(|p| p[1]).sum::<f64>() / n];
    // In coordinates around the mean, solve for x² + y² + D x + E y + F = 0.
    let (mut sxx, mut sxy, mut syy, mut sxz, mut syz, mut sz) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    for p in points {
        let (u, v) = (p[0] - mean[0], p[1] - mean[1]);
        let z = u * u + v * v;
        sxx += u * u;
        sxy += u * v;
        syy += v * v;
        sxz += u * z;
        syz += v * z;
        sz += z;
    }
    // Around the mean Σu = Σv = 0, so F = -Σz / n and D, E follow from a 2 by 2 system.
    let det = sxx * syy - sxy * sxy;
    if det.abs() <= f64::EPSILON * (sxx * syy).max(f64::MIN_POSITIVE) {
        return None;
    }
    let d = (-sxz * syy + syz * sxy) / det;
    let e = (-syz * sxx + sxz * sxy) / det;
    let (u, v) = (-d / 2.0, -e / 2.0);
    let r2 = u * u + v * v + sz / n;
    (r2 > 0.0).then(|| ([mean[0] + u, mean[1] + v], r2.sqrt()))
}

/// The area the four corners of a rectangle lose to roundings of radius `r`.
pub(crate) fn corner_loss(r: f64) -> f64 {
    (4.0 - PI) * r * r
}

/// The area the corner `c` of a counter-clockwise outline (coming from `a`, going on to `b`)
/// loses to a rounding of radius `r`; negative for an inside corner, which gains it.
pub(crate) fn fillet_loss(a: P2, c: P2, b: P2, r: f64) -> f64 {
    let (u, w) = ([c[0] - a[0], c[1] - a[1]], [b[0] - c[0], b[1] - c[1]]);
    let turn = (u[0] * w[1] - u[1] * w[0]).atan2(u[0] * w[0] + u[1] * w[1]);
    turn.signum() * r * r * ((turn.abs() / 2.0).tan() - turn.abs() / 2.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recognize::recognize;
    use crate::synth;

    fn spec(holes: &[(f64, f64, f64)], r: Option<f64>) -> PlateSpec {
        PlateSpec { x: 80.0, y: 50.0, z: 5.0, holes: holes.iter().map(|&(x, y, d)| HoleSpec { x, y, d }).collect(), outline_fillet_r: r }
    }

    /// An L: 60 along X and 15 deep, with a 15 wide leg up to 40, one hole in each arm.
    fn ell(r: Option<f64>) -> ProfileSpec {
        ProfileSpec {
            corners: vec![[0.0, 0.0], [60.0, 0.0], [60.0, 15.0], [15.0, 15.0], [15.0, 40.0], [0.0, 40.0]],
            z: 5.0,
            holes: vec![HoleSpec { x: 7.5, y: 30.0, d: 5.0 }, HoleSpec { x: 45.0, y: 7.5, d: 6.0 }],
            outline_fillet_r: r,
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
            Recognition::Profile { profile, offset } => (profile, offset),
            other => panic!("not recognised as a profile: {other:?}"),
        }
    }

    fn reason(mesh: &Mesh) -> String {
        match recognize(mesh) {
            Recognition::Unrecognized { reason } => reason,
            other => panic!("unexpectedly recognised {other:?}"),
        }
    }

    #[test]
    fn fits_a_circle_exactly() {
        let ring: Vec<P2> = (0..12).map(|i| f64::from(i) * PI / 6.0).map(|a| [3.0 + 2.0 * a.cos(), -1.0 + 2.0 * a.sin()]).collect();
        let ([x, y], r) = fit_circle(&ring).expect("a circle");
        assert!((x - 3.0).abs() < 1e-12 && (y + 1.0).abs() < 1e-12 && (r - 2.0).abs() < 1e-12);
    }

    #[test]
    fn recognises_holes_and_rounded_corners() {
        let expected = spec(&[(10.0, 10.0, 5.0), (40.0, 25.0, 12.0), (70.0, 38.5, 6.6)], Some(4.0));
        let (plate, offset) = found(&synth::plate(&expected, 32, [0.0; 3]));
        assert_eq!(plate, expected);
        assert_eq!(offset, [0.0; 3]);
    }

    #[test]
    fn recognises_sharp_corners_and_coarse_holes() {
        let expected = spec(&[(20.0, 25.0, 8.0), (60.0, 25.0, 8.0)], None);
        assert_eq!(found(&synth::plate(&expected, 12, [0.0; 3])).0, expected);
    }

    #[test]
    fn survives_single_precision_files() {
        let expected = spec(&[(12.5, 30.0, 3.4), (55.0, 20.0, 9.0)], Some(2.5));
        let mesh = synth::plate(&expected, 48, [0.0; 3]);
        let reread = crate::stl::parse(&crate::stl::write_binary(&mesh)).expect("round trip");
        assert_eq!(found(&reread).0, expected);
    }

    #[test]
    fn moves_an_offset_plate_to_the_origin() {
        let expected = spec(&[(40.0, 25.0, 10.0)], Some(5.0));
        let (plate, offset) = found(&synth::plate(&expected, 32, [-40.0, -25.0, 3.0]));
        assert_eq!(plate, expected);
        assert_eq!(offset, [-40.0, -25.0, 3.0]);
    }

    #[test]
    fn recognises_finely_faceted_corners() {
        let expected = spec(&[(40.0, 25.0, 10.0)], Some(0.5));
        assert_eq!(found(&synth::plate(&expected, 720, [0.0; 3])).0, expected);
    }

    #[test]
    fn recognises_an_l_shaped_outline_with_inside_and_outside_corners() {
        for r in [None, Some(2.0)] {
            let expected = ell(r);
            let (profile, offset) = found_profile(&synth::profile(&expected, 32, [0.0; 3]));
            assert_eq!(profile, expected, "radius {r:?}");
            assert_eq!(offset, [0.0; 3]);
        }
    }

    #[test]
    fn recognises_an_offset_l_from_a_single_precision_file() {
        let expected = ell(Some(3.0));
        let mesh = synth::profile(&expected, 48, [100.0, -20.0, 0.0]);
        let reread = crate::stl::parse(&crate::stl::write_binary(&mesh)).expect("round trip");
        let (profile, offset) = found_profile(&reread);
        assert_eq!(profile, expected);
        assert_eq!(offset, [100.0, -20.0, 0.0]);
    }

    #[test]
    fn recognises_slanted_sides() {
        let expected = ProfileSpec {
            corners: vec![[0.0, 0.0], [50.0, 0.0], [60.0, 20.0], [25.0, 35.0], [0.0, 20.0]],
            z: 4.0,
            holes: vec![HoleSpec { x: 25.0, y: 15.0, d: 8.0 }],
            outline_fillet_r: Some(3.0),
        };
        let mesh = synth::profile(&expected, 64, [0.0; 3]);
        let reread = crate::stl::parse(&crate::stl::write_binary(&mesh)).expect("round trip");
        assert_eq!(found_profile(&reread).0, expected);
    }

    #[test]
    fn chamfered_corners_make_an_eight_sided_outline() {
        // One facet per quarter circle is a 45° chamfer: an exact eight-sided outline.
        let (profile, _) = found_profile(&synth::plate(&spec(&[], Some(5.0)), 4, [0.0; 3]));
        let corners = vec![[5.0, 0.0], [75.0, 0.0], [80.0, 5.0], [80.0, 45.0], [75.0, 50.0], [5.0, 50.0], [0.0, 45.0], [0.0, 5.0]];
        assert_eq!(profile, ProfileSpec { corners, z: 5.0, holes: vec![], outline_fillet_r: None });
    }

    #[test]
    fn round_outline_is_not_a_polygon() {
        let circle: Vec<P2> = (0..64).map(|i| f64::from(i) * PI / 32.0).map(|a| [30.0 + 30.0 * a.cos(), 30.0 + 30.0 * a.sin()]).collect();
        assert!(reason(&synth::prism(&circle, &[], 5.0, [0.0; 3])).contains("curved sides"));
    }

    #[test]
    fn hexagonal_cut_out_is_not_a_hole() {
        assert!(reason(&synth::plate(&spec(&[(40.0, 25.0, 10.0)], None), 6, [0.0; 3])).contains("6 corners"));
    }

    #[test]
    fn standing_plate_is_not_recognised() {
        let mesh = synth::plate(&spec(&[(40.0, 25.0, 10.0)], None), 32, [0.0; 3]);
        let turned = Mesh::new(mesh.triangles.iter().map(|t| t.map(|p| [p[0], p[2], -p[1]])).collect());
        assert!(reason(&turned).contains("not a flat plate"));
    }

    #[test]
    fn mesh_with_a_gap_is_not_recognised() {
        let mut mesh = synth::plate(&spec(&[(40.0, 25.0, 10.0)], Some(3.0)), 32, [0.0; 3]);
        mesh.triangles.remove(0);
        reason(&mesh);
    }

    #[test]
    fn synthetic_volumes_match_the_specs() {
        let s = spec(&[(40.0, 25.0, 10.0)], Some(5.0));
        let mesh = synth::plate(&s, 128, [0.0; 3]);
        assert!((mesh.volume() - s.volume()).abs() / s.volume() < 1e-3);
        let l = ell(Some(2.0));
        let mesh = synth::profile(&l, 128, [0.0; 3]);
        assert!((mesh.volume() - l.volume()).abs() / l.volume() < 1e-3);
    }

    #[test]
    #[allow(clippy::float_cmp)] // the generator puts every top corner at exactly the same height
    fn synthetic_top_faces_are_not_folded() {
        let plate = synth::plate(&spec(&[(10.0, 10.0, 5.0), (40.0, 25.0, 12.0), (70.0, 38.5, 6.6)], Some(4.0)), 32, [0.0; 3]);
        let profile = synth::profile(&ell(Some(2.0)), 32, [0.0; 3]);
        for mesh in [plate, profile] {
            for [a, b, c] in &mesh.triangles {
                let n = cross(sub(*b, *a), sub(*c, *a));
                if a[2] == b[2] && b[2] == c[2] {
                    assert!(n[2].abs() > 1e-9, "degenerate cap triangle");
                    assert_eq!(n[2] > 0.0, a[2] > 0.0, "cap triangle facing the wrong way");
                }
            }
        }
    }
}
