//! Feature recognition: finding the parametric part a mesh was made from.
//!
//! meshwright follows mesh2cad's approach and parameter names, one family of parts at a time.
//! Recognised so far: plates, an outline of straight sides extruded along Z, optionally with one
//! radius on every vertical corner, with round through holes; see [`crate::plate`]. A
//! rectangular outline becomes a [`PlateSpec`], driven by its sizes; any other a [`ProfileSpec`].
//! Slots, counterbores and pockets follow; whatever is not recognised stays a mesh.

use std::fmt::Write as _;

use crate::design::{
    Design, DesignBuilder, Extent, FeatureKind, HoleKind, Operation, Param, Plane, Profiles, fmt_number, polygon_sketch, rectangle_sketch,
};
use crate::mesh::{Mesh, Vec3};

/// What recognition found.
#[derive(Debug, Clone, PartialEq)]
pub enum Recognition {
    /// A rectangular plate, possibly with holes and rounded corners.
    Plate {
        /// The plate, its corner at the origin.
        plate: PlateSpec,
        /// Where the plate's corner was in the file; the design puts it at the origin.
        offset: Vec3,
    },
    /// A plate with any outline of straight sides, possibly with holes and rounded corners.
    Profile {
        /// The plate, its bounding box's corner at the origin.
        profile: ProfileSpec,
        /// Where that corner was in the file; the design puts it at the origin.
        offset: Vec3,
    },
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
    /// The plate as a parametric history, its corner at the origin; see [`PlateSpec::to_design_at`].
    #[must_use]
    pub fn to_design(&self, name: &str) -> Design {
        self.to_design_at(name, [0.0; 3])
    }

    /// The plate as a parametric history with its lowest corner at `at`, as mesh2cad writes it:
    /// parameters `base_x`, `base_y`, `base_t`, the centre `base_cx`, `base_cy` and the bottom
    /// `base_z0`, `hole_<n>_x/_y/_d` (in world coordinates) and `fillet_r`; a sketch and an
    /// extrusion at the origin, a move to where the plate is, a hole per hole and a fillet of the
    /// four vertical corners.
    #[must_use]
    pub fn to_design_at(&self, name: &str, at: Vec3) -> Design {
        let mut b = DesignBuilder::new(name);
        b.param(Param::user("base_x", self.x)).param(Param::user("base_y", self.y)).param(Param::user("base_t", self.z));
        b.param(Param::user("base_cx", snap_mm(at[0] + self.x / 2.0)))
            .param(Param::user("base_cy", snap_mm(at[1] + self.y / 2.0)))
            .param(Param::user("base_z0", snap_mm(at[2])));
        hole_params(&mut b, &self.holes, at);
        if let Some(r) = self.outline_fillet_r {
            b.param(Param::user("fillet_r", r));
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
            FeatureKind::Extrude { sketch, profiles: Profiles::All, extent: Extent::positive("base_t"), operation: Operation::NewBody },
        );
        b.feature(
            "Move1",
            vec![],
            FeatureKind::Move {
                bodies: vec!["Body1".into()],
                translate: ["base_cx - base_x / 2".into(), "base_cy - base_y / 2".into(), "base_z0".into()],
            },
        );
        hole_features(&mut b, &self.holes, at, "base_z0 + base_t", self.z);
        if self.outline_fillet_r.is_some() {
            let mid = at[2] + self.z / 2.0;
            let side = |a: u32, b: u32| format!("F{extrude}:side:l{a}|F{extrude}:side:l{b}");
            let (x0, y0, x1, y1) = (at[0], at[1], at[0] + self.x, at[1] + self.y);
            b.feature(
                "Fillet1",
                vec![],
                FeatureKind::Fillet {
                    edge_names: vec![side(1, 4), side(1, 2), side(2, 3), side(3, 4)],
                    edges: vec![[x0, y0, mid], [x1, y0, mid], [x1, y1, mid], [x0, y1, mid]],
                    radius: "fillet_r".into(),
                },
            );
        }
        b.build()
    }

    /// One line describing the plate, for the loss report.
    #[must_use]
    pub fn describe(&self) -> String {
        describe([self.x, self.y, self.z], None, self.holes.len(), self.outline_fillet_r)
    }

