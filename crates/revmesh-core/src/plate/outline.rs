//! Reading the top face's outline as straight sides with rounded corners, and the area the
//! roundings take.

use super::geometry::{distance, fit_circle, line_distance};
use super::{
    CARRY_LEN_SHARE, CARRY_TURN_SHARE, COLLINEAR_SIN, CURVE_LEN_SPREAD, GENTLE_LEN_SPREAD, MAX_CURVE_TURN, MAX_FACET_TURN, P2,
    PARALLEL_DENOM,
};
use crate::recognize::ArcSide;
use std::f64::consts::PI;

/// Reads `outline` (counter-clockwise) as a polygon of sides whose corners are all sharp or all
/// rounded with one radius, where any side may instead be a circular arc (a curved side).
/// Returns the corners as if they were sharp, the corner radius, and for each side the arc it
/// is (`None` for a straight side).
#[allow(clippy::too_many_lines)] // one scan, read best in one piece
#[allow(clippy::type_complexity)] // corners, corner radius, arc per side
pub(super) fn outline_shape(outline: &[P2], tol: f64) -> Result<(Vec<P2>, Option<f64>, Vec<Option<ArcSide>>), String> {
    let mut v = merge_collinear(outline);
    let n = v.len();
    if n < 3 {
        return Err("the outline is degenerate".into());
    }
    // Start right after the longest edge: it is a straight side, so no rounded corner runs across
    // the start of the list.
    let longest = (0..n).max_by(|&a, &b| distance(v[a], v[(a + 1) % n]).total_cmp(&distance(v[b], v[(b + 1) % n]))).unwrap_or(0);
    v.rotate_left((longest + 1) % n);
    let len = |i: usize| distance(v[i % n], v[(i + 1) % n]);
    let turns: Vec<f64> = (0..n)
        .map(|i| {
            let (a, b, c) = (v[(i + n - 1) % n], v[i], v[(i + 1) % n]);
            let (u, w) = ([b[0] - a[0], b[1] - a[1]], [c[0] - b[0], c[1] - b[1]]);
            (u[0] * w[1] - u[1] * w[0]).atan2(u[0] * w[0] + u[1] * w[1])
        })
        .collect();
    let turn = |i: usize| turns[i % n];
    // Facet `j` carries on a run of facets that started with facet `first`: about as long, and
    // turning the same small way as the run's first turn, at corner `first + 1`.
    let carries_on = |first: usize, j: usize| {
        (len(j) - len(first)).abs() <= CARRY_LEN_SHARE * len(first) && turn(j).abs() <= MAX_FACET_TURN && turn(j) * turn(first + 1) > 0.0
    };

    // A rounded corner is a run of facets of about equal length that each turn a little the
    // same way, whose corners lie on one circle (fitted to all of them, which averages out the
    // noise of single-precision files) and which meets the sides on either side tangentially.
    // It can only start where a side ends, never inside a run, so a long curve is scanned once.
    let mut arcs: Vec<(usize, usize, f64)> = Vec::new(); // first and last facet corner, radius
    let mut i = 0;
    while i + 2 < n {
        if i > 0 && carries_on(i - 1, i) && turn(i).abs() >= CARRY_TURN_SHARE * turn(i + 1).abs() {
            i += 1;
            continue;
        }
        let before = v[(i + n - 1) % n];
        let mut fit = CircleSums::new(v[i]);
        fit.add(v[i]);
        fit.add(v[i + 1]);
        let mut found = None;
        let mut end = i + 1;
        // Some exporters (SolidCraft among them) start or end an arc with a facet of another
        // length; a facet that turns the same small way and whose far corner keeps the run's
        // first and last corners on one circle carries the run on too.
        let on_fit = |fit: &CircleSums, end: usize| {
            let same_way = turn(end).abs() <= MAX_FACET_TURN && (end == i + 1 || turn(end) * turn(i + 1) > 0.0) && turn(i + 1) != 0.0;
            same_way && {
                let mut with = fit.clone();
                with.add(v[(end + 1) % n]);
                with.circle()
                    .is_some_and(|(centre, r)| [v[i], v[end], v[(end + 1) % n]].iter().all(|p| (distance(*p, centre) - r).abs() <= tol))
            }
        };
        while end + 1 < n && (carries_on(i, end) || on_fit(&fit, end)) {
            end += 1;
            fit.add(v[end]);
            let Some((centre, r)) = fit.circle() else { continue };
            // Each side is held to its own length: a chord of the circle turns from the tangent by
            // half its own angle, a tangent side (however short) by nothing. Measured against a
            // neighbouring facet instead, a shorter facet of the arc itself could pass for a side.
            // The angle alone allows a long side to miss the circle by more than the tolerance:
            // the circle must also touch each side's line, its centre `r` from it.
            let touches = |a: P2, b: P2| (line_distance(centre, a, b) - r).abs() <= tol;
            // A rounded corner's sides meet at the corner and its arc spans less than a
            // semicircle; a longer tangent run (a slot-shaped end, say) is an arc side, left
            // to the gentle-run scan below.
            let (ca, cb) = ([v[i][0] - centre[0], v[i][1] - centre[1]], [v[end][0] - centre[0], v[end][1] - centre[1]]);
            let angle = (cb[1].atan2(cb[0]) - ca[1].atan2(ca[0])).rem_euclid(2.0 * PI);
            let span = angle.min(2.0 * PI - angle);
            if span < PI - 1e-6
                && intersect(before, v[i], v[end], v[(end + 1) % n]).is_some()
                && tangent(before, v[i], centre, len(i + n - 1) / (2.0 * r))
                && tangent(v[(end + 1) % n], v[end], centre, len(end) / (2.0 * r))
                && touches(before, v[i])
                && touches(v[end], v[(end + 1) % n])
                && v[i..=end].iter().all(|p| (distance(*p, centre) - r).abs() <= tol)
            {
                found = Some((end, r));
                break;
            }
        }
        if let Some((last, r)) = found {
            arcs.push((i, last, r));
            i = last;
        } else {
            // A run of three or more facets on one circle that met no tangent side is a curve
            // (or a round outline), not scanned again from inside it, so a long curve is scanned
            // once however uneven its facets. An arc can only start inside the run where a side
            // meets the circle tangentially: the scan goes on at the first such corner, or after
            // the run when it has none.
            let on_one_circle = |(centre, r): (P2, f64)| v[i..=end].iter().all(|p| (distance(*p, centre) - r).abs() <= tol);
            // A curve's facets are of a size; a long straight side and the first facets of a
            // small round also fit one (huge) circle within the tolerance, and must not be
            // skipped.
            let (short, long) = (i..end).map(len).fold((f64::INFINITY, 0.0_f64), |(lo, hi), l| (lo.min(l), hi.max(l)));
            i = match fit.circle() {
                Some((centre, r)) if end > i + 2 && long <= CURVE_LEN_SPREAD * short && on_one_circle((centre, r)) => {
                    (i + 1..end).find(|&j| tangent(v[j - 1], v[j], centre, len(j - 1) / (2.0 * r))).unwrap_or(end)
                }
                _ => i + 1,
            };
        }
    }
    let mut on_arc = vec![false; n];
    for &(first, last, _) in &arcs {
        on_arc[first..=last].iter_mut().for_each(|c| *c = true);
    }

    // A curved side is a run of at least three gentle facets — none part of a rounded corner,
    // the corners at each facet's ends turning a little the same way — whose corners lie on one
    // circle: an arc side from the run's first corner to its last. A gentle run on no circle is
    // a non-circular curve, and a run all the way round a round outline; both are refused.
    // Length only decides whether a run carries on: a curve's first facet may follow a much
    // longer straight side.
    // A facet curves when a corner at either of its ends turns gently: a run may start right
    // after a sharp junction and end right before one (an arc joined to a slanted side at more
    // than MAX_CURVE_TURN, or closing onto a reflex corner, still spans all its facets).
    // Whether the run really is one circle is the fit below; its lengths carry it on.
    let curving =
        |j: usize| !on_arc[j % n] && !on_arc[(j + 1) % n] && (turn(j).abs() <= MAX_CURVE_TURN || turn((j + 1) % n).abs() <= MAX_CURVE_TURN);
    let carries_curve = |from: usize, to: usize| {
        carries_on(from, to) || (len(to) <= GENTLE_LEN_SPREAD * len(from) && len(from) <= GENTLE_LEN_SPREAD * len(to))
    };
    let mut arc_sides: Vec<(usize, usize, P2, f64)> = Vec::new(); // from corner, to corner, centre, radius
    let mut scanned = 0;
    let mut j = 0;
    while scanned < n {
        if !curving(j) {
            j = (j + 1) % n;
            scanned += 1;
            continue;
        }
        let first = j;
        let mut steps = 1;
        while steps < n && curving((first + steps) % n) && carries_curve(first + steps - 1, first + steps) {
            steps += 1;
        }
        if steps >= 3 {
            let w: Vec<P2> = (0..=steps).map(|k| v[(first + k) % n]).collect();
            let (short, long) = (0..steps).map(|k| len(first + k)).fold((f64::INFINITY, 0.0_f64), |(lo, hi), l| (lo.min(l), hi.max(l)));
            let Some((centre, r)) = fit_circle(&w) else {
                return Err("the outline has a curved side that is not a circular arc (only arc sides are recognised so far)".into());
            };
            if long > CURVE_LEN_SPREAD * short || !w.iter().all(|p| (distance(*p, centre) - r).abs() <= tol) {
                return Err("the outline has a curved side that is not a circular arc (only arc sides are recognised so far)".into());
            }
            let theta: f64 = (1..=steps).map(|s| turn(first + s)).sum();
            if steps == n || theta >= 2.0 * PI - 1e-6 {
                return Err("the outline is round all the way round (round outlines are not recognised yet)".into());
            }
            arc_sides.push((first, (first + steps) % n, centre, r));
        }
        j = (first + steps) % n;
        scanned += steps;
    }
    if !arcs.is_empty() && !arc_sides.is_empty() {
        return Err("rounded corners mixed with arc sides are not recognised yet".into());
    }

    // A corner strictly inside an arc side's run is not a corner; a rounded corner becomes the
    // point where its two sides meet; sharp corners stay.
    let inside_arc_side = |k: usize| {
        arc_sides.iter().any(|&(f, t, _, _)| {
            let span = (t + n - f) % n;
            let d = (k + n - f) % n;
            span > 1 && d > 0 && d < span
        })
    };
    let mut corners = Vec::new();
    let mut corner_src = Vec::new();
    let mut k = 0;
    while k < n {
        if inside_arc_side(k) {
            k += 1;
            continue;
        }
        if let Some(&(first, last, _)) = arcs.iter().find(|a| a.0 == k) {
            let corner = intersect(v[(first + n - 1) % n], v[first], v[last], v[(last + 1) % n])
                .ok_or("a corner is rounded all the way round (slot-shaped ends are not recognised yet)")?;
            corners.push(corner);
            corner_src.push(first);
            k = last + 1;
        } else {
            corners.push(v[k]);
            corner_src.push(k);
            k += 1;
        }
    }
    // Two corners with an arc side between them close a valid outline (a chord and its arc);
    // fewer corners, or two straight sides, are degenerate.
    if corners.len() < 2 || corners.len() == 2 && arc_sides.is_empty() {
        return Err("the outline is degenerate".into());
    }
    let arc_of = |src: usize| {
        arc_sides
            .iter()
            .find(|(f, _, _, _)| *f == src)
            .map(|&(_, _, c, r)| ArcSide { centre: c.map(crate::mesh::snap_mm), r: crate::mesh::snap_mm(r) })
    };
    let side_arcs: Vec<Option<ArcSide>> = corner_src.iter().map(|&src| arc_of(src)).collect();
    if arcs.is_empty() {
        return Ok((corners, None, side_arcs));
    }
    if arcs.len() != corners.len() {
        return Err("only some corners of the outline are rounded (mixed corners are not recognised yet)".into());
    }
    let (low, high) = arcs.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), a| (lo.min(a.2), hi.max(a.2)));
    if high - low > tol {
        return Err("the corners are rounded with different radii (mixed radii are not recognised yet)".into());
    }
    Ok((corners, Some(arcs.iter().map(|a| a.2).sum::<f64>() / arcs.len() as f64), side_arcs))
}

