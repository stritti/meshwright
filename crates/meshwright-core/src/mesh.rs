//! Triangle meshes and the measurements recognition is built on.

/// A point or vector in model space, in millimetres.
pub type Vec3 = [f64; 3];

/// One triangle, its corners in counter-clockwise order seen from outside.
pub type Triangle = [Vec3; 3];

/// An axis-aligned bounding box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    /// The smallest corner.
    pub min: Vec3,
    /// The largest corner.
    pub max: Vec3,
}

impl Aabb {
    /// The box's size along each axis.
    #[must_use]
    pub fn size(&self) -> Vec3 {
        [self.max[0] - self.min[0], self.max[1] - self.min[1], self.max[2] - self.min[2]]
    }
}

/// A triangle soup, as STL stores it: every triangle carries its own corners.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Mesh {
    /// The triangles.
    pub triangles: Vec<Triangle>,
}

impl Mesh {
    /// A mesh of these triangles.
    #[must_use]
    pub fn new(triangles: Vec<Triangle>) -> Self {
        Self { triangles }
    }

    /// Whether the mesh has no triangles.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.triangles.is_empty()
    }

    /// The bounding box, or `None` for an empty mesh.
    #[must_use]
    pub fn bounds(&self) -> Option<Aabb> {
        let mut corners = self.triangles.iter().flatten();
        let first = *corners.next()?;
        let mut aabb = Aabb { min: first, max: first };
        for p in corners {
            for ((min, max), value) in aabb.min.iter_mut().zip(aabb.max.iter_mut()).zip(p) {
                *min = min.min(*value);
                *max = max.max(*value);
            }
        }
        Some(aabb)
    }

    /// The enclosed volume in mm³ (divergence theorem). Exact for a closed, consistently oriented
    /// mesh; meaningless otherwise.
    #[must_use]
    pub fn volume(&self) -> f64 {
        self.triangles.iter().map(|[a, b, c]| dot(*a, cross(*b, *c))).sum::<f64>() / 6.0
    }

    /// The surface area in mm².
    #[must_use]
    pub fn area(&self) -> f64 {
        self.triangles.iter().map(|[a, b, c]| length(cross(sub(*b, *a), sub(*c, *a))) / 2.0).sum()
    }
}

/// `a - b`.
#[must_use]
pub fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// The dot product.
#[must_use]
pub fn dot(a: Vec3, b: Vec3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// The cross product.
#[must_use]
pub fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

/// The Euclidean length.
#[must_use]
pub fn length(a: Vec3) -> f64 {
    dot(a, a).sqrt()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A closed tetrahedron with legs of length `s` along the axes, outward-facing.
    #[allow(clippy::many_single_char_names)]
    pub(crate) fn tetrahedron(s: f64) -> Mesh {
        let o = [0.0, 0.0, 0.0];
        let x = [s, 0.0, 0.0];
        let y = [0.0, s, 0.0];
        let z = [0.0, 0.0, s];
        Mesh::new(vec![[o, y, x], [o, x, z], [o, z, y], [x, y, z]])
    }

    #[test]
    fn tetrahedron_volume_and_bounds() {
        let mesh = tetrahedron(10.0);
        assert!((mesh.volume() - 1000.0 / 6.0).abs() < 1e-9);
        let bounds = mesh.bounds().expect("non-empty");
        assert_eq!(bounds.size(), [10.0, 10.0, 10.0]);
    }

    #[test]
    fn empty_mesh_has_no_bounds() {
        assert!(Mesh::default().bounds().is_none());
        assert!(Mesh::default().volume().abs() < f64::EPSILON);
    }
}
