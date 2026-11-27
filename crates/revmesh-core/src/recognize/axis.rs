//! The axis a part is extruded along, and the rotations that align it with Z.
//!
//! mesh2cad tries a prism along X, Y and Z and takes whichever fits. revmesh has one pipeline,
//! written for a Z extrusion, so it rotates the mesh into a frame where the part's axis is +Z,
//! recognises there, and rotates the result back. Both rotations are proper (determinant +1), so
//! volumes, areas and loop windings survive unchanged. When no principal axis fits, roadmap 6.1
//! fits the prism's axis itself: [`prism_axis`] finds the two dominant parallel planes by
//! fixed-seed RANSAC and returns their normal as a [`Frame`], another proper rotation.
//!
//! SolidCraft's origin planes fix the sketch side of each frame (solidcore's `Plane`: `YZ` has
//! sketch axes (Y, Z) and normal +X, `XZ` has (X, −Z) and normal +Y, `XY` has (X, Y) and normal
//! +Z), so the frame's profile coordinates `(u, v)` are exactly the sketch coordinates on the
//! matching plane, and `Extent::positive` extrudes along the axis. mesh2cad stores the outline
//! the same way: X extrusions as (y, z), Y as (x, z), Z as (x, y).

use crate::mesh::{Mesh, Vec3, cross, dot, length, sub};
use crate::plate::Rng;

/// A principal axis a part may be extruded along.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    /// Along X: the profile lies in the YZ plane, as (y, z).
    X,
    /// Along Y: the profile lies in the XZ plane, as (x, −z).
    Y,
    /// Along Z: the profile lies in the XY plane, as (x, y).
    Z,
}

impl Axis {
    /// Every axis, in the order recognition tries them. Z comes first: an axis-aligned box is a
    /// prism along all three, and as a Z plate it keeps mesh2cad's `base_*` names (mesh2cad's box
    /// vocabulary takes it too). X and Y follow for parts standing upright.
    pub const ALL: [Axis; 3] = [Axis::Z, Axis::X, Axis::Y];

    /// The world axis index: X = 0, Y = 1, Z = 2.
    #[must_use]
    pub fn index(self) -> usize {
        match self {
            Axis::X => 0,
            Axis::Y => 1,
            Axis::Z => 2,
        }
    }

    /// The axis's letter: `'X'`, `'Y'` or `'Z'`.
    #[must_use]
    pub fn letter(self) -> char {
        match self {
            Axis::X => 'X',
            Axis::Y => 'Y',
            Axis::Z => 'Z',
        }
    }