/// Running sums for a least-squares circle (Kåsa's algebraic fit) through points added one at a
/// time, around a nearby origin to keep the sums small.
#[derive(Clone)]
pub(super) struct CircleSums {
    pub(super) origin: P2,
    pub(super) n: f64,
    pub(super) x: f64,
    pub(super) y: f64,
    pub(super) xx: f64,
    pub(super) xy: f64,
    pub(super) yy: f64,
    pub(super) xz: f64,
    pub(super) yz: f64,
    pub(super) z: f64,
}

impl CircleSums {
    pub(super) fn new(origin: P2) -> Self {
        Self { origin, n: 0.0, x: 0.0, y: 0.0, xx: 0.0, xy: 0.0, yy: 0.0, xz: 0.0, yz: 0.0, z: 0.0 }
    }

    pub(super) fn add(&mut self, p: P2) {
        let (x, y) = (p[0] - self.origin[0], p[1] - self.origin[1]);
        let z = x * x + y * y;
        self.n += 1.0;
        self.x += x;
        self.y += y;
        self.xx += x * x;
        self.xy += x * y;
        self.yy += y * y;
        self.xz += x * z;
        self.yz += y * z;
        self.z += z;
    }

    /// The circle x² + y² + D x + E y + F = 0 closest to the points, as centre and radius.
    pub(super) fn circle(&self) -> Option<(P2, f64)> {
        // Around the points' mean the system splits: F follows from D and E.
        let (mx, my) = (self.x / self.n, self.y / self.n);
        let (sxx, sxy, syy) = (self.xx - self.n * mx * mx, self.xy - self.n * mx * my, self.yy - self.n * my * my);
        let (rx, ry) = (self.xz - mx * self.z, self.yz - my * self.z);
        let det = sxx * syy - sxy * sxy;
        if det.abs() <= f64::EPSILON * (sxx * syy).max(f64::MIN_POSITIVE) {
            return None;
        }
        let d = (-rx * syy + ry * sxy) / det;
        let e = (-ry * sxx + rx * sxy) / det;
        let f = -(self.z + d * self.x + e * self.y) / self.n;
        let (cx, cy) = (-d / 2.0, -e / 2.0);
        let r2 = cx * cx + cy * cy - f;
        (r2 > 0.0).then(|| ([self.origin[0] + cx, self.origin[1] + cy], r2.sqrt()))
    }
}

