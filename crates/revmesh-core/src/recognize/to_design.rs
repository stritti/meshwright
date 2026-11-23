use std::collections::HashMap;

use super::spec::{BossSpec, HoleHead, HoleSpec, PadSpec, PlateSpec, PocketSpec, ProfileSpec, SlotSpec};

use crate::design::{
    Design, DesignBuilder, Extent, FeatureKind, HoleKind, Operation, Param, PatternKind, Plane, Profiles, SlotDimensions, fmt_number,
    polygon_sketch, polygons_sketch, rectangle_sketch, slot_sketch,
};
use crate::mesh::{Vec3, snap_mm};
use crate::patterns::{HoleGroup, group_holes};

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
        slot_params(&mut b, &self.slots, at);
        boss_params(&mut b, &self.bosses, at);
        block_params(&mut b, &self.pads, &self.pockets, at);
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
        cutout_features(&mut b, &self.cutouts, at);
        hole_features(&mut b, &self.holes, at, "base_z0 + base_t", "base_z0", self.z);
        slot_features(&mut b, &self.slots, at);
        boss_features(&mut b, &self.bosses, "base_z0 + base_t");
        block_features(&mut b, &self.pads, &self.pockets, "base_z0 + base_t", "base_z0");
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
        slot_params(&mut b, &self.slots, at);
        boss_params(&mut b, &self.bosses, at);
        block_params(&mut b, &self.pads, &self.pockets, at);
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
        cutout_features(&mut b, &self.cutouts, at);
        hole_features(&mut b, &self.holes, at, "extrude_0 + extrude_len", "extrude_0", self.z);
        slot_features(&mut b, &self.slots, at);
        boss_features(&mut b, &self.bosses, "extrude_0 + extrude_len");
        block_features(&mut b, &self.pads, &self.pockets, "extrude_0 + extrude_len", "extrude_0");
        b.build()
    }
}

/// The slots' parameters, as mesh2cad names them: `slot_<n>_len` (between the end centres),
/// `_w`, the middle `_x`, `_y` (in world coordinates for a plate whose corner is at `at`) and
/// `_angle` (of the long axis from X).
fn slot_params(b: &mut DesignBuilder, slots: &[SlotSpec], at: Vec3) {
    for (i, s) in slots.iter().enumerate() {
        let p = format!("slot_{}", i + 1);
        b.param(Param::user(format!("{p}_len"), s.len))
            .param(Param::user(format!("{p}_w"), s.w))
            .param(Param::user(format!("{p}_x"), snap_mm(at[0] + s.x)))
            .param(Param::user(format!("{p}_y"), snap_mm(at[1] + s.y)))
            .param(Param::angle(format!("{p}_angle"), s.angle));
    }
}

/// A sketch of each slot on the XY plane, in world coordinates for a plate whose corner is at
/// `at`, and a cut through everything both ways from it. The sketch's dimensions are model
/// parameters driven by the slot's user parameters.
fn slot_features(b: &mut DesignBuilder, slots: &[SlotSpec], at: Vec3) {
    for (i, s) in slots.iter().enumerate() {
        let p = format!("slot_{}", i + 1);
        let mut model = |expr: String, angle: bool| {
            let name = b.next_model_param_name();
            b.param(if angle { Param::model_angle(&name, expr) } else { Param::model(&name, expr) });
            name
        };
        let centre = [snap_mm(at[0] + s.x), snap_mm(at[1] + s.y)];
        let dims = SlotDimensions {
            len: model(format!("{p}_len"), false),
            w: model(format!("{p}_w"), false),
            x: format!("{p}_x"),
            y: format!("{p}_y"),
            angle: Some(model(format!("{p}_angle"), true)),
        };
        let n = i + 1;
        let sketch = b.feature(
            format!("Slot{n}Sketch"),
            vec![],
            FeatureKind::Sketch { plane: Plane::Origin { name: "XY" }, sketch: slot_sketch(centre, s.len, s.w, s.angle, &dims) },
        );
        b.feature(
            format!("Slot{n}"),
            vec![],
            FeatureKind::Extrude { sketch, profiles: Profiles::All, extent: Extent::through_all(), operation: Operation::Cut },
        );
    }
}

