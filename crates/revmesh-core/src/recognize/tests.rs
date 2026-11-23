use super::spec::{BossSpec, HoleSpec, PadSpec, PlateSpec, PocketSpec, ProfileSpec, SlotSpec};
use super::{Recognition, recognize};

use crate::design::Design;
use crate::mesh::Mesh;
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
    };
    let json: serde_json::Value = serde_json::from_str(&spec.to_design_at("", [5.0, 0.0, 0.0]).to_json()).expect("valid");
    let features = json["features"].as_array().expect("features");
    let names: Vec<&str> = features.iter().filter_map(|f| f["name"].as_str()).collect();
    assert_eq!(names, ["Sketch1", "Extrude1", "Move1", "CutoutsSketch", "Cutouts"]);
    let sketch = &features[3]["sketch"];
    assert_eq!(sketch["curves"].as_array().map(Vec::len), Some(3));
    assert_eq!(sketch["points"][1]["pos"], serde_json::json!([15.0, 10.0]));
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
        bosses: vec![],
        cutouts: vec![vec![[15.8, 15.8], [16.2, 15.8], [16.0, 16.2]]],
        outline_fillet_r: Some(5.0),
    };
    let json: serde_json::Value = serde_json::from_str(&ell.to_design("").to_json()).expect("valid");
    let names: Vec<String> =
        json["features"].as_array().expect("features").iter().filter_map(|f| f["name"].as_str().map(String::from)).collect();
    assert_eq!(names, ["Sketch1", "Extrude1", "Move1", "Fillet1", "CutoutsSketch", "Cutouts", "Hole1"]);
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
        bosses: vec![BossSpec { x: 20.0, y: 15.0, d: 10.0, h: 4.0 }],
        outline_fillet_r: None,
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

/// The design the spike opened in SolidCraft 0.45: a plate with a centred hole and rounded
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
    };
    let actual = spec.to_design("plate-with-hole").to_json_pretty();
    let golden = include_str!("../../tests/data/plate-with-hole.json");
    assert_eq!(actual.trim(), golden.trim(), "regenerate with `cargo run -p revmesh-core --example plate_design`");
}
