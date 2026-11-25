use super::spec::{BossSpec, HoleSpec, OutlinePocketSpec, PadSpec, PlateSpec, PocketSpec, ProfileSpec, SlotSpec};
use super::{Axis, Recognition, recognize};

use crate::design::{Design, dimensioned_sides, fmt_number};
use crate::mesh::Mesh;
use crate::mesh::tests::tetrahedron;
use crate::synth;

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
fn translated_block_has_the_same_volume() {
    let translated = block(100.0, 100.0, 5.0, [1e9; 3]);
    assert!((translated.volume() - 50_000.0).abs() < 1e-6);
}

#[test]
fn recognition_preserves_small_bottom_offset() {
    let mesh = block(1000.0, 100.0, 5.0, [0.0, 0.0, 0.009]);
    let Recognition::Plate { offset, .. } = recognize(&mesh) else {
        panic!("expected plate");
    };
    assert!((offset[2] - 0.009).abs() < 1e-9);
}

#[test]
fn block_volume_is_positive() {
    assert!((block(2.0, 3.0, 4.0, [0.0; 3]).volume() - 24.0).abs() < 1e-9);
}

#[test]
fn recognises_block_on_origin() {
    let found = recognize(&block(60.0, 40.0, 6.0, [0.0; 3]));
    let plate = PlateSpec {
        x: 60.0,
        y: 40.0,
        z: 6.0,
        holes: vec![],
        cutouts: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![],
        slots: vec![],
        outline_fillet_r: None,
        edge_fillet_r: None,
    };
    assert_eq!(found, Recognition::Plate { plate, offset: [0.0; 3] });
}

#[test]
fn moves_offset_block_to_the_origin() {
    let plate = PlateSpec {
        x: 1.0,
        y: 1.0,
        z: 1.0,
        holes: vec![],
        cutouts: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![],
        slots: vec![],
        outline_fillet_r: None,
        edge_fillet_r: None,
    };
    assert_eq!(recognize(&block(1.0, 1.0, 1.0, [5.0, 0.0, 0.0])), Recognition::Plate { plate, offset: [5.0, 0.0, 0.0] });
}

#[test]
fn describes_plates() {
    let mut spec = PlateSpec {
        x: 70.0,
        y: 40.0,
        z: 6.0,
        holes: vec![],
        cutouts: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![],
        slots: vec![],
        outline_fillet_r: Some(3.0),
        edge_fillet_r: None,
    };
    assert_eq!(spec.describe(), "A 70 x 40 x 6 mm plate with 3 mm rounded corners");
    spec.holes = vec![HoleSpec { x: 1.0, y: 1.0, d: 1.0, head: None, blind: None }; 2];
    assert_eq!(spec.describe(), "A 70 x 40 x 6 mm plate with 2 holes and 3 mm rounded corners");
}

/// A slot is a sketch with its angle measured from the linked X axis (along X too, so that
/// any slot can be turned), and a cut through everything. Its middle is a point placed by
/// the position parameters, which may cross the origin.
#[test]
fn slots_become_sketches_and_cuts() {
    let slot = |x: f64, angle: f64| SlotSpec { x, y: 20.0, len: 16.0, w: 5.0, angle };
    let spec = PlateSpec {
        x: 60.0,
        y: 40.0,
        z: 6.0,
        holes: vec![],
        cutouts: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![],
        slots: vec![slot(15.0, 0.0), slot(40.0, 30.0)],
        outline_fillet_r: None,
        edge_fillet_r: None,
    };
    let json: serde_json::Value = serde_json::from_str(&spec.to_design_at("", [-30.0, 0.0, 0.0]).to_json()).expect("valid");
    let features = json["features"].as_array().expect("features");
    let names: Vec<&str> = features.iter().filter_map(|f| f["name"].as_str()).collect();
    assert_eq!(names, ["Sketch1", "Extrude1", "Move1", "Slot1Sketch", "Slot1", "Slot2Sketch", "Slot2"]);
    assert_eq!(features[4]["operation"], "cut");
    assert_eq!(features[4]["extent"]["through_all"], true);
    let constraint = |f: usize, kind: &str| -> Vec<serde_json::Value> {
        features[f]["sketch"]["constraints"].as_array().expect("constraints").iter().filter(|c| c["type"] == kind).cloned().collect()
    };
    assert_eq!(constraint(3, "angle").len(), 1);
    assert_eq!(constraint(5, "angle").len(), 1);
    assert_eq!(features[5]["sketch"]["links"][0]["source"]["name"], "x");
    let model = |name: &serde_json::Value| -> String {
        json["params"].as_array().expect("params").iter().find(|p| &p["name"] == name).expect("param")["expr"]
            .as_str()
            .expect("expr")
            .to_owned()
    };
    // Slot 1 lies at X = -15, slot 2 at X = 10.
    let middle = |f: usize| features[f]["sketch"]["points"][8].clone();
    assert_eq!(middle(3)["exprs"]["x"], "slot_1_x");
    assert_eq!(middle(3)["pos"][0], -15.0);
    assert_eq!(middle(5)["exprs"]["y"], "slot_2_y");
    assert_eq!(constraint(3, "coincident").len(), 1);
    assert_eq!(model(&constraint(5, "angle")[0]["param"]), "slot_2_angle");
    assert!(json["params"].as_array().expect("params").iter().any(|p| p["name"] == "slot_2_angle" && p["expr"] == "30 deg"));
    assert!((spec.volume() - (60.0 * 40.0 - 2.0 * (16.0 * 5.0 + std::f64::consts::PI * 6.25)) * 6.0).abs() < 1e-9);
    assert_eq!(spec.describe(), "A 60 x 40 x 6 mm plate with 2 slots");
}

