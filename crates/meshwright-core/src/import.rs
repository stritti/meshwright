//! The whole import: STL bytes in, design and loss report out.

use serde::Deserialize;

use crate::design::{Design, DesignBuilder, FeatureKind, MeshData, fmt_number};
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
    let mesh = stl::parse(bytes)?;
    let body = body_name(file_name);
    if options.mode == Mode::Auto {
        let recognised = match recognize(&mesh) {
            Recognition::Plate { plate, offset } => Ok((plate.to_design(""), plate.describe(), plate.volume(), offset)),
            Recognition::Profile { profile, offset } => Ok((profile.to_design(""), profile.describe(), profile.volume(), offset)),
            Recognition::Unrecognized { reason } => Err(reason),
        };
        match recognised {
            Ok((design, summary, volume, offset)) => {
                let mut report = LossReport::new(summary);
                if offset.iter().any(|c| *c != 0.0) {
                    let [x, y, z] = offset.map(fmt_number);
                    report.push(LossItem {
                        severity: Severity::Info,
                        title: "Moved to the origin".into(),
                        subject: body.clone(),
                        message: format!("The plate's corner was at ({x}, {y}, {z}) mm in the file; the design starts it at the origin."),
                        code: "meshwright.moved",
                    });
                }
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
                return Ok(ImportResult { design, report });
            }
            Err(reason) => {
                let mut result = mesh_import(&mesh, file_name, &body);
                result.report.push(LossItem {
                    severity: Severity::Degraded,
                    title: "Kept as a mesh".into(),
                    subject: body,
                    message: format!("No parametric features were recognised: {reason}."),
                    code: "meshwright.unrecognized",
                });
                return Ok(result);
            }
        }
    }
    Ok(mesh_import(&mesh, file_name, &body))
}

fn mesh_import(mesh: &Mesh, file_name: &str, body: &str) -> ImportResult {
    let mut b = DesignBuilder::new("");
    b.feature(
        body,
        vec![body.to_owned()],
        FeatureKind::MeshImport { file: file_name.to_owned(), meshes: vec![MeshData::from_mesh(body, mesh)] },
    );
    let report = LossReport::new(format!("A mesh of {} triangles", mesh.triangles.len()));
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
        let spec = PlateSpec { x: 60.0, y: 40.0, z: 5.0, holes: vec![HoleSpec { x: 30.0, y: 20.0, d: 10.0 }], outline_fillet_r: Some(4.0) };
        let bytes = stl::write_binary(&crate::synth::plate(&spec, 24, [-30.0, -20.0, 0.0]));
        let result = import(&bytes, "bracket.stl", &ImportOptions::default()).expect("imports");
        let json: serde_json::Value = serde_json::from_str(&result.design.to_json()).expect("valid");
        let types: Vec<&str> = json["features"].as_array().expect("features").iter().filter_map(|f| f["type"].as_str()).collect();
        assert_eq!(types, ["sketch", "extrude", "hole", "fillet"]);
        assert_eq!(result.report.summary, "A 60 x 40 x 5 mm plate with 1 hole and 4 mm rounded corners");
        let codes: Vec<&str> = result.report.items.iter().map(|i| i.code).collect();
        assert_eq!(codes, ["meshwright.moved", "meshwright.smoothed"]);
        assert!(result.report.items[0].message.contains("(-30, -20, 0)"), "{}", result.report.items[0].message);
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
    fn body_names_follow_file_names() {
        assert_eq!(body_name("bracket.stl"), "bracket");
        assert_eq!(body_name("a.b.stl"), "a.b");
        assert_eq!(body_name(".stl"), "Mesh");
    }
}
