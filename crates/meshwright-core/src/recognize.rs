//! Feature recognition: finding the parametric part a mesh was made from.
//!
//! This is where mesh2cad's recognition is ported, step by step. The first step recognises only
//! an axis-aligned block standing on the origin. Plates with holes, slots and fillets follow; the
//! design they produce is already modelled by [`PlateSpec`], so the output side is fixed before
//! the recognisers are written.

use crate::design::{
    Design, DesignBuilder, Extent, FeatureKind, HoleKind, Operation, Param, Plane, Profiles, fmt_number, rectangle_sketch,
};
use crate::mesh::{Mesh, cross, length, sub};

/// Relative tolerance for comparing measured and expected sizes.
pub const TOLERANCE: f64 = 1e-6;

/// What recognition found.
#[derive(Debug, Clone, PartialEq)]
pub enum Recognition {
    /// A rectangular plate, possibly with holes and rounded corners.
    Plate(PlateSpec),
    /// Nothing the recognisers know; the mesh is kept as it is.
    Unrecognized {
        /// Why, in the user's words.
        reason: String,
    },
}

/// A rectangular plate from the origin, extruded up, with vertical through holes.
#[derive(Debug, Clone, PartialEq)]
pub struct PlateSpec {
    /// Size along X.
    pub x: f64,
    /// Size along Y.
    pub y: f64,
    /// Thickness along Z.
    pub z: f64,
    /// Through holes.
    pub holes: Vec<HoleSpec>,
    /// The radius of the four vertical corner edges, if they are rounded.
    pub outline_fillet_r: Option<f64>,
}

/// A vertical through hole.
#[derive(Debug, Clone, PartialEq)]
pub struct HoleSpec {
    /// Centre X.
    pub x: f64,
    /// Centre Y.
    pub y: f64,
    /// Diameter.
    pub d: f64,
}

impl PlateSpec {
    /// The plate as a parametric history: parameters `base_x`, `base_y`, `base_z`,
    /// `hole_<n>_x/_y/_d` and `outline_fillet_r` (the names mesh2cad gives them), a sketch,
    /// an extrusion, a hole per hole and a fillet of the corners.
    #[must_use]
    pub fn to_design(&self, name: &str) -> Design {
        let mut b = DesignBuilder::new(name);
        b.param(Param::user("base_x", self.x)).param(Param::user("base_y", self.y)).param(Param::user("base_z", self.z));
        for (i, hole) in self.holes.iter().enumerate() {
            let n = i + 1;
            b.param(Param::user(format!("hole_{n}_x"), hole.x))
                .param(Param::user(format!("hole_{n}_y"), hole.y))
                .param(Param::user(format!("hole_{n}_d"), hole.d));
        }
        if let Some(r) = self.outline_fillet_r {
            b.param(Param::user("outline_fillet_r", r));
        }
        b.param(Param::model("d1", "base_x")).param(Param::model("d2", "base_y"));

        let sketch = b.feature(
            "Sketch1",
            vec![],
            FeatureKind::Sketch { plane: Plane::Origin { name: "XY" }, sketch: rectangle_sketch(self.x, self.y) },
        );
        let extrude = b.feature(
            "Extrude1",
            vec!["Body1".into()],
            FeatureKind::Extrude { sketch, profiles: Profiles::All, extent: Extent::positive("base_z"), operation: Operation::NewBody },
        );
        for (i, hole) in self.holes.iter().enumerate() {
            let n = i + 1;
            b.feature(
                format!("Hole{n}"),
                vec![],
                FeatureKind::Hole {
                    position: [hole.x, hole.y, self.z],
                    at: [format!("hole_{n}_x"), format!("hole_{n}_y"), "base_z".into()],
                    direction: [0.0, 0.0, -1.0],
                    diameter: format!("hole_{n}_d"),
                    hole: HoleKind::Simple,
                },
            );
        }
        if self.outline_fillet_r.is_some() {
            let mid = self.z / 2.0;
            let side = |a: u32, b: u32| format!("F{extrude}:side:l{a}|F{extrude}:side:l{b}");
            b.feature(
                "Fillet1",
                vec![],
                FeatureKind::Fillet {
                    edge_names: vec![side(1, 4), side(1, 2), side(2, 3), side(3, 4)],
                    edges: vec![[0.0, 0.0, mid], [self.x, 0.0, mid], [self.x, self.y, mid], [0.0, self.y, mid]],
                    radius: "outline_fillet_r".into(),
                },
            );
        }
        b.build()
    }