/// Cut-outs are one sketch and a cut through everything, after the outline's fillet (a
/// cut-out may lie in an inside corner's rounding) and before the holes.
#[test]
fn cut_outs_are_cut_after_the_fillet() {
    let spec = PlateSpec {
        x: 60.0,
        y: 40.0,
        z: 6.0,
        holes: vec![],
        slots: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![],
        cutouts: vec![vec![[10.0, 10.0], [20.0, 10.0], [15.0, 20.0]]],
        outline_fillet_r: None,
        edge_fillet_r: None,
    };
    let json: serde_json::Value = serde_json::from_str(&spec.to_design_at("", [5.0, 0.0, 0.0]).to_json()).expect("valid");
    let features = json["features"].as_array().expect("features");
    let names: Vec<&str> = features.iter().filter_map(|f| f["name"].as_str()).collect();
    assert_eq!(names, ["Sketch1", "Extrude1", "Move1", "CutoutsSketch", "Cutouts"]);
    let sketch = &features[3]["sketch"];
    // Three sides plus the construction line along the sketch's X axis for the angle.
    assert_eq!(sketch["curves"].as_array().map(Vec::len), Some(4));
    // Points 1 and 2 are the X axis the angle is measured from; the cut-out's corners follow.
    assert_eq!(sketch["points"][3]["pos"], serde_json::json!([15.0, 10.0]));
    assert_eq!((features[4]["operation"].as_str(), &features[4]["extent"]["through_all"]), (Some("cut"), &serde_json::json!(true)));
    assert!((spec.volume() - (60.0 * 40.0 - 50.0) * 6.0).abs() < 1e-9);
    assert_eq!(spec.describe(), "A 60 x 40 x 6 mm plate with 1 cut-out");
    let ell = ProfileSpec {
        corners: vec![[0.0, 0.0], [60.0, 0.0], [60.0, 15.0], [15.0, 15.0], [15.0, 40.0], [0.0, 40.0]],
        z: 5.0,
        holes: vec![HoleSpec { x: 7.5, y: 30.0, d: 5.0, head: None, blind: None }],
        slots: vec![],
        pads: vec![],
        pockets: vec![],
        outline_pockets: Vec::new(),
        bosses: vec![],
        cutouts: vec![vec![[15.8, 15.8], [16.2, 15.8], [16.0, 16.2]]],
        outline_fillet_r: Some(5.0),
    };
    let json: serde_json::Value = serde_json::from_str(&ell.to_design("").to_json()).expect("valid");
    let names: Vec<String> =
        json["features"].as_array().expect("features").iter().filter_map(|f| f["name"].as_str().map(String::from)).collect();
    assert_eq!(names, ["Sketch1", "Extrude1", "Move1", "Fillet1", "CutoutsSketch", "Cutouts", "Hole1"]);
}

/// Roadmap 5.1: every side of an outline and of a cut-out carries a `Length` constraint driven by
/// its own model parameter, which a user parameter `outline_<i>_len` / `cutout_<n>_<i>_len` drives
/// — the sketch re-dimensions in SolidCraft side by side, like the rectangle's `base_x`/`base_y`.
#[test]
fn outline_and_cutout_sides_are_dimensioned() {
    let spec = ProfileSpec {
        corners: vec![[0.0, 0.0], [60.0, 0.0], [60.0, 15.0], [30.0, 40.0], [0.0, 40.0]],
        z: 5.0,
        holes: vec![],
        slots: vec![],
        pads: vec![],
        pockets: vec![],
        outline_pockets: Vec::new(),
        bosses: vec![],
        cutouts: vec![vec![[10.0, 10.0], [20.0, 10.0], [15.0, 20.0]]],
        outline_fillet_r: None,
    };
    let json: serde_json::Value = serde_json::from_str(&spec.to_design("").to_json()).expect("valid");
    let params = json["params"].as_array().expect("params");
    let param = |name: &str| {
        params.iter().find(|p| p["name"] == name).map(|p| (p["expr"].as_str().expect("expr"), p["model"].as_bool().expect("model")))
    };
    // One user parameter per driving side with the side's length. Sides group by line direction:
    // the horizontal sides 1 and 4 (60 and 30 mm) and the vertical sides 2 and 5 (15 and 40 mm)
    // each form a parallel group, so the longest of each (sides 1 and 5) is derived; the slanted
    // diagonal (side 3) drives the angle instead of a length — with the other lengths fixed, the
    // angle determines it, and driving both would over-constrain the sketch.
    let sides: [(usize, f64); 2] = [(1, 15.0), (3, 30.0)];
    for (i, len) in sides {
        let snapped = (len * 1e4).round() / 1e4;
        let (expr, model) = param(&format!("outline_{}_len", i + 1)).expect("user param");
        assert!(!model);
        assert_eq!(expr, format!("{} mm", fmt_number(snapped)));
    }
    assert!(
        param("outline_1_len").is_none() && param("outline_3_len").is_none() && param("outline_5_len").is_none(),
        "the longest side per parallel group and the angle side are derived"
    );
    let sketch = &json["features"][0]["sketch"];
    let constraints = sketch["constraints"].as_array().expect("constraints");
    let lengths: Vec<u64> = constraints.iter().filter(|c| c["type"] == "length").filter_map(|c| c["l"].as_u64()).collect();
    assert_eq!(lengths, [1, 3], "one Length per driving side, none on the derived ones");
    // The four axis-aligned sides keep their horizontal/vertical constraints.
    assert_eq!(constraints.iter().filter(|c| c["type"] == "horizontal" || c["type"] == "vertical").count(), 4);
    for c in lengths.iter().enumerate() {
        let (_, i) = c;
        let length = &constraints.iter().find(|c| c["type"] == "length" && c["l"].as_u64() == Some(*i)).expect("length");
        let model = length["param"].as_str().expect("param");
        let (expr, is_model) = param(model).expect("model param");
        assert!(is_model);
        assert_eq!(expr, format!("outline_{}_len", i + 1));
    }
    // The cut-out's sides are dimensioned the same way, as `cutout_1_<i>_len`, except its angle
    // side (side 3, the longest slanted one), which carries `cutout_1_3_angle` instead.
    let cutouts = &json["features"].as_array().expect("features").iter().find(|f| f["name"] == "CutoutsSketch").expect("cutouts sketch");
    let cut_lengths: Vec<&serde_json::Value> =
        cutouts["sketch"]["constraints"].as_array().expect("constraints").iter().filter(|c| c["type"] == "length").collect();
    assert_eq!(cut_lengths.len(), 2);
    for (i, c) in cut_lengths.iter().enumerate() {
        let model = c["param"].as_str().expect("param");
        let (expr, _) = param(model).expect("model param");
        assert_eq!(expr, format!("cutout_1_{}_len", i + 1));
    }
    assert_eq!(param("cutout_1_1_len").map(|(e, _)| e), Some("10 mm"));
    assert!(param("cutout_1_3_len").is_none(), "the angle side's length is derived");
    // The shared cut-out sketch counts its linked axis the same way: l4, p4, p5 past its three
    // corners (codex finding on #72).
    assert_eq!(cutouts["sketch"]["counters"]["l"].as_u64(), Some(4), "the axis line l4 is counted: {cutouts}");
    assert_eq!(cutouts["sketch"]["counters"]["p"].as_u64(), Some(5), "the axis points p4, p5 are counted: {cutouts}");
    // Roadmap 5.1: the longest slanted side carries an `Angle` from the sketch's X axis, driven
    // by its own user parameter `outline_<i>_angle` (side 3 here, the only slanted side).
    let (expr, model) = param("outline_3_angle").expect("user angle param");
    assert!(!model);
    let deg: f64 = expr.strip_suffix(" deg").expect("deg").parse().expect("number");
    let want = (40.0_f64 - 15.0).atan2(30.0 - 60.0).to_degrees().rem_euclid(360.0);
    assert!((deg - want).abs() < 1e-9, "{deg} vs {want}");
    let angles: Vec<&serde_json::Value> = constraints.iter().filter(|c| c["type"] == "angle").collect();
    assert_eq!(angles.len(), 1, "one Angle per outline, on its longest slanted side");
    // The counters count the linked axis's IDs too (l6, p6, p7), so a later edit allocates fresh
    // IDs (codex finding on #72).
    assert_eq!(sketch["counters"]["l"].as_u64(), Some(6), "the axis line l6 is counted: {sketch}");
    assert_eq!(sketch["counters"]["p"].as_u64(), Some(7), "the axis points p6, p7 are counted: {sketch}");
    assert_eq!(angles[0]["b"].as_u64(), Some(2), "on side 3 (line index 2)");
    let (expr, is_model) = param(angles[0]["param"].as_str().expect("param")).expect("model param");
    assert!(is_model);
    assert_eq!(expr, "outline_3_angle");
    let value = angles[0]["value"].as_f64().expect("value");
    assert!((value - want.to_radians()).abs() < 1e-12, "the constraint's value is in radians");
    // The X axis is linked in as one construction line, as in the slot's sketch.
    let links = sketch["links"].as_array().expect("links");
    assert_eq!(links.len(), 1);
    assert_eq!(links[0]["kind"], "project");
    assert_eq!(links[0]["source"]["type"], "sketch_axis");
    assert_eq!(links[0]["source"]["name"], "x");
    assert_eq!(sketch["curves"].as_array().expect("curves").iter().filter(|c| c["construction"] == true).count(), 1);
    // The cut-out's triangle gets its angle on its longest slanted side, as `cutout_1_3_angle`.
    let cut_angles: Vec<&serde_json::Value> =
        cutouts["sketch"]["constraints"].as_array().expect("constraints").iter().filter(|c| c["type"] == "angle").collect();
    assert_eq!(cut_angles.len(), 1);
    let (expr, _) = param(cut_angles[0]["param"].as_str().expect("param")).expect("model param");
    assert_eq!(expr, "cutout_1_3_angle");
}

