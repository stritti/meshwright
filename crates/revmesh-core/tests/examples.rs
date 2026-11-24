//! The example parts in `examples/` (written by `cargo run -p revmesh-core --example
//! examples_stl -- examples`) import as the editable designs the docs' gallery
//! (`docs/guide/examples.md`) shows: the summary, the features in the timeline and the
//! parameters a user can change.

use revmesh_core::{ImportOptions, import};

/// Imports `examples/<name>.stl` and returns the summary, the feature names and the user
/// parameters as `name value`.
fn imported(name: &str) -> (String, Vec<String>, Vec<String>) {
    let path = format!("{}/../../examples/{name}.stl", env!("CARGO_MANIFEST_DIR"));
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let result = import(&bytes, &format!("{name}.stl"), &ImportOptions::default()).expect("imports");
    let json: serde_json::Value = serde_json::from_str(&result.design.to_json_pretty()).expect("design JSON");
    let features = json["features"].as_array().expect("features").iter().map(|f| f["name"].as_str().expect("name").to_owned()).collect();
    let params = result.design.params.iter().filter(|p| !p.model).map(|p| format!("{} {}", p.name, p.expr)).collect();
    (result.report.summary, features, params)
}

fn assert_example(name: &str, summary: &str, features: &[&str], params: &[&str]) {
    let (got_summary, got_features, got_params) = imported(name);
    assert_eq!(got_summary, summary, "{name}: summary");
    assert_eq!(got_features, features, "{name}: features");
    for param in params {
        assert!(got_params.iter().any(|p| p == param), "{name}: no parameter `{param}` in {got_params:?}");
    }
}

#[test]
fn sbc_mount() {
    assert_example(
        "sbc-mount",
        "A 95 x 66 x 3 mm plate with 8 holes and 4 mm rounded corners",
        &["Sketch1", "Extrude1", "Move1", "HoleGrid1", "HoleGrid1Pattern", "HoleGrid2", "HoleGrid2Pattern", "Fillet1"],
        &["base_x 95 mm", "base_t 3 mm", "hole_grid_2_pitch_x 58 mm", "hole_grid_2_pitch_y 49 mm", "hole_grid_2_d 2.7 mm", "fillet_r 4 mm"],
    );
}

#[test]
fn nema17_motor_plate() {
    assert_example(
        "nema17-motor-plate",
        "A 60 x 60 x 5 mm plate with 9 holes and 5 mm rounded corners",
        &["Sketch1", "Extrude1", "Move1", "HoleGrid1", "HoleGrid1Pattern", "HoleGrid2", "HoleGrid2Pattern", "Hole1", "Fillet1"],
        &["hole_grid_2_pitch_x 31 mm", "hole_grid_2_d 3.4 mm", "hole_1_d 22.5 mm", "fillet_r 5 mm"],
    );
}

#[test]
fn square_flange() {
    assert_example(
        "square-flange",
        "A 80 x 80 x 8 mm plate with 7 holes and 8 mm rounded corners",
        &["Sketch1", "Extrude1", "Move1", "HoleCircle1", "HoleCircle1Pattern", "Hole1", "Fillet1"],
        &["hole_circle_1_n 6", "hole_circle_1_r 27.5 mm", "hole_circle_1_d 6.6 mm", "hole_1_d 30 mm"],
    );
}

#[test]
fn corner_plate() {
    assert_example(
        "corner-plate",
        "A 70 x 50 x 4 mm plate with a 6-sided outline, 4 holes and 3 mm rounded corners",
        &["Sketch1", "Extrude1", "Move1", "Fillet1", "HoleRow1", "HoleRow1Pattern", "Hole1"],
        &["extrude_len 4 mm", "hole_row_1_n 3", "hole_row_1_pitch 22 mm", "outline_fillet_r 3 mm"],
    );
}

#[test]
fn chamfered_cover() {
    assert_example(
        "chamfered-cover",
        "A 80 x 50 x 4 mm plate with a 8-sided outline and 5 holes",
        &["Sketch1", "Extrude1", "Move1", "HoleRow1", "HoleRow1Pattern", "Hole1"],
        &["extrude_len 4 mm", "hole_row_1_n 4", "hole_row_1_pitch 20 mm", "hole_1_d 12 mm"],
    );
}

#[test]
fn spacer_bar() {
    assert_example(
        "spacer-bar",
        "A 20 x 12 x 70 mm plate with a 4-sided outline and 1 hole",
        &["Sketch1", "Extrude1", "Move1", "Hole1"],
        &["extrude_len 70 mm", "extrude_0 15 mm", "hole_1_d 6.5 mm"],
    );
}

#[test]
fn rounded_block() {
    assert_example(
        "rounded-block",
        "A 60 x 40 x 25 mm plate with 4 mm rounded edges",
        &["Sketch1", "Extrude1", "Move1", "Fillet1"],
        &["base_x 60 mm", "base_t 25 mm", "edge_fillet_r 4 mm"],
    );
}
