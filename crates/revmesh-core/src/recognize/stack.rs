//! A stack of outlines, each extruded over its own span along one axis, and its design.

use super::axis::Axis;
use super::to_design::dimensioned_polygon_params;

use crate::Locale;
use crate::design::{
    Design, DesignBuilder, Extent, FeatureKind, Operation, Param, Plane, Profiles, fmt_number, polygons_sketch_dimensioned,
};
use crate::mesh::{Vec3, snap_mm};

/// A chamfer of the top band's top edges (roadmap 3.12): `w` in from each top edge on the top face
/// and `h` down from the top along the walls — a bevel when small, a roof when the chamfers of
/// opposite edges meet. All top edges carry it, with the same `w` and `h`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Chamfer {
    /// In from the top edge on the top face.
    pub w: f64,
    /// Down from the top along the walls.
    pub h: f64,
}

impl Chamfer {
    /// The volume the chamfer takes off a band whose top outline is the convex `outline`, in
    /// mm³: the top slice of height `h` less the inward-offset polygon integrated over it
    /// (Steiner: the offset by `d` has area `A − P d + K d²`, `K = Σ cot(θ/2)` over the interior
    /// angles θ — and `turn` below is the exterior angle, so cot(θ/2) = tan(turn/2)).
    #[allow(clippy::many_single_char_names)] // the corners a, c, b and the side vectors d1, d2
    fn removed(&self, outline: &[[f64; 2]]) -> f64 {
        let n = outline.len();
        let side = |i: usize| {
            let (a, b) = (outline[i], outline[(i + 1) % n]);
            (b[0] - a[0]).hypot(b[1] - a[1])
        };
        let perimeter: f64 = (0..n).map(side).sum();
        let corners: f64 = (0..n)
            .map(|i| {
                let (a, c, b) = (outline[(i + n - 1) % n], outline[i], outline[(i + 1) % n]);
                let d1 = [(c[0] - a[0]) / side((i + n - 1) % n), (c[1] - a[1]) / side((i + n - 1) % n)];
                let d2 = [(b[0] - c[0]) / side(i), (b[1] - c[1]) / side(i)];
                let turn = (d1[0] * d2[1] - d1[1] * d2[0]).atan2(d1[0] * d2[0] + d1[1] * d2[1]);
                (turn / 2.0).tan() // tan(turn/2) = cot(θ/2), θ the interior angle
            })
            .sum();
        self.h * (perimeter * self.w / 2.0 - corners * self.w * self.w / 3.0)
    }
}