/// The cut-outs as one sketch (in world coordinates for a plate whose corner is at `at`) and a
/// cut through everything both ways from it. It comes after the outline's fillet: a cut-out may
/// lie in the material an inside corner's rounding adds.
fn cutout_features(b: &mut DesignBuilder, cutouts: &[Vec<[f64; 2]>], at: Vec3) {
    if cutouts.is_empty() {
        return;
    }
    let world: Vec<Vec<[f64; 2]>> =
        cutouts.iter().map(|c| c.iter().map(|p| [snap_mm(at[0] + p[0]), snap_mm(at[1] + p[1])]).collect()).collect();
    let sketch =
        b.feature("CutoutsSketch", vec![], FeatureKind::Sketch { plane: Plane::Origin { name: "XY" }, sketch: polygons_sketch(&world) });
    b.feature(
        "Cutouts",
        vec![],
        FeatureKind::Extrude { sketch, profiles: Profiles::All, extent: Extent::through_all(), operation: Operation::Cut },
    );
}

/// The bosses' parameters, as mesh2cad names them: `boss_<n>_d`, `_h` (above the top face) and
/// the centre `_x`, `_y` (in world coordinates for a plate whose corner is at `at`).
fn boss_params(b: &mut DesignBuilder, bosses: &[BossSpec], at: Vec3) {
    for (i, s) in bosses.iter().enumerate() {
        let p = format!("boss_{}", i + 1);
        b.param(Param::user(format!("{p}_d"), s.d))
            .param(Param::user(format!("{p}_h"), s.h))
            .param(Param::user(format!("{p}_x"), snap_mm(at[0] + s.x)))
            .param(Param::user(format!("{p}_y"), snap_mm(at[1] + s.y)));
    }
}

/// A cylinder joined to the plate per boss, standing on the top face at height `top` (its
/// expression).
fn boss_features(b: &mut DesignBuilder, bosses: &[BossSpec], top: &str) {
    for i in 1..=bosses.len() {
        let p = format!("boss_{i}");
        b.feature(
            format!("Boss{i}"),
            vec![],
            FeatureKind::Cylinder {
                base: [format!("{p}_x"), format!("{p}_y"), top.into()],
                axis: [0.0, 0.0, 1.0],
                radius: format!("{p}_d / 2"),
                height: format!("{p}_h"),
                operation: Operation::Join,
            },
        );
    }
}

/// The pads' and the pockets' parameters: `pad_<n>_w` (along X), `_l` (along Y), `_h` and the
/// middle `_x`, `_y`, as mesh2cad names a pad, and `pocket_<n>_w`, `_l`, `_depth`, `_x`, `_y`
/// alike (in world coordinates for a plate whose corner is at `at`).
fn block_params(b: &mut DesignBuilder, pads: &[PadSpec], pockets: &[PocketSpec], at: Vec3) {
    let middle = |b: &mut DesignBuilder, p: &str, x: f64, y: f64| {
        b.param(Param::user(format!("{p}_x"), snap_mm(at[0] + x))).param(Param::user(format!("{p}_y"), snap_mm(at[1] + y)));
    };
    for (i, s) in pads.iter().enumerate() {
        let p = format!("pad_{}", i + 1);
        b.param(Param::user(format!("{p}_w"), s.w)).param(Param::user(format!("{p}_l"), s.l)).param(Param::user(format!("{p}_h"), s.h));
        middle(b, &p, s.x, s.y);
    }
    for (i, s) in pockets.iter().enumerate() {
        let p = format!("pocket_{}", i + 1);
        b.param(Param::user(format!("{p}_w"), s.w))
            .param(Param::user(format!("{p}_l"), s.l))
            .param(Param::user(format!("{p}_depth"), s.depth));
        middle(b, &p, s.x, s.y);
    }
}

/// A box joined to the plate per pad, standing on the top face at height `top` (its
/// expression), and a box cut out of it per pocket, down from `top` or up from `bottom`.
fn block_features(b: &mut DesignBuilder, pads: &[PadSpec], pockets: &[PocketSpec], top: &str, bottom: &str) {
    let corner = |p: &str, z: String| [format!("{p}_x - {p}_w / 2"), format!("{p}_y - {p}_l / 2"), z];
    for i in 1..=pads.len() {
        let p = format!("pad_{i}");
        let kind = FeatureKind::Box {
            corner: corner(&p, top.into()),
            length: format!("{p}_w"),
            width: format!("{p}_l"),
            height: format!("{p}_h"),
            operation: Operation::Join,
        };
        b.feature(format!("Pad{i}"), vec![], kind);
    }
    for (i, s) in pockets.iter().enumerate() {
        let p = format!("pocket_{}", i + 1);
        let z = if s.bottom { bottom.to_owned() } else { format!("{top} - {p}_depth") };
        let kind = FeatureKind::Box {
            corner: corner(&p, z),
            length: format!("{p}_w"),
            width: format!("{p}_l"),
            height: format!("{p}_depth"),
            operation: Operation::Cut,
        };
        b.feature(format!("Pocket{}", i + 1), vec![], kind);
    }
}

