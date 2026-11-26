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

/// A fixed-seed PRNG (SplitMix64): every sampling algorithm (RANSAC) in the plugin runs on
/// `wasm32-unknown-unknown` with no system randomness, so results must be reproducible bit for
/// bit — the seed is fixed.
pub(crate) struct Rng(u64);

impl Rng {
    /// The fixed seed's generator.
    pub(crate) fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// The next pseudo-random `u64`.
    pub(crate) fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A pseudo-random index below `n`.
    pub(crate) fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

/// The circle through three points, `None` when they are (nearly) collinear.
fn circle_from_3(a: P2, b: P2, c: P2) -> Option<(P2, f64)> {
    let d = 2.0 * (a[0] * (b[1] - c[1]) + b[0] * (c[1] - a[1]) + c[0] * (a[1] - b[1]));
    if d.abs() <= f64::EPSILON {
        return None;
    }
    let a2 = a[0] * a[0] + a[1] * a[1];
    let b2 = b[0] * b[0] + b[1] * b[1];
    let c2 = c[0] * c[0] + c[1] * c[1];
    let ux = (a2 * (b[1] - c[1]) + b2 * (c[1] - a[1]) + c2 * (a[1] - b[1])) / d;
    let uy = (a2 * (c[0] - b[0]) + b2 * (a[0] - c[0]) + c2 * (b[0] - a[0])) / d;
    let r = distance([ux, uy], a);
    (r.is_finite() && r > 0.0).then_some(([ux, uy], r))
}

/// The torus about a known axis, fitted to profile points `(radial, axial)`: about the axis a
/// torus is a circle in the profile plane — roadmap 6.3, a round at a hole's rim or a boss's
/// foot. The band's corners lie on it; a corner an exporter added within a facet lies inside it.
/// So the fit runs on the distinct sample locations (the samples repeat around the axis) as a
/// fixed-seed RANSAC circle (`ransac_circle`, as a rim loop's in 6.2) refined by the algebraic
/// fit on its inliers, and every profile point must then lie on the result within one coarsest
/// facet's sagitta. Returns centre (major radius, height of the centre circle) and radius (the
/// round's); `None` when fewer than three distinct locations lie on one circle or span too
/// narrow an arc.
pub(crate) fn fit_torus(profile: &[P2], tol: f64) -> Option<(P2, f64)> {
    let mut distinct: Vec<P2> = Vec::with_capacity(profile.len());
    for p in profile {
        if distinct.iter().all(|q| distance(*q, *p) > tol) {
            distinct.push(*p);
        }
    }
    if distinct.len() < 3 {
        return None;
    }
    let (_, _, inliers) = ransac_circle(&distinct, tol)?;
    if inliers.len() < 3 {
        return None;
    }
    let on: Vec<P2> = inliers.iter().map(|&i| distinct[i]).collect();
    let (c, f) = fit_circle(&on)?;
    // Angular support: locations clustered in a narrow arc leave the unconstrained fit
    // ill-conditioned (binary-STL f32 rounding then swings the radius), so they must span at
    // least MIN_TORUS_ARC about the centre; otherwise the caller falls back to the per-corner
    // formula, which uses the round's known tangency.
    let mut angles: Vec<f64> = on.iter().map(|p| (p[1] - c[1]).atan2(p[0] - c[0])).collect();
    angles.sort_by(f64::total_cmp);
    let mut max_gap = angles.first().map_or(0.0, |a| a + 2.0 * std::f64::consts::PI - angles[angles.len() - 1]);
    for w in angles.windows(2) {
        max_gap = max_gap.max(w[1] - w[0]);
    }
    if 2.0 * std::f64::consts::PI - max_gap < MIN_TORUS_ARC {
        return None;
    }
    let bound = f * (1.0 - (super::MAX_HOLE_STEP / 2.0).cos()) + tol;
    (f > tol && profile.iter().all(|p| ((p[0] - c[0]).hypot(p[1] - c[1]) - f).abs() <= bound)).then_some((c, f))
}

/// Least arc about the fitted centre the on-circle profile locations must span for the torus fit
/// to be well conditioned (45°): a quarter round's bands spread over up to 90°, a cluster in a
/// narrow arc does not.
const MIN_TORUS_ARC: f64 = std::f64::consts::PI / 4.0;

/// A circle fitted by fixed-seed RANSAC (Schnabel, Wahl, Klein 2007): 3-point minimal samples,
/// inliers within `tol`, the best sample by inlier count and then by residual sum, with the
/// iteration count adapted to the inlier ratio seen so far (`N = ln(1 - p) / ln(1 - w^3)`,
/// confidence `p = 0.99`); refined by the algebraic fit on its inliers. Returns the circle and
/// the inliers' indices, or `None` when no sample has three inliers.
pub(crate) fn ransac_circle(points: &[P2], tol: f64) -> Option<(P2, f64, Vec<usize>)> {
    let n = points.len();
    if n < 3 {
        return None;
    }
    let mut rng = Rng::new(0x5EED_C1EC_1E00);
    let mut best: Option<(usize, f64, P2, f64, Vec<usize>)> = None;
    let mut w = 0.5_f64; // the inlier ratio estimate, adapted as samples come in
    let mut iter = 0_usize;
    loop {
        let n_max = ((1.0 - 0.99_f64).ln() / (1.0 - w * w * w).ln()).ceil().clamp(3.0, 512.0);
        if iter as f64 >= n_max {
            break;
        }
        iter += 1;
        let (i, j, k) = (rng.below(n), rng.below(n), rng.below(n));
        if i == j || j == k || i == k {
            continue;
        }
        let Some((centre, r)) = circle_from_3(points[i], points[j], points[k]) else { continue };
        let inliers: Vec<usize> = (0..n).filter(|&i| (distance(points[i], centre) - r).abs() <= tol).collect();
        let residual: f64 = inliers.iter().map(|&i| (distance(points[i], centre) - r).powi(2)).sum();
        if inliers.len() >= 3
            && best.as_ref().is_none_or(|(count, res, _, _, _)| inliers.len() > *count || (inliers.len() == *count && residual < *res))
        {
            best = Some((inliers.len(), residual, centre, r, inliers));
            w = best.as_ref().expect("just set").0 as f64 / n as f64;
        }
    }
    let (_, _, centre, r, inliers) = best?;
    let pts: Vec<P2> = inliers.iter().map(|&i| points[i]).collect();
    let (centre, r) = fit_circle(&pts).unwrap_or((centre, r));
    Some((centre, r, inliers))
}

/// The least-squares circle through `points` (Kåsa's algebraic fit), as centre and radius.
#[allow(clippy::many_single_char_names)] // the textbook symbols
pub(crate) fn fit_circle(points: &[P2]) -> Option<(P2, f64)> {
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
