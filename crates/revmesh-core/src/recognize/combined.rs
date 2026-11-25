//! A part that is the intersection of outlines extruded along X, Y and Z, and its design.

use super::axis::Axis;
use super::to_design::dimensioned_polygon_params;

use crate::Locale;
use crate::design::{
    Design, DesignBuilder, Extent, FeatureKind, Operation, Param, Plane, Profiles, fmt_number, polygon_sketch_dimensioned,
    polygons_sketch_dimensioned,
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

/// A round hole along `axis`: a circle of diameter `d` round `centre` (in the axis's profile
/// coordinates, as an [`Extrusion`]'s outline), from `from` to `to` along the axis. A through hole
/// runs from 0 to the box's size; a blind one opens on one of the box's faces.
#[derive(Debug, Clone, PartialEq)]
pub struct CombinedHole {
    /// The axis it is drilled along.
    pub axis: Axis,
    /// Its centre across the axis.
    pub centre: [f64; 2],
    /// Its diameter.
    pub d: f64,
    /// Where it starts along the axis.
    pub from: f64,
    /// Where it ends along the axis.
    pub to: f64,
}

/// Corners of the polygon a circle's volume is taken with: as large as the circle, so where the
/// part is as deep all over the hole the volume is exact.
const CIRCLE_CORNERS: u32 = 256;

impl CombinedHole {
    /// Whether it runs right through the part along its axis.
    #[must_use]
    pub fn through(&self, size: [f64; 3]) -> bool {
        self.from <= 0.0 && self.to >= size[self.axis.index()]
    }

    /// Its circle as a counter-clockwise polygon of the circle's area.
    fn ring(&self) -> Vec<[f64; 2]> {
        let n = f64::from(CIRCLE_CORNERS);
        let step = std::f64::consts::TAU / n;
        let r = self.d / 2.0 * (step / step.sin()).sqrt();
        (0..CIRCLE_CORNERS).map(|i| f64::from(i) * step).map(|t| [self.centre[0] + r * t.cos(), self.centre[1] + r * t.sin()]).collect()
    }
}

/// The intersection of two or three outlines (or one, with cuts or holes), each extruded along its
/// own axis across the part's bounding box, less straight blind cuts and round holes; an axis whose
/// outline would be the box's side has none.
#[derive(Debug, Clone, PartialEq)]
pub struct CombinedSpec {
    /// The bounding box's size along X, Y and Z.
    pub size: [f64; 3],
    /// The outlines, at most one per axis, in the order Z, X, Y.
    pub extrusions: Vec<Extrusion>,
    /// The cuts, none overlapping another.
    pub cuts: Vec<Cut>,
    /// The round holes, by axis and position.
    pub holes: Vec<CombinedHole>,
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
        // Blind holes are taken out as cuts are; through holes are openings in their axis's
        // outline (where holes along two axes cross, the intersection takes the crossing once).
        let blind: Vec<Cut> = self
            .holes
            .iter()
            .filter(|h| !h.through(self.size))
            .map(|h| Cut { axis: h.axis, outline: h.ring(), from: h.from, to: h.to })
            .collect();
        let outlines = |cut: Option<&Cut>| {
            let rings = |axis: Axis| match cut {
                // Along the cut's axis its own outline (it lies within the part's there).
                Some(c) if c.axis == axis => Some(vec![c.outline.clone()]),
                _ => {
                    let mut rings = self.extrusions.iter().find(|e| e.axis == axis).map(Extrusion::rings);
                    for h in self.holes.iter().filter(|h| h.axis == axis && h.through(self.size)) {
                        let rings = rings.get_or_insert_with(|| vec![self.box_side(axis)]);
                        rings.push(h.ring().into_iter().rev().collect());
                    }
                    rings
                }
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
            .chain(&blind)
            .map(|c| {
                let (z, x, y) = outlines(Some(c));
                let (mut lo, mut hi) = ([0.0; 3], self.size);
                (lo[c.axis.index()], hi[c.axis.index()]) = (c.from, c.to);
                crate::combined::volume::intersection_volume_in(lo, hi, z.as_deref(), x.as_deref(), y.as_deref())
            })
            .sum();
        whole - cuts
    }

    /// The box's side across `axis`, counter-clockwise in its profile coordinates.
    fn box_side(&self, axis: Axis) -> Vec<[f64; 2]> {
        let [u, v] = axis.profile_axes();
        let (w, h) = (self.size[u], self.size[v]);
        // Along Y the second coordinate is −Z: the side lies below 0.
        let low = if axis.profile_sign(1) < 0.0 { -h } else { 0.0 };
        vec![[0.0, low], [w, low], [w, low + h], [0.0, low + h]]
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
        let parts: Vec<String> = [
            count(self.holes.len(), "hole"),
            count(self.extrusions.iter().map(|e| e.cutouts.len()).sum(), "cutout"),
            count(self.cuts.len(), "pocket"),
        ]
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
    /// sketch on the axis's plane — dimensioned like a plate's outline, the outline's sides by
    /// `outline_<a>_<i>_len` and `outline_<a>_<i>_angle`, its openings by `cutout_<n>_<i>_len`
    /// and `cutout_<n>_<i>_angle` — and an extrusion along its normal, the first a new body and
    /// each further one intersecting it; and a move to where the part is (its start along each
    /// outline's axis is that outline's `extrude_<a>_0`).
    #[must_use]
    pub fn to_design_at(&self, name: &str, at: Vec3) -> Design {
        let mut b = DesignBuilder::new(name);
        for e in &self.extrusions {
            let (a, i) = (e.axis.letter().to_ascii_lowercase(), e.axis.index());
            b.param(Param::user(format!("extrude_{a}_len"), self.size[i])).param(Param::user(format!("extrude_{a}_0"), snap_mm(at[i])));
        }
        // The openings are numbered across the outlines, in design order.
        let mut cutout_n = 0;
        for (k, e) in self.extrusions.iter().enumerate() {
            let a = e.axis.letter().to_ascii_lowercase();
            // The outline's sketch, dimensioned like a plate's: the outline's driving sides by
            // `outline_<a>_<i>_len` and its longest slanted side by `outline_<a>_<i>_angle`, the
            // openings' by `cutout_<n>_<i>_len`/`_angle`. The openings follow the outline, its
            // lines l1..l<n> first.
            let polygons = [vec![e.outline.clone()], e.cutouts.clone()].concat();
            let mut lens = Vec::with_capacity(polygons.len());
            let mut angles = Vec::with_capacity(polygons.len());
            for (i, poly) in polygons.iter().enumerate() {
                let prefix = if i == 0 {
                    format!("outline_{a}")
                } else {
                    cutout_n += 1;
                    format!("cutout_{cutout_n}")
                };
                let (l, angle) = dimensioned_polygon_params(&mut b, &prefix, poly);
                lens.push(l);
                angles.push(angle);
            }
            let sketch = b.feature(
                format!("Sketch{}", k + 1),
                vec![],
                FeatureKind::Sketch {
                    plane: Plane::Origin { name: e.axis.plane() },
                    sketch: polygons_sketch_dimensioned(&polygons, &lens, &angles),
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
        self.cut_features(&mut b, at);
        self.hole_features(&mut b, at);
        let translate = [Axis::X, Axis::Y, Axis::Z].map(|axis| self.origin(axis, at));
        b.feature("Move1", vec![], FeatureKind::Move { bodies: vec!["Body1".into()], translate });
        b.build()
    }

    /// Where the part starts along `axis`: its outline's `extrude_<a>_0`, or the number where it
    /// has none (the move's translation).
    fn origin(&self, axis: Axis, at: Vec3) -> String {
        if self.outline(axis).is_some() {
            format!("extrude_{}_0", axis.letter().to_ascii_lowercase())
        } else {
            fmt_number(snap_mm(at[axis.index()]))
        }
    }

    /// The cuts before the move, in the part's own frame: each starts at its floor (or at the
    /// box's face) along its plane's normal, `cut_<n>_0` in the file less where the part starts.
    fn cut_features(&self, b: &mut DesignBuilder, at: Vec3) {
        for (k, c) in self.cuts.iter().enumerate() {
            let (n, i) = (k + 1, c.axis.index());
            b.param(Param::user(format!("cut_{n}_len"), snap_mm(c.to - c.from)))
                .param(Param::user(format!("cut_{n}_0"), snap_mm(at[i] + c.from)));
            // The cut's outline dimensioned by `cut_<n>_<i>_len` / `cut_<n>_<i>_angle`.
            let (lens, angle) = dimensioned_polygon_params(b, &format!("cut_{n}"), &c.outline);
            let sketch = b.feature(
                format!("Cut{n}Sketch"),
                vec![],
                FeatureKind::Sketch {
                    plane: Plane::Origin { name: c.axis.plane() },
                    sketch: polygon_sketch_dimensioned(&c.outline, &lens, angle.as_ref()),
                },
            );
            b.feature(
                format!("Cut{n}"),
                vec![],
                FeatureKind::Extrude {
                    sketch,
                    profiles: Profiles::All,
                    extent: Extent::positive(format!("cut_{n}_len")).starting_at(less(format!("cut_{n}_0"), &self.origin(c.axis, at))),
                    operation: Operation::Cut,
                },
            );
        }
    }

    /// The holes as cylinders cut out of the part, also in its own frame: a through hole from 1 mm
    /// before the part to 1 mm after it, a blind one from 1 mm outside the face it opens on.
    fn hole_features(&self, b: &mut DesignBuilder, at: Vec3) {
        for (k, h) in self.holes.iter().enumerate() {
            let (n, a) = (k + 1, h.axis.index());
            b.param(Param::user(format!("hole_{n}_d"), h.d));
            let mut base = [String::new(), String::new(), String::new()];
            for (i, &w) in h.axis.profile_axes().iter().enumerate() {
                let name = format!("hole_{n}_{}", h.axis.profile_letter(i));
                b.param(Param::user(&name, snap_mm(at[w] + h.axis.profile_sign(i) * h.centre[i])));
                base[w] = less(name, &self.origin([Axis::X, Axis::Y, Axis::Z][w], at));
            }
            let length = if self.outline(h.axis).is_some() {
                format!("extrude_{}_len", h.axis.letter().to_ascii_lowercase())
            } else {
                fmt_number(self.size[a])
            };
            let mut dir = h.axis.dir();
            let height = if h.through(self.size) {
                base[a] = "-1".into();
                format!("{length} + 2")
            } else {
                b.param(Param::user(format!("hole_{n}_depth"), snap_mm(h.to - h.from)));
                if h.from <= 0.0 {
                    base[a] = "-1".into();
                } else {
                    base[a] = format!("{length} + 1");
                    dir = dir.map(|c| -c);
                }
                format!("hole_{n}_depth + 1")
            };
            b.feature(
                format!("Hole{n}"),
                vec![],
                FeatureKind::Cylinder { base, axis: dir, radius: format!("hole_{n}_d / 2"), height, operation: Operation::Cut },
            );
        }
    }
}

/// `value` less `origin` (an expression or a number), written without `- 0` or `- -3`.
fn less(value: String, origin: &str) -> String {
    match origin {
        "0" => value,
        o if o.starts_with('-') => format!("{value} + {}", &o[1..]),
        o => format!("{value} - {o}"),
    }
}