/// Drops facet corners that lie on a straight line between their neighbours.
pub(super) fn merge_collinear(ring: &[P2]) -> Vec<P2> {
    let n = ring.len();
    (0..n)
        .filter(|&i| {
            let (a, b, c) = (ring[(i + n - 1) % n], ring[i], ring[(i + 1) % n]);
            let (u, w) = ([b[0] - a[0], b[1] - a[1]], [c[0] - b[0], c[1] - b[1]]);
            let sin = (u[0] * w[1] - u[1] * w[0]) / (u[0].hypot(u[1]) * w[0].hypot(w[1]));
            !(sin.abs() <= COLLINEAR_SIN && u[0] * w[0] + u[1] * w[1] > 0.0)
        })
        .map(|i| ring[i])
        .collect()
}

/// Whether the side from `from` to `at` touches the circle around `centre` at `at`. A facet of
/// the circle turns away from the tangent by half its angle, whose sine is `sin_half`; half of
/// that is the allowance, so an arc's own corners are never taken for its ends.
pub(super) fn tangent(from: P2, at: P2, centre: P2, sin_half: f64) -> bool {
    let side = [at[0] - from[0], at[1] - from[1]];
    let radius = [at[0] - centre[0], at[1] - centre[1]];
    let cos = (side[0] * radius[0] + side[1] * radius[1]) / (side[0].hypot(side[1]) * radius[0].hypot(radius[1]));
    cos.abs() <= (0.5 * sin_half).min(0.05)
}

