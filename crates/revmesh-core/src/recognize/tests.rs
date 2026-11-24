use super::spec::{BossSpec, HoleSpec, PadSpec, PlateSpec, PocketSpec, ProfileSpec, SlotSpec};
use super::{Axis, Recognition, recognize};

use crate::design::Design;
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

/// The design the spike opened in SolidCraft 0.48: a plate with a centred hole and rounded
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