    /// The volume of the plate this spec describes, in mm³: what SolidCraft measures once it has
    /// rebuilt the design.
    #[must_use]
    pub fn volume(&self) -> f64 {
        let corners = self.outline_fillet_r.map_or(0.0, crate::plate::corner_loss);
        let holes: f64 = self.holes.iter().map(|h| std::f64::consts::PI * h.d * h.d / 4.0).sum();
        (self.x * self.y - corners - holes) * self.z
    }
}

/// A plate whose outline is any polygon of straight sides (counter-clockwise, in the first
/// quadrant), extruded up from the XY plane, with vertical through holes.
#[derive(Debug, Clone, PartialEq)]
pub struct ProfileSpec {
    /// The outline's corners as if they were sharp, counter-clockwise.
    pub corners: Vec<[f64; 2]>,
    /// Thickness along Z.
    pub z: f64,
    /// Through holes.
    pub holes: Vec<HoleSpec>,
    /// The radius of every vertical corner edge, inside and outside corners alike, if they are
    /// rounded.
    pub outline_fillet_r: Option<f64>,
}

impl ProfileSpec {
    /// The plate as a parametric history, its bounding box's corner at the origin; see
    /// [`ProfileSpec::to_design_at`].
    #[must_use]
    pub fn to_design(&self, name: &str) -> Design {
        self.to_design_at(name, [0.0; 3])
    }

    /// The plate as a parametric history with its bounding box's lowest corner at `at`, as
    /// mesh2cad writes an extruded outline: parameters `extrude_len` (the thickness),
    /// `extrude_0` (where it starts along Z), `hole_<n>_x/_y/_d` (in world coordinates) and
    /// `outline_fillet_r`; a sketch of the outline (see [`polygon_sketch`]), an extrusion, a move
    /// to where the plate is, a fillet of every vertical corner and a hole per hole. The fillet
    /// comes before the holes: an inside corner's rounding adds material, and a hole may lie in
    /// it.
    #[must_use]
    pub fn to_design_at(&self, name: &str, at: Vec3) -> Design {
        let mut b = DesignBuilder::new(name);
        b.param(Param::user("extrude_len", self.z)).param(Param::user("extrude_0", snap_mm(at[2])));
        hole_params(&mut b, &self.holes, at);
        if let Some(r) = self.outline_fillet_r {
            b.param(Param::user("outline_fillet_r", r));
        }
        let sketch = b.feature(
            "Sketch1",
            vec![],
            FeatureKind::Sketch { plane: Plane::Origin { name: "XY" }, sketch: polygon_sketch(&self.corners) },
        );
        let extrude = b.feature(
            "Extrude1",
            vec!["Body1".into()],
            FeatureKind::Extrude {
                sketch,
                profiles: Profiles::All,
                extent: Extent::positive("extrude_len"),
                operation: Operation::NewBody,
            },
        );
        b.feature(
            "Move1",
            vec![],
            FeatureKind::Move { bodies: vec!["Body1".into()], translate: [fmt_number(at[0]), fmt_number(at[1]), "extrude_0".into()] },
        );
        if self.outline_fillet_r.is_some() {
            let n = self.corners.len();
            // Corner i joins line l<i> (ending there) and line l<i+1> (starting there).
            let edge_names = (0..n)
                .map(|i| {
                    let (a, b) = (if i == 0 { n } else { i }, i + 1);
                    let (a, b) = (a.min(b), a.max(b));
                    format!("F{extrude}:side:l{a}|F{extrude}:side:l{b}")
                })
                .collect();
            let edges = self.corners.iter().map(|c| [at[0] + c[0], at[1] + c[1], at[2] + self.z / 2.0]).collect();
            b.feature("Fillet1", vec![], FeatureKind::Fillet { edge_names, edges, radius: "outline_fillet_r".into() });
        }
        hole_features(&mut b, &self.holes, at, "extrude_0 + extrude_len", self.z);
        b.build()
    }

    /// One line describing the plate, for the loss report.
    #[must_use]
    pub fn describe(&self) -> String {
        let size = self.size();
        describe([size[0], size[1], self.z], Some(self.corners.len()), self.holes.len(), self.outline_fillet_r)
    }

