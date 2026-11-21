//! The whole import: STL bytes in, design and loss report out.

use serde::Deserialize;

use crate::design::{Design, DesignBuilder, FeatureKind, MeshData};
use crate::mesh::Mesh;
use crate::recognize::{Recognition, recognize};
use crate::report::{LossItem, LossReport, Severity};
use crate::stl::{self, StlError};

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
        match recognize(&mesh) {
            Recognition::Plate(spec) => {
                return Ok(ImportResult { design: spec.to_design(""), report: LossReport::new(spec.describe()) });
            }
            Recognition::Unrecognized { reason } => {
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
    fn body_names_follow_file_names() {
        assert_eq!(body_name("bracket.stl"), "bracket");
        assert_eq!(body_name("a.b.stl"), "a.b");
        assert_eq!(body_name(".stl"), "Mesh");
    }
}
