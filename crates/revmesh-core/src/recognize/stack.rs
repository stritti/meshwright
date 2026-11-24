//! A stack of outlines, each extruded over its own span along one axis, and its design.

use super::axis::Axis;

use crate::Locale;
use crate::design::{
    Design, DesignBuilder, Extent, FeatureKind, Operation, Param, Plane, Profiles, fmt_number, polygon_sketch, polygons_sketch,
};
use crate::mesh::{Vec3, snap_mm};

/// One band of a stack: `outline` (with `cutouts` through it) extruded from `from` to `to` along
/// the stack's axis.
#[derive(Debug, Clone, PartialEq)]
pub struct Band {
    /// Its corners, counter-clockwise, in the axis's profile coordinates (see [`Axis::rotate`])
    /// with the part's bounding box's lowest corner (in that frame) at the origin: the sketch
    /// coordinates on the axis's plane.
    pub outline: Vec<[f64; 2]>,
    /// Openings right through the band, each its corners counter-clockwise.
    pub cutouts: Vec<Vec<[f64; 2]>>,
    /// Where it starts along the axis.
    pub from: f64,
    /// Where it ends along the axis.
    pub to: f64,
}

impl Band {
    /// The outline and its openings as rings with the material on their left: the outline
    /// counter-clockwise, the openings clockwise.
    #[must_use]
    pub fn rings(&self) -> Vec<Vec<[f64; 2]>> {
        let mut rings = vec![self.outline.clone()];
        rings.extend(self.cutouts.iter().map(|c| c.iter().rev().copied().collect()));
        rings
    }

    /// The section's area.
    fn area(&self) -> f64 {
        self.rings()
            .iter()
            .map(|r| (0..r.len()).map(|i| r[i][0] * r[(i + 1) % r.len()][1] - r[(i + 1) % r.len()][0] * r[i][1]).sum::<f64>() / 2.0)
            .sum()
    }
}

/// A union of bands stacked along one axis, the lowest from 0, each the next's foot.
#[derive(Debug, Clone, PartialEq)]
pub struct StackSpec {
    /// The bounding box's size in the axis's frame: across it (u, v) and along it.
    pub size: [f64; 3],
    /// The bands, from the lowest up; two in a row never have the same section.
    pub bands: Vec<Band>,
}

impl StackSpec {
    /// The volume of the part, in mm³: what SolidCraft measures once it has rebuilt the design.
    #[must_use]
    pub fn volume(&self) -> f64 {
        self.bands.iter().map(|b| b.area() * (b.to - b.from)).sum()
    }

    /// One line describing the part, stacked along `axis`, for the loss report.
    #[must_use]
    pub fn describe(&self, axis: Axis) -> String {
        self.describe_in(Locale::En, axis)
    }

    /// [`Self::describe`] in `locale`'s words.
    #[must_use]
    pub fn describe_in(&self, locale: Locale, axis: Axis) -> String {
        let world = axis.unrotate(self.size).map(f64::abs);
        let [x, y, z] = world.map(|v| locale.decimal(&fmt_number(v)));
        let n = self.bands.len();
        locale.format("summary.stack", &[("x", &x), ("y", &y), ("z", &z), ("n", &n), ("axis", &axis.letter())])
    }

    /// The stack as a parametric history with its frame's origin at the world point `at`, along
    /// `axis`, as mesh2cad writes a union of extrusions: per band `k` the parameters
    /// `extrude_<a><k>_len` and `extrude_<a><k>_0` (where it starts; a band above the first
    /// starts where the one below ends, so changing a length moves the bands above), a sketch of
    /// its section on the axis's plane, and an extrusion along the plane's normal from its start,
    /// the first a new body and the next joined to it; then a move to where the part is.
    #[must_use]
    pub fn to_design_on(&self, name: &str, at: Vec3, axis: Axis) -> Design {
        let mut b = DesignBuilder::new(name);
        let a = axis.letter().to_ascii_lowercase();
        for (k, band) in self.bands.iter().enumerate() {
            let n = k + 1;
            b.param(Param::user(format!("extrude_{a}{n}_len"), snap_mm(band.to - band.from)));
            if k == 0 {
                b.param(Param::user(format!("extrude_{a}1_0"), snap_mm(at[axis.index()])));
            } else {
                b.param(Param::user_expr(format!("extrude_{a}{n}_0"), format!("extrude_{a}{k}_0 + extrude_{a}{k}_len")));
            }
        }
        for (k, band) in self.bands.iter().enumerate() {
            let n = k + 1;
            let sketch = b.feature(
                format!("Sketch{n}"),
                vec![],
                FeatureKind::Sketch {
                    plane: Plane::Origin { name: axis.plane() },
                    // The openings in the band's sketch, its outline's lines l1..l<m> first.
                    sketch: if band.cutouts.is_empty() {
                        polygon_sketch(&band.outline)
                    } else {
                        polygons_sketch(&[vec![band.outline.clone()], band.cutouts.clone()].concat())
                    },
                },
            );
            let extent = Extent::positive(format!("extrude_{a}{n}_len"));
            b.feature(
                format!("Extrude{n}"),
                if k == 0 { vec!["Body1".into()] } else { vec![] },
                FeatureKind::Extrude {
                    sketch,
                    // The outline's loop with its openings as holes (every profile would fill them).
                    profiles: if band.cutouts.is_empty() {
                        Profiles::All
                    } else {
                        Profiles::Curves { loops: vec![(1..=band.outline.len()).map(|i| format!("l{i}")).collect()] }
                    },
                    extent: if k == 0 { extent } else { extent.starting_at(format!("extrude_{a}{n}_0 - extrude_{a}1_0")) },
                    operation: if k == 0 { Operation::NewBody } else { Operation::Join },
                },
            );
        }
        let mut translate = axis.translate(at);
        translate[axis.index()] = format!("extrude_{a}1_0");
        b.feature("Move1", vec![], FeatureKind::Move { bodies: vec!["Body1".into()], translate });
        b.build()
    }
}
