//! The volume of an intersection of extruded outlines, exactly.
//!
//! The part is sliced along Z. At a height `z` the outline along X gives the slice's Y intervals,
//! the one along Y its X intervals, and the slice is the outline along Z clipped to each
//! rectangle of an X and a Y interval. Between the heights where an interval's end passes a corner
//! of the Z outline, a rectangle's corner crosses one of its sides, or an outline has a corner,
//! every corner of the slice moves linearly with `z`, so its area is quadratic and two-point
//! Gauss–Legendre integration is exact.

#![allow(clippy::many_single_char_names)] // points a, b, c, p, q, as in the geometry

type P2 = [f64; 2];

/// The volume of the intersection, within the box from the origin to `size`, of the outline
/// along Z (in (x, y)), the one along X (in (y, z)) and the one along Y (in (x, z)); `None` for an
/// axis whose outline is the box's side.
pub(crate) fn intersection_volume(size: [f64; 3], z: Option<&[P2]>, x: Option<&[P2]>, y: Option<&[P2]>) -> f64 {
    let rect = |w: f64, h: f64| vec![[0.0, 0.0], [w, 0.0], [w, h], [0.0, h]];
    let pz = z.map_or_else(|| rect(size[0], size[1]), <[P2]>::to_vec);
    let px = x.map_or_else(|| rect(size[1], size[2]), <[P2]>::to_vec);
    let py = y.map_or_else(|| rect(size[0], size[2]), <[P2]>::to_vec);
    // The heights where an outline along X or Y has a corner.
    let mut levels: Vec<f64> = px.iter().chain(&py).map(|p| p[1]).chain([0.0, size[2]]).filter(|h| (0.0..=size[2]).contains(h)).collect();
    sort_dedup(&mut levels);
    let mut total = 0.0;
    for w in levels.windows(2) {
        let (z0, z1) = (w[0], w[1]);
        if z1 - z0 <= 0.0 {
            continue;
        }
        // Between corners the intervals' ends move linearly: read them at two heights.
        let (za, zb) = (z0 + (z1 - z0) / 4.0, z0 + 3.0 * (z1 - z0) / 4.0);
        let lines = |poly: &[P2]| -> Vec<(f64, f64)> {
            let (a, b) = (ends(poly, za), ends(poly, zb));
            // Each end as value = c + k z.
            a.iter()
                .zip(&b)
                .map(|(va, vb)| {
                    let k = (vb - va) / (zb - za);
                    (va - k * za, k)
                })
                .collect()
        };
        let (xs, ys) = (lines(&py), lines(&px));
        let mut cuts = vec![z0, z1];
        let mut cut = |t: f64| {
            if t > z0 && t < z1 {
                cuts.push(t);
            }
        };
        for &(c, k) in &xs {
            for p in &pz {
                if k != 0.0 {
                    cut((p[0] - c) / k);
                }
            }
        }
        for &(c, k) in &ys {
            for p in &pz {
                if k != 0.0 {
                    cut((p[1] - c) / k);
                }
            }
        }
        // A rectangle's corner (x(z), y(z)) crossing a side of the Z outline.
        let n = pz.len();
        for &(cx, kx) in &xs {
            for &(cy, ky) in &ys {
                for i in 0..n {
                    let (p, q) = (pz[i], pz[(i + 1) % n]);
                    let d = [q[0] - p[0], q[1] - p[1]];
                    // cx + kx z = p.x + s d.x, cy + ky z = p.y + s d.y.
                    let den = kx * d[1] - ky * d[0];
                    if den.abs() > 1e-15 {
                        let t = ((p[0] - cx) * d[1] - (p[1] - cy) * d[0]) / den;
                        let s = if d[0].abs() > d[1].abs() { (cx + kx * t - p[0]) / d[0] } else { (cy + ky * t - p[1]) / d[1] };
                        if (0.0..=1.0).contains(&s) {
                            cut(t);
                        }
                    }
                }
            }
        }
        sort_dedup(&mut cuts);
        for w in cuts.windows(2) {
            let (a, b) = (w[0], w[1]);
            let (mid, half) = (f64::midpoint(a, b), (b - a) / 2.0);
            let g = half / 3.0_f64.sqrt();
            total += half * (slice_area(&pz, &px, &py, mid - g) + slice_area(&pz, &px, &py, mid + g));
        }
    }
    total
}

fn sort_dedup(v: &mut Vec<f64>) {
    v.sort_by(f64::total_cmp);
    v.dedup_by(|a, b| (*a - *b).abs() <= 1e-12 * (1.0 + b.abs()));
}