/// In a fully axis-aligned outline the longest horizontal and vertical sides are derived through
/// loop closure, so they get no driving `Length` — driving them too would leave no solution when
/// one parameter changes (the L's `outline_1_len = outline_3_len + outline_5_len`).
#[test]
fn orthogonal_outline_skips_the_dependent_sides() {
    let spec = ProfileSpec {
        corners: vec![[0.0, 0.0], [60.0, 0.0], [60.0, 15.0], [15.0, 15.0], [15.0, 40.0], [0.0, 40.0]],
        z: 5.0,
        holes: vec![],
        slots: vec![],
        pads: vec![],
        pockets: vec![],
        outline_pockets: Vec::new(),
        bosses: vec![],
        cutouts: vec![],
        outline_fillet_r: None,
    };
    let json: serde_json::Value = serde_json::from_str(&spec.to_design("").to_json()).expect("valid");
    let params = json["params"].as_array().expect("params");
    let has = |name: &str| params.iter().any(|p| p["name"] == name);
    // Sides 0 (60 mm horizontal) and 5 (40 mm vertical) are the longest per direction: derived.
    for i in [1, 2, 3, 4] {
        assert!(has(&format!("outline_{}_len", i + 1)), "outline_{}_len exists", i + 1);
    }
    assert!(!has("outline_1_len") && !has("outline_6_len"));
    let constraints = json["features"][0]["sketch"]["constraints"].as_array().expect("constraints");
    let lengths: Vec<u64> = constraints.iter().filter(|c| c["type"] == "length").filter_map(|c| c["l"].as_u64()).collect();
    assert_eq!(lengths, [1, 2, 3, 4], "one Length per driving side, none on the derived ones");
    // All six sides keep their horizontal/vertical constraints.
    assert_eq!(constraints.iter().filter(|c| c["type"] == "horizontal" || c["type"] == "vertical").count(), 6);
    // No side is slanted, so no angle parameter and no Angle constraint.
    assert!(!params.iter().any(|p| p["name"].as_str().is_some_and(|n| n.ends_with("_angle"))));
    assert_eq!(constraints.iter().filter(|c| c["type"] == "angle").count(), 0);
}

/// The longest slanted side of a polygon is the one that carries the `Angle` constraint; an
/// axis-aligned polygon has none. Parallel slanted sides share their angle, so only one of them
/// could drive it anyway.
#[test]
fn dimensioned_angle_picks_the_longest_slanted_side() {
    use crate::design::dimensioned_angle;
    assert_eq!(dimensioned_angle(&[[0.0, 0.0], [60.0, 0.0], [75.0, 30.0], [10.0, 30.0]]), Some(1));
    assert_eq!(dimensioned_angle(&[[0.0, 0.0], [60.0, 0.0], [60.0, 40.0], [0.0, 40.0]]), None);
    // A triangle's three slanted sides are three directions; the longest drives the angle.
    assert_eq!(dimensioned_angle(&[[0.0, 0.0], [100.0, 0.0], [50.0, 0.1]]), Some(2), "the two long slanted sides tie; the last wins");
}

/// A slanted side pointing below the X axis keeps its own direction in [0, 360) degrees: the
/// `Angle` constraint turns the line as drawn, so collapsing the angle to [0, 180) would mirror
/// the side and change the part.
#[test]
fn slanted_side_below_the_x_axis_keeps_its_direction() {
    let spec = ProfileSpec {
        outline_pockets: vec![],
        corners: vec![[0.0, 0.0], [60.0, 0.0], [40.0, 30.0]],
        z: 5.0,
        holes: vec![],
        slots: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![],
        cutouts: vec![],
        outline_fillet_r: None,
    };
    let json: serde_json::Value = serde_json::from_str(&spec.to_design("").to_json()).expect("valid");
    let params = json["params"].as_array().expect("params");
    let expr = |name: &str| params.iter().find(|p| p["name"] == name).map(|p| p["expr"].as_str().expect("expr").to_owned());
    // Side 3 ([40, 30] to [0, 0]) is the longest slanted side; its direction is 216.87 deg, not 36.87.
    let deg: f64 = expr("outline_3_angle").expect("angle param").strip_suffix(" deg").expect("deg").parse().expect("number");
    let want = (0.0_f64 - 30.0).atan2(0.0 - 40.0).to_degrees().rem_euclid(360.0);
    assert!((deg - want).abs() < 1e-9, "{deg} vs {want}");
    assert!(deg > 180.0, "the side points below the X axis: {deg}");
}