/// The holes, grouped into patterns (see [`crate::patterns`]), each with the parameter prefix
/// mesh2cad gives it: `hole_<n>`, `hole_row_<n>`, `hole_grid_<n>` or `hole_circle_<n>`, numbered
/// per kind.
fn hole_groups(holes: &[HoleSpec]) -> Vec<(String, HoleGroup)> {
    let mut counts: HashMap<&'static str, usize> = HashMap::new();
    group_holes(holes)
        .into_iter()
        .map(|group| {
            let prefix = match &group {
                HoleGroup::Single(_) => "hole",
                HoleGroup::Row { .. } => "hole_row",
                HoleGroup::Grid { .. } => "hole_grid",
                HoleGroup::Circle { .. } => "hole_circle",
            };
            let n = counts.entry(prefix).or_insert(0);
            *n += 1;
            (format!("{prefix}_{n}"), group)
        })
        .collect()
}

/// The holes' parameters, positions in world coordinates for a plate whose corner is at `at`.
#[allow(clippy::many_single_char_names)] // x, y, d, r as in the parameter names
fn hole_params(b: &mut DesignBuilder, holes: &[HoleSpec], at: Vec3) {
    let (ax, ay) = (at[0], at[1]);
    for (p, group) in hole_groups(holes) {
        match group {
            HoleGroup::Single(h) => {
                b.param(Param::user(format!("{p}_x"), snap_mm(ax + h.x)))
                    .param(Param::user(format!("{p}_y"), snap_mm(ay + h.y)))
                    .param(Param::user(format!("{p}_d"), h.d));
                if let Some(blind) = h.blind {
                    b.param(Param::user(format!("{p}_depth"), blind.depth));
                }
                match h.head {
                    None => {}
                    Some(HoleHead::Counterbore { d, depth, .. }) => {
                        b.param(Param::user(format!("{p}_cbore_d"), d)).param(Param::user(format!("{p}_cbore_depth"), depth));
                    }
                    Some(HoleHead::Countersink { d, angle, .. }) => {
                        b.param(Param::user(format!("{p}_csk_d"), d)).param(Param::angle(format!("{p}_csk_angle"), angle));
                    }
                }
            }
            HoleGroup::Row { n, pitch, d, x, y, .. } => {
                b.param(Param::count(format!("{p}_n"), n))
                    .param(Param::user(format!("{p}_pitch"), pitch))
                    .param(Param::user(format!("{p}_d"), d))
                    .param(Param::user(format!("{p}_x"), snap_mm(ax + x)))
                    .param(Param::user(format!("{p}_y"), snap_mm(ay + y)));
            }
            HoleGroup::Grid { nx, ny, pitch_x, pitch_y, d, x, y } => {
                b.param(Param::count(format!("{p}_nx"), nx))
                    .param(Param::count(format!("{p}_ny"), ny))
                    .param(Param::user(format!("{p}_pitch_x"), pitch_x))
                    .param(Param::user(format!("{p}_pitch_y"), pitch_y))
                    .param(Param::user(format!("{p}_d"), d))
                    .param(Param::user(format!("{p}_x"), snap_mm(ax + x)))
                    .param(Param::user(format!("{p}_y"), snap_mm(ay + y)));
            }
            HoleGroup::Circle { n, d, r, cx, cy, angle } => {
                b.param(Param::count(format!("{p}_n"), n))
                    .param(Param::user(format!("{p}_d"), d))
                    .param(Param::user(format!("{p}_r"), r))
                    .param(Param::user(format!("{p}_cx"), snap_mm(ax + cx)))
                    .param(Param::user(format!("{p}_cy"), snap_mm(ay + cy)))
                    .param(Param::angle(format!("{p}_angle"), angle));
            }
        }
    }
}