/// Where the line through `a` and `b` meets the line through `c` and `d`.
pub(super) fn intersect(a: P2, b: P2, c: P2, d: P2) -> Option<P2> {
    let (u, w) = ([b[0] - a[0], b[1] - a[1]], [d[0] - c[0], d[1] - c[1]]);
    let den = u[0] * w[1] - u[1] * w[0];
    if den.abs() <= PARALLEL_DENOM * u[0].hypot(u[1]) * w[0].hypot(w[1]) {
        return None;
    }
    let t = ((c[0] - a[0]) * w[1] - (c[1] - a[1]) * w[0]) / den;
    Some([a[0] + t * u[0], a[1] + t * u[1]])
}

/// The area the four corners of a rectangle lose to roundings of radius `r`.
pub(crate) fn corner_loss(r: f64) -> f64 {
    (4.0 - PI) * r * r
}

/// The volume a sharp box `x` by `y` by `z` loses when all twelve edges are rounded with radius
/// `r`: each edge, away from the corners, loses a square minus a quarter circle, and each corner
/// a cube minus a sphere octant (mesh2cad's `_edge_fillet_removed`).
pub(crate) fn edge_fillet_removed(x: f64, y: f64, z: f64, r: f64) -> f64 {
    4.0 * r * r * (1.0 - PI / 4.0) * (x + y + z - 6.0 * r) + 8.0 * r * r * r * (1.0 - PI / 6.0)
}

/// The area the corner `c` of a counter-clockwise outline (coming from `a`, going on to `b`)
/// loses to a rounding of radius `r`; negative for an inside corner, which gains it.
pub(crate) fn fillet_loss(a: P2, c: P2, b: P2, r: f64) -> f64 {
    let (u, w) = ([c[0] - a[0], c[1] - a[1]], [b[0] - c[0], b[1] - c[1]]);
    let turn = (u[0] * w[1] - u[1] * w[0]).atan2(u[0] * w[0] + u[1] * w[1]);
    turn.signum() * r * r * ((turn.abs() / 2.0).tan() - turn.abs() / 2.0)
}
