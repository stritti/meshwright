//! A part that is the intersection of outlines extruded along X, Y and Z, and its design.

use super::axis::Axis;

use crate::Locale;
use crate::design::{Design, DesignBuilder, Extent, FeatureKind, Operation, Param, Plane, Profiles, fmt_number, polygon_sketch};
use crate::mesh::{Vec3, snap_mm};

/// An outline extruded right across the part along `axis`.
#[derive(Debug, Clone, PartialEq)]
pub struct Extrusion {
    /// The axis it is extruded along.
    pub axis: Axis,
    /// Its corners, counter-clockwise, in the axis's profile coordinates (see [`Axis::rotate`])
    /// with the part's bounding box's lowest corner at the origin: the sketch coordinates on the
    /// axis's plane ([`Axis::plane`]). Along Y the second coordinate is −Z.
    pub outline: Vec<[f64; 2]>,
}

/// The intersection of two or three outlines, each extruded along its own axis across the
/// part's bounding box; an axis whose outline would be the box's side has none.
#[derive(Debug, Clone, PartialEq)]
pub struct CombinedSpec {
    /// The bounding box's size along X, Y and Z.
    pub size: [f64; 3],
    /// The outlines, at most one per axis, in the order Z, X, Y.
    pub extrusions: Vec<Extrusion>,
}

impl CombinedSpec {
    /// The outline along `axis`, if the part has one.
    #[must_use]
    pub fn outline(&self, axis: Axis) -> Option<&[[f64; 2]]> {
        self.extrusions.iter().find(|e| e.axis == axis).map(|e| e.outline.as_slice())
    }

    /// The volume of the part, in mm³: what SolidCraft measures once it has rebuilt the design.
    #[must_use]
    pub fn volume(&self) -> f64 {
        // Along Y the profile's second coordinate is −Z; the volume takes the outline in (x, z).
        let front: Option<Vec<[f64; 2]>> = self.outline(Axis::Y).map(|o| o.iter().map(|p| [p[0], -p[1]]).collect());
        crate::combined::volume::intersection_volume(self.size, self.outline(Axis::Z), self.outline(Axis::X), front.as_deref())
    }

    /// One line describing the part, for the loss report.
    #[must_use]
    pub fn describe(&self) -> String {
        self.describe_in(Locale::En)
    }

    /// [`Self::describe`] in `locale`'s words.
    #[must_use]
    pub fn describe_in(&self, locale: Locale) -> String {
        let [x, y, z] = self.size.map(|v| locale.decimal(&fmt_number(v)));
        let mut axes: Vec<String> = self.extrusions.iter().map(|e| e.axis.letter().to_string()).collect();
        axes.sort();
        let last = axes.pop().unwrap_or_default();
        let axes = if axes.is_empty() { last } else { locale.format("summary.list", &[("rest", &axes.join(", ")), ("last", &last)]) };
        locale.format("summary.combined", &[("x", &x), ("y", &y), ("z", &z), ("axes", &axes)])
    }

    /// The part as a parametric history, its bounding box's corner at the origin; see
    /// [`CombinedSpec::to_design_at`].
    #[must_use]
    pub fn to_design(&self, name: &str) -> Design {
        self.to_design_at(name, [0.0; 3])
    }

    /// The part as a parametric history with its bounding box's lowest corner at `at`, as
    /// mesh2cad writes an intersection of extrusions: parameters `extrude_<a>_len` (the length
    /// along axis `a`) and `extrude_<a>_0` (where it starts) for each outline; per outline a
    /// sketch on the axis's plane and an extrusion along its normal, the first a new body and
    /// each further one intersecting it; and a move to where the part is (its start along each
    /// outline's axis is that outline's `extrude_<a>_0`).
    #[must_use]
    pub fn to_design_at(&self, name: &str, at: Vec3) -> Design {
        let mut b = DesignBuilder::new(name);
        for e in &self.extrusions {
            let (a, i) = (e.axis.letter().to_ascii_lowercase(), e.axis.index());
            b.param(Param::user(format!("extrude_{a}_len"), self.size[i])).param(Param::user(format!("extrude_{a}_0"), snap_mm(at[i])));
        }
        for (k, e) in self.extrusions.iter().enumerate() {
            let a = e.axis.letter().to_ascii_lowercase();
            let sketch = b.feature(
                format!("Sketch{}", k + 1),
                vec![],
                FeatureKind::Sketch { plane: Plane::Origin { name: e.axis.plane() }, sketch: polygon_sketch(&e.outline) },
            );
            b.feature(
                format!("Extrude{}", k + 1),
                if k == 0 { vec!["Body1".into()] } else { vec![] },
                FeatureKind::Extrude {
                    sketch,
                    profiles: Profiles::All,
                    extent: Extent::positive(format!("extrude_{a}_len")),
                    operation: if k == 0 { Operation::NewBody } else { Operation::Intersect },
                },
            );
        }
        let translate = [Axis::X, Axis::Y, Axis::Z].map(|axis| {
            if self.outline(axis).is_some() {
                format!("extrude_{}_0", axis.letter().to_ascii_lowercase())
            } else {
                fmt_number(snap_mm(at[axis.index()]))
            }
        });
        b.feature("Move1", vec![], FeatureKind::Move { bodies: vec!["Body1".into()], translate });
        b.build()
    }
}
