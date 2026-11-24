//! Real STL files from Printables (`plugin/revmesh/tests/printables/`, licences and sources in
//! its README; the `printables-*.json` cases beside it run them in SolidCraft):
//! parts other people designed and exported, not generated here. Where the author published the
//! source (a DXF or an OpenSCAD file), the expected parameters are that source's numbers, so the
//! test checks revmesh against the design, not against itself.
//!
//! The files revmesh keeps as a mesh are here too, with the reason it gives: they are the next
//! recognisers' test cases, and their assertions change when a recogniser learns them.

use revmesh_core::{ImportOptions, import};

/// Imports `tests/printables/<name>.stl`: the summary, the feature names, the user parameters as
/// `name expression` and the report's item messages.
fn imported(name: &str) -> (String, Vec<String>, Vec<String>, Vec<String>) {
    let path = format!("{}/../../plugin/revmesh/tests/printables/{name}.stl", env!("CARGO_MANIFEST_DIR"));
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let result = import(&bytes, &format!("{name}.stl"), &ImportOptions::default()).expect("imports");
    let json: serde_json::Value = serde_json::from_str(&result.design.to_json_pretty()).expect("design JSON");
    let features = json["features"].as_array().expect("features").iter().map(|f| f["name"].as_str().expect("name").to_owned()).collect();
    let params = result.design.params.iter().filter(|p| !p.model).map(|p| format!("{} {}", p.name, p.expr)).collect();
    let items = result.report.items.iter().map(|i| i.message.clone()).collect();
    (result.report.summary, features, params, items)
}

fn assert_part(name: &str, summary: &str, features: &[&str], params: &[&str]) {
    let (got_summary, got_features, got_params, _) = imported(name);
    assert_eq!(got_summary, summary, "{name}: summary");
    assert_eq!(got_features, features, "{name}: features");
    for param in params {
        assert!(got_params.iter().any(|p| p == param), "{name}: no parameter `{param}` in {got_params:?}");
    }
}

/// diegotrap's NEMA 17 plate for aluminium extrusion (CC BY 4.0). Every number is the author's
/// DXF's: a 42 x 80 outline with 2 mm chamfers, the Ø24 pilot bore, four Ø3.2 holes on 31 mm and
/// two slots 5.25 wide with their ends' centres 24 mm apart, 10 mm either side of the middle.
#[test]
fn nema17_extrusion_plate_matches_the_authors_dxf() {
    assert_part(
        "nema17-extrusion-plate",
        "A 42 x 80 x 4 mm plate with a 8-sided outline, 5 holes and 2 slots",
        &["Sketch1", "Extrude1", "Move1", "HoleGrid1", "HoleGrid1Pattern", "Hole1", "Slot1Sketch", "Slot1", "Slot2Sketch", "Slot2"],
        &[
            "extrude_len 4 mm",
            "hole_grid_1_pitch_x 31 mm",
            "hole_grid_1_pitch_y 31 mm",
            "hole_grid_1_d 3.2 mm",
            "hole_1_x 0 mm",
            "hole_1_y 0 mm",
            "hole_1_d 24 mm",
            "slot_1_len 24 mm",
            "slot_1_w 5.25 mm",
            "slot_1_x -10 mm",
            "slot_1_y -37.5 mm",
            "slot_2_x 10 mm",
        ],
    );
}

/// Andy's simple tiny calibration block (CC0). The author's OpenSCAD file: a 30 x 20 x 10 cube
/// less a 20 x 10 pocket 7 deep, 5 mm in from two sides.
#[test]
fn tiny_calibration_block_matches_the_authors_scad() {
    assert_part(
        "tiny-calibration-block",
        "A 30 x 20 x 10 mm plate with 1 pocket",
        &["Sketch1", "Extrude1", "Move1", "Pocket1"],
        &[
            "base_x 30 mm",
            "base_y 20 mm",
            "base_t 10 mm",
            "pocket_1_w 20 mm",
            "pocket_1_l 10 mm",
            "pocket_1_depth 7 mm",
            "pocket_1_x 15 mm",
            "pocket_1_y 10 mm",
        ],
    );
}

/// derfley's 2 mm NEMA 17 spacer plate (CC BY 4.0): the motor's 42.3 mm square with chamfered
/// corners, its four holes on the 31 mm square and a Ø27 opening for the pilot.
#[test]
fn nema17_spacer_plate() {
    assert_part(
        "nema17-spacer-plate",
        "A 42.3 x 42.3 x 2 mm plate with a 8-sided outline and 5 holes",
        &["Sketch1", "Extrude1", "Move1", "HoleGrid1", "HoleGrid1Pattern", "Hole1"],
        &["extrude_len 2 mm", "hole_grid_1_pitch_x 31 mm", "hole_grid_1_d 3.5 mm", "hole_1_d 27 mm"],
    );
}

/// The calibration block by `TechnoSwiss` (CC BY 4.0). Its title says countersunk, but the mesh has a
/// flat step 2 mm down at Ø5.9: a counterbore, which is what revmesh makes of it. The slot is a
/// rectangular pocket 5 mm deep.
#[test]
fn calibration_cube_has_a_counterbore_and_a_pocket() {
    assert_part(
        "calibration-cube",
        "A 20 x 20 x 10 mm plate with 1 hole and 1 pocket",
        &["Sketch1", "Extrude1", "Move1", "Hole1", "Pocket1"],
        &["base_x 20 mm", "base_t 10 mm", "hole_1_cbore_depth 2 mm", "pocket_1_w 5.5 mm", "pocket_1_l 2.4 mm", "pocket_1_depth 5 mm"],
    );
}

/// Ken S's 90° angle bracket (CC0). The author's OpenSCAD file: 30 wide, a 40 x 5 base and a
/// 40 high upright with a gusset (one outline seen from the side, extruded 30 along X), and two
/// Ø10 holes, one in the base at (15, 15) and one in the upright 25 up (`hole_offset`).
#[test]
fn angle_bracket_matches_the_authors_scad() {
    assert_part(
        "angle-bracket",
        "A 30 x 40 x 40 mm part of outlines extruded along X with 2 holes",
        &["Sketch1", "Extrude1", "Hole1", "Hole2", "Move1"],
        &[
            "extrude_x_len 30 mm",
            "hole_1_d 10 mm",
            "hole_1_x 15 mm",
            "hole_1_z 25 mm",
            "hole_2_d 10 mm",
            "hole_2_x 15 mm",
            "hole_2_y 15 mm",
        ],
    );
}

/// Gwilherm Kerherve's mending plate (CC0): countersinks that taper through the whole 3 mm
/// plate. Not recognised yet.
#[test]
fn mending_plate_stays_a_mesh() {
    let (summary, _, params, items) = imported("mending-plate");
    assert_eq!(summary, "A mesh of 1052 triangles");
    assert_eq!(params, Vec::<String>::new());
    assert!(items.iter().any(|m| m.contains("tapered holes are not recognised yet")), "{items:?}");
}

/// Mateusz Tokarz's flat mending plate 4 x 20 x 60 (CC BY 4.0): its top edges are rounded all
/// round, so the top and bottom faces differ. Not recognised yet.
#[test]
fn flat_mending_plate_stays_a_mesh() {
    let (summary, _, params, items) = imported("flat-mending-plate-4x20x60");
    assert_eq!(summary, "A mesh of 1248 triangles");
    assert_eq!(params, Vec::<String>::new());
    assert!(items.iter().any(|m| m.contains("top and bottom faces differ")), "{items:?}");
}