    /// The origin plane a sketch of the profile lies on: `YZ` for X, `XZ` for Y, `XY` for Z.
    #[must_use]
    pub fn plane(self) -> &'static str {
        match self {
            Axis::X => "YZ",
            Axis::Y => "XZ",
            Axis::Z => "XY",
        }
    }

    /// The +axis direction in world coordinates: the sketch plane's normal, so an extrusion with
    /// `Extent::positive` runs along it.
    #[must_use]
    pub fn dir(self) -> Vec3 {
        let mut d = [0.0; 3];
        d[self.index()] = 1.0;
        d
    }

    /// A world point in the recognition frame (a proper rotation taking the axis to +Z).
    #[must_use]
    pub fn rotate(self, p: Vec3) -> Vec3 {
        match self {
            Axis::X => [p[1], p[2], p[0]],
            Axis::Y => [p[0], -p[2], p[1]],
            Axis::Z => p,
        }
    }

    /// A recognition-frame point back in world coordinates.
    #[must_use]
    pub fn unrotate(self, p: Vec3) -> Vec3 {
        match self {
            Axis::X => [p[2], p[0], p[1]],
            Axis::Y => [p[0], p[2], -p[1]],
            Axis::Z => p,
        }
    }

    /// The mesh with every corner rotated into the recognition frame.
    #[must_use]
    pub fn rotate_mesh(self, mesh: &Mesh) -> Mesh {
        Mesh::new(mesh.triangles.iter().map(|t| t.map(|p| self.rotate(p))).collect())
    }

    /// The mesh with every corner rotated out of the recognition frame, back to world: a Z prism
    /// becomes a prism along this axis (tests and fixtures build standing parts this way).
    #[must_use]
    pub fn unrotate_mesh(self, mesh: &Mesh) -> Mesh {
        Mesh::new(mesh.triangles.iter().map(|t| t.map(|p| self.unrotate(p))).collect())
    }

    /// Which world axis each profile coordinate runs along: `[u, v]` as world axis indices.
    #[must_use]
    pub fn profile_axes(self) -> [usize; 2] {
        match self {
            Axis::X => [1, 2],
            Axis::Y => [0, 2],
            Axis::Z => [0, 1],
        }
    }

    /// The sign of profile coordinate `i` in its world axis (the XZ plane's sketch y is −Z).
    #[must_use]
    pub fn profile_sign(self, i: usize) -> f64 {
        match (self, i) {
            (Axis::Y, 1) => -1.0,
            _ => 1.0,
        }
    }

    /// The world axis letter (`'x'`, `'y'`, `'z'`) profile coordinate `i` is named after in
    /// parameters, as mesh2cad names a hole's two profile coordinates.
    #[must_use]
    pub fn profile_letter(self, i: usize) -> char {
        "xyz".as_bytes()[self.profile_axes()[i]] as char
    }

    /// The world coordinate of profile coordinate `i` (0: u, 1: v) with value `value`, for a part
    /// whose bounding box's lowest corner is at `at`.
    #[must_use]
    pub fn profile_world(self, at: Vec3, i: usize, value: f64) -> f64 {
        at[self.profile_axes()[i]] + self.profile_sign(i) * value
    }

    /// The world point of the profile point `(u, v)` at depth `w` along the axis, for a part whose
    /// bounding box's lowest corner is at `at`.
    #[must_use]
    pub fn world(self, at: Vec3, u: f64, v: f64, w: f64) -> Vec3 {
        let d = self.unrotate([u, v, w]);
        [at[0] + d[0], at[1] + d[1], at[2] + d[2]]
    }

    /// The sketch coordinates of the profile point `p` on the plane, for the same `at`: the world
    /// point's components along the plane's axes.
    #[must_use]
    pub fn sketch_point(self, at: Vec3, p: [f64; 2]) -> [f64; 2] {
        let w = self.world(at, p[0], p[1], 0.0);
        let axes = self.profile_axes();
        [self.profile_sign(0) * w[axes[0]], self.profile_sign(1) * w[axes[1]]]
    }

    /// The `move` feature's translation that takes the sketched body to `at`: the corner's world
    /// coordinates, with the axis component as the `extrude_0` expression.
    #[must_use]
    pub fn translate(self, at: Vec3) -> [String; 3] {
        let mut out = [const { String::new() }; 3];
        for (i, slot) in out.iter_mut().enumerate() {
            *slot = if i == self.index() { "extrude_0".into() } else { crate::design::fmt_number(at[i]) };
        }
        out
    }
}

/// A proper rotation (determinant +1) taking the file's frame to a recognition frame in which a
/// prism's axis is +Z: the orthonormal, right-handed rows `u`, `v`, `w` (the axis). Rotating a
/// mesh with it keeps volumes, areas and loop windings unchanged, so the Z pipeline runs as is.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    u: Vec3,
    v: Vec3,
    w: Vec3,
}

impl Frame {
    /// The canonical frame whose axis is the unit vector `w`: `u` is built from the world axis
    /// least parallel to `w`, so the same `w` always gives the same basis.
    #[must_use]
    #[allow(clippy::many_single_char_names)] // the basis vectors u, v, w and helper h
    pub fn from_axis(w: Vec3) -> Self {
        let i = (0..3).min_by(|&a, &b| w[a].abs().total_cmp(&w[b].abs())).unwrap_or(0);
        let mut h = [0.0; 3];
        h[i] = 1.0;
        let u = cross(w, h);
        let u = [u[0] / length(u), u[1] / length(u), u[2] / length(u)];
        let v = cross(w, u);
        Self { u, v, w }
    }

    /// A world point in the recognition frame.
    #[must_use]
    pub fn rotate(&self, p: Vec3) -> Vec3 {
        [dot(p, self.u), dot(p, self.v), dot(p, self.w)]
    }

    /// A recognition-frame point back in world coordinates.
    #[must_use]
    pub fn unrotate(&self, p: Vec3) -> Vec3 {
        [
            self.u[0] * p[0] + self.v[0] * p[1] + self.w[0] * p[2],
            self.u[1] * p[0] + self.v[1] * p[1] + self.w[1] * p[2],
            self.u[2] * p[0] + self.v[2] * p[1] + self.w[2] * p[2],
        ]
    }

    /// The mesh with every corner rotated into the recognition frame.
    #[must_use]
    pub fn rotate_mesh(&self, mesh: &Mesh) -> Mesh {
        Mesh::new(mesh.triangles.iter().map(|t| t.map(|p| self.rotate(p))).collect())
    }
}

/// A facet's plane sample: unit normal, twice its area, one corner.
struct Facet {
    n: Vec3,
    area2: f64,
    p: Vec3,
}

