//! 2D geometry on the plate's top face: distances, areas, boundary loops and circle fits.

use std::collections::{BTreeMap, HashMap};

use super::{P2, WELD_GRID_TOL};
/// The distance from `p` to the line through `a` and `b`.
pub(super) fn line_distance(p: P2, a: P2, b: P2) -> f64 {
    let (u, w) = ([b[0] - a[0], b[1] - a[1]], [p[0] - a[0], p[1] - a[1]]);
    (u[0] * w[1] - u[1] * w[0]).abs() / u[0].hypot(u[1])
}

/// The closed loop's sides, from each corner to the next.
pub(super) fn segments(l: &[P2]) -> impl Iterator<Item = (P2, P2)> + '_ {
    l.iter().enumerate().map(|(i, a)| (*a, l[(i + 1) % l.len()]))
}

/// How far the longest side of `l`, a loop meshed round a circle of radius `r`, comes inside
/// that circle.
pub(super) fn sagitta(l: &[P2], r: f64) -> f64 {
    let longest = segments(l).map(|(a, b)| distance(a, b)).fold(0.0, f64::max).min(2.0 * r);
    r - (r * r - longest * longest / 4.0).sqrt()
}

/// The distance from `p` to the segment from `a` to `b`.
pub(super) fn segment_distance(p: P2, a: P2, b: P2) -> f64 {
    let (u, w) = ([b[0] - a[0], b[1] - a[1]], [p[0] - a[0], p[1] - a[1]]);
    let len2 = u[0] * u[0] + u[1] * u[1];
    let t = if len2 == 0.0 { 0.0 } else { ((w[0] * u[0] + w[1] * u[1]) / len2).clamp(0.0, 1.0) };
    distance(p, [a[0] + t * u[0], a[1] + t * u[1]])
}

/// The closed boundary loops of a set of counter-clockwise triangles: outlines run
/// counter-clockwise, holes clockwise.
pub(super) fn boundary_loops(triangles: &[[P2; 3]], tol: f64) -> Result<Vec<Vec<P2>>, String> {
    // Corners that are the same point in the file can differ in the last bit after the offset
    // was subtracted, so they are welded on a grid well below the tolerance.
    let grid = tol / WELD_GRID_TOL;
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
pub(super) fn signed_area(ring: &[P2]) -> f64 {
    let n = ring.len();
    (0..n)
        .map(|i| {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            a[0] * b[1] - b[0] * a[1]
        })
        .sum::<f64>()
        / 2.0
}

pub(super) fn distance(a: P2, b: P2) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

/// The least-squares circle through `points` (Kåsa's algebraic fit), as centre and radius.
#[allow(clippy::many_single_char_names)] // the textbook symbols
pub(super) fn fit_circle(points: &[P2]) -> Option<(P2, f64)> {
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