/// One band of a stack: `outline` (with `cutouts` through it) extruded from `from` to `to` along
/// the stack's axis, its top edges chamfered when it is the top band and has a `chamfer`.
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
    /// The chamfer of its top edges, on the top band only (roadmap 3.12: a bevel or a roof).
    pub chamfer: Option<Chamfer>,
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
        self.bands.iter().map(|b| b.area() * (b.to - b.from) - b.chamfer.map_or(0.0, |c| c.removed(&b.outline))).sum()
    }

    /// One line describing the part, stacked along `axis`, for the loss report.
    #[must_use]
    pub fn describe(&self, axis: Axis) -> String {
        self.describe_in(Locale::En, axis)
    }

    /// [`Self::describe`] in `locale`'s words.
    #[must_use]
    #[allow(clippy::many_single_char_names)] // the sizes x, y, z, the band count n and the chamfer c
    pub fn describe_in(&self, locale: Locale, axis: Axis) -> String {
        let world = axis.unrotate(self.size).map(f64::abs);
        let [x, y, z] = world.map(|v| locale.decimal(&fmt_number(v)));
        let n = self.bands.len();
        let part = locale.format("summary.stack", &[("x", &x), ("y", &y), ("z", &z), ("n", &n), ("axis", &axis.letter())]);
        let Some(c) = self.bands.last().and_then(|b| b.chamfer) else { return part };
        let w = locale.decimal(&fmt_number(c.w));
        let h = locale.decimal(&fmt_number(c.h));
        let chamfer = locale.format("summary.chamfer", &[("w", &w), ("h", &h)]);
        locale.format("summary.with", &[("plate", &part), ("parts", &chamfer)])
    }

    /// The stack as a parametric history with its frame's origin at the world point `at`, along
    /// `axis`, as mesh2cad writes a union of extrusions: per band `k` the parameters
    /// `extrude_<a><k>_len` and `extrude_<a><k>_0` (where it starts; a band above the first
    /// starts where the one below ends, so changing a length moves the bands above), a sketch of
    /// its section on the axis's plane — dimensioned like a plate's outline, the band's outline
    /// by `outline_<k>_<i>_len` and `outline_<k>_<i>_angle`, its openings by
    /// `cutout_<n>_<i>_len`/`_angle` — and an extrusion along the plane's normal from its start,
    /// the first a new body and the next joined to it; then a move to where the part is, and the
    /// top band's top edges chamfered (a bevel or a roof, roadmap 3.12) as `Chamfer1`, driven by
    /// `chamfer` (a 45° one) or `chamfer_w` and `chamfer_h`.
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
        // The top band's chamfer (roadmap 3.12); a 45° one keeps mesh2cad's single parameter.
        let chamfer = self.bands.last().and_then(|band| band.chamfer);
        let (distance, distance2) = match chamfer {
            Some(c) if (c.w - c.h).abs() < 1e-9 => {
                b.param(Param::user("chamfer", snap_mm(c.w)));
                ("chamfer".to_owned(), None)
            }
            Some(c) => {
                b.param(Param::user("chamfer_w", snap_mm(c.w))).param(Param::user("chamfer_h", snap_mm(c.h)));
                ("chamfer_w".to_owned(), Some("chamfer_h".to_owned()))
            }
            None => (String::new(), None),
        };
        // The openings are numbered across the bands, in design order.
        let mut cutout_n = 0;
        let mut top_extrude = 0;
        for (k, band) in self.bands.iter().enumerate() {
            let n = k + 1;
            // The openings in the band's sketch, its outline's lines l1..l<m> first.
            let polygons = [vec![band.outline.clone()], band.cutouts.clone()].concat();
            let mut lens = Vec::with_capacity(polygons.len());
            let mut angles = Vec::with_capacity(polygons.len());
            for (i, poly) in polygons.iter().enumerate() {
                let prefix = if i == 0 {
                    format!("outline_{n}")
                } else {
                    cutout_n += 1;
                    format!("cutout_{cutout_n}")
                };
                let (l, angle) = dimensioned_polygon_params(&mut b, &prefix, poly);
                lens.push(l);
                angles.push(angle);
            }
            let sketch = b.feature(
                format!("Sketch{n}"),
                vec![],
                FeatureKind::Sketch {
                    plane: Plane::Origin { name: axis.plane() },
                    sketch: polygons_sketch_dimensioned(&polygons, &lens, &angles),
                },
            );
            let extent = Extent::positive(format!("extrude_{a}{n}_len"));
            top_extrude = b.feature(
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
        if let Some(top) = self.bands.last().filter(|band| band.chamfer.is_some()) {
            // The chamfer of the top band's top edges (roadmap 3.12): one edge per side of its
            // outline, picked at the side's middle on the top, named after the extrusion's side
            // and top faces. The first distance runs along the top face (in from the edge),
            // the second down the side.
            let m = top.outline.len();
            let edge_names: Vec<String> = (1..=m).map(|i| format!("F{top_extrude}:side:l{i}|F{top_extrude}:top")).collect();
            let edges: Vec<Vec3> = (0..m)
                .map(|i| {
                    let (a, b) = (top.outline[i], top.outline[(i + 1) % m]);
                    axis.world(at, f64::midpoint(a[0], b[0]), f64::midpoint(a[1], b[1]), top.to)
                })
                .collect();
            b.feature("Chamfer1", vec![], FeatureKind::Chamfer { edge_names, edges, distance, distance2 });
        }
        b.build()
    }
}
