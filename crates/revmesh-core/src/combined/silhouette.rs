//! The silhouette of a set of triangles in the plane: the outline of their union.
//!
//! The union's boundary lies on the triangles' edges. An edge two triangles share from opposite
//! sides is covered on both and dropped; the rest are split where they cross or touch, and a piece
//! is kept where the union lies on one side of it only, turned so that the union is on its left.
//! The pieces are then chained into loops: counter-clockwise outlines, clockwise openings.

#![allow(clippy::many_single_char_names)] // points a, b, c, p, q, as in the geometry

use std::collections::{HashMap, HashSet};

pub(crate) type P2 = [f64; 2];

/// A point by its coordinates' bits, and a directed edge by its ends'.
type Key = (u64, u64);
type EdgeKey = (Key, Key);

/// At most this many edges are split against each other (the work grows with their square).
const MAX_EDGES: usize = 4096;

fn sub(a: P2, b: P2) -> P2 {
    [a[0] - b[0], a[1] - b[1]]
}

fn cross(a: P2, b: P2) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}

fn dot(a: P2, b: P2) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

fn dist(a: P2, b: P2) -> f64 {
    sub(a, b)[0].hypot(sub(a, b)[1])
}

/// Twice the signed area of a loop: positive counter-clockwise.
pub(crate) fn area2(ring: &[P2]) -> f64 {
    let n = ring.len();
    (0..n).map(|i| cross(ring[i], ring[(i + 1) % n])).sum()
}

/// How far `p` lies from the segment `a`–`b`.
pub(crate) fn segment_distance(p: P2, a: P2, b: P2) -> f64 {
    let d = sub(b, a);
    let len2 = dot(d, d);
    let t = if len2 > 0.0 { (dot(sub(p, a), d) / len2).clamp(0.0, 1.0) } else { 0.0 };
    dist(p, [a[0] + t * d[0], a[1] + t * d[1]])
}

/// Whether `p` lies in the triangle (counter-clockwise), its edges included.
fn in_triangle(t: &[P2; 3], p: P2) -> bool {
    (0..3).all(|i| cross(sub(t[(i + 1) % 3], t[i]), sub(p, t[i])) >= 0.0)
}

/// The loops bounding the union of `triangles`, corners closer than `eps` to the line through
/// their neighbours dropped. `Err` when there are too many edges to split.
pub(super) fn loops(triangles: &[[P2; 3]], eps: f64) -> Result<Vec<Vec<P2>>, String> {
    // Counter-clockwise, the slivers dropped.
    let tris: Vec<[P2; 3]> = triangles
        .iter()
        .filter_map(|t| {
            let a = cross(sub(t[1], t[0]), sub(t[2], t[0]));
            if a.abs() <= eps * eps {
                None
            } else if a > 0.0 {
                Some(*t)
            } else {
                Some([t[0], t[2], t[1]])
            }
        })
        .collect();
    let edges = open_edges(&tris);
    if edges.len() > MAX_EDGES {
        return Err(format!("its silhouette has more than {MAX_EDGES} edges"));
    }
    let pieces = split(&edges, eps);
    let boxes: Vec<[f64; 4]> = tris
        .iter()
        .map(|t| {
            let xs = t.map(|p| p[0]);
            let ys = t.map(|p| p[1]);
            [
                xs.iter().copied().fold(f64::INFINITY, f64::min),
                xs.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                ys.iter().copied().fold(f64::INFINITY, f64::min),
                ys.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            ]
        })
        .collect();
    let inside =
        |p: P2| tris.iter().zip(&boxes).any(|(t, b)| p[0] >= b[0] && p[0] <= b[1] && p[1] >= b[2] && p[1] <= b[3] && in_triangle(t, p));
    let mut kept = Vec::new();
    for (a, b) in pieces {
        let len = dist(a, b);
        let d = [(b[0] - a[0]) / len, (b[1] - a[1]) / len];
        let m = [f64::midpoint(a[0], b[0]), f64::midpoint(a[1], b[1])];
        let delta = (len / 4.0).min(eps);
        let left = inside([m[0] - d[1] * delta, m[1] + d[0] * delta]);
        let right = inside([m[0] + d[1] * delta, m[1] - d[0] * delta]);
        match (left, right) {
            (true, false) => kept.push((a, b)),
            (false, true) => kept.push((b, a)),
            _ => {}
        }
    }
    let rings = chain(kept, eps)?;
    Ok(rings.into_iter().map(|r| drop_collinear(r, eps)).filter(|r| r.len() >= 3).collect())
}

