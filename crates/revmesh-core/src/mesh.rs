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
        let Some([origin, ..]) = self.triangles.first() else { return 0.0 };
        self.triangles.iter().map(|[a, b, c]| dot(sub(*a, *origin), cross(sub(*b, *origin), sub(*c, *origin)))).sum::<f64>() / 6.0
    }

    /// Multiplies every coordinate by `factor` in place (an inch STL read as millimetres).
    pub fn scale_in_place(&mut self, factor: f64) {
        for triangle in &mut self.triangles {
            for point in triangle {
                for value in point {
                    *value *= factor;
                }
            }
        }
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

/// Rounding steps per millimetre: recognised sizes are rounded to micrometres, because STL stores
/// single-precision floats, so a 35 mm hole position comes back as 35.0000019…, which nobody wants
/// in the parameter panel.
const SNAP_PER_MM: f64 = 1000.0;

/// A coordinate rounded to micrometres, as recognised sizes are: a sum of an offset and a size
/// must not show float noise in the parameter panel. Negative zero becomes `0.0`.
#[must_use]
pub fn snap_mm(value: f64) -> f64 {
    let snapped = (value * SNAP_PER_MM).round() / SNAP_PER_MM;
    if snapped == 0.0 { 0.0 } else { snapped }
}

/// A sketch dimension rounded to 0.1 µm: coarse enough to hide single-precision STL noise
/// (35.0000019 becomes 35), fine enough to keep thin profiles valid (a 0.1 mm high triangle's
/// slanted sides stay 50.0001 mm, not 50, which would collapse the triangle). Negative zero
/// becomes `0.0`.
#[must_use]
pub fn snap_dim(value: f64) -> f64 {
    let snapped = (value * 1e4).round() / 1e4;
    if snapped == 0.0 { 0.0 } else { snapped }
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