/// Where the line at height `h` (the second coordinate) crosses the polygon's sides, in order.
fn ends(poly: &[P2], h: f64) -> Vec<f64> {
    let n = poly.len();
    let mut out: Vec<f64> = (0..n)
        .filter_map(|i| {
            let (p, q) = (poly[i], poly[(i + 1) % n]);
            ((p[1] <= h) != (q[1] <= h)).then(|| p[0] + (h - p[1]) * (q[0] - p[0]) / (q[1] - p[1]))
        })
        .collect();
    out.sort_by(f64::total_cmp);
    out
}

/// The area of the slice at height `z`.
fn slice_area(pz: &[P2], px: &[P2], py: &[P2], z: f64) -> f64 {
    let (xs, ys) = (ends(py, z), ends(px, z));
    let mut area = 0.0;
    for [x0, x1] in xs.as_chunks::<2>().0 {
        for [y0, y1] in ys.as_chunks::<2>().0 {
            area += clipped_area(pz, [*x0, *x1, *y0, *y1]);
        }
    }
    area
}

/// The area of the polygon inside the rectangle `[x0, x1, y0, y1]` (Sutherland–Hodgman: the
/// rectangle is convex, so the area is right for any simple polygon).
fn clipped_area(poly: &[P2], r: [f64; 4]) -> f64 {
    let mut ring = poly.to_vec();
    // Each side as (axis, bound, keep below).
    for (axis, bound, below) in [(0, r[0], false), (0, r[1], true), (1, r[2], false), (1, r[3], true)] {
        let inside = |p: &P2| if below { p[axis] <= bound } else { p[axis] >= bound };
        let n = ring.len();
        let mut out = Vec::with_capacity(n + 4);
        for i in 0..n {
            let (p, q) = (ring[i], ring[(i + 1) % n]);
            if inside(&p) {
                out.push(p);
            }
            if inside(&p) != inside(&q) {
                let t = (bound - p[axis]) / (q[axis] - p[axis]);
                out.push([p[0] + t * (q[0] - p[0]), p[1] + t * (q[1] - p[1])]);
            }
        }
        ring = out;
        if ring.is_empty() {
            return 0.0;
        }
    }
    let n = ring.len();
    ((0..n).map(|i| ring[i][0] * ring[(i + 1) % n][1] - ring[(i + 1) % n][0] * ring[i][1]).sum::<f64>() / 2.0).abs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boxes_and_prisms() {
        assert!((intersection_volume([2.0, 3.0, 4.0], None, None, None) - 24.0).abs() < 1e-9);
        // A triangle along X: half the box.
        let tri = [[0.0, 0.0], [3.0, 0.0], [0.0, 4.0]];
        assert!((intersection_volume([2.0, 3.0, 4.0], None, Some(&tri), None) - 12.0).abs() < 1e-9);
    }

    /// The L-shaped plate under a ramp the SolidCraft fixture is made of: 37 000 mm³ as SolidCraft
    /// measures it; and with the front outline as well, 36 100 mm³.
    #[test]
    fn the_fixtures_volumes() {
        let l = [[0.0, 0.0], [60.0, 0.0], [60.0, 15.0], [20.0, 15.0], [20.0, 40.0], [0.0, 40.0]];
        let ramp = [[0.0, 0.0], [40.0, 0.0], [40.0, 10.0], [15.0, 30.0], [0.0, 30.0]];
        let v = intersection_volume([60.0, 40.0, 30.0], Some(&l), Some(&ramp), None);
        assert!((v - 37_000.0).abs() < 1e-6, "{v}");
        let front = [[0.0, 0.0], [60.0, 0.0], [60.0, 20.0], [48.0, 30.0], [0.0, 30.0]];
        let v = intersection_volume([60.0, 40.0, 30.0], Some(&l), Some(&ramp), Some(&front));
        assert!((v - 36_100.0).abs() < 1e-6, "{v}");
    }

    /// Two triangles crossing each other: a corner of the slice's rectangle passes through the
    /// outline along Z's sides, which the integration has to cut at.
    #[test]
    fn crossing_triangles_match_a_fine_sum() {
        let size = [10.0, 10.0, 10.0];
        let pz = [[0.0, 0.0], [10.0, 3.0], [2.0, 10.0]];
        let px = [[0.0, 0.0], [10.0, 0.0], [4.0, 10.0]];
        let py = [[0.0, 2.0], [10.0, 0.0], [10.0, 10.0], [1.0, 9.0]];
        let exact = intersection_volume(size, Some(&pz), Some(&px), Some(&py));
        let n = 20_000;
        let fine: f64 = (0..n).map(|i| slice_area(&pz, &px, &py, (f64::from(i) + 0.5) * 10.0 / f64::from(n)) * 10.0 / f64::from(n)).sum();
        assert!((exact - fine).abs() < 1e-4 * fine, "{exact} vs {fine}");
    }
}