/// The triangles' edges no other triangle shares from the opposite side.
fn open_edges(tris: &[[P2; 3]]) -> Vec<(P2, P2)> {
    let key = |p: P2| -> Key { (p[0].to_bits(), p[1].to_bits()) };
    let mut count: HashMap<EdgeKey, i32> = HashMap::new();
    let mut order = Vec::new();
    for t in tris {
        for i in 0..3 {
            let (a, b) = (t[i], t[(i + 1) % 3]);
            let (ka, kb) = (key(a), key(b));
            if let Some(n) = count.get_mut(&(kb, ka)) {
                *n -= 1;
                continue;
            }
            let n = count.entry((ka, kb)).or_insert(0);
            if *n == 0 {
                order.push((a, b));
            }
            *n += 1;
        }
    }
    order.into_iter().filter(|(a, b)| count.get(&(key(*a), key(*b))).is_some_and(|n| *n > 0)).collect()
}

/// The edges cut where they cross or touch each other, pieces shorter than `eps` and repeats
/// dropped.
fn split(edges: &[(P2, P2)], eps: f64) -> Vec<(P2, P2)> {
    let mut cuts: Vec<Vec<f64>> = vec![vec![0.0, 1.0]; edges.len()];
    for i in 0..edges.len() {
        for j in i + 1..edges.len() {
            let (a, b) = edges[i];
            let (c, d) = edges[j];
            let (r, s) = (sub(b, a), sub(d, c));
            // Each end of one on the other.
            for (k, p, (o, e)) in [(i, c, (a, b)), (i, d, (a, b)), (j, a, (c, d)), (j, b, (c, d))] {
                if segment_distance(p, o, e) <= eps {
                    let de = sub(e, o);
                    cuts[k].push((dot(sub(p, o), de) / dot(de, de)).clamp(0.0, 1.0));
                }
            }
            let den = cross(r, s);
            if den.abs() > 1e-12 * (dot(r, r) * dot(s, s)).sqrt() {
                let t = cross(sub(c, a), s) / den;
                let u = cross(sub(c, a), r) / den;
                if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
                    cuts[i].push(t);
                    cuts[j].push(u);
                }
            }
        }
    }
    let mut seen = HashSet::new();
    let mut pieces = Vec::new();
    for ((a, b), mut ts) in edges.iter().zip(cuts) {
        ts.sort_by(f64::total_cmp);
        let at = |t: f64| [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])];
        let mut last = at(ts[0]);
        for &t in &ts[1..] {
            let p = at(t);
            if dist(last, p) > eps {
                let q = |p: P2| ((p[0] / eps).round() as i64, (p[1] / eps).round() as i64);
                let k = if q(last) <= q(p) { (q(last), q(p)) } else { (q(p), q(last)) };
                if seen.insert(k) {
                    pieces.push((last, p));
                }
                last = p;
            }
        }
    }
    pieces
}

/// The directed pieces joined end to start into closed loops.
pub(crate) fn chain(mut pieces: Vec<(P2, P2)>, eps: f64) -> Result<Vec<Vec<P2>>, String> {
    let mut rings = Vec::new();
    while let Some((start, mut end)) = pieces.pop() {
        let mut ring = vec![start];
        while dist(end, start) > eps {
            let Some(k) = pieces.iter().position(|(a, _)| dist(*a, end) <= eps) else {
                return Err("its silhouette does not close".into());
            };
            let (a, b) = pieces.swap_remove(k);
            ring.push(a);
            end = b;
        }
        rings.push(ring);
    }
    Ok(rings)
}

/// The loop without the corners that lie within `eps` of the line through their neighbours, on
/// it between them.
pub(crate) fn drop_collinear(mut ring: Vec<P2>, eps: f64) -> Vec<P2> {
    let mut i = 0;
    while ring.len() > 3 && i < ring.len() {
        let n = ring.len();
        let (a, b, c) = (ring[(i + n - 1) % n], ring[i], ring[(i + 1) % n]);
        if segment_distance(b, a, c) <= eps && dot(sub(b, a), sub(c, b)) > 0.0 {
            ring.remove(i);
            i = i.saturating_sub(1);
        } else {
            i += 1;
        }
    }
    ring
}