/// Facet normals within this angle count as parallel to a candidate plane (5°).
const PLANE_PARALLEL_COS: f64 = 0.996_194_698_091_745_5; // cos(5°)

/// An axis within this angle of a world axis is left to the X/Y/Z attempts (2°).
const WORLD_AXIS_COS: f64 = 0.999_390_827_019_095_7; // cos(2°)

/// The facets of `mesh` as plane samples, degenerate ones dropped.
fn facets_of(mesh: &Mesh) -> Vec<Facet> {
    mesh.triangles
        .iter()
        .filter_map(|t| {
            let n = cross(sub(t[1], t[0]), sub(t[2], t[0]));
            let len = length(n);
            (len > 0.0).then(|| Facet { n: [n[0] / len, n[1] / len, n[2] / len], area2: len, p: t[0] })
        })
        .collect()
}

/// Whether facet `j` lies on the plane `(n, d)`: its normal parallel and all its corners within
/// `tol` of it.
fn on_plane(facets: &[Facet], triangles: &[crate::mesh::Triangle], j: usize, n: Vec3, d: f64, tol: f64) -> bool {
    let g = &facets[j];
    dot(g.n, n).abs() >= PLANE_PARALLEL_COS && triangles[j].iter().all(|p| (dot(*p, n) - d).abs() <= tol)
}

/// The dominant plane of `facets` (sampled over the strided `pool`) by fixed-seed RANSAC
/// (Schnabel, Wahl, Klein 2007, as `ransac_circle` in `plate/geometry.rs` does for circles):
/// one facet per sample is the 3-point minimal sample (its own plane), inliers are the pool's
/// facets on the plane within `tol`, scored by area, the iteration count adapted to the inlier
/// ratio. Returns the refined unit normal and offset.
#[allow(clippy::many_single_char_names)] // points and normals n, p, the areas a, as in the geometry
fn ransac_plane(facets: &[Facet], triangles: &[crate::mesh::Triangle], pool: &[usize], tol: f64) -> Option<(Vec3, f64)> {
    let mut rng = Rng::new(0xA51A_6E01);
    let mut best: Option<(f64, Vec3, f64)> = None; // inlier area, unit normal, offset
    let mut w = 0.5_f64; // the inlier ratio estimate, adapted as samples come in
    let mut iter = 0_usize;
    loop {
        let n_max = ((1.0 - 0.99_f64).ln() / (1.0 - w).ln()).ceil().clamp(3.0, 512.0);
        if iter as f64 >= n_max {
            break;
        }
        iter += 1;
        let f = &facets[pool[rng.below(pool.len())]];
        let (n, d) = (f.n, dot(f.p, f.n));
        let mut area = 0.0;
        let mut count = 0_usize;
        for &j in pool {
            if on_plane(facets, triangles, j, n, d, tol) {
                area += facets[j].area2;
                count += 1;
            }
        }
        // A plane needs more than the sampled facet behind it; a quad cap is two facets.
        if count >= 2 && best.as_ref().is_none_or(|(a, _, _)| area > *a) {
            best = Some((area, n, d));
            w = count as f64 / pool.len() as f64;
        }
    }
    let (_, n, d) = best?;
    // Refine on the pool's inliers: the area-weighted mean normal and offset.
    let mut sum = [0.0; 3];
    let mut d_sum = 0.0;
    let mut area = 0.0;
    for &j in pool {
        if on_plane(facets, triangles, j, n, d, tol) {
            for k in 0..3 {
                sum[k] += facets[j].n[k] * facets[j].area2;
            }
            d_sum += dot(facets[j].p, n) * facets[j].area2;
            area += facets[j].area2;
        }
    }
    Some(([sum[0] / length(sum), sum[1] / length(sum), sum[2] / length(sum)], d_sum / area))
}

