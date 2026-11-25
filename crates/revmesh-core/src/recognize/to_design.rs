use std::collections::HashMap;

use super::axis::Axis;
use super::spec::{BossChamfer, BossSpec, HoleHead, HoleSpec, OutlinePocketSpec, PadSpec, PlateSpec, PocketSpec, ProfileSpec, SlotSpec};

use crate::design::{
    Design, DesignBuilder, Extent, FeatureKind, HoleKind, Operation, Param, PatternKind, Plane, PolygonAngle, Profiles, SlotDimensions,
    dimensioned_angle, dimensioned_sides, polygon_sketch_dimensioned, polygons_sketch_dimensioned, rectangle_sketch, slot_sketch,
};
use crate::mesh::{Vec3, snap_dim, snap_mm};
use crate::patterns::{HoleGroup, group_holes};

/// The expressions for a point's world coordinates: along the extrusion axis the face's
/// expression, along the profile axes the parameters named after their world axis letters.
fn world_exprs(prefix: &str, axis: Axis, face: &str) -> [String; 3] {
    let mut out = [const { String::new() }; 3];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = match axis.profile_axes().iter().position(|&a| a == i) {
            Some(k) => format!("{prefix}_{}", axis.profile_letter(k)),
            None => String::from(face),
        };
    }
    out
}

