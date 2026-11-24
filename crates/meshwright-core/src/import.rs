//! The whole import: STL bytes in, design and loss report out.

use serde::Deserialize;

use crate::design::{Design, DesignBuilder, FeatureKind, MeshData};
use crate::mesh::Mesh;
use crate::recognize::{Recognition, recognize};
use crate::report::{LossItem, LossReport, Severity};
use crate::stl::{self, StlError};

/// Volume changes from smoothing facets below this fraction are not worth a line in the report.
const SMOOTHING_NOTE: f64 = 1e-4;

/// What the user chose in the import dialog (the entry's `input_schema` in `plugin.json`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ImportOptions {
    /// What to make of the mesh.
    pub mode: Mode,
}

impl ImportOptions {
    /// The options from the JSON SolidCraft passes; `""` and `{}` mean the defaults.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        if json.trim().is_empty() { Ok(Self::default()) } else { serde_json::from_str(json) }
    }
}

/// What to make of the mesh.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Recognise features, and keep the mesh where nothing is recognised.
    #[default]
    Auto,
    /// Keep the mesh as a mesh body, without recognition.
    Mesh,
}

/// A finished import.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportResult {
    /// The design to open.
    pub design: Design,
    /// What could not be carried over.
    pub report: LossReport,
}

/// Imports the STL file `file_name` whose content is `bytes`.
pub fn import(bytes: &[u8], file_name: &str, options: &ImportOptions) -> Result<ImportResult, StlError> {
    Ok(import_mesh(stl::parse(bytes)?, file_name, options))
}

/// Imports a mesh already read from the STL file `file_name` (see [`stl::StlReader`]). The mesh
/// moves into the design when it is kept as a mesh body, so it is never copied.
#[must_use]
pub fn import_mesh(mesh: Mesh, file_name: &str, options: &ImportOptions) -> ImportResult {
    let body = body_name(file_name);
    if options.mode == Mode::Auto {
        let recognised = match recognize(&mesh) {
            Recognition::Plate { plate, offset } => Ok((plate.to_design_at("", offset), plate.describe(), plate.volume())),
            Recognition::Profile { profile, offset } => Ok((profile.to_design_at("", offset), profile.describe(), profile.volume())),
            Recognition::Unrecognized { reason } => Err(reason),
        };
        match recognised {
            Ok((design, summary, volume)) => {
                let mut report = LossReport::new(summary);
                let change = volume / mesh.volume() - 1.0;
                if change.abs() > SMOOTHING_NOTE {
                    report.push(LossItem {
                        severity: Severity::Info,
                        title: "Facets smoothed".into(),
                        subject: body,
                        message: format!(
                            "Holes and rounded corners are rebuilt as true circles, so the volume changes by {:+.2}% from the mesh's facets.",
                            change * 100.0
                        ),
                        code: "meshwright.smoothed",
                    });
                }
                return ImportResult { design, report };
            }
            Err(reason) => {
                let mut result = mesh_import(mesh, file_name, &body);
                result.report.push(LossItem {
                    severity: Severity::Degraded,
                    title: "Kept as a mesh".into(),
                    subject: body,
                    message: format!("No parametric features were recognised: {reason}."),
                    code: "meshwright.unrecognized",
                });
                return result;
            }
        }
    }
    mesh_import(mesh, file_name, &body)
}

fn mesh_import(mesh: Mesh, file_name: &str, body: &str) -> ImportResult {
    let report = LossReport::new(format!("A mesh of {} triangles", mesh.triangles.len()));
    let mut b = DesignBuilder::new("");
    b.feature(body, vec![body.to_owned()], FeatureKind::MeshImport { file: file_name.to_owned(), meshes: vec![MeshData::new(body, mesh)] });
    ImportResult { design: b.build(), report }
}