/// The dominant opposite-facing parallel plane to `(n, d1)`: the anti-parallel facets' offsets
/// along `n`, clustered where they are within `tol`, the dominant cluster that is not coplanar
/// with the first plane. Returns its area (twice) and mean offset.
#[allow(clippy::many_single_char_names)] // offsets o and areas a, as in the geometry
fn opposite_plane(facets: &[Facet], n: Vec3, d1: f64, tol: f64) -> Option<(f64, f64)> {
    let mut opposite: Vec<(f64, f64)> = facets // offset along n, area2
        .iter()
        .filter(|g| dot(g.n, n) <= -PLANE_PARALLEL_COS)
        .map(|g| (dot(g.p, n), g.area2))
        .filter(|(o, _)| (*o - d1).abs() > tol)
        .collect();
    if opposite.is_empty() {
        return None;
    }
    opposite.sort_by(|a, b| a.0.total_cmp(&b.0));
    // Cluster the sorted offsets where they are within `tol` of each other; the dominant
    // cluster's area-weighted mean offset is the second plane.
    let mut best: Option<(f64, f64)> = None; // area, mean offset
    let mut cluster: Vec<(f64, f64)> = Vec::new();
    let flush = |cluster: &mut Vec<(f64, f64)>, best: &mut Option<(f64, f64)>| {
        if !cluster.is_empty() {
            let area: f64 = cluster.iter().map(|c| c.1).sum();
            let mean = cluster.iter().map(|c| c.0 * c.1).sum::<f64>() / area;
            if best.as_ref().is_none_or(|(a, _)| area > *a) {
                *best = Some((area, mean));
            }
        }
        cluster.clear();
    };
    for &(o, a) in &opposite {
        if let Some(last) = cluster.last()
            && (o - last.0).abs() <= tol
        {
            cluster.push((o, a));
        } else {
            flush(&mut cluster, &mut best);
            cluster.push((o, a));
        }
    }
    flush(&mut cluster, &mut best);
    best
}