/// The two position parameters of a point `(u, v)` in the profile plane, named after their world
/// axis letters (mesh2cad's `hole_<n>_y`, `hole_<n>_z` on an X extrusion), in world coordinates
/// for a part whose bounding box's lowest corner is at `at`.
fn pos_params(b: &mut DesignBuilder, name: &str, axis: Axis, at: Vec3, p: [f64; 2]) {
    for (i, value) in p.iter().enumerate() {
        b.param(Param::user(format!("{name}_{}", axis.profile_letter(i)), snap_mm(axis.profile_world(at, i, *value))));
    }
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
    /// four vertical corners. A plate is always a Z extrusion.
    #[must_use]
    pub fn to_design_at(&self, name: &str, at: Vec3) -> Design {
        let axis = Axis::Z;
        let mut b = DesignBuilder::new(name);
        b.param(Param::user("base_x", self.x)).param(Param::user("base_y", self.y)).param(Param::user("base_t", self.z));
        b.param(Param::user("base_cx", snap_mm(at[0] + self.x / 2.0)))
            .param(Param::user("base_cy", snap_mm(at[1] + self.y / 2.0)))
            .param(Param::user("base_z0", snap_mm(at[2])));
        hole_params(&mut b, &self.holes, at, axis);
        slot_params(&mut b, &self.slots, at, axis);
        boss_params(&mut b, &self.bosses, at, axis);
        block_params(&mut b, &self.pads, &self.pockets, at, axis);
        if let Some(r) = self.outline_fillet_r {
            b.param(Param::user("fillet_r", r));
        }
        if let Some(r) = self.edge_fillet_r {
            b.param(Param::user("edge_fillet_r", r));
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
        if self.edge_fillet_r.is_some() {
            // All twelve edges of the box, as mesh2cad's `.edges().fillet(edge_fillet_r)` does;
            // the holes are cut after the fillet. The extrude's caps are `top` (at the far end)
            // and `start` (on the sketch plane); the sides are l1 bottom, l2 right, l3 top, l4
            // left (see `rectangle_sketch`).
            let mid = at[2] + self.z / 2.0;
            let (x0, y0, x1, y1) = (at[0], at[1], at[0] + self.x, at[1] + self.y);
            let side = |a: usize, b: usize| format!("F{extrude}:side:l{a}|F{extrude}:side:l{b}");
            let mut edge_names = vec![side(1, 4), side(1, 2), side(2, 3), side(3, 4)];
            let mut edges = vec![[x0, y0, mid], [x1, y0, mid], [x1, y1, mid], [x0, y1, mid]];
            let mids = [[f64::midpoint(x0, x1), y0], [x1, f64::midpoint(y0, y1)], [f64::midpoint(x0, x1), y1], [x0, f64::midpoint(y0, y1)]];
            for (k, m) in mids.iter().enumerate() {
                let i = k + 1;
                edge_names.push(format!("F{extrude}:side:l{i}|F{extrude}:top"));
                edges.push([m[0], m[1], at[2] + self.z]);
                edge_names.push(format!("F{extrude}:side:l{i}|F{extrude}:start"));
                edges.push([m[0], m[1], at[2]]);
            }
            b.feature("Fillet1", vec![], FeatureKind::Fillet { edge_names, edges, radius: "edge_fillet_r".into() });
        }
        cutout_features(&mut b, &self.cutouts, at, axis);
        hole_features(&mut b, &self.holes, at, axis, "base_z0 + base_t", "base_z0", self.z, extrude);
        slot_features(&mut b, &self.slots, at, axis);
        boss_features(&mut b, &self.bosses, at, self.z, axis, "base_z0 + base_t", extrude);
        block_features(&mut b, &self.pads, &self.pockets, &[], at, axis, "base_z0 + base_t", "base_z0");
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
    /// The profile as a parametric history, its bounding box's corner at the origin; see
    /// [`ProfileSpec::to_design_at`].
    #[must_use]
    pub fn to_design(&self, name: &str) -> Design {
        self.to_design_at(name, [0.0; 3])
    }

    /// The profile as a parametric history with its bounding box's lowest corner at `at`,
    /// extruded along Z; see [`ProfileSpec::to_design_on`].
    #[must_use]
    pub fn to_design_at(&self, name: &str, at: Vec3) -> Design {
        self.to_design_on(name, at, Axis::Z)
    }

    /// The profile as a parametric history with its bounding box's lowest corner at `at`,
    /// extruded along `axis`, as mesh2cad writes an extruded outline: parameters `extrude_len`
    /// (the thickness), `extrude_0` (where it starts along the axis), `outline_<i>_len` per side,
    /// `outline_<i>_angle` on the longest slanted side, `hole_<n>_<a>/_<b>/_d` for the profile
    /// plane's world axis letters and `outline_fillet_r`;
    /// a sketch of the outline on the axis's plane (see [`Axis::plane`]), every side dimensioned
    /// by its own parameter, an extrusion along the plane's normal, a move to where the part is,
    /// a fillet of every corner edge parallel to the axis and a hole per hole. The fillet comes
    /// before the holes: an inside corner's rounding adds material, and a hole may lie in it.
    #[must_use]
    pub fn to_design_on(&self, name: &str, at: Vec3, axis: Axis) -> Design {
        let mut b = DesignBuilder::new(name);
        let top = "extrude_0 + extrude_len";
        b.param(Param::user("extrude_len", self.z)).param(Param::user("extrude_0", snap_mm(at[axis.index()])));
        hole_params(&mut b, &self.holes, at, axis);
        slot_params(&mut b, &self.slots, at, axis);
        boss_params(&mut b, &self.bosses, at, axis);
        block_params(&mut b, &self.pads, &self.pockets, at, axis);
        if let Some(r) = self.outline_fillet_r {
            b.param(Param::user("outline_fillet_r", r));
        }
        let outline_lens = side_len_params(&mut b, "outline", &self.corners);
        let outline_angle = side_angle_param(&mut b, "outline", &self.corners);
        let sketch = b.feature(
            "Sketch1",
            vec![],
            FeatureKind::Sketch {
                plane: Plane::Origin { name: axis.plane() },
                sketch: polygon_sketch_dimensioned(&self.corners, &outline_lens, outline_angle.as_ref()),
            },
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
        b.feature("Move1", vec![], FeatureKind::Move { bodies: vec!["Body1".into()], translate: axis.translate(at) });
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
            let edges = self.corners.iter().map(|c| axis.world(at, c[0], c[1], self.z / 2.0)).collect();
            b.feature("Fillet1", vec![], FeatureKind::Fillet { edge_names, edges, radius: "outline_fillet_r".into() });
        }
        cutout_features(&mut b, &self.cutouts, at, axis);
        hole_features(&mut b, &self.holes, at, axis, top, "extrude_0", self.z, extrude);
        slot_features(&mut b, &self.slots, at, axis);
        boss_features(&mut b, &self.bosses, at, self.z, axis, top, extrude);
        block_features(&mut b, &self.pads, &self.pockets, &self.outline_pockets, at, axis, top, "extrude_0");
        b.build()
    }
}

/// One user parameter `<prefix>_<i>_angle` for the polygon's longest slanted side (see
/// [`dimensioned_angle`]) and a model parameter driving its `Angle` constraint in the sketch, so
/// a slanted polygon re-angles in SolidCraft too. `None` when no side is slanted.
fn side_angle_param(b: &mut DesignBuilder, prefix: &str, corners: &[[f64; 2]]) -> Option<PolygonAngle> {
    let side = dimensioned_angle(corners)?;
    let (a, c) = (corners[side], corners[(side + 1) % corners.len()]);
    // The side's own direction, as drawn: the angle from the X axis in [0, 360) degrees (the
    // `Angle` constraint turns line `b` as drawn, so collapsing to [0, 180) would mirror a
    // side pointing below the X axis).
    let degrees = (c[1] - a[1]).atan2(c[0] - a[0]).to_degrees().rem_euclid(360.0);
    let user = format!("{prefix}_{}_angle", side + 1);
    b.param(Param::angle(&user, degrees));
    let model = b.next_model_param_name();
    b.param(Param::model_angle(&model, &user));
    Some(PolygonAngle { side, degrees, param: model })
}

/// One user parameter `<prefix>_<i>_len` per driving side of `corners` (see
/// [`dimensioned_sides`]) and a model parameter driving its `Length` constraint in the sketch, so
/// the polygon re-dimensions in SolidCraft side by side, like the rectangle's `base_x`/`base_y`.
/// The side that drives the `Angle` (see [`dimensioned_angle`]) does not drive a length too: with
/// the other sides' lengths fixed, the angle determines it, and driving both would leave no
/// solution when the angle parameter changes. Returns the model parameters' names, `None` for a
/// side that does not drive.
fn side_len_params(b: &mut DesignBuilder, prefix: &str, corners: &[[f64; 2]]) -> Vec<Option<String>> {
    let n = corners.len();
    let mut dim = dimensioned_sides(corners);
    if let Some(s) = dimensioned_angle(corners) {
        dim[s] = false;
    }
    (0..n)
        .map(|i| {
            if !dim[i] {
                return None;
            }
            let (a, c) = (corners[i], corners[(i + 1) % n]);
            let len = snap_dim((a[0] - c[0]).hypot(a[1] - c[1]));
            let user = format!("{prefix}_{}_len", i + 1);
            b.param(Param::user(&user, len));
            let model = b.next_model_param_name();
            b.param(Param::model(&model, &user));
            Some(model)
        })
        .collect()
}

/// The slots' parameters, as mesh2cad names them: `slot_<n>_len` (between the end centres),
/// `_w`, the middle's two profile-plane coordinates (in world coordinates, named after their
/// world axis letters) and `_angle` (of the long axis from the profile's first axis).
fn slot_params(b: &mut DesignBuilder, slots: &[SlotSpec], at: Vec3, axis: Axis) {
    for (i, s) in slots.iter().enumerate() {
        let p = format!("slot_{}", i + 1);
        b.param(Param::user(format!("{p}_len"), s.len)).param(Param::user(format!("{p}_w"), s.w));
        pos_params(b, &p, axis, at, [s.x, s.y]);
        b.param(Param::angle(format!("{p}_angle"), s.angle));
    }
}

/// The expression placing a slot's middle in sketch coordinates on the axis's plane: the
/// parameter named after the profile coordinate's world axis, negated where the plane's sketch
/// axis runs against the world axis (the XZ plane's sketch y is -Z).
fn sketch_pos_expr(p: &str, axis: Axis, i: usize) -> String {
    let name = format!("{p}_{}", axis.profile_letter(i));
    if axis.profile_sign(i) < 0.0 { format!("-{name}") } else { name }
}

/// A sketch of each slot on the profile plane, at its world profile coordinates for a part whose
/// corner is at `at`, and a cut through everything both ways from it. The sketch's dimensions are
/// model parameters driven by the slot's user parameters.
fn slot_features(b: &mut DesignBuilder, slots: &[SlotSpec], at: Vec3, axis: Axis) {
    for (i, s) in slots.iter().enumerate() {
        let p = format!("slot_{}", i + 1);
        let mut model = |expr: String, angle: bool| {
            let name = b.next_model_param_name();
            b.param(if angle { Param::model_angle(&name, expr) } else { Param::model(&name, expr) });
            name
        };
        let centre = axis.sketch_point(at, [s.x, s.y]).map(snap_mm);
        let dims = SlotDimensions {
            len: model(format!("{p}_len"), false),
            w: model(format!("{p}_w"), false),
            x: sketch_pos_expr(&p, axis, 0),
            y: sketch_pos_expr(&p, axis, 1),
            angle: Some(model(format!("{p}_angle"), true)),
        };
        let n = i + 1;
        let sketch = b.feature(
            format!("Slot{n}Sketch"),
            vec![],
            FeatureKind::Sketch { plane: Plane::Origin { name: axis.plane() }, sketch: slot_sketch(centre, s.len, s.w, s.angle, &dims) },
        );
        b.feature(
            format!("Slot{n}"),
            vec![],
            FeatureKind::Extrude { sketch, profiles: Profiles::All, extent: Extent::through_all(), operation: Operation::Cut },
        );
    }
}

/// The cut-outs as one sketch on the profile plane (at their world profile coordinates for a part
/// whose corner is at `at`) and a cut through everything both ways from it. It comes after the
/// outline's fillet: a cut-out may lie in the material an inside corner's rounding adds.
fn cutout_features(b: &mut DesignBuilder, cutouts: &[Vec<[f64; 2]>], at: Vec3, axis: Axis) {
    if cutouts.is_empty() {
        return;
    }
    let sketched: Vec<Vec<[f64; 2]>> = cutouts.iter().map(|c| c.iter().map(|p| axis.sketch_point(at, *p).map(snap_mm)).collect()).collect();
    let lens: Vec<Vec<Option<String>>> =
        sketched.iter().enumerate().map(|(k, poly)| side_len_params(b, &format!("cutout_{}", k + 1), poly)).collect();
    let angles: Vec<Option<PolygonAngle>> =
        sketched.iter().enumerate().map(|(k, poly)| side_angle_param(b, &format!("cutout_{}", k + 1), poly)).collect();
    let sketch = b.feature(
        "CutoutsSketch",
        vec![],
        FeatureKind::Sketch { plane: Plane::Origin { name: axis.plane() }, sketch: polygons_sketch_dimensioned(&sketched, &lens, &angles) },
    );
    b.feature(
        "Cutouts",
        vec![],
        FeatureKind::Extrude { sketch, profiles: Profiles::All, extent: Extent::through_all(), operation: Operation::Cut },
    );
}

/// The bosses' parameters, as mesh2cad names them: `boss_<n>_d`, `_h` (above the face the boss
/// stands on), `_fillet` (the round at its foot, if it has one) and the centre's two
/// profile-plane coordinates (in world coordinates).
fn boss_params(b: &mut DesignBuilder, bosses: &[BossSpec], at: Vec3, axis: Axis) {
    for (i, s) in bosses.iter().enumerate() {
        let p = format!("boss_{}", i + 1);
        b.param(Param::user(format!("{p}_d"), s.d)).param(Param::user(format!("{p}_h"), s.h));
        if let Some(f) = s.fillet {
            b.param(Param::user(format!("{p}_fillet"), f));
        }
        pos_params(b, &p, axis, at, [s.x, s.y]);
        if let Some(draft) = s.draft_deg {
            b.param(Param::angle(format!("{p}_draft_deg"), draft));
        } else if let Some(BossChamfer { w, h }) = s.chamfer {
            // A 45° chamfer is one parameter, as mesh2cad writes it.
            if (w - h).abs() < 1e-9 {
                b.param(Param::user(format!("{p}_chamfer"), w));
            } else {
                b.param(Param::user(format!("{p}_chamfer_h"), h)).param(Param::user(format!("{p}_chamfer_w"), w));
            }
        }
    }
}

/// A cylinder joined to the part per boss, standing on the face at height `top` (its
/// expression) along the axis, and a fillet of its foot where it has one: the edge between the
/// extrusion `extrude`'s top face and the cylinder's side, picked at the boss's +X side on a
/// part `z` thick whose corner is at `at`.
fn boss_features(b: &mut DesignBuilder, bosses: &[BossSpec], at: Vec3, z: f64, axis: Axis, top: &str, extrude: u32) {
    for (i, s) in bosses.iter().enumerate() {
        let p = format!("boss_{}", i + 1);
        let kind = if s.draft_deg.is_some() {
            FeatureKind::Cone {
                base: world_exprs(&p, axis, top),
                axis: axis.dir(),
                radius: format!("{p}_d / 2"),
                top_radius: format!("{p}_d / 2 - {p}_h * tan({p}_draft_deg)"),
                height: format!("{p}_h"),
                operation: Operation::Join,
            }
        } else {
            FeatureKind::Cylinder {
                base: world_exprs(&p, axis, top),
                axis: axis.dir(),
                radius: format!("{p}_d / 2"),
                height: format!("{p}_h"),
                operation: Operation::Join,
            }
        };
        let cylinder = b.feature(format!("Boss{}", i + 1), vec![], kind);
        if let Some(BossChamfer { w, h }) = s.chamfer {
            // The top face faces up most, so the first distance is along it (in from the side).
            let (distance, distance2) = if (w - h).abs() < 1e-9 {
                (format!("{p}_chamfer"), None)
            } else {
                (format!("{p}_chamfer_w"), Some(format!("{p}_chamfer_h")))
            };
            b.feature(
                format!("Boss{}Chamfer", i + 1),
                vec![],
                FeatureKind::Chamfer {
                    edge_names: vec![format!("F{cylinder}:side|F{cylinder}:top")],
                    edges: vec![axis.world(at, s.x + s.d / 2.0, s.y, z + s.h)],
                    distance,
                    distance2,
                },
            );
        }
        if s.fillet.is_some() {
            b.feature(
                format!("Boss{}Fillet", i + 1),
                vec![],
                FeatureKind::Fillet {
                    edge_names: vec![format!("F{extrude}:top|F{cylinder}:side")],
                    edges: vec![axis.world(at, s.x + s.d / 2.0, s.y, z)],
                    radius: format!("{p}_fillet"),
                },
            );
        }
    }
}

/// The pads' and the pockets' parameters: `pad_<n>_w` (along the profile's first axis), `_l`
/// (along its second), `_h` and the middle's two profile-plane coordinates, as mesh2cad names
/// a pad, and `pocket_<n>_w`, `_l`, `_depth`, and the middle's alike (in world coordinates for a
/// part whose corner is at `at`).
fn block_params(b: &mut DesignBuilder, pads: &[PadSpec], pockets: &[PocketSpec], at: Vec3, axis: Axis) {
    for (i, s) in pads.iter().enumerate() {
        let p = format!("pad_{}", i + 1);
        b.param(Param::user(format!("{p}_w"), s.w)).param(Param::user(format!("{p}_l"), s.l)).param(Param::user(format!("{p}_h"), s.h));
        pos_params(b, &p, axis, at, [s.x, s.y]);
    }
    for (i, s) in pockets.iter().enumerate() {
        let p = format!("pocket_{}", i + 1);
        b.param(Param::user(format!("{p}_w"), s.w))
            .param(Param::user(format!("{p}_l"), s.l))
            .param(Param::user(format!("{p}_depth"), s.depth));
        pos_params(b, &p, axis, at, [s.x, s.y]);
    }
}

/// A box joined to the part per pad, standing on the face at `top`, and a box cut out of it per
/// pocket, from `top` or from `bottom`; a pocket of any other outline of straight sides gets
/// its own sketch and cut instead. The box's corner and extents are in world axes: along
/// the extrusion axis the pad's height or the pocket's depth, along the profile axes its width
/// and length.
#[allow(clippy::too_many_arguments)] // the two block kinds, their frame and faces
fn block_features(
    b: &mut DesignBuilder,
    pads: &[PadSpec],
    pockets: &[PocketSpec],
    outline_pockets: &[OutlinePocketSpec],
    at: Vec3,
    axis: Axis,
    top: &str,
    bottom: &str,
) {
    let box_at = |p: &str, along: &str, extent: &str| -> ([String; 3], [String; 3]) {
        let mut corner = [const { String::new() }; 3];
        let mut size = [const { String::new() }; 3];
        for i in 0..3 {
            match axis.profile_axes().iter().position(|&a| a == i) {
                Some(0) => {
                    corner[i] = format!("{p}_{} - {p}_w / 2", axis.profile_letter(0));
                    size[i] = format!("{p}_w");
                }
                Some(1) => {
                    corner[i] = format!("{p}_{} - {p}_l / 2", axis.profile_letter(1));
                    size[i] = format!("{p}_l");
                }
                _ => {
                    corner[i] = String::from(along);
                    size[i] = String::from(extent);
                }
            }
        }
        (corner, size)
    };
    for i in 1..=pads.len() {
        let p = format!("pad_{i}");
        let (corner, size) = box_at(&p, top, &format!("{p}_h"));
        b.feature(
            format!("Pad{i}"),
            vec![],
            FeatureKind::Box {
                corner,
                length: size[0].clone(),
                width: size[1].clone(),
                height: size[2].clone(),
                operation: Operation::Join,
            },
        );
    }
    for (i, s) in pockets.iter().enumerate() {
        let p = format!("pocket_{}", i + 1);
        let along = if s.bottom { bottom.to_owned() } else { format!("{top} - {p}_depth") };
        let (corner, size) = box_at(&p, &along, &format!("{p}_depth"));
        b.feature(
            format!("Pocket{}", i + 1),
            vec![],
            FeatureKind::Box {
                corner,
                length: size[0].clone(),
                width: size[1].clone(),
                height: size[2].clone(),
                operation: Operation::Cut,
            },
        );
    }
    // A pocket of any other outline of straight sides gets its own sketch, every side
    // dimensioned by `pocket_<n>_<i>_len` (numbered after the rectangular pockets), and a cut
    // from its face down by `pocket_<n>_depth`.
    for (i, s) in outline_pockets.iter().enumerate() {
        let n = pockets.len() + i + 1;
        let p = format!("pocket_{n}");
        b.param(Param::user(format!("{p}_depth"), s.depth));
        let poly: Vec<[f64; 2]> = s.corners.iter().map(|c| axis.sketch_point(at, *c).map(snap_mm)).collect();
        let lens = side_len_params(b, &p, &poly);
        let sketch = b.feature(
            format!("Pocket{n}Sketch"),
            vec![],
            FeatureKind::Sketch { plane: Plane::Origin { name: axis.plane() }, sketch: polygon_sketch_dimensioned(&poly, &lens) },
        );
        let start = if s.bottom { bottom.to_owned() } else { format!("{top} - {p}_depth") };
        b.feature(
            format!("Pocket{n}"),
            vec![],
            FeatureKind::Extrude {
                sketch,
                profiles: Profiles::All,
                extent: Extent::positive(format!("{p}_depth")).starting_at(start),
                operation: Operation::Cut,
            },
        );
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

/// The holes' parameters, positions in world coordinates for a part whose corner is at `at`,
/// named after the profile plane's world axis letters.
#[allow(clippy::many_single_char_names)] // x, y, d, r as in the parameter names
fn hole_params(b: &mut DesignBuilder, holes: &[HoleSpec], at: Vec3, axis: Axis) {
    for (p, group) in hole_groups(holes) {
        match group {
            HoleGroup::Single(h) => {
                pos_params(b, &p, axis, at, [h.x, h.y]);
                b.param(Param::user(format!("{p}_d"), h.d));
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
                    // mesh2cad's names: the top rim's `_rim_r`, the bottom's `_bot_rim_r`.
                    Some(HoleHead::Round { r, bottom }) => {
                        b.param(Param::user(format!("{p}_{}rim_r", if bottom { "bot_" } else { "" }), r));
                    }
                }
            }
            HoleGroup::Row { n, pitch, d, x, y, .. } => {
                b.param(Param::count(format!("{p}_n"), n))
                    .param(Param::user(format!("{p}_pitch"), pitch))
                    .param(Param::user(format!("{p}_d"), d));
                pos_params(b, &p, axis, at, [x, y]);
            }
            HoleGroup::Grid { nx, ny, pitch_x, pitch_y, d, x, y } => {
                b.param(Param::count(format!("{p}_nx"), nx))
                    .param(Param::count(format!("{p}_ny"), ny))
                    .param(Param::user(format!("{p}_pitch_x"), pitch_x))
                    .param(Param::user(format!("{p}_pitch_y"), pitch_y))
                    .param(Param::user(format!("{p}_d"), d));
                pos_params(b, &p, axis, at, [x, y]);
            }
            HoleGroup::Circle { n, d, r, cx, cy, angle } => {
                b.param(Param::count(format!("{p}_n"), n)).param(Param::user(format!("{p}_d"), d)).param(Param::user(format!("{p}_r"), r));
                for (i, c) in [cx, cy].iter().enumerate() {
                    b.param(Param::user(format!("{p}_c{}", axis.profile_letter(i)), snap_mm(axis.profile_world(at, i, *c))));
                }
                b.param(Param::angle(format!("{p}_angle"), angle));
            }
        }
    }
}

/// The part the holes are drilled into: where its corner is in the file, the extrusion axis, the
/// two faces' expressions, the thickness and the drilling direction from the +axis face.
struct HoleCtx<'a> {
    at: Vec3,
    axis: Axis,
    top: &'a str,
    bottom: &'a str,
    z: f64,
    down: Vec3,
    /// The extrusion whose faces (`top`, `start`) the holes open on.
    extrude: u32,
}

/// A through hole per hole, or per pattern a hole and a pattern feature that repeats it, drilled
/// from the face at depth `z` along the axis (the `top` expression); a hole with a counterbore or
/// countersink on the other face (at depth 0, the `bottom` expression) is drilled from there
/// along the axis.
#[allow(clippy::too_many_arguments)] // the part's frame, its faces and the extrusion they belong to
fn hole_features(b: &mut DesignBuilder, holes: &[HoleSpec], at: Vec3, axis: Axis, top: &str, bottom: &str, z: f64, extrude: u32) {
    // Drilled from the +axis face: against the axis. (Built component-wise, so no -0.0.)
    let down = {
        let mut d = [0.0; 3];
        d[axis.index()] = -1.0;
        d
    };
    let ctx = HoleCtx { at, axis, top, bottom, z, down, extrude };
    let mut counts = [0, 0, 0, 0]; // singles, rows, grids, circles
    for (p, group) in hole_groups(holes) {
        match group {
            HoleGroup::Single(h) => {
                counts[0] += 1;
                single_hole(b, &ctx, &p, &h, counts[0]);
            }
            group => pattern_hole(b, &ctx, &p, &group, &mut counts),
        }
    }
}

/// One hole in no pattern: drilled from the face its counterbore, countersink or blind floor is
/// on, or from the +axis face into the part.
#[allow(clippy::many_single_char_names)] // x, y, d, r as in the parameter names
fn single_hole(b: &mut DesignBuilder, ctx: &HoleCtx, p: &str, h: &HoleSpec, n: usize) {
    let (kind, from_bottom) = match h.head {
        None => (HoleKind::Simple, h.blind.is_some_and(|b| b.bottom)),
        Some(HoleHead::Counterbore { bottom, .. }) => {
            (HoleKind::Counterbore { cb_diameter: format!("{p}_cbore_d"), cb_depth: format!("{p}_cbore_depth") }, bottom)
        }
        Some(HoleHead::Countersink { bottom, .. }) => {
            (HoleKind::Countersink { cs_diameter: format!("{p}_csk_d"), cs_angle: format!("{p}_csk_angle") }, bottom)
        }
        // A rounded rim: a plain hole, and a fillet of its edge on that face below.
        Some(HoleHead::Round { .. }) => (HoleKind::Simple, false),
    };
    let (face_w, face, direction) = if from_bottom { (0.0, ctx.bottom, ctx.axis.dir()) } else { (ctx.z, ctx.top, ctx.down) };
    let hole = b.feature(
        format!("Hole{n}"),
        vec![],
        FeatureKind::Hole {
            position: ctx.axis.world(ctx.at, h.x, h.y, face_w),
            at: world_exprs(p, ctx.axis, face),
            direction,
            diameter: format!("{p}_d"),
            depth: h.blind.map(|_| format!("{p}_depth")),
            hole: kind,
        },
    );
    if let Some(HoleHead::Round { bottom, .. }) = h.head {
        // The edge between the face and the hole's bore, as SolidCraft names it.
        let (face, w, radius) = if bottom { ("start", 0.0, format!("{p}_bot_rim_r")) } else { ("top", ctx.z, format!("{p}_rim_r")) };
        b.feature(
            format!("Hole{n}Rim"),
            vec![],
            FeatureKind::Fillet {
                edge_names: vec![format!("F{}:{face}|F{hole}:bore:side:0", ctx.extrude)],
                edges: vec![ctx.axis.world(ctx.at, h.x + h.d / 2.0, h.y, w)],
                radius,
            },
        );
    }
}

/// A pattern's seed hole: drilled from the +axis face at depth `z`.
fn seed_hole(b: &mut DesignBuilder, ctx: &HoleCtx, name: String, profile: [f64; 2], p: &str) -> u32 {
    b.feature(
        name,
        vec![],
        FeatureKind::Hole {
            position: ctx.axis.world(ctx.at, profile[0], profile[1], ctx.z),
            at: world_exprs(p, ctx.axis, ctx.top),
            direction: ctx.down,
            diameter: format!("{p}_d"),
            depth: None,
            hole: HoleKind::Simple,
        },
    )
}

/// One hole and a pattern feature per pattern: a row, a grid or a bolt circle, all in the profile
/// plane, so the pattern's directions and axis are the frame's vectors rotated back to world.
#[allow(clippy::many_single_char_names)] // x, y, d, r as in the parameter names
fn pattern_hole(b: &mut DesignBuilder, ctx: &HoleCtx, p: &str, group: &HoleGroup, counts: &mut [usize; 4]) {
    let axis = ctx.axis;
    match group {
        HoleGroup::Single(_) => unreachable!("a single hole has no pattern"),
        HoleGroup::Row { x: hx, y: hy, angle, .. } => {
            counts[1] += 1;
            let n = counts[1];
            let seed = seed_hole(b, ctx, format!("HoleRow{n}"), [*hx, *hy], p);
            let (sin, cos) = angle.to_radians().sin_cos();
            let pattern = PatternKind::Rectangular {
                dir1: axis.unrotate([cos, sin, 0.0]),
                count1: format!("{p}_n"),
                spacing1: format!("{p}_pitch"),
                dir2: None,
                count2: None,
                spacing2: None,
            };
            b.feature(format!("HoleRow{n}Pattern"), vec![], FeatureKind::Pattern { features: vec![seed], pattern });
        }
        HoleGroup::Grid { x: hx, y: hy, .. } => {
            counts[2] += 1;
            let n = counts[2];
            let seed = seed_hole(b, ctx, format!("HoleGrid{n}"), [*hx, *hy], p);
            let pattern = PatternKind::Rectangular {
                dir1: axis.unrotate([1.0, 0.0, 0.0]),
                count1: format!("{p}_nx"),
                spacing1: format!("{p}_pitch_x"),
                dir2: Some(axis.unrotate([0.0, 1.0, 0.0])),
                count2: Some(format!("{p}_ny")),
                spacing2: Some(format!("{p}_pitch_y")),
            };
            b.feature(format!("HoleGrid{n}Pattern"), vec![], FeatureKind::Pattern { features: vec![seed], pattern });
        }
        HoleGroup::Circle { r, cx, cy, angle, .. } => {
            counts[3] += 1;
            let n = counts[3];
            let (sin, cos) = angle.to_radians().sin_cos();
            let profile = [cx + r * cos, cy + r * sin]; // Copy fields of &HoleGroup
            let mut at_exprs = [const { String::new() }; 3];
            for (i, slot) in at_exprs.iter_mut().enumerate() {
                *slot = match axis.profile_axes().iter().position(|&a| a == i) {
                    Some(k) => {
                        // The centre's parameter is a world coordinate; the radius's offset is
                        // in profile coordinates, so it carries the profile sign (the second
                        // profile coordinate maps to -Z on a Y extrusion).
                        let trig = if k == 0 { ("cos", "+") } else { ("sin", if axis.profile_sign(k) < 0.0 { "-" } else { "+" }) };
                        format!("{p}_c{} {} {p}_r * {}({p}_angle)", axis.profile_letter(k), trig.1, trig.0)
                    }
                    _ => String::from(ctx.top),
                };
            }
            let seed = b.feature(
                format!("HoleCircle{n}"),
                vec![],
                FeatureKind::Hole {
                    position: axis.world(ctx.at, profile[0], profile[1], ctx.z),
                    at: at_exprs,
                    direction: ctx.down,
                    diameter: format!("{p}_d"),
                    depth: None,
                    hole: HoleKind::Simple,
                },
            );
            let mut origin = [const { String::new() }; 3];
            for (i, slot) in origin.iter_mut().enumerate() {
                *slot = match axis.profile_axes().iter().position(|&a| a == i) {
                    Some(k) => format!("{p}_c{}", axis.profile_letter(k)),
                    _ => "0".into(),
                };
            }
            let pattern = PatternKind::Circular { origin, axis: axis.dir(), count: format!("{p}_n"), angle: "360 deg".into() };
            b.feature(format!("HoleCircle{n}Pattern"), vec![], FeatureKind::Pattern { features: vec![seed], pattern });
        }
    }
}