/// The body name SolidCraft would give: the file name without its extension.
fn body_name(file_name: &str) -> String {
    let stem = file_name.rsplit_once('.').map_or(file_name, |(stem, _)| stem);
    if stem.is_empty() { "Mesh".into() } else { stem.into() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recognize::Axis;

    const TETRA: &[u8] = include_bytes!("../tests/data/tetra.stl");

    #[test]
    fn options_default_when_empty() {
        assert_eq!(ImportOptions::from_json("").expect("ok"), ImportOptions::default());
        assert_eq!(ImportOptions::from_json("{}").expect("ok"), ImportOptions::default());
        assert_eq!(ImportOptions::from_json(r#"{"mode":"mesh"}"#).expect("ok").mode, Mode::Mesh);
        assert!(ImportOptions::from_json(r#"{"colour":1}"#).is_err());
    }

    #[test]
    fn unrecognised_mesh_is_kept_and_reported() {
        let result = import(TETRA, "tetra.stl", &ImportOptions::default()).expect("imports");
        let json: serde_json::Value = serde_json::from_str(&result.design.to_json()).expect("valid");
        assert_eq!(json["features"][0]["type"], "mesh_import");
        assert_eq!(json["features"][0]["body_names"][0], "tetra");
        assert_eq!(json["features"][0]["meshes"][0]["triangles"].as_array().map(Vec::len), Some(12));
        assert_eq!(result.report.items[0].code, "meshwright.unrecognized");
    }

    #[test]
    fn mesh_mode_reports_nothing_lost() {
        let result = import(TETRA, "tetra.stl", &ImportOptions { mode: Mode::Mesh }).expect("imports");
        assert_eq!(result.report.items, []);
    }

    #[test]
    fn plate_import_writes_history_and_notes() {
        use crate::recognize::{HoleSpec, PlateSpec};
        let spec = PlateSpec {
            x: 60.0,
            y: 40.0,
            z: 5.0,
            holes: vec![HoleSpec { x: 30.0, y: 20.0, d: 10.0, head: None, blind: None }],
            cutouts: vec![],
            pads: vec![],
            pockets: vec![],
            bosses: vec![],
            slots: vec![],
            outline_fillet_r: Some(4.0),
        };
        let bytes = stl::write_binary(&crate::synth::plate(&spec, 24, [-30.0, -20.0, 0.0]));
        let result = import(&bytes, "bracket.stl", &ImportOptions::default()).expect("imports");
        let json: serde_json::Value = serde_json::from_str(&result.design.to_json()).expect("valid");
        let types: Vec<&str> = json["features"].as_array().expect("features").iter().filter_map(|f| f["type"].as_str()).collect();
        assert_eq!(types, ["sketch", "extrude", "move", "hole", "fillet"]);
        assert_eq!(result.report.summary, "A 60 x 40 x 5 mm plate with 1 hole and 4 mm rounded corners");
        let codes: Vec<&str> = result.report.items.iter().map(|i| i.code).collect();
        assert_eq!(codes, ["meshwright.smoothed"]);
        // The plate stays where it is in the file: centred on the origin, as mesh2cad writes it.
        let params: Vec<(&str, &str)> =
            result.design.params.iter().filter(|p| !p.model).map(|p| (p.name.as_str(), p.expr.as_str())).collect();
        assert_eq!(
            params,
            [
                ("base_x", "60 mm"),
                ("base_y", "40 mm"),
                ("base_t", "5 mm"),
                ("base_cx", "0 mm"),
                ("base_cy", "0 mm"),
                ("base_z0", "0 mm"),
                ("hole_1_x", "0 mm"),
                ("hole_1_y", "0 mm"),
                ("hole_1_d", "10 mm"),
                ("fillet_r", "4 mm"),
            ]
        );
        assert_eq!(json["features"][2]["translate"], serde_json::json!(["base_cx - base_x / 2", "base_cy - base_y / 2", "base_z0"]));
        assert_eq!(json["features"][3]["position"], serde_json::json!([0.0, 0.0, 5.0]));
        assert_eq!(json["features"][4]["edges"][0], serde_json::json!([-30.0, -20.0, 2.5]));
    }

    /// Counterbores and countersinks become SolidCraft's own hole kinds with mesh2cad's names; a
    /// counterbore on the bottom face is drilled up from there.
    #[test]
    fn stepped_holes_become_hole_kinds() {
        let bytes = include_bytes!("../../../plugin/meshwright/tests/stepped-holes.stl");
        let result = import(bytes, "stepped-holes.stl", &ImportOptions::default()).expect("imports");
        let json: serde_json::Value = serde_json::from_str(&result.design.to_json()).expect("valid");
        let holes: Vec<&serde_json::Value> =
            json["features"].as_array().expect("features").iter().filter(|f| f["type"] == "hole").collect();
        assert_eq!(
            holes[0]["hole"],
            serde_json::json!({"type": "counterbore", "cb_diameter": "hole_1_cbore_d", "cb_depth": "hole_1_cbore_depth"})
        );
        assert_eq!(holes[1]["direction"], serde_json::json!([0.0, 0.0, 1.0]));
        assert_eq!(holes[1]["at"][2], "base_z0");
        assert_eq!(
            holes[2]["hole"],
            serde_json::json!({"type": "countersink", "cs_diameter": "hole_3_csk_d", "cs_angle": "hole_3_csk_angle"})
        );
        let param = |name: &str| result.design.params.iter().find(|p| p.name == name).map(|p| p.expr.as_str());
        assert_eq!(param("hole_2_cbore_depth"), Some("2.5 mm"));
        assert_eq!(param("hole_3_csk_angle"), Some("90 deg"));
    }

    /// The plugin test's pattern plate: a grid, a row and a bolt circle become one hole and a
    /// pattern feature each, the bore stays a single hole.
    #[test]
    fn hole_patterns_become_pattern_features() {
        let bytes = include_bytes!("../../../plugin/meshwright/tests/plate-patterns.stl");
        let result = import(bytes, "plate-patterns.stl", &ImportOptions::default()).expect("imports");
        let json: serde_json::Value = serde_json::from_str(&result.design.to_json()).expect("valid");
        let names: Vec<&str> = json["features"].as_array().expect("features").iter().filter_map(|f| f["name"].as_str()).collect();
        assert_eq!(
            names,
            [
                "Sketch1",
                "Extrude1",
                "Move1",
                "HoleGrid1",
                "HoleGrid1Pattern",
                "HoleRow1",
                "HoleRow1Pattern",
                "HoleCircle1",
                "HoleCircle1Pattern",
                "Hole1",
                "Fillet1"
            ]
        );
        let grid = &json["features"][4];
        assert_eq!(grid["features"], serde_json::json!([4]));
        assert_eq!(grid["pattern"]["count2"], "hole_grid_1_ny");
        let circle = &json["features"][8]["pattern"];
        assert_eq!(circle["type"], "circular");
        assert_eq!(circle["origin"], serde_json::json!(["hole_circle_1_cx", "hole_circle_1_cy", "0"]));
        let param = |name: &str| result.design.params.iter().find(|p| p.name == name).map(|p| p.expr.as_str());
        assert_eq!(param("hole_grid_1_nx"), Some("2"));
        assert_eq!(param("hole_row_1_pitch"), Some("15 mm"));
        assert_eq!(param("hole_circle_1_angle"), Some("15 deg"));
        assert_eq!(param("hole_1_d"), Some("10 mm"));
    }

    #[test]
    fn profile_import_keeps_its_place() {
        use crate::recognize::{HoleSpec, ProfileSpec};
        let spec = ProfileSpec {
            corners: vec![[0.0, 0.0], [60.0, 0.0], [60.0, 15.0], [15.0, 15.0], [15.0, 40.0], [0.0, 40.0]],
            z: 5.0,
            holes: vec![HoleSpec { x: 7.5, y: 30.0, d: 5.0, head: None, blind: None }],
            cutouts: vec![],
            pads: vec![],
            pockets: vec![],
            bosses: vec![],
            slots: vec![],
            axis: Axis::Z,
            outline_fillet_r: None,
        };
        let bytes = stl::write_binary(&crate::synth::profile(&spec, 32, [20.0, 10.0, 3.0]));
        let result = import(&bytes, "l.stl", &ImportOptions::default()).expect("imports");
        let json: serde_json::Value = serde_json::from_str(&result.design.to_json()).expect("valid");
        let names: Vec<&str> = result.design.params.iter().filter(|p| !p.model).map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["extrude_len", "extrude_0", "hole_1_x", "hole_1_y", "hole_1_d"]);
        assert_eq!(result.design.params[1].expr, "3 mm");
        assert_eq!(result.design.params[2].expr, "27.5 mm");
        assert_eq!(json["features"][2]["translate"], serde_json::json!(["20", "10", "extrude_0"]));
        assert_eq!(json["features"][3]["at"], serde_json::json!(["hole_1_x", "hole_1_y", "extrude_0 + extrude_len"]));
    }

    /// Keeps the STL fixture of SolidCraft's plugin test and its expectations in step.
    #[test]
    fn plugin_test_plate_fixture() {
        let case: serde_json::Value =
            serde_json::from_str(include_str!("../../../plugin/meshwright/tests/plate-holes.json")).expect("valid case");
        let bytes = include_bytes!("../../../plugin/meshwright/tests/plate-holes.stl");
        let result = import(bytes, "plate-holes.stl", &ImportOptions::default()).expect("imports");
        let Recognition::Plate { plate, .. } = recognize(&stl::parse(bytes).expect("parses")) else { panic!("not a plate") };
        let expect = &case["expect"];
        assert_eq!(result.design.features.len() as u64, expect["reply"]["features"].as_u64().expect("count"));
        assert_eq!(result.report.summary, expect["reply"]["loss_report"]["summary"].as_str().expect("summary"));
        let volume = expect["model"]["total"]["volume_mm3"].as_f64().expect("volume");
        assert!((plate.volume() - volume).abs() < 1e-6 * volume, "{} vs {volume}", plate.volume());
    }

    #[test]
    fn streamed_design_matches_and_writes_single_precision() {
        let result = import(TETRA, "tetra.stl", &ImportOptions { mode: Mode::Mesh }).expect("imports");
        let mut streamed = Vec::new();
        result.design.write_json(&mut streamed).expect("writes");
        assert_eq!(String::from_utf8(streamed).expect("utf-8"), result.design.to_json());

        let mesh = Mesh::new(vec![[[0.1, 0.0, 0.0], [0.0, 0.2, 0.0], [0.0, 0.0, 0.3]]]);
        let json = import_mesh(mesh, "t.stl", &ImportOptions { mode: Mode::Mesh }).design.to_json();
        assert!(json.contains(r#""positions":[0.1,0.0,0.0,0.0,0.2,0.0,0.0,0.0,0.3]"#), "{json}");
        assert!(json.contains(r#""triangles":[0,1,2]"#), "{json}");
    }

    #[test]
    fn broken_files_are_errors() {
        assert_eq!(import(b"solid e\nendsolid e\n", "e.stl", &ImportOptions::default()), Err(StlError::Empty));
    }

    #[test]
    fn body_names_follow_file_names() {
        assert_eq!(body_name("bracket.stl"), "bracket");
        assert_eq!(body_name("a.b.stl"), "a.b");
        assert_eq!(body_name(".stl"), "Mesh");
    }
}