/// A parallelogram's two horizontal and two parallel slanted sides each form a group, so one
/// side per group is derived and only two of the four lengths drive.
#[test]
fn parallelogram_skips_one_side_per_parallel_group() {
    let corners = vec![[0.0, 0.0], [60.0, 0.0], [70.0, 30.0], [10.0, 30.0]];
    assert_eq!(dimensioned_sides(&corners), [true, true, false, false]);
}

/// With an angle side, a right triangle's legs with their horizontal and vertical constraints
/// already fix the diagonal's direction, so one leg's length is released too (see
/// [`dimensioned_sides_for_angle`]): the longer leg (the base, side 1) is derived through loop
/// closure and only the shorter leg drives — driving both legs plus the angle would
/// over-constrain the sketch. A triangle with a horizontal base but no right angle needs no
/// extra release: its other slanted side's direction stays free.
#[test]
fn dimensioned_sides_for_angle_releases_a_leg_of_a_right_triangle() {
    use crate::design::dimensioned_sides_for_angle;
    assert_eq!(dimensioned_sides_for_angle(&[[0.0, 0.0], [40.0, 0.0], [40.0, 30.0]], 2), [false, true, false]);
    assert_eq!(dimensioned_sides_for_angle(&[[0.0, 0.0], [60.0, 0.0], [40.0, 30.0]], 2), [true, true, false]);
    // The pentagon of `outline_and_cutout_sides_are_dimensioned` stays as it is.
    assert_eq!(
        dimensioned_sides_for_angle(&[[0.0, 0.0], [60.0, 0.0], [60.0, 15.0], [30.0, 40.0], [0.0, 40.0]], 2),
        [false, true, false, true, false]
    );
}

/// A right-triangle outline or cut-out whose diagonal drives the `Angle` gets exactly `2n - 2 = 4`
/// driving constraints for its three free corners — horizontal and vertical on the legs, one
/// `Length` on the shorter leg, the `Angle` on the diagonal; the longer leg's (the base's) length
/// is derived, so bumping `outline_3_angle` by a degree in SolidCraft still has a solution.
#[test]
fn right_triangle_releases_a_leg_length_for_the_angle() {
    let spec = ProfileSpec {
        corners: vec![[0.0, 0.0], [40.0, 0.0], [40.0, 30.0]],
        z: 5.0,
        holes: vec![],
        slots: vec![],
        pads: vec![],
        pockets: vec![],
        outline_pockets: Vec::new(),
        bosses: vec![],
        cutouts: vec![vec![[10.0, 10.0], [30.0, 10.0], [30.0, 20.0]]],
        outline_fillet_r: None,
    };
    let json: serde_json::Value = serde_json::from_str(&spec.to_design("").to_json()).expect("valid");
    let params = json["params"].as_array().expect("params");
    let param = |name: &str| {
        params.iter().find(|p| p["name"] == name).map(|p| (p["expr"].as_str().expect("expr"), p["model"].as_bool().expect("model")))
    };
    // The outline's base (side 1, 40 mm, the longer leg) and the diagonal (side 3, the angle
    // side) are derived; the vertical leg (side 2) drives.
    assert!(param("outline_1_len").is_none() && param("outline_3_len").is_none(), "the base's and the diagonal's lengths are derived");
    let (expr, model) = param("outline_2_len").expect("user param");
    assert!(!model);
    assert_eq!(expr, "30 mm");
    assert!(param("outline_3_angle").is_some(), "the diagonal carries the angle");
    let sketch = &json["features"][0]["sketch"];
    let constraints = sketch["constraints"].as_array().expect("constraints");
    let lengths: Vec<u64> = constraints.iter().filter(|c| c["type"] == "length").filter_map(|c| c["l"].as_u64()).collect();
    assert_eq!(lengths, [1], "one Length, on the vertical leg (line index 1)");
    assert_eq!(constraints.iter().filter(|c| c["type"] == "horizontal" || c["type"] == "vertical").count(), 2);
    let angles: Vec<&serde_json::Value> = constraints.iter().filter(|c| c["type"] == "angle").collect();
    assert_eq!(angles.len(), 1);
    assert_eq!(angles[0]["b"].as_u64(), Some(2), "the Angle sits on the diagonal (line index 2)");
    // The right-triangle cut-out the same way: its base (side 1, 20 mm) is derived, its vertical
    // leg (side 2) drives, the diagonal carries `cutout_1_3_angle`.
    let cutouts = &json["features"].as_array().expect("features").iter().find(|f| f["name"] == "CutoutsSketch").expect("cutouts sketch");
    assert!(param("cutout_1_1_len").is_none() && param("cutout_1_3_len").is_none());
    assert_eq!(param("cutout_1_2_len").map(|(e, _)| e), Some("10 mm"));
    assert!(param("cutout_1_3_angle").is_some());
    let cut_constraints = cutouts["sketch"]["constraints"].as_array().expect("constraints");
    let cut_lengths: Vec<u64> = cut_constraints.iter().filter(|c| c["type"] == "length").filter_map(|c| c["l"].as_u64()).collect();
    // In the cut-outs' shared sketch the X axis is curve 0, so the cut-out's side 2 is curve 2.
    assert_eq!(cut_lengths, [2], "one Length, on the cut-out's vertical leg (side 2)");
    assert_eq!(cut_constraints.iter().filter(|c| c["type"] == "angle").count(), 1);
}

/// A thin profile keeps its slanted lengths' precision (0.1 µm snap): a 0.1 mm high triangle's
/// slanted sides stay 50.0001 mm, so the dimensioned sketch does not collapse to zero area.
#[test]
fn thin_triangle_keeps_slanted_length_precision() {
    let spec = ProfileSpec {
        corners: vec![[0.0, 0.0], [100.0, 0.0], [50.0, 0.1]],
        z: 5.0,
        holes: vec![],
        slots: vec![],
        pads: vec![],
        pockets: vec![],
        outline_pockets: Vec::new(),
        bosses: vec![],
        cutouts: vec![],
        outline_fillet_r: None,
    };
    let json: serde_json::Value = serde_json::from_str(&spec.to_design("").to_json()).expect("valid");
    let params = json["params"].as_array().expect("params");
    let expr = |name: &str| params.iter().find(|p| p["name"] == name).map(|p| p["expr"].as_str().expect("expr"));
    assert_eq!(expr("outline_1_len"), Some("100 mm"));
    assert_eq!(expr("outline_2_len"), Some("50.0001 mm"), "the slanted side keeps 0.1 µm precision");
}

