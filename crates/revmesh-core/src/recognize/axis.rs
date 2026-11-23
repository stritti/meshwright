//! The axis a part is extruded along, and the rotations that align it with Z.
//!
//! mesh2cad tries a prism along X, Y and Z and takes whichever fits. revmesh has one pipeline,
//! written for a Z extrusion, so it rotates the mesh into a frame where the part's axis is +Z,
//! recognises there, and rotates the result back. Both rotations are proper (determinant +1), so
//! volumes, areas and loop windings survive unchanged.
//!
//! SolidCraft's origin planes fix the sketch side of each frame (solidcore's `Plane`: `YZ` has
//! sketch axes (Y, Z) and normal +X, `XZ` has (X, −Z) and normal +Y, `XY` has (X, Y) and normal
//! +Z), so the frame's profile coordinates `(u, v)` are exactly the sketch coordinates on the
//! matching plane, and `Extent::positive` extrudes along the axis. mesh2cad stores the outline
//! the same way: X extrusions as (y, z), Y as (x, z), Z as (x, y).

use crate::mesh::{Mesh, Vec3};

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
    fn parameter_letters_follow_the_world_axes() {
        assert_eq!([Axis::Z.profile_letter(0), Axis::Z.profile_letter(1)], ['x', 'y']);
        assert_eq!([Axis::X.profile_letter(0), Axis::X.profile_letter(1)], ['y', 'z']);
        assert_eq!([Axis::Y.profile_letter(0), Axis::Y.profile_letter(1)], ['x', 'z']);
        let at = [10.0, 20.0, 30.0];
        assert_eq!(Axis::X.profile_world(at, 1, 2.0), 32.0);
        assert_eq!(Axis::Y.profile_world(at, 1, 2.0), 28.0);
    }
}
