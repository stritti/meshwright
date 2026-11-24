//! Round holes in a combined extrusion: through holes and blind holes along X, Y or Z.
//!
//! mesh2cad writes them as `hole_<n>_d` and the hole's two coordinates across its axis, a blind
//! one with `hole_<n>_depth` (`csg.py`). Exporters mesh a bore's wall with corners staggered
//! along it (SolidCraft's standard export does), so its facets are tilted a little to every axis.
//! The bores are found first: facets nearly parallel to an axis, grouped by shared corners, whose
//! corners all lie on one circle round the axis, facing its centre, and going right round it.
//! They are set aside; the rest of the part shows each bore as a round opening through an outline
//! (a through hole) or a round cut from a face (a blind hole), which then becomes the hole.

#![allow(clippy::many_single_char_names)] // points a, b, c, p, q, as in the geometry

use std::collections::HashMap;

use super::silhouette::P2;

use crate::mesh::Vec3;
use crate::recognize::Axis;

/// A facet: its corners and its unit normal.
type Facet = ([Vec3; 3], Vec3);

/// A bore's facet is this nearly parallel to its axis: its normal's component along it (about 3°).
const BORE_PARALLEL: f64 = 0.05;

/// A facet joins a found bore's wall only this nearly parallel to its axis (30°): a narrow facet
/// where the corners are staggered tilts more than the candidates, never across the axis.
const WALL_ALONG: f64 = 0.5;

/// A bore has at least this many corners round it.
const MIN_CORNERS: usize = 8;

/// Its corners leave no gap round the axis wider than this (as a plate's holes, 45°).
const MAX_GAP: f64 = std::f64::consts::FRAC_PI_4;

/// A bore: a round hole's wall, its circle in the axis's profile coordinates, from `lo` to `hi`
/// along the axis.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Bore {
    pub(super) axis: Axis,
    pub(super) centre: P2,
    pub(super) r: f64,
    pub(super) lo: f64,
    pub(super) hi: f64,
}

impl Bore {
    /// The rectangle the bore covers seen along `axis` (another axis), as two counter-clockwise
    /// triangles in that axis's profile coordinates: its span along its own axis by its circle's
    /// diameter.
    pub(super) fn across(&self, axis: Axis) -> [[P2; 3]; 2] {
        let a = self.axis.index();
        let centre = self.axis.world([0.0; 3], self.centre[0], self.centre[1], 0.0);
        // The third axis, square to both.
        let c = 3 - a - axis.index();
        let corner = |along: f64, side: f64| {
            let mut p = centre;
            p[a] = along;
            p[c] = centre[c] + side * self.r;
            let q = axis.rotate(p);
            [q[0], q[1]]
        };
        let [p, q, r, s] = [corner(self.lo, -1.0), corner(self.hi, -1.0), corner(self.hi, 1.0), corner(self.lo, 1.0)];
        let ccw = |t: [P2; 3]| {
            let area = (t[1][0] - t[0][0]) * (t[2][1] - t[0][1]) - (t[1][1] - t[0][1]) * (t[2][0] - t[0][0]);
            if area < 0.0 { [t[0], t[2], t[1]] } else { t }
        };
        [ccw([p, q, r]), ccw([p, r, s])]
    }

    /// Whether the point (part coordinates) lies on the bore's wall: on its cylinder, within its
    /// span.
    fn holds(&self, p: Vec3, tol: f64) -> bool {
        let q = self.axis.rotate(p);
        ((q[0] - self.centre[0]).hypot(q[1] - self.centre[1]) - self.r).abs() <= tol && q[2] >= self.lo - tol && q[2] <= self.hi + tol
    }

    /// Whether every corner of `ring` lies on the bore's circle.
    pub(super) fn matches(&self, ring: &[P2], tol: f64) -> bool {
        ring.len() >= MIN_CORNERS && ring.iter().all(|p| ((p[0] - self.centre[0]).hypot(p[1] - self.centre[1]) - self.r).abs() <= tol)
    }
}

/// The bores among `facets`, the facets left over, and the bores' walls with each one's axis.
pub(super) fn find_bores(facets: &[Facet], tol: f64) -> (Vec<Bore>, Vec<Facet>, Vec<(Axis, Facet)>) {
    let mut taken: Vec<Option<Axis>> = vec![None; facets.len()];
    let mut bores = Vec::new();
    for axis in [Axis::X, Axis::Y, Axis::Z] {
        let a = axis.index();
        // Nearly parallel to the axis (flat sides too: they fit no circle).
        let candidates: Vec<usize> = (0..facets.len()).filter(|&i| taken[i].is_none() && facets[i].1[a].abs() <= BORE_PARALLEL).collect();
        for group in groups(facets, &candidates) {
            if let Some(bore) = bore(facets, &group, axis, tol) {
                for &i in &group {
                    taken[i] = Some(axis);
                }
                bores.push(bore);
            }
        }
    }
    // A facet tilted more than the candidates (a narrow one where the corners are staggered) is
    // the bore's too when all its corners lie on the bore's cylinder and it runs along the axis
    // (a blind hole's floor made of the rim's corners only lies on the cylinder too, across it).
    for (i, (t, n)) in facets.iter().enumerate() {
        if taken[i].is_none() {
            taken[i] = bores.iter().find(|b| n[b.axis.index()].abs() <= WALL_ALONG && t.iter().all(|p| b.holds(*p, tol))).map(|b| b.axis);
        }
    }
    let rest = facets.iter().zip(&taken).filter(|(_, t)| t.is_none()).map(|(f, _)| *f).collect();
    let walls = facets.iter().zip(&taken).filter_map(|(f, t)| t.map(|axis| (axis, *f))).collect();
    (bores, rest, walls)
}