/// Roadmap 5.3: a pocket of any outline of straight sides (not just a rectangle) gets its own
/// sketch — every side a `Length` driven by `pocket_<n>_<i>_len` — and a cut from its face down
/// by `pocket_<n>_depth`, numbered after the rectangular pockets.
#[test]
fn outline_pockets_get_their_own_sketch_and_cut() {
    let spec = ProfileSpec {
        corners: vec![[0.0, 0.0], [60.0, 0.0], [60.0, 40.0], [0.0, 40.0]],
        z: 6.0,
        holes: vec![],
        slots: vec![],
        pads: vec![],
        pockets: vec![PocketSpec { x: 10.0, y: 10.0, w: 8.0, l: 8.0, depth: 2.0, bottom: false }],
        outline_pockets: vec![OutlinePocketSpec { corners: vec![[30.0, 10.0], [50.0, 10.0], [40.0, 25.0]], depth: 3.0, bottom: false }],
        bosses: vec![],
        cutouts: vec![],
        outline_fillet_r: None,
    };
    let json: serde_json::Value = serde_json::from_str(&spec.to_design("").to_json()).expect("valid");
    let params = json["params"].as_array().expect("params");
    let expr = |name: &str| params.iter().find(|p| p["name"] == name).map(|p| p["expr"].as_str().expect("expr"));
    assert_eq!(expr("pocket_1_w"), Some("8 mm"), "the rectangular pocket is pocket_1");
    assert_eq!(expr("pocket_2_depth"), Some("3 mm"));
    // No parallel sides in a triangle: two sides drive a length; the longest slanted side
    // (side 3) drives the angle instead, its length is derived.
    assert_eq!(expr("pocket_2_1_len"), Some("20 mm"));
    assert_eq!(expr("pocket_2_2_len"), Some("18.0278 mm"));
    assert!(expr("pocket_2_3_len").is_none(), "the angle side's length is derived");
    assert!(expr("pocket_2_3_angle").is_some(), "the longest slanted side carries the angle");
    let features = json["features"].as_array().expect("features");
    let names: Vec<&str> = features.iter().filter_map(|f| f["name"].as_str()).collect();
    assert_eq!(names, ["Sketch1", "Extrude1", "Move1", "Pocket1", "Pocket2Sketch", "Pocket2"]);
    let sketch = &features[4]["sketch"];
    let constraints = sketch["constraints"].as_array().expect("constraints");
    let lengths: Vec<u64> = constraints.iter().filter(|c| c["type"] == "length").filter_map(|c| c["l"].as_u64()).collect();
    assert_eq!(lengths, [0, 1], "one Length per driving side of the triangular mouth");
    assert_eq!(constraints.iter().filter(|c| c["type"] == "angle").count(), 1, "one Angle on the longest slanted side");
    let cut = &features[5];
    assert_eq!(cut["type"].as_str(), Some("extrude"));
    assert_eq!(cut["operation"].as_str(), Some("cut"));
    assert_eq!(cut["extent"]["distance"].as_str(), Some("pocket_2_depth"));
    assert_eq!(cut["extent"]["start_offset"].as_str(), Some("extrude_0 + extrude_len - pocket_2_depth"), "cut from the top face down");
}

/// A pad is a box joined to the plate on its top face; a pocket a box cut from the top face
/// down, or from the bottom face up.
#[test]
fn pads_and_pockets_become_boxes() {
    let spec = PlateSpec {
        x: 60.0,
        y: 40.0,
        z: 6.0,
        holes: vec![],
        slots: vec![],
        pads: vec![PadSpec { x: 15.0, y: 20.0, w: 10.0, l: 8.0, h: 3.0 }],
        pockets: vec![
            PocketSpec { x: 40.0, y: 10.0, w: 12.0, l: 6.0, depth: 2.0, bottom: false },
            PocketSpec { x: 40.0, y: 30.0, w: 12.0, l: 6.0, depth: 1.5, bottom: true },
        ],
        bosses: vec![],
        cutouts: vec![],
        outline_fillet_r: None,
        edge_fillet_r: None,
    };
    let json: serde_json::Value = serde_json::from_str(&spec.to_design("").to_json()).expect("valid");
    let f = &json["features"];
    assert_eq!((f[3]["type"].as_str(), f[3]["operation"].as_str()), (Some("box"), Some("join")));
    assert_eq!(f[3]["corner"], serde_json::json!(["pad_1_x - pad_1_w / 2", "pad_1_y - pad_1_l / 2", "base_z0 + base_t"]));
    assert_eq!(f[4]["corner"][2], "base_z0 + base_t - pocket_1_depth");
    assert_eq!(f[5]["corner"][2], "base_z0");
    assert_eq!((f[5]["operation"].as_str(), f[5]["height"].as_str()), (Some("cut"), Some("pocket_2_depth")));
    assert!((spec.volume() - (60.0 * 40.0 * 6.0 + 240.0 - 144.0 - 108.0)).abs() < 1e-9);
    assert_eq!(spec.describe(), "A 60 x 40 x 6 mm plate with 1 pad and 2 pockets");
}

/// A boss is a cylinder joined to the plate, standing on the top face's expression.
#[test]
fn bosses_become_cylinders() {
    let spec = PlateSpec {
        x: 60.0,
        y: 40.0,
        z: 6.0,
        holes: vec![],
        slots: vec![],
        cutouts: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![BossSpec { x: 20.0, y: 15.0, d: 10.0, h: 4.0, fillet: None, draft_deg: None, chamfer: None }],
        outline_fillet_r: None,
        edge_fillet_r: None,
    };
    let json: serde_json::Value = serde_json::from_str(&spec.to_design_at("", [5.0, 0.0, 2.0]).to_json()).expect("valid");
    let boss = &json["features"][3];
    assert_eq!(boss["type"], "cylinder");
    assert_eq!(boss["base"], serde_json::json!(["boss_1_x", "boss_1_y", "base_z0 + base_t"]));
    assert_eq!(
        (boss["radius"].as_str(), boss["height"].as_str(), boss["operation"].as_str()),
        (Some("boss_1_d / 2"), Some("boss_1_h"), Some("join"))
    );
    assert!(json["params"].as_array().expect("params").iter().any(|p| p["name"] == "boss_1_x" && p["expr"] == "25 mm"));
    assert!((spec.volume() - (60.0 * 40.0 * 6.0 + std::f64::consts::PI * 25.0 * 4.0)).abs() < 1e-9);
    assert_eq!(spec.describe(), "A 60 x 40 x 6 mm plate with 1 boss");
}