/// A through hole per hole, or per pattern a hole and a pattern feature that repeats it, drilled
/// down from the top face at height `top` (its expression) of a plate `z` thick whose corner is
/// at `at`; a hole with a counterbore or countersink on the bottom face (at height `bottom`) is
/// drilled up from there.
#[allow(clippy::many_single_char_names)] // x, y, d, r as in the parameter names
fn hole_features(b: &mut DesignBuilder, holes: &[HoleSpec], at: Vec3, top: &str, bottom: &str, z: f64) {
    let top_z = at[2] + z;
    let hole = |b: &mut DesignBuilder, name: String, position: [f64; 2], x: String, y: String, d: String| {
        b.feature(
            name,
            vec![],
            FeatureKind::Hole {
                position: [position[0], position[1], top_z],
                at: [x, y, top.into()],
                direction: [0.0, 0.0, -1.0],
                diameter: d,
                depth: None,
                hole: HoleKind::Simple,
            },
        )
    };
    let (mut singles, mut rows, mut grids, mut circles) = (0, 0, 0, 0);
    for (p, group) in hole_groups(holes) {
        let (x, y, d) = (format!("{p}_x"), format!("{p}_y"), format!("{p}_d"));
        match group {
            HoleGroup::Single(h) => {
                singles += 1;
                let (kind, from_bottom) = match h.head {
                    None => (HoleKind::Simple, h.blind.is_some_and(|b| b.bottom)),
                    Some(HoleHead::Counterbore { bottom, .. }) => {
                        (HoleKind::Counterbore { cb_diameter: format!("{p}_cbore_d"), cb_depth: format!("{p}_cbore_depth") }, bottom)
                    }
                    Some(HoleHead::Countersink { bottom, .. }) => {
                        (HoleKind::Countersink { cs_diameter: format!("{p}_csk_d"), cs_angle: format!("{p}_csk_angle") }, bottom)
                    }
                };
                let (face_z, face, direction) = if from_bottom { (at[2], bottom, 1.0) } else { (top_z, top, -1.0) };
                b.feature(
                    format!("Hole{singles}"),
                    vec![],
                    FeatureKind::Hole {
                        position: [at[0] + h.x, at[1] + h.y, face_z],
                        at: [x, y, face.into()],
                        direction: [0.0, 0.0, direction],
                        diameter: d,
                        depth: h.blind.map(|_| format!("{p}_depth")),
                        hole: kind,
                    },
                );
            }
            HoleGroup::Row { x: hx, y: hy, angle, .. } => {
                rows += 1;
                let seed = hole(b, format!("HoleRow{rows}"), [at[0] + hx, at[1] + hy], x, y, d);
                let (sin, cos) = angle.to_radians().sin_cos();
                let pattern = PatternKind::Rectangular {
                    dir1: [cos, sin, 0.0],
                    count1: format!("{p}_n"),
                    spacing1: format!("{p}_pitch"),
                    dir2: None,
                    count2: None,
                    spacing2: None,
                };
                b.feature(format!("HoleRow{rows}Pattern"), vec![], FeatureKind::Pattern { features: vec![seed], pattern });
            }
            HoleGroup::Grid { x: hx, y: hy, .. } => {
                grids += 1;
                let seed = hole(b, format!("HoleGrid{grids}"), [at[0] + hx, at[1] + hy], x, y, d);
                let pattern = PatternKind::Rectangular {
                    dir1: [1.0, 0.0, 0.0],
                    count1: format!("{p}_nx"),
                    spacing1: format!("{p}_pitch_x"),
                    dir2: Some([0.0, 1.0, 0.0]),
                    count2: Some(format!("{p}_ny")),
                    spacing2: Some(format!("{p}_pitch_y")),
                };
                b.feature(format!("HoleGrid{grids}Pattern"), vec![], FeatureKind::Pattern { features: vec![seed], pattern });
            }
            HoleGroup::Circle { r, cx, cy, angle, .. } => {
                circles += 1;
                let (sin, cos) = angle.to_radians().sin_cos();
                let position = [at[0] + cx + r * cos, at[1] + cy + r * sin];
                let seed = hole(
                    b,
                    format!("HoleCircle{circles}"),
                    position,
                    format!("{p}_cx + {p}_r * cos({p}_angle)"),
                    format!("{p}_cy + {p}_r * sin({p}_angle)"),
                    d,
                );
                let pattern = PatternKind::Circular {
                    origin: [format!("{p}_cx"), format!("{p}_cy"), "0".into()],
                    axis: [0.0, 0.0, 1.0],
                    count: format!("{p}_n"),
                    angle: "360 deg".into(),
                };
                b.feature(format!("HoleCircle{circles}Pattern"), vec![], FeatureKind::Pattern { features: vec![seed], pattern });
            }
        }
    }
}
