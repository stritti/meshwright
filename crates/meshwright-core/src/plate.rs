//! Recognising a plate: a rectangular outline, optionally with one radius on all four vertical
//! corners, extruded along Z, with round vertical through holes.
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
//! 3. The outline must be the bounding rectangle, its corners all sharp or all rounded with the
//!    same radius; each hole must be round.
//! 4. The plate rebuilt from those numbers must have the mesh's volume within 1%, the check
//!    mesh2cad documents for its scripts; otherwise the mesh is kept.

use std::collections::{BTreeMap, HashMap};
use std::f64::consts::PI;

use crate::mesh::{Mesh, Vec3, cross, length, sub};
use crate::recognize::{HoleSpec, PlateSpec, Recognition};

/// Largest relative difference between the rebuilt plate's volume and the mesh's.
pub const MAX_VOLUME_ERROR: f64 = 0.01;

/// Fewest corners a polygon needs to be taken for a round hole. Six would turn hexagonal nut
/// traps into round holes.
pub const MIN_HOLE_SEGMENTS: usize = 8;

/// Recognised sizes are rounded to this step (1 µm): STL stores single-precision floats, so a
/// 35 mm hole position comes back as 35.0000019…, which nobody wants in the parameter panel.
const SNAP: f64 = 1e-3;

/// How far a corner may lie from where the recogniser expects it, in millimetres, for a part
/// whose largest size is `scale`. STL coordinates have about seven significant digits.
fn tolerance(scale: f64) -> f64 {
    (scale * 1e-5).max(1e-3)
}

fn snap(value: f64) -> f64 {
    let snapped = (value / SNAP).round() * SNAP;
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
    let radius = outline_radius(outline, size[0], size[1], tol)?;
    let mut holes = loops.iter().filter(|l| signed_area(l) < 0.0).map(|l| round_hole(l, tol)).collect::<Result<Vec<_>, _>>()?;
    holes.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));

    let plate = PlateSpec { x: snap(size[0]), y: snap(size[1]), z: snap(size[2]), holes, outline_fillet_r: radius.map(snap) };
    let error = (plate.volume() - mesh_volume) / mesh_volume;
    if error.abs() > MAX_VOLUME_ERROR {
        return Err(format!("the rebuilt plate would differ from the mesh by {:.1}% in volume", error.abs() * 100.0));
    }
    let offset = bounds.min.map(|c| if c.abs() <= tol { 0.0 } else { snap(c) });
    Ok(Recognition::Plate { plate, offset })
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
    (0..n).map(|i| {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        a[0] * b[1] - b[0] * a[1]
    })
    .sum::<f64>()
        / 2.0
}

/// Checks that `outline` is the `x` by `y` rectangle from the origin with all four corners
/// sharp (`None`) or rounded with one radius, and returns that radius.
fn outline_radius(outline: &[P2], x: f64, y: f64, tol: f64) -> Result<Option<f64>, String> {
    let not_rectangle = || "the outline is not a rectangle (other outlines are not recognised yet)".to_owned();
    // The four sides as (axis that is constant, its value, the other axis' length).
    let sides = [(1, 0.0, x), (0, x, y), (1, y, x), (0, 0.0, y)];
    let on = |side: usize, p: &P2| (p[sides[side].0] - sides[side].1).abs() <= tol;
    // Where each side's straight part starts and ends tells the corner radius, eight times.
    let mut estimates = Vec::with_capacity(8);
    for side in 0..4 {
        let along = 1 - sides[side].0;
        let positions = outline.iter().filter(|p| on(side, p)).map(|p| p[along]);
        let (low, high) = positions.fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| (lo.min(v), hi.max(v)));
        if low > high {
            return Err(not_rectangle());
        }
        estimates.extend([low, sides[side].2 - high]);
    }
    let (low, high) = estimates.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| (lo.min(*v), hi.max(*v)));
    if high - low > tol {
        return Err("the corners are not rounded alike (different radii, chamfers or notches are not recognised yet)".into());
    }
    let r = estimates.iter().sum::<f64>() / 8.0;
    let r = if r <= tol { 0.0 } else { r };
    if r > 0.0 && 2.0 * r >= x.min(y) - tol {
        return Err("the outline is rounded all the way (slot-shaped plates are not recognised yet)".into());
    }

    let centres = [[x - r, r], [x - r, y - r], [r, y - r], [r, r]];
    let quadrants = [[1.0, -1.0], [1.0, 1.0], [-1.0, 1.0], [-1.0, -1.0]];
    let on_arc = |corner: usize, p: &P2| {
        let d = [p[0] - centres[corner][0], p[1] - centres[corner][1]];
        (d[0].hypot(d[1]) - r).abs() <= tol && (0..2).all(|i| d[i] * quadrants[corner][i] >= -tol)
    };
    let mut arc_edges = [0_usize; 4];
    for i in 0..outline.len() {
        let (a, b) = (&outline[i], &outline[(i + 1) % outline.len()]);
        if (0..4).any(|side| on(side, a) && on(side, b)) {
            continue;
        }
        let corner = (0..4).find(|&c| r > 0.0 && on_arc(c, a) && on_arc(c, b)).ok_or_else(not_rectangle)?;
        arc_edges[corner] += 1;
    }
    if r > 0.0 && arc_edges.iter().any(|n| *n < 2) {
        return Err("a corner is cut straight rather than rounded (chamfers are not recognised yet)".into());
    }
    Ok((r > 0.0).then_some(r))
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

/// The area a plate's corner loses to a rounding of radius `r`.
pub(crate) fn corner_loss(r: f64) -> f64 {
    (4.0 - PI) * r * r
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recognize::recognize;
    use crate::synth;

    fn spec(holes: &[(f64, f64, f64)], r: Option<f64>) -> PlateSpec {
        PlateSpec { x: 80.0, y: 50.0, z: 5.0, holes: holes.iter().map(|&(x, y, d)| HoleSpec { x, y, d }).collect(), outline_fillet_r: r }
    }

    fn found(mesh: &Mesh) -> (PlateSpec, Vec3) {
        match recognize(mesh) {
            Recognition::Plate { plate, offset } => (plate, offset),
            Recognition::Unrecognized { reason } => panic!("not recognised: {reason}"),
        }
    }

    fn reason(mesh: &Mesh) -> String {
        match recognize(mesh) {
            Recognition::Unrecognized { reason } => reason,
            Recognition::Plate { plate, .. } => panic!("unexpectedly recognised {plate:?}"),
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
    fn hexagonal_cut_out_is_not_a_hole() {
        assert!(reason(&synth::plate(&spec(&[(40.0, 25.0, 10.0)], None), 6, [0.0; 3])).contains("6 corners"));
    }

    #[test]
    fn chamfered_corners_are_not_fillets() {
        // One segment per quarter circle is a 45° chamfer.
        assert!(reason(&synth::plate(&spec(&[], Some(5.0)), 4, [0.0; 3])).contains("chamfers"));
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
    fn synthetic_plate_volume_matches_the_polygon() {
        let s = spec(&[(40.0, 25.0, 10.0)], Some(5.0));
        let mesh = synth::plate(&s, 400, [0.0; 3]);
        assert!((mesh.volume() - s.volume()).abs() / s.volume() < 1e-3);
        assert!(mesh.volume() < s.volume() + 1e-9 || s.holes.is_empty());
    }
}