/// The volume SolidCraft measured for the golden design (docs/design-format.md).
#[test]
fn plate_volume_matches_solidcraft() {
    let spec = PlateSpec {
        x: 70.0,
        y: 40.0,
        z: 6.0,
        holes: vec![HoleSpec { x: 35.0, y: 20.0, d: 8.0, head: None, blind: None }],
        cutouts: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![],
        slots: vec![],
        outline_fillet_r: Some(3.0),
        edge_fillet_r: None,
    };
    assert!((spec.volume() - 16_452.05).abs() < 0.01, "{}", spec.volume());
}

#[test]
fn describes_profiles() {
    let spec = ProfileSpec {
        corners: vec![[0.0, 0.0], [60.0, 0.0], [60.0, 15.0], [15.0, 15.0], [15.0, 40.0], [0.0, 40.0]],
        z: 5.0,
        holes: vec![HoleSpec { x: 7.5, y: 30.0, d: 5.0, head: None, blind: None }],
        cutouts: vec![],
        pads: vec![],
        pockets: vec![],
        outline_pockets: Vec::new(),
        bosses: vec![],
        slots: vec![],
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
        holes: vec![HoleSpec { x: 7.5, y: 30.0, d: 5.0, head: None, blind: None }],
        cutouts: vec![],
        pads: vec![],
        pockets: vec![],
        outline_pockets: Vec::new(),
        bosses: vec![],
        slots: vec![],
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
        holes: vec![HoleSpec { x: 16.0, y: 16.0, d: 0.4, head: None, blind: None }],
        cutouts: vec![],
        pads: vec![],
        pockets: vec![],
        outline_pockets: Vec::new(),
        bosses: vec![],
        slots: vec![],
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

/// A box is a prism along all three axes; Z is tried first, so it keeps mesh2cad's `base_*`
/// plate whatever way round it lies.
#[test]
fn box_long_in_x_stays_a_z_plate() {
    let found = recognize(&block(60.0, 10.0, 10.0, [0.0; 3]));
    let Recognition::Plate { plate, offset } = found else { panic!("not a plate: {found:?}") };
    assert_eq!((plate.x, plate.y, plate.z, offset), (60.0, 10.0, 10.0, [0.0; 3]));
}

/// A plate extruded along X (the mesh rotated by `(x, y, z) -> (y, z, x)`) is recognised as an
/// outline on the YZ plane: a profile with axis X, the rectangle's corners as its outline
/// (mesh2cad's extrusion module writes it so, not as a `base_*` plate), the hole's position
/// parameters named after the world axes Y and Z, and the design sketched on `YZ`, extruded
/// along +X and moved where the part is.
#[test]
fn x_extruded_plate_becomes_a_yz_profile() {
    let spec = PlateSpec {
        x: 40.0,
        y: 20.0,
        z: 60.0,
        holes: vec![HoleSpec { x: 10.0, y: 5.0, d: 6.0, head: None, blind: None }],
        cutouts: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![],
        slots: vec![],
        outline_fillet_r: None,
        edge_fillet_r: None,
    };
    let mesh = Axis::X.unrotate_mesh(&synth::plate(&spec, 32, [3.0, 7.0, 11.0]));
    let Recognition::Profile { profile, offset, axis } = recognize(&mesh) else { panic!("not recognised: {:?}", recognize(&mesh)) };
    assert_eq!(axis, Axis::X);
    // The profile's corner lies at the part's corner the rotation mapped to the origin's frame.
    assert_eq!(offset, [11.0, 3.0, 7.0]);
    assert_eq!(profile.corners, [[0.0, 0.0], [40.0, 0.0], [40.0, 20.0], [0.0, 20.0]]);
    assert_eq!(profile.z, 60.0);
    assert_eq!(profile.holes, [HoleSpec { x: 10.0, y: 5.0, d: 6.0, head: None, blind: None }]);
    // The mesh's faceted hole differs from the true circle by the smoothing the report notes.
    assert!((profile.volume() - mesh.volume()).abs() / mesh.volume() < 1e-3);

    let json: serde_json::Value = serde_json::from_str(&profile.to_design_on("", offset, axis).to_json()).expect("valid");
    let features = json["features"].as_array().expect("features");
    assert_eq!(features[0]["plane"]["name"], "YZ");
    assert_eq!(features[1]["extent"]["direction"], "positive");
    assert_eq!(features[2]["translate"], serde_json::json!(["extrude_0", "3", "7"]));
    let param = |name: &str| {
        json["params"].as_array().expect("params").iter().find(|p| p["name"] == name).map(|p| p["expr"].as_str().expect("expr"))
    };
    assert_eq!(param("extrude_len"), Some("60 mm"));
    assert_eq!(param("extrude_0"), Some("11 mm"));
    assert_eq!(param("hole_1_y"), Some("13 mm"));
    assert_eq!(param("hole_1_z"), Some("12 mm"));
    assert_eq!(param("hole_1_d"), Some("6 mm"));
    let hole = features.iter().find(|f| f["type"] == "hole").expect("a hole");
    assert_eq!(hole["direction"], serde_json::json!([-1.0, 0.0, 0.0]));
    assert_eq!(hole["at"], serde_json::json!(["extrude_0 + extrude_len", "hole_1_y", "hole_1_z"]));
    assert_eq!(hole["position"], serde_json::json!([71.0, 13.0, 12.0]));
}

/// A profile extruded along Y is sketched on the XZ plane (whose sketch y is -Z), extruded along
/// +Y, and its hole's parameters are named `hole_1_x` and `hole_1_z` in world coordinates.
#[test]
fn y_extruded_profile_uses_the_xz_plane() {
    let spec = ProfileSpec {
        corners: vec![[0.0, 0.0], [30.0, 0.0], [30.0, 10.0], [10.0, 10.0], [10.0, 25.0], [0.0, 25.0]],
        z: 50.0,
        holes: vec![HoleSpec { x: 5.0, y: 20.0, d: 4.0, head: None, blind: None }],
        cutouts: vec![],
        pads: vec![],
        pockets: vec![],
        outline_pockets: Vec::new(),
        bosses: vec![],
        slots: vec![],
        outline_fillet_r: Some(2.0),
    };
    let mesh = Axis::Y.unrotate_mesh(&synth::profile(&spec, 32, [3.0, 7.0, 11.0]));
    let Recognition::Profile { profile, offset, axis } = recognize(&mesh) else { panic!("not recognised: {:?}", recognize(&mesh)) };
    assert_eq!(axis, Axis::Y);
    // The profile's corner lies at the part's (min x, min y, max z).
    assert_eq!(offset, [3.0, 11.0, -7.0]);
    assert_eq!(profile.corners, spec.corners);
    assert_eq!(profile.z, 50.0);
    assert_eq!(profile.holes, spec.holes);
    assert_eq!(profile.outline_fillet_r, Some(2.0));

    let json: serde_json::Value = serde_json::from_str(&profile.to_design_on("", offset, axis).to_json()).expect("valid");
    let features = json["features"].as_array().expect("features");
    assert_eq!(features[0]["plane"]["name"], "XZ");
    assert_eq!(features[2]["translate"], serde_json::json!(["3", "extrude_0", "-7"]));
    let param = |name: &str| {
        json["params"].as_array().expect("params").iter().find(|p| p["name"] == name).map(|p| p["expr"].as_str().expect("expr"))
    };
    assert_eq!(param("extrude_0"), Some("11 mm"));
    assert_eq!(param("hole_1_x"), Some("8 mm"));
    assert_eq!(param("hole_1_z"), Some("-27 mm"));
    assert_eq!(param("outline_fillet_r"), Some("2 mm"));
    let hole = features.iter().find(|f| f["type"] == "hole").expect("a hole");
    assert_eq!(hole["direction"], serde_json::json!([0.0, -1.0, 0.0]));
    assert_eq!(hole["at"], serde_json::json!(["hole_1_x", "extrude_0 + extrude_len", "hole_1_z"]));
    assert_eq!(hole["position"], serde_json::json!([8.0, 61.0, -27.0]));
    // The fillet's edge picks are the corners' world points at mid-depth along Y.
    let fillet = features.iter().find(|f| f["type"] == "fillet").expect("a fillet");
    assert_eq!(fillet["edges"][0], serde_json::json!([3.0, 36.0, -7.0]));
    assert_eq!(fillet["edge_names"][0], "F2:side:l1|F2:side:l6");
}

/// The design the spike opened in SolidCraft 0.49: a plate with a centred hole and rounded
/// corners. The golden file was opened in SolidCraft and rebuilt without errors or warnings
/// after changing `base_x` and `hole_1_d`.
#[test]
fn plate_design_matches_golden_file() {
    let spec = PlateSpec {
        x: 70.0,
        y: 40.0,
        z: 6.0,
        holes: vec![HoleSpec { x: 35.0, y: 20.0, d: 8.0, head: None, blind: None }],
        cutouts: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![],
        slots: vec![],
        outline_fillet_r: Some(3.0),
        edge_fillet_r: None,
    };
    let actual = spec.to_design("plate-with-hole").to_json_pretty();
    let golden = include_str!("../../tests/data/plate-with-hole.json");
    assert_eq!(actual.trim(), golden.trim(), "regenerate with `cargo run -p revmesh-core --example plate_design`");
}

/// A box rounded all over (mesh2cad's `edge_fillet_r`) is a plate whose one fillet rounds all
/// twelve edges of the extrude — the four vertical ones and the eight on the top and bottom
/// caps — before the holes are cut, as mesh2cad's `.edges().fillet()` does.
#[test]
fn rounded_box_fillets_all_twelve_edges_before_the_holes() {
    let spec = PlateSpec {
        x: 60.0,
        y: 40.0,
        z: 30.0,
        holes: vec![HoleSpec { x: 30.0, y: 20.0, d: 6.0, head: None, blind: None }],
        cutouts: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![],
        slots: vec![],
        outline_fillet_r: None,
        edge_fillet_r: Some(5.0),
    };
    let json: serde_json::Value = serde_json::from_str(&spec.to_design_at("", [2.0, 3.0, 1.0]).to_json()).expect("valid");
    let features = json["features"].as_array().expect("features");
    let fillet = &features[3];
    assert_eq!(fillet["type"], "fillet");
    assert_eq!(features[4]["type"], "hole", "the hole is cut after the fillet");
    let names: Vec<&str> = fillet["edge_names"].as_array().expect("names").iter().filter_map(|n| n.as_str()).collect();
    assert_eq!(names.len(), 12);
    for name in ["F2:side:l1|F2:side:l4", "F2:side:l1|F2:side:l2", "F2:side:l2|F2:side:l3", "F2:side:l3|F2:side:l4"] {
        assert!(names.contains(&name), "{name}");
    }
    for i in 1..=4 {
        assert!(names.iter().any(|n| *n == format!("F2:side:l{i}|F2:top")), "l{i}|top");
        assert!(names.iter().any(|n| *n == format!("F2:side:l{i}|F2:start")), "l{i}|start");
    }
    assert_eq!(fillet["radius"], "edge_fillet_r");
    let edges = fillet["edges"].as_array().expect("edges");
    assert_eq!(edges[0], serde_json::json!([2.0, 3.0, 16.0]), "a vertical edge, mid-height");
    assert_eq!(edges[4], serde_json::json!([32.0, 3.0, 31.0]), "l1's middle on the top cap");
    assert_eq!(edges[5], serde_json::json!([32.0, 3.0, 1.0]), "l1's middle on the start cap");
    assert!(json["params"].as_array().expect("params").iter().any(|p| p["name"] == "edge_fillet_r" && p["expr"] == "5 mm"));
    let removed =
        4.0 * 25.0 * (1.0 - std::f64::consts::PI / 4.0) * (60.0 + 40.0 + 30.0 - 30.0) + 8.0 * 125.0 * (1.0 - std::f64::consts::PI / 6.0);
    let expected = 60.0 * 40.0 * 30.0 - removed - std::f64::consts::PI * 3.0 * 3.0 * 30.0;
    assert!((spec.volume() - expected).abs() < 1e-9, "{} vs {expected}", spec.volume());
    assert_eq!(spec.describe(), "A 60 x 40 x 30 mm plate with 1 hole and 5 mm rounded edges");
}

/// On a Y extrusion the XZ plane's sketch y is -Z: the slot's middle point and the bolt circle's
/// seed are placed by expressions in world parameters, and the second profile coordinate carries
/// a minus sign there, so a rebuild lands where the numeric picks say. (On X and Z both profile
/// signs are +1 and the expressions keep the plus.)
#[test]
fn y_extrusion_negates_the_second_profile_coordinate_in_expressions() {
    let holes = |cx: f64, cy: f64| {
        (0..4)
            .map(|k| {
                let a = std::f64::consts::FRAC_PI_2 * f64::from(k);
                HoleSpec { x: cx + 12.0 * a.cos(), y: cy + 12.0 * a.sin(), d: 4.0, head: None, blind: None }
            })
            .collect::<Vec<_>>()
    };
    let spec = ProfileSpec {
        corners: vec![[0.0, 0.0], [40.0, 0.0], [40.0, 30.0], [0.0, 30.0]],
        z: 50.0,
        holes: holes(20.0, 15.0),
        cutouts: vec![],
        pads: vec![],
        pockets: vec![],
        outline_pockets: Vec::new(),
        bosses: vec![],
        slots: vec![SlotSpec { x: 10.0, y: 8.0, len: 16.0, w: 5.0, angle: 30.0 }],
        outline_fillet_r: None,
    };
    let at = [3.0, 11.0, -7.0];
    let json: serde_json::Value = serde_json::from_str(&spec.to_design_on("", at, Axis::Y).to_json()).expect("valid");
    let features = json["features"].as_array().expect("features");
    // The slot's middle point: sketch (u, v) = (slot_1_x, -slot_1_z), placed at [13, 15].
    let slot_sketch = features.iter().find(|f| f["name"] == "Slot1Sketch").expect("slot sketch");
    let p6 = &slot_sketch["sketch"]["points"][8];
    assert_eq!(p6["exprs"]["x"], "slot_1_x");
    assert_eq!(p6["exprs"]["y"], "-slot_1_z");
    assert_eq!(p6["pos"], serde_json::json!([13.0, 15.0]));
    let param = |name: &str| {
        json["params"].as_array().expect("params").iter().find(|p| p["name"] == name).map(|p| p["expr"].as_str().expect("expr"))
    };
    assert_eq!(param("slot_1_x"), Some("13 mm"));
    assert_eq!(param("slot_1_z"), Some("-15 mm"));
    // The bolt circle's seed: world (x, y, z) expressions, the z one with the minus sign.
    let seed = features.iter().find(|f| f["name"] == "HoleCircle1").expect("circle seed");
    assert_eq!(
        seed["at"],
        serde_json::json!([
            "hole_circle_1_cx + hole_circle_1_r * cos(hole_circle_1_angle)",
            "extrude_0 + extrude_len",
            "hole_circle_1_cz - hole_circle_1_r * sin(hole_circle_1_angle)"
        ])
    );
    assert_eq!(seed["position"], serde_json::json!([35.0, 61.0, -22.0]));
    assert_eq!(param("hole_circle_1_cx"), Some("23 mm"));
    assert_eq!(param("hole_circle_1_cz"), Some("-22 mm"));

    // On X both profile signs are +1: the same expressions keep the plus.
    let json: serde_json::Value = serde_json::from_str(&spec.to_design_on("", [11.0, 3.0, 7.0], Axis::X).to_json()).expect("valid");
    let features = json["features"].as_array().expect("features");
    let seed = features.iter().find(|f| f["name"] == "HoleCircle1").expect("circle seed");
    assert_eq!(
        seed["at"],
        serde_json::json!([
            "extrude_0 + extrude_len",
            "hole_circle_1_cy + hole_circle_1_r * cos(hole_circle_1_angle)",
            "hole_circle_1_cz + hole_circle_1_r * sin(hole_circle_1_angle)"
        ])
    );
    let slot_sketch = features.iter().find(|f| f["name"] == "Slot1Sketch").expect("slot sketch");
    let p6 = &slot_sketch["sketch"]["points"][8];
    assert_eq!(p6["exprs"]["x"], "slot_1_y");
    assert_eq!(p6["exprs"]["y"], "slot_1_z");
}

/// A drafted boss is a cone narrowing by its angle; a chamfered one a cylinder and a chamfer of
/// its top edge, one distance for 45°, two otherwise (along the top first).
#[test]
fn drafted_and_chamfered_bosses_in_the_design() {
    use super::spec::BossChamfer;
    let boss = |x: f64| BossSpec { x, y: 20.0, d: 10.0, h: 6.0, fillet: None, draft_deg: None, chamfer: None };
    let spec = PlateSpec {
        x: 80.0,
        y: 40.0,
        z: 6.0,
        holes: vec![],
        slots: vec![],
        bosses: vec![
            BossSpec { draft_deg: Some(5.0), ..boss(15.0) },
            BossSpec { chamfer: Some(BossChamfer { w: 1.0, h: 1.0 }), ..boss(40.0) },
            BossSpec { chamfer: Some(BossChamfer { w: 1.5, h: 0.5 }), ..boss(65.0) },
        ],
        pads: vec![],
        pockets: vec![],
        cutouts: vec![],
        outline_fillet_r: None,
        edge_fillet_r: None,
    };
    let json: serde_json::Value = serde_json::from_str(&spec.to_design("").to_json()).expect("valid");
    let features = json["features"].as_array().expect("features");
    let named = |name: &str| features.iter().find(|f| f["name"] == name).expect(name).clone();
    assert_eq!(named("Boss1")["type"], "cone");
    assert_eq!(named("Boss1")["top_radius"], "boss_1_d / 2 - boss_1_h * tan(boss_1_draft_deg)");
    assert_eq!(named("Boss2Chamfer")["distance"], "boss_2_chamfer");
    assert_eq!(
        (named("Boss3Chamfer")["distance"].as_str(), named("Boss3Chamfer")["distance2"].as_str()),
        (Some("boss_3_chamfer_w"), Some("boss_3_chamfer_h"))
    );
    let params: Vec<&str> = json["params"].as_array().expect("params").iter().filter_map(|p| p["name"].as_str()).collect();
    for name in ["boss_1_draft_deg", "boss_2_chamfer", "boss_3_chamfer_h", "boss_3_chamfer_w"] {
        assert!(params.contains(&name), "{name}");
    }
    let r = 5.0_f64;
    let top = r - 6.0 * 5.0_f64.to_radians().tan();
    let cone = std::f64::consts::PI * 6.0 / 3.0 * (r * r + r * top + top * top);
    let cylinder = std::f64::consts::PI * 25.0 * 6.0;
    let cut = |w: f64, h: f64| 2.0 * std::f64::consts::PI * (r - w / 3.0) * w * h / 2.0;
    assert!((spec.volume() - (80.0 * 40.0 * 6.0 + cone + 2.0 * cylinder - cut(1.0, 1.0) - cut(1.5, 0.5))).abs() < 1e-9);
}