/// The facets `among` grouped by the corners they share.
fn groups(facets: &[Facet], among: &[usize]) -> Vec<Vec<usize>> {
    let key = |p: &Vec3| p.map(f64::to_bits);
    let mut parent: Vec<usize> = (0..among.len()).collect();
    let mut owner: HashMap<[u64; 3], usize> = HashMap::new();
    for (k, &i) in among.iter().enumerate() {
        for p in &facets[i].0 {
            match owner.get(&key(p)) {
                Some(&j) => {
                    let (a, b) = (root(&mut parent, k), root(&mut parent, j));
                    parent[a] = b;
                }
                None => {
                    owner.insert(key(p), k);
                }
            }
        }
    }
    let mut out: std::collections::BTreeMap<usize, Vec<usize>> = std::collections::BTreeMap::new();
    for (k, &i) in among.iter().enumerate() {
        let r = root(&mut parent, k);
        out.entry(r).or_default().push(i);
    }
    out.into_values().collect()
}

/// The root of `i`'s set in the union-find forest `parent`, its path halved on the way.
fn root(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

/// The group as a bore along `axis`, if it is one.
fn bore(facets: &[Facet], group: &[usize], axis: Axis, tol: f64) -> Option<Bore> {
    let corners: Vec<Vec3> = group.iter().flat_map(|&i| facets[i].0).map(|p| axis.rotate(p)).collect();
    let flat: Vec<P2> = corners.iter().map(|q| [q[0], q[1]]).collect();
    if flat.len() < MIN_CORNERS {
        return None;
    }
    let (centre, r) = crate::plate::fit_circle(&flat)?;
    if flat.iter().any(|p| ((p[0] - centre[0]).hypot(p[1] - centre[1]) - r).abs() > tol) {
        return None;
    }
    // Facing the centre: a hole, not a boss.
    for &i in group {
        let (t, n) = facets[i];
        let m = axis.rotate(n);
        let q = t.map(|p| axis.rotate(p));
        let mid = [(q[0][0] + q[1][0] + q[2][0]) / 3.0, (q[0][1] + q[1][1] + q[2][1]) / 3.0];
        if m[0] * (centre[0] - mid[0]) + m[1] * (centre[1] - mid[1]) <= 0.0 {
            return None;
        }
    }
    // Right round the axis, and so are its ends, each square to it: seen across the axis the bore
    // then covers a rectangle (a hole ending in a slanted face stays an opening of its facets).
    let round = |at: Option<f64>| {
        let mut angles: Vec<f64> = corners
            .iter()
            .filter(|q| at.is_none_or(|w| (q[2] - w).abs() <= tol))
            .map(|q| (q[1] - centre[1]).atan2(q[0] - centre[0]))
            .collect();
        angles.sort_by(f64::total_cmp);
        let (Some(first), Some(last)) = (angles.first(), angles.last()) else { return false };
        let wrap = first + std::f64::consts::TAU - last;
        angles.windows(2).map(|w| w[1] - w[0]).chain([wrap]).all(|gap| gap <= MAX_GAP)
    };
    let lo = corners.iter().map(|q| q[2]).fold(f64::INFINITY, f64::min);
    let hi = corners.iter().map(|q| q[2]).fold(f64::NEG_INFINITY, f64::max);
    (round(None) && round(Some(lo)) && round(Some(hi))).then_some(Bore { axis, centre, r, lo, hi })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A blind bore 32 facets round, its floor a fan of the rim's corners only: the floor lies on
    /// the bore's cylinder too, but across its axis, and is no part of its wall.
    #[test]
    fn a_floor_of_rim_corners_is_no_wall() {
        use std::f64::consts::TAU;
        let rim = |i: usize, z: f64| -> Vec3 {
            let t = i as f64 * TAU / 32.0;
            [10.0 + 3.0 * t.cos(), 10.0 + 3.0 * t.sin(), z]
        };
        let facet = |t: [Vec3; 3]| -> Facet {
            let n = crate::mesh::cross(crate::mesh::sub(t[1], t[0]), crate::mesh::sub(t[2], t[0]));
            let len = crate::mesh::length(n);
            (t, n.map(|c| c / len))
        };
        let mut facets = Vec::new();
        for i in 0..32 {
            let j = (i + 1) % 32;
            // Facing the axis: wound so that the normal points inwards.
            facets.push(facet([rim(i, 2.0), rim(i, 10.0), rim(j, 2.0)]));
            facets.push(facet([rim(j, 2.0), rim(i, 10.0), rim(j, 10.0)]));
        }
        for i in 1..31 {
            facets.push(facet([rim(0, 2.0), rim(i, 2.0), rim(i + 1, 2.0)]));
        }
        let (bores, rest, walls) = find_bores(&facets, 1e-3);
        assert_eq!(walls.len(), 64);
        assert_eq!(bores.len(), 1, "{bores:?}");
        assert_eq!((bores[0].lo, bores[0].hi), (2.0, 10.0));
        assert_eq!(rest.len(), 30);
        assert!(rest.iter().all(|(_, n)| n[2].abs() > 0.99));
    }
}
