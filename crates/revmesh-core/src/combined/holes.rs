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

/// The bores among `facets`, and the facets left over.
pub(super) fn find_bores(facets: &[Facet], tol: f64) -> (Vec<Bore>, Vec<Facet>) {
    let mut taken = vec![false; facets.len()];
    let mut bores = Vec::new();
    for axis in [Axis::X, Axis::Y, Axis::Z] {
        let a = axis.index();
        // Nearly parallel to the axis (flat sides too: they fit no circle).
        let candidates: Vec<usize> = (0..facets.len()).filter(|&i| !taken[i] && facets[i].1[a].abs() <= BORE_PARALLEL).collect();
        for group in groups(facets, &candidates) {
            if let Some(bore) = bore(facets, &group, axis, tol) {
                for &i in &group {
                    taken[i] = true;
                }
                bores.push(bore);
            }
        }
    }
    // A facet tilted more than the candidates (a narrow one where the corners are staggered) is
    // the bore's too when all its corners lie on the bore's cylinder.
    for (i, (t, _)) in facets.iter().enumerate() {
        if !taken[i] && bores.iter().any(|b| t.iter().all(|p| b.holds(*p, tol))) {
            taken[i] = true;
        }
    }
    let rest = facets.iter().zip(&taken).filter(|(_, t)| !**t).map(|(f, _)| *f).collect();
    (bores, rest)
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
    // Right round the axis.
    let mut angles: Vec<f64> = flat.iter().map(|p| (p[1] - centre[1]).atan2(p[0] - centre[0])).collect();
    angles.sort_by(f64::total_cmp);
    let wrap = angles[0] + std::f64::consts::TAU - angles[angles.len() - 1];
    if angles.windows(2).map(|w| w[1] - w[0]).chain([wrap]).any(|gap| gap > MAX_GAP) {
        return None;
    }
    let lo = corners.iter().map(|q| q[2]).fold(f64::INFINITY, f64::min);
    let hi = corners.iter().map(|q| q[2]).fold(f64::NEG_INFINITY, f64::max);
    Some(Bore { axis, centre, r, lo, hi })
}