    /// The size of the outline's bounding box along X and Y.
    #[must_use]
    pub fn size(&self) -> [f64; 2] {
        let max = |axis: usize| self.corners.iter().map(|c| c[axis]).fold(0.0, f64::max);
        [max(0), max(1)]
    }

    /// The volume of the plate this spec describes, in mm³.
    #[must_use]
    pub fn volume(&self) -> f64 {
        let n = self.corners.len();
        let mut area = (0..n)
            .map(|i| {
                let (a, b) = (self.corners[i], self.corners[(i + 1) % n]);
                a[0] * b[1] - b[0] * a[1]
            })
            .sum::<f64>()
            / 2.0;
        if let Some(r) = self.outline_fillet_r {
            for i in 0..n {
                let (a, c, b) = (self.corners[(i + n - 1) % n], self.corners[i], self.corners[(i + 1) % n]);
                area -= crate::plate::fillet_loss(a, c, b, r);
            }
        }
        let holes: f64 = self.holes.iter().map(|h| std::f64::consts::PI * h.d * h.d / 4.0).sum();
        (area - holes) * self.z
    }
}

/// Hole positions in world coordinates, for a plate whose corner is at `at`.
fn hole_params(b: &mut DesignBuilder, holes: &[HoleSpec], at: Vec3) {
    for (i, hole) in holes.iter().enumerate() {
        let n = i + 1;
        b.param(Param::user(format!("hole_{n}_x"), snap_mm(at[0] + hole.x)))
            .param(Param::user(format!("hole_{n}_y"), snap_mm(at[1] + hole.y)))
            .param(Param::user(format!("hole_{n}_d"), hole.d));
    }
}

/// A through hole per hole, drilled down from the top face at height `top` (its expression)
/// of a plate `z` thick whose corner is at `at`.
fn hole_features(b: &mut DesignBuilder, holes: &[HoleSpec], at: Vec3, top: &str, z: f64) {
    for (i, hole) in holes.iter().enumerate() {
        let n = i + 1;
        b.feature(
            format!("Hole{n}"),
            vec![],
            FeatureKind::Hole {
                position: [at[0] + hole.x, at[1] + hole.y, at[2] + z],
                at: [format!("hole_{n}_x"), format!("hole_{n}_y"), top.into()],
                direction: [0.0, 0.0, -1.0],
                diameter: format!("hole_{n}_d"),
                hole: HoleKind::Simple,
            },
        );
    }
}

/// A coordinate rounded to micrometres, as recognised sizes are: a sum of an offset and a size
/// must not show float noise in the parameter panel.
fn snap_mm(value: f64) -> f64 {
    let snapped = (value * 1000.0).round() / 1000.0;
    if snapped == 0.0 { 0.0 } else { snapped }
}

/// "A 70 x 40 x 6 mm plate with a 6-sided outline, 2 holes and 3 mm rounded corners".
fn describe(size: [f64; 3], sides: Option<usize>, holes: usize, fillet_r: Option<f64>) -> String {
    let [x, y, z] = size.map(fmt_number);
    let mut parts = Vec::new();
    if let Some(n) = sides {
        parts.push(format!("a {n}-sided outline"));
    }
    match holes {
        0 => {}
        1 => parts.push("1 hole".to_owned()),
        n => parts.push(format!("{n} holes")),
    }
    if let Some(r) = fillet_r {
        parts.push(format!("{} mm rounded corners", fmt_number(r)));
    }
    let mut text = format!("A {x} x {y} x {z} mm plate");
    if let Some((last, rest)) = parts.split_last() {
        text.push_str(" with ");
        if !rest.is_empty() {
            let _ = write!(text, "{} and ", rest.join(", "));
        }
        text.push_str(last);
    }
    text
}