    /// One line describing the plate, for the loss report.
    #[must_use]
    pub fn describe(&self) -> String {
        let plate = format!("A {} x {} x {} mm plate", fmt_number(self.x), fmt_number(self.y), fmt_number(self.z));
        match self.holes.len() {
            0 => plate,
            n => format!("{plate} with {n} hole(s)"),
        }
    }
}

/// Looks for a part the recognisers know in `mesh`.
#[must_use]
pub fn recognize(mesh: &Mesh) -> Recognition {
    let Some(bounds) = mesh.bounds() else {
        return Recognition::Unrecognized { reason: "the mesh has no triangles".into() };
    };
    let [x, y, z] = bounds.size();
    let scale = x.max(y).max(z);
    let on_origin = bounds.min.iter().all(|c| c.abs() <= TOLERANCE * scale);
    let box_volume = x * y * z;
    let axis_aligned = mesh.triangles.iter().all(|[a, b, c]| {
        let n = cross(sub(*b, *a), sub(*c, *a));
        let len = length(n);
        len == 0.0 || n.iter().any(|component| (component.abs() / len - 1.0).abs() <= TOLERANCE)
    });
    if axis_aligned && box_volume > 0.0 && (mesh.volume() - box_volume).abs() <= TOLERANCE * box_volume {
        if on_origin {
            return Recognition::Plate(PlateSpec { x, y, z, holes: Vec::new(), outline_fillet_r: None });
        }
        return Recognition::Unrecognized { reason: "a block that does not stand on the origin (not supported yet)".into() };
    }
    Recognition::Unrecognized { reason: "no feature recogniser matched (only plain blocks are recognised so far)".into() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::tests::tetrahedron;

    /// A closed, outward-facing box from the origin to `(x, y, z)`, offset by `at`.
    pub(crate) fn block(x: f64, y: f64, z: f64, at: [f64; 3]) -> Mesh {
        let p = |i: f64, j: f64, k: f64| [at[0] + i * x, at[1] + j * y, at[2] + k * z];
        let quad = |a, b, c, d| [[a, b, c], [a, c, d]];
        let faces = [
            quad(p(0., 0., 0.), p(0., 1., 0.), p(1., 1., 0.), p(1., 0., 0.)),
            quad(p(0., 0., 1.), p(1., 0., 1.), p(1., 1., 1.), p(0., 1., 1.)),
            quad(p(0., 0., 0.), p(1., 0., 0.), p(1., 0., 1.), p(0., 0., 1.)),
            quad(p(0., 1., 0.), p(0., 1., 1.), p(1., 1., 1.), p(1., 1., 0.)),
            quad(p(0., 0., 0.), p(0., 0., 1.), p(0., 1., 1.), p(0., 1., 0.)),
            quad(p(1., 0., 0.), p(1., 1., 0.), p(1., 1., 1.), p(1., 0., 1.)),
        ];
        Mesh::new(faces.into_iter().flatten().collect())
    }

    #[test]
    fn block_volume_is_positive() {
        assert!((block(2.0, 3.0, 4.0, [0.0; 3]).volume() - 24.0).abs() < 1e-9);
    }

    #[test]
    fn recognises_block_on_origin() {
        let found = recognize(&block(60.0, 40.0, 6.0, [0.0; 3]));
        assert_eq!(found, Recognition::Plate(PlateSpec { x: 60.0, y: 40.0, z: 6.0, holes: vec![], outline_fillet_r: None }));
    }

    #[test]
    fn leaves_offset_block_alone() {
        assert!(matches!(recognize(&block(1.0, 1.0, 1.0, [5.0, 0.0, 0.0])), Recognition::Unrecognized { .. }));
    }

    #[test]
    fn leaves_tetrahedron_alone() {
        assert!(matches!(recognize(&tetrahedron(10.0)), Recognition::Unrecognized { .. }));
    }

    /// The design the spike opened in SolidCraft 0.45: a plate with a centred hole and rounded
    /// corners. The golden file was opened in SolidCraft and rebuilt without errors or warnings
    /// after changing `base_x` and `hole_1_d`.
    #[test]
    fn plate_design_matches_golden_file() {
        let spec = PlateSpec { x: 70.0, y: 40.0, z: 6.0, holes: vec![HoleSpec { x: 35.0, y: 20.0, d: 8.0 }], outline_fillet_r: Some(3.0) };
        let actual = spec.to_design("plate-with-hole").to_json_pretty();
        let golden = include_str!("../tests/data/plate-with-hole.json");
        assert_eq!(actual.trim(), golden.trim(), "regenerate with `cargo run -p meshwright-core --example plate_design`");
    }
}