/// The prism axis of `mesh` found by fixed-seed RANSAC over its facets (Schnabel, Wahl, Klein
/// 2007): a prism's two caps are the two dominant parallel planes, and their normal is the
/// axis. `None` when no such pair exists, when the part does not lie between the two planes, or
/// when the axis is (nearly) a world axis — those parts the X/Y/Z attempts already cover.
#[allow(clippy::many_single_char_names)] // the normal n, offsets d, areas a, as in the geometry
pub(crate) fn prism_axis(mesh: &Mesh, tol: f64) -> Option<Frame> {
    let facets = facets_of(mesh);
    if facets.len() < 3 {
        return None;
    }
    // A deterministic strided sample pool, so the RANSAC cost stays bounded on big meshes.
    let step = (facets.len() / 10_000).max(1);
    let pool: Vec<usize> = (0..facets.len()).step_by(step).collect();
    let (n, d1) = ransac_plane(&facets, &mesh.triangles, &pool, tol)?;
    // The inlier area over all facets, for the pair's significance.
    let area1: f64 =
        facets.iter().enumerate().filter(|(j, _)| on_plane(&facets, &mesh.triangles, *j, n, d1, tol)).map(|(_, g)| g.area2).sum();
    let (area2, d2) = opposite_plane(&facets, n, d1, tol)?;
    // A prism's two caps carry comparable area, the part lies between them, and they are apart.
    if area2 < 0.5 * area1 || (d2 - d1).abs() <= tol {
        return None;
    }
    let (lo, hi) = (d1.min(d2), d1.max(d2));
    if !mesh.triangles.iter().flatten().all(|p| {
        let o = dot(*p, n);
        o >= lo - tol && o <= hi + tol
    }) {
        return None;
    }
    // A canonical sign, as `unit_dir` fixes one: the largest-magnitude component positive.
    let i = (0..3).max_by(|&a, &b| n[a].abs().total_cmp(&n[b].abs())).unwrap_or(0);
    let n = if n[i] < 0.0 { [-n[0], -n[1], -n[2]] } else { n };
    // A world axis is the X/Y/Z attempts' job, already tried.
    for e in [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]] {
        if dot(n, e).abs() >= WORLD_AXIS_COS {
            return None;
        }
    }
    Some(Frame::from_axis(n))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotations_are_proper_and_inverse() {
        let p = [3.0, -5.0, 7.0];
        for axis in Axis::ALL {
            assert_eq!(axis.unrotate(axis.rotate(p)), p, "{axis:?}");
            // A proper rotation keeps the cross product: rotate(x) x rotate(y) = rotate(x x y).
            let (x, y) = ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
            let cross = |a: Vec3, b: Vec3| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
            assert_eq!(cross(axis.rotate(x), axis.rotate(y)), axis.rotate(cross(x, y)), "{axis:?}");
            // The axis itself becomes +Z.
            assert_eq!(axis.rotate(axis.dir()), [0.0, 0.0, 1.0], "{axis:?}");
        }
    }

    #[test]
    fn profile_coordinates_match_the_planes() {
        let at = [10.0, 20.0, 30.0];
        // Z: profile (u, v) is world (x, y); the XY plane sketch takes them as they are.
        assert_eq!(Axis::Z.world(at, 1.0, 2.0, 3.0), [11.0, 22.0, 33.0]);
        assert_eq!(Axis::Z.sketch_point(at, [1.0, 2.0]), [11.0, 22.0]);
        // X: profile (u, v) is world (y, z); the YZ plane sketch takes them as they are.
        assert_eq!(Axis::X.world(at, 1.0, 2.0, 3.0), [13.0, 21.0, 32.0]);
        assert_eq!(Axis::X.sketch_point(at, [1.0, 2.0]), [21.0, 32.0]);
        // Y: profile (u, v) is world (x, -z); the XZ plane's sketch y is -Z.
        assert_eq!(Axis::Y.world(at, 1.0, 2.0, 3.0), [11.0, 23.0, 28.0]);
        assert_eq!(Axis::Y.sketch_point(at, [1.0, 2.0]), [11.0, -28.0]);
    }

    #[test]
    #[allow(clippy::many_single_char_names)] // the axis w and point p
    fn frames_are_proper_rotations_taking_the_axis_to_z() {
        for w in [[0.3, 0.5, 0.81], [-0.6, 0.2, 0.77], [0.0, 1.0, 0.0]] {
            let len = length(w);
            let w = [w[0] / len, w[1] / len, w[2] / len];
            let frame = Frame::from_axis(w);
            let z = frame.rotate(w);
            assert!(z[0].abs() < 1e-12 && z[1].abs() < 1e-12 && (z[2] - 1.0).abs() < 1e-12, "{w:?} {z:?}");
            let p = [3.0, -5.0, 7.0];
            let q = frame.unrotate(frame.rotate(p));
            assert!((q[0] - p[0]).abs() < 1e-12 && (q[1] - p[1]).abs() < 1e-12 && (q[2] - p[2]).abs() < 1e-12, "{w:?} {q:?}");
            // A proper rotation keeps the cross product: rotate(x) x rotate(y) = rotate(x x y).
            let (x, y) = ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
            let cr = cross(frame.rotate(x), frame.rotate(y));
            let want = frame.rotate(cross(x, y));
            assert!((cr[0] - want[0]).abs() < 1e-12 && (cr[1] - want[1]).abs() < 1e-12 && (cr[2] - want[2]).abs() < 1e-12);
            // The same axis gives the same basis, reproducibly.
            assert_eq!(Frame::from_axis(w), frame);
        }
    }

    #[test]
    #[allow(clippy::many_single_char_names)] // the axis k, angle c/s, point p
    fn prism_axis_needs_two_parallel_planes() {
        // A tetrahedron has no pair of parallel faces: no axis.
        let tetra = crate::mesh::tests::tetrahedron(10.0);
        assert!(prism_axis(&tetra, 0.01).is_none());
        // An axis-aligned box's caps are horizontal: a world axis, left to the Z attempt.
        let b = crate::recognize::tests::block(30.0, 20.0, 5.0, [0.0; 3]);
        assert!(prism_axis(&b, 0.01).is_none());
        // The same box turned about (1, 1, 1): the caps give the axis.
        let len = 3.0_f64.sqrt();
        let (s, c) = (0.9_f64.sin(), 0.9_f64.cos());
        let k = [1.0 / len, 1.0 / len, 1.0 / len];
        let rot = |p: Vec3| {
            let d = dot(p, k);
            let cr = cross(k, p);
            [
                p[0] * c + cr[0] * s + k[0] * d * (1.0 - c),
                p[1] * c + cr[1] * s + k[1] * d * (1.0 - c),
                p[2] * c + cr[2] * s + k[2] * d * (1.0 - c),
            ]
        };
        let turned = Mesh::new(b.triangles.iter().map(|t| t.map(rot)).collect());
        let frame = prism_axis(&turned, 0.01).expect("a slanted box has an axis");
        // The cap normal (Z after the turn) becomes +Z, not the rotation axis itself.
        let z = frame.rotate(rot([0.0, 0.0, 1.0]));
        assert!(z[0].abs() < 1e-6 && z[1].abs() < 1e-6 && (z[2] - 1.0).abs() < 1e-6, "{z:?}");
    }

    #[test]
    fn parameter_letters_follow_the_world_axes() {
        assert_eq!([Axis::Z.profile_letter(0), Axis::Z.profile_letter(1)], ['x', 'y']);
        assert_eq!([Axis::X.profile_letter(0), Axis::X.profile_letter(1)], ['y', 'z']);
        assert_eq!([Axis::Y.profile_letter(0), Axis::Y.profile_letter(1)], ['x', 'z']);
        let at = [10.0, 20.0, 30.0];
        assert_eq!(Axis::X.profile_world(at, 1, 2.0), 32.0);
        assert_eq!(Axis::Y.profile_world(at, 1, 2.0), 28.0);
    }
}