/// Looks for a part the recognisers know in `mesh`.
#[must_use]
pub fn recognize(mesh: &Mesh) -> Recognition {
    crate::plate::recognize_plate(mesh).unwrap_or_else(|reason| Recognition::Unrecognized { reason })
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
        let plate = PlateSpec { x: 60.0, y: 40.0, z: 6.0, holes: vec![], outline_fillet_r: None };
        assert_eq!(found, Recognition::Plate { plate, offset: [0.0; 3] });
    }

    #[test]
    fn moves_offset_block_to_the_origin() {
        let plate = PlateSpec { x: 1.0, y: 1.0, z: 1.0, holes: vec![], outline_fillet_r: None };
        assert_eq!(recognize(&block(1.0, 1.0, 1.0, [5.0, 0.0, 0.0])), Recognition::Plate { plate, offset: [5.0, 0.0, 0.0] });
    }

    #[test]
    fn describes_plates() {
        let mut spec = PlateSpec { x: 70.0, y: 40.0, z: 6.0, holes: vec![], outline_fillet_r: Some(3.0) };
        assert_eq!(spec.describe(), "A 70 x 40 x 6 mm plate with 3 mm rounded corners");
        spec.holes = vec![HoleSpec { x: 1.0, y: 1.0, d: 1.0 }; 2];
        assert_eq!(spec.describe(), "A 70 x 40 x 6 mm plate with 2 holes and 3 mm rounded corners");
    }

    /// The volume SolidCraft measured for the golden design (docs/design-format.md).
    #[test]
    fn plate_volume_matches_solidcraft() {
        let spec = PlateSpec { x: 70.0, y: 40.0, z: 6.0, holes: vec![HoleSpec { x: 35.0, y: 20.0, d: 8.0 }], outline_fillet_r: Some(3.0) };
        assert!((spec.volume() - 16_452.05).abs() < 0.01, "{}", spec.volume());
    }

    #[test]
    fn describes_profiles() {
        let spec = ProfileSpec {
            corners: vec![[0.0, 0.0], [60.0, 0.0], [60.0, 15.0], [15.0, 15.0], [15.0, 40.0], [0.0, 40.0]],
            z: 5.0,
            holes: vec![HoleSpec { x: 7.5, y: 30.0, d: 5.0 }],
            outline_fillet_r: Some(2.0),
        };
        assert_eq!(spec.describe(), "A 60 x 40 x 5 mm plate with a 6-sided outline, 1 hole and 2 mm rounded corners");
        let names = |d: &Design| -> Vec<String> {
            let json: serde_json::Value = serde_json::from_str(&d.to_json()).expect("valid");
            json["features"][3]["edge_names"].as_array().expect("names").iter().filter_map(|n| n.as_str().map(String::from)).collect()
        };
        let mut no_hole = spec.clone();
        no_hole.holes.clear();
        assert_eq!(names(&no_hole.to_design(""))[..2], ["F2:side:l1|F2:side:l6".to_owned(), "F2:side:l1|F2:side:l2".to_owned()]);
    }

    /// An L of 60 x 15 plus 15 x 25, minus a hole, with five convex corners and one concave.
    #[test]
    fn profile_volume_counts_inside_and_outside_corners() {
        let spec = ProfileSpec {
            corners: vec![[0.0, 0.0], [60.0, 0.0], [60.0, 15.0], [15.0, 15.0], [15.0, 40.0], [0.0, 40.0]],
            z: 5.0,
            holes: vec![HoleSpec { x: 7.5, y: 30.0, d: 5.0 }],
            outline_fillet_r: Some(2.0),
        };
        let corner = (4.0 - std::f64::consts::PI) / 4.0 * 4.0;
        let expected = (60.0 * 15.0 + 15.0 * 25.0 - 5.0 * corner + corner - std::f64::consts::PI * 6.25) * 5.0;
        assert!((spec.volume() - expected).abs() < 1e-9, "{} vs {expected}", spec.volume());
    }

    /// A hole in the material an inside-corner rounding adds only exists once the fillet is
    /// there, so the fillet comes first in the history.
    #[test]
    fn profile_fillet_comes_before_the_holes() {
        let spec = ProfileSpec {
            corners: vec![[0.0, 0.0], [60.0, 0.0], [60.0, 15.0], [15.0, 15.0], [15.0, 40.0], [0.0, 40.0]],
            z: 5.0,
            holes: vec![HoleSpec { x: 16.0, y: 16.0, d: 0.4 }],
            outline_fillet_r: Some(5.0),
        };
        let json: serde_json::Value = serde_json::from_str(&spec.to_design("").to_json()).expect("valid");
        let types: Vec<&str> = json["features"].as_array().expect("features").iter().filter_map(|f| f["type"].as_str()).collect();
        assert_eq!(types, ["sketch", "extrude", "move", "fillet", "hole"]);
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
