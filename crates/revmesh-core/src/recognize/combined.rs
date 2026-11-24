//! A part that is the intersection of outlines extruded along X, Y and Z, and its design.

use super::axis::Axis;

use crate::Locale;
use crate::design::{
    Design, DesignBuilder, Extent, FeatureKind, Operation, Param, Plane, Profiles, fmt_number, polygon_sketch, polygons_sketch,
};
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
    /// Openings of straight sides right through it, each its corners counter-clockwise, in the
    /// same coordinates.
    pub cutouts: Vec<Vec<[f64; 2]>>,
}

impl Extrusion {
    /// The outline and its openings as rings with the material on their left: the outline
    /// counter-clockwise, the openings clockwise.
    #[must_use]
    pub fn rings(&self) -> Vec<Vec<[f64; 2]>> {
        let mut rings = vec![self.outline.clone()];
        rings.extend(self.cutouts.iter().map(|c| c.iter().rev().copied().collect()));
        rings
    }
}

/// A straight blind cut: `outline` extruded along `axis` from `from` to `to` (along the axis, the
/// part's bounding box's lowest corner at the origin), taken out of the part. An end at the box's
/// face (0, or the box's size) is where it opens.
#[derive(Debug, Clone, PartialEq)]
pub struct Cut {
    /// The axis it is extruded along.
    pub axis: Axis,
    /// Its corners, counter-clockwise, in the axis's profile coordinates, as an
    /// [`Extrusion`]'s.
    pub outline: Vec<[f64; 2]>,
    /// Where it starts along the axis.
    pub from: f64,
    /// Where it ends along the axis.
    pub to: f64,
}

/// The intersection of two or three outlines (or one, with cuts), each extruded along its own
/// axis across the part's bounding box, less straight blind cuts; an axis whose outline would be
/// the box's side has none.
#[derive(Debug, Clone, PartialEq)]
pub struct CombinedSpec {
    /// The bounding box's size along X, Y and Z.
    pub size: [f64; 3],
    /// The outlines, at most one per axis, in the order Z, X, Y.
    pub extrusions: Vec<Extrusion>,
    /// The cuts, none overlapping another.
    pub cuts: Vec<Cut>,
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
        let outlines = |cut: Option<&Cut>| {
            let rings = |axis: Axis| match cut {
                // Along the cut's axis its own outline (it lies within the part's there).
                Some(c) if c.axis == axis => Some(vec![c.outline.clone()]),
                _ => self.extrusions.iter().find(|e| e.axis == axis).map(Extrusion::rings),
            };
            // Along Y the profile's second coordinate is −Z; the volume takes the outline in (x, z).
            let front: Option<Vec<Vec<[f64; 2]>>> =
                rings(Axis::Y).map(|rs| rs.iter().map(|r| r.iter().map(|p| [p[0], -p[1]]).collect()).collect());
            (rings(Axis::Z), rings(Axis::X), front)
        };
        let (z, x, y) = outlines(None);
        let whole = crate::combined::volume::intersection_volume(self.size, z.as_deref(), x.as_deref(), y.as_deref());
        let cuts: f64 = self
            .cuts
            .iter()
            .map(|c| {
                let (z, x, y) = outlines(Some(c));
                let (mut lo, mut hi) = ([0.0; 3], self.size);
                (lo[c.axis.index()], hi[c.axis.index()]) = (c.from, c.to);
                crate::combined::volume::intersection_volume_in(lo, hi, z.as_deref(), x.as_deref(), y.as_deref())
            })
            .sum();
        whole - cuts
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
        let part = locale.format("summary.combined", &[("x", &x), ("y", &y), ("z", &z), ("axes", &axes)]);
        let count = |n: usize, kind: &str| match n {
            0 => None,
            1 => Some(locale.text(&format!("summary.{kind}.one")).to_owned()),
            n => Some(locale.format(&format!("summary.{kind}.other"), &[("n", &n)])),
        };
        let parts: Vec<String> = [count(self.extrusions.iter().map(|e| e.cutouts.len()).sum(), "cutout"), count(self.cuts.len(), "pocket")]
            .into_iter()
            .flatten()
            .collect();
        let Some((last, rest)) = parts.split_last() else { return part };
        let parts =
            if rest.is_empty() { last.clone() } else { locale.format("summary.list", &[("rest", &rest.join(", ")), ("last", last)]) };
        locale.format("summary.with", &[("plate", &part), ("parts", &parts)])
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
                FeatureKind::Sketch {
                    plane: Plane::Origin { name: e.axis.plane() },
                    // The openings in the outline's sketch, its lines l1..l<n> first.
                    sketch: if e.cutouts.is_empty() {
                        polygon_sketch(&e.outline)
                    } else {
                        polygons_sketch(&[vec![e.outline.clone()], e.cutouts.clone()].concat())
                    },
                },
            );
            b.feature(
                format!("Extrude{}", k + 1),
                if k == 0 { vec!["Body1".into()] } else { vec![] },
                FeatureKind::Extrude {
                    sketch,
                    // Every profile would fill the openings too: the outline's loop is the profile,
                    // and the loops inside it are its holes.
                    profiles: if e.cutouts.is_empty() {
                        Profiles::All
                    } else {
                        Profiles::Curves { loops: vec![(1..=e.outline.len()).map(|i| format!("l{i}")).collect()] }
                    },
                    extent: Extent::positive(format!("extrude_{a}_len")),
                    operation: if k == 0 { Operation::NewBody } else { Operation::Intersect },
                },
            );
        }
        // Where the part starts along each axis: the move's translation.
        let origin = |axis: Axis| {
            if self.outline(axis).is_some() {
                format!("extrude_{}_0", axis.letter().to_ascii_lowercase())
            } else {
                fmt_number(snap_mm(at[axis.index()]))
            }
        };
        // The cuts before the move, in the part's own frame: each starts at its floor (or at the
        // box's face) along its plane's normal, `cut_<n>_0` in the file less where the part starts.
        for (k, c) in self.cuts.iter().enumerate() {
            let (n, i) = (k + 1, c.axis.index());
            b.param(Param::user(format!("cut_{n}_len"), snap_mm(c.to - c.from)))
                .param(Param::user(format!("cut_{n}_0"), snap_mm(at[i] + c.from)));
            let start = match origin(c.axis).as_str() {
                "0" => format!("cut_{n}_0"),
                o if o.starts_with('-') => format!("cut_{n}_0 + {}", &o[1..]),
                o => format!("cut_{n}_0 - {o}"),
            };
            let sketch = b.feature(
                format!("Cut{n}Sketch"),
                vec![],
                FeatureKind::Sketch { plane: Plane::Origin { name: c.axis.plane() }, sketch: polygon_sketch(&c.outline) },
            );
            b.feature(
                format!("Cut{n}"),
                vec![],
                FeatureKind::Extrude {
                    sketch,
                    profiles: Profiles::All,
                    extent: Extent::positive(format!("cut_{n}_len")).starting_at(start),
                    operation: Operation::Cut,
                },
            );
        }
        let translate = [Axis::X, Axis::Y, Axis::Z].map(origin);
        b.feature("Move1", vec![], FeatureKind::Move { bodies: vec!["Body1".into()], translate });
        b.build()
    }
}
