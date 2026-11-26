//! The whole import: mesh file bytes in, design and loss report out.

use serde::Deserialize;

use crate::design::{Design, DesignBuilder, FeatureKind, MeshData};
use crate::format::{self, MeshError};
use crate::i18n::Locale;
use crate::mesh::Mesh;
use crate::recognize::{Recognition, recognize};
use crate::report::{LossItem, LossReport, Severity};

/// Volume changes from smoothing facets below this fraction are not worth a line in the report.
const SMOOTHING_NOTE: f64 = 1e-4;

/// Millimetres per inch, for a file the user says is in inches.
pub const INCH_TO_MM: f64 = 25.4;

/// The units the mesh file is in. Mesh files are unitless; millimetres are assumed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Units {
    /// The default: coordinates are millimetres.
    #[default]
    Mm,
    /// The file is in inches; it is converted to millimetres on import.
    Inch,
}

/// What the user chose in the import dialog (the entry's `input_schema` in `plugin.json`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ImportOptions {
    /// What to make of the mesh.
    pub mode: Mode,
    /// The file's units.
    pub units: Units,
    /// The language of the loss report. Not a dialog option: the plugin sets the locale it was
    /// built for.
    #[serde(skip)]
    pub locale: Locale,
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

/// Imports the mesh file `file_name` (STL, OBJ or PLY, see [`crate::format`]) whose content is
/// `bytes`.
pub fn import(bytes: &[u8], file_name: &str, options: &ImportOptions) -> Result<ImportResult, MeshError> {
    let mut mesh = format::parse(bytes, file_name)?;
    convert_units(&mut mesh, options.units)?;
    Ok(import_mesh(&mesh, file_name, options))
}

/// Applies a file's `units` to the mesh read from it: an inch file is scaled to millimetres in
/// place, and the coordinates are range-checked afterwards, because scaling can push one past
/// the f32 range the design is written in. Call this before [`import_mesh`], which expects
/// millimetres; it never copies the mesh, so even a 32 MiB mesh-mode import holds it only once
/// besides the design's own copy.
pub fn convert_units(mesh: &mut Mesh, units: Units) -> Result<(), MeshError> {
    if units == Units::Inch {
        mesh.scale_in_place(INCH_TO_MM);
        for (i, triangle) in mesh.triangles.iter().enumerate() {
            if triangle.iter().flatten().any(|v| !v.is_finite()) {
                return Err(MeshError::NonFinite { triangle: i + 1 });
            }
            if triangle.iter().flatten().any(|v| v.abs() > f64::from(f32::MAX)) {
                return Err(MeshError::OutOfRange { triangle: i + 1 });
            }
        }
    }
    Ok(())
}

/// Imports a mesh already read from the mesh file `file_name` (see [`format::MeshReader`]). The mesh is
/// borrowed and must already be in millimetres ([`convert_units`] applies the file's units in
/// place first): it is cloned only when it is kept as a mesh body, so a recognised import holds
/// no second mesh (the plugin's sandbox fallback can still rebuild one from the same borrow).
#[must_use]
pub fn import_mesh(mesh: &Mesh, file_name: &str, options: &ImportOptions) -> ImportResult {
    let mut result = import_mesh_mm(mesh, file_name, options);
    if options.units == Units::Inch {
        result.report.push(LossItem {
            severity: Severity::Info,
            title: options.locale.text("report.units.title").into(),
            subject: body_name(file_name),
            message: options.locale.text("report.units.message").into(),
            code: "revmesh.units",
        });
    }
    result
}

fn import_mesh_mm(mesh: &Mesh, file_name: &str, options: &ImportOptions) -> ImportResult {
    let body = body_name(file_name);
    let locale = options.locale;
    if options.mode == Mode::Auto {
        let found = recognize(mesh);
        let recognised = match &found {
            Recognition::Plate { plate, offset } => Ok((plate.to_design_at("", *offset), plate.describe_in(locale), plate.volume())),
            Recognition::Profile { profile, offset, axis } => {
                Ok((profile.to_design_on("", *offset, *axis), profile.describe_in(locale), profile.volume()))
            }
            Recognition::Combined { spec, offset } => Ok((spec.to_design_at("", *offset), spec.describe_in(locale), spec.volume())),
            Recognition::Stack { spec, offset, axis } => {
                Ok((spec.to_design_on("", *offset, *axis), spec.describe_in(locale, *axis), spec.volume()))
            }
            Recognition::Unrecognized { reason } => Err(reason.clone()),
        };
        match recognised {
            Ok((design, summary, volume)) => {
                let mut report = LossReport::new(summary);
                let change = volume / mesh.volume() - 1.0;
                if change.abs() > SMOOTHING_NOTE {
                    report.push(LossItem {
                        severity: Severity::Info,
                        title: locale.text("report.smoothed.title").into(),
                        subject: body.clone(),
                        message: locale
                            .format("report.smoothed.message", &[("change", &locale.decimal(&format!("{:+.2}", change * 100.0)))]),
                        code: "revmesh.smoothed",
                    });
                }
                // The per-face surface deviation between the mesh and the recognised faces,
                // computed in the sandbox for every recognised kind (roadmap 5.4).
                let deviations = crate::recognize::deviations(mesh, &found);
                if !deviations.is_empty() {
                    let faces = deviations
                        .iter()
                        .map(|(label, d)| format!("{label}: {} mm", locale.decimal(&crate::design::fmt_number(*d))))
                        .collect::<Vec<_>>()
                        .join(", ");
                    report.push(LossItem {
                        severity: Severity::Info,
                        title: locale.text("report.deviation.title").into(),
                        subject: body,
                        message: locale.format("report.deviation.message", &[("faces", &faces)]),
                        code: "revmesh.deviation",
                    });
                }
                return ImportResult { design, report };
            }
            Err(reason) => {
                let mut result = mesh_import(mesh.clone(), file_name, &body, locale);
                result.report.push(LossItem {
                    severity: Severity::Degraded,
                    title: locale.text("report.unrecognized.title").into(),
                    subject: body,
                    message: locale.format("report.unrecognized.message", &[("reason", &reason)]),
                    code: "revmesh.unrecognized",
                });
                return result;
            }
        }
    }
    mesh_import(mesh.clone(), file_name, &body, locale)
}

fn mesh_import(mesh: Mesh, file_name: &str, body: &str, locale: Locale) -> ImportResult {
    let report = LossReport::new(locale.format("report.mesh.summary", &[("triangles", &mesh.triangles.len())]));
    let mut b = DesignBuilder::new("");
    b.feature(body, vec![body.to_owned()], FeatureKind::MeshImport { file: file_name.to_owned(), meshes: vec![MeshData::new(body, mesh)] });
    ImportResult { design: b.build(), report }
}

/// What the app's sandbox self-check said about a recognised design (roadmap 7.1): the plugin
/// stages the serialised design with `evaluate.stage` and runs `document.inspect` on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelfCheck {
    /// The app opened the design and every timeline feature evaluated.
    Passed,
    /// The app could not rebuild it; why.
    Failed(String),
    /// The check could not run (the capability is unavailable, or the call errored — an error for
    /// that call only); the design is written unchecked.
    Unavailable,
}

impl SelfCheck {
    /// What a `document.inspect` reply says: a failed command is a top-level `error`, and a
    /// timeline feature that failed to evaluate carries its own `error` (SolidCraft's control
    /// protocol serialises it as `null` when the feature is fine). Anything else means the design
    /// rebuilt.
    #[must_use]
    pub fn from_inspect_reply(reply: &str) -> SelfCheck {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(reply) else {
            return SelfCheck::Failed(reply.to_owned());
        };
        if let Some(error) = v.get("error").and_then(serde_json::Value::as_str) {
            return SelfCheck::Failed(error.to_owned());
        }
        if let Some(timeline) = v.get("timeline").and_then(serde_json::Value::as_array) {
            for feature in timeline {
                if let Some(error) = feature.get("error").and_then(serde_json::Value::as_str).filter(|e| !e.is_empty()) {
                    let name = feature.get("name").and_then(serde_json::Value::as_str).unwrap_or("?");
                    return SelfCheck::Failed(format!("feature `{name}`: {error}"));
                }
            }
        }
        SelfCheck::Passed
    }
}

/// Whether the design is just a mesh body, with no recognised features to check.
#[must_use]
pub fn design_has_mesh_body(design: &Design) -> bool {
    design.features.iter().any(|f| matches!(f.kind, FeatureKind::MeshImport { .. }))
}

/// Chooses the import's result after the app's sandbox self-check (roadmap 7.1): a recognised
/// design that failed the check falls back to the mesh body, with the reason in the loss report.
/// The update `notice` is kept in the report either way.
#[must_use]
pub fn apply_self_check(
    result: ImportResult,
    mesh: &Mesh,
    file_name: &str,
    options: &ImportOptions,
    notice: Option<LossItem>,
    check: &SelfCheck,
) -> ImportResult {
    let mut out = match check {
        SelfCheck::Failed(reason) => {
            let options = ImportOptions { mode: Mode::Mesh, ..options.clone() };
            let mut fallback = import_mesh(mesh, file_name, &options);
            fallback.report.push(LossItem {
                severity: Severity::Degraded,
                title: options.locale.text("report.selfcheck.title").into(),
                subject: body_name(file_name),
                message: options.locale.format("report.selfcheck.message", &[("reason", reason)]),
                code: "revmesh.selfcheck",
            });
            fallback
        }
        SelfCheck::Passed | SelfCheck::Unavailable => result,
    };
    if let Some(notice) = notice {
        out.report.push(notice);
    }
    out
}

/// The body name SolidCraft would give: the file name without its extension.
#[must_use]
pub fn body_name(file_name: &str) -> String {
    let stem = file_name.rsplit_once('.').map_or(file_name, |(stem, _)| stem);
    if stem.is_empty() { "Mesh".into() } else { stem.into() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stl;

    const TETRA: &[u8] = include_bytes!("../tests/data/tetra.stl");

    #[test]
    fn options_default_when_empty() {
        assert_eq!(ImportOptions::from_json("").expect("ok"), ImportOptions::default());
        assert_eq!(ImportOptions::from_json("{}").expect("ok"), ImportOptions::default());
        assert_eq!(ImportOptions::from_json(r#"{"mode":"mesh"}"#).expect("ok").mode, Mode::Mesh);
        assert!(ImportOptions::from_json(r#"{"colour":1}"#).is_err());
        assert_eq!(ImportOptions::from_json("").expect("ok").units, Units::Mm);
        assert_eq!(ImportOptions::from_json(r#"{"units":"inch"}"#).expect("ok").units, Units::Inch);
    }

    #[test]
    fn inspect_replies_say_whether_the_design_rebuilt() {
        assert_eq!(SelfCheck::from_inspect_reply(r#"{"timeline": []}"#), SelfCheck::Passed);
        assert_eq!(SelfCheck::from_inspect_reply(r#"{"error": "cannot open"}"#), SelfCheck::Failed("cannot open".into()));
        let reply = r#"{"timeline": [{"id": 1, "name": "Hole1", "type": "Hole", "error": null}, {"id": 2, "name": "Extrude1", "type": "Extrude", "error": "over-constrained sketch"}]}"#;
        assert_eq!(SelfCheck::from_inspect_reply(reply), SelfCheck::Failed("feature `Extrude1`: over-constrained sketch".into()));
        assert_eq!(SelfCheck::from_inspect_reply("not json"), SelfCheck::Failed("not json".into()));
    }

    #[test]
    fn a_failed_self_check_falls_back_to_the_mesh_and_keeps_the_update_notice() {
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
            side_bosses: Vec::new(),
            slots: vec![],
            outline_fillet_rs: Vec::new(),
            edge_fillet_r: None,
        };
        let options = ImportOptions::default();
        let result = import_mesh(&crate::synth::plate(&spec, 24, [0.0; 3]), "bracket.stl", &options);
        assert!(!design_has_mesh_body(&result.design));
        let notice = LossItem {
            severity: Severity::Info,
            title: "update".into(),
            subject: "bracket".into(),
            message: "a newer revmesh is out".into(),
            code: "revmesh.update_available",
        };
        let out = apply_self_check(
            result,
            &crate::synth::plate(&spec, 24, [0.0; 3]),
            "bracket.stl",
            &options,
            Some(notice.clone()),
            &SelfCheck::Failed("feature `Hole1`: boom".into()),
        );
        assert!(design_has_mesh_body(&out.design));
        assert!(out.report.items.iter().any(|i| i.code == "revmesh.selfcheck" && i.message.contains("feature `Hole1`: boom")));
        assert!(out.report.items.contains(&notice));
        // A passed or skipped check keeps the recognised design, and the notice.
        for check in [SelfCheck::Passed, SelfCheck::Unavailable] {
            let result = import_mesh(&crate::synth::plate(&spec, 24, [0.0; 3]), "bracket.stl", &options);
            let out =
                apply_self_check(result, &crate::synth::plate(&spec, 24, [0.0; 3]), "bracket.stl", &options, Some(notice.clone()), &check);
            assert!(!design_has_mesh_body(&out.design));
            assert!(!out.report.items.iter().any(|i| i.code == "revmesh.selfcheck"));
            assert!(out.report.items.contains(&notice));
        }
    }

    #[test]
    fn inch_files_are_converted_to_millimetres() {
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
            side_bosses: Vec::new(),
            slots: vec![],
            outline_fillet_rs: Vec::new(),
            edge_fillet_r: None,
        };
        let mesh_mm = crate::synth::plate(&spec, 24, [0.0; 3]);
        let mut mesh_in = mesh_mm.clone();
        mesh_in.scale_in_place(1.0 / INCH_TO_MM);
        convert_units(&mut mesh_in, Units::Inch).expect("in range");
        let options = ImportOptions { units: Units::Inch, ..ImportOptions::default() };
        let result = import_mesh(&mesh_in, "bracket.stl", &options);
        let mm = import_mesh(&mesh_mm, "bracket.stl", &ImportOptions::default());
        // The design is the same as the millimetre import's (parameters snap to a micrometre).
        assert_eq!(result.design.to_json(), mm.design.to_json());
        assert!(result.report.items.iter().any(|i| i.code == "revmesh.units"));
        assert!(!mm.report.items.iter().any(|i| i.code == "revmesh.units"));
    }

    #[test]
    fn inch_conversion_range_checks_the_scaled_coordinates() {
        // 1e38 parses fine (below f32::MAX), but 25.4e38 does not survive the f32 write.
        let mesh = Mesh { triangles: vec![[[0.0, 0.0, 0.0], [1.0e38, 0.0, 0.0], [0.0, 1.0, 0.0]]] };
        assert!(matches!(convert_units(&mut mesh.clone(), Units::Mm), Ok(())));
        assert_eq!(convert_units(&mut mesh.clone(), Units::Inch), Err(MeshError::OutOfRange { triangle: 1 }));
        let bytes = stl::write_binary(&mesh);
        let err = import(&bytes, "big.stl", &ImportOptions { units: Units::Inch, ..ImportOptions::default() })
            .expect_err("1e38 inches overflow the f32 range");
        assert!(matches!(err, MeshError::OutOfRange { triangle: 1 }), "{err}");
    }

    #[test]
    fn unrecognised_mesh_is_kept_and_reported() {
        let result = import(TETRA, "tetra.stl", &ImportOptions::default()).expect("imports");
        let json: serde_json::Value = serde_json::from_str(&result.design.to_json()).expect("valid");
        assert_eq!(json["features"][0]["type"], "mesh_import");
        assert_eq!(json["features"][0]["body_names"][0], "tetra");
        assert_eq!(json["features"][0]["meshes"][0]["triangles"].as_array().map(Vec::len), Some(12));
        assert_eq!(result.report.items[0].code, "revmesh.unrecognized");
    }

    #[test]
    fn reports_speak_the_chosen_locale() {
        use crate::recognize::{HoleSpec, PlateSpec, ProfileSpec};
        let german = ImportOptions { locale: Locale::De, ..ImportOptions::default() };
        let result = import(TETRA, "tetra.stl", &german).expect("imports");
        assert_eq!(result.report.summary, "Ein Netz aus 4 Dreiecken");
        assert_eq!(result.report.items[0].title, "Als Netz behalten");
        assert!(result.report.items[0].message.starts_with("Es wurden keine parametrischen Features erkannt: "));
        let mesh = ImportOptions { mode: Mode::Mesh, locale: Locale::De, ..ImportOptions::default() };
        assert_eq!(import(TETRA, "tetra.stl", &mesh).expect("imports").report.summary, "Ein Netz aus 4 Dreiecken");

        let mut spec = PlateSpec {
            x: 60.0,
            y: 40.0,
            z: 5.0,
            holes: vec![HoleSpec { x: 30.0, y: 20.0, d: 10.0, head: None, blind: None }],
            cutouts: vec![],
            pads: vec![],
            pockets: vec![],
            bosses: vec![],
            side_bosses: Vec::new(),
            slots: vec![],
            outline_fillet_rs: vec![2.5; 4],
            edge_fillet_r: None,
        };
        let bytes = stl::write_binary(&crate::synth::plate(&spec, 24, [0.0; 3]));
        let result = import(&bytes, "bracket.stl", &german).expect("imports");
        assert_eq!(result.report.summary, "Eine 60 x 40 x 5 mm Platte mit 1 Bohrung und 2,5 mm verrundeten Ecken");
        assert_eq!(result.report.items[0].title, "Facetten geglättet");
        assert!(result.report.items[0].message.contains(" %."), "{}", result.report.items[0].message);
        assert!(result.report.items[0].message.contains(','), "a decimal comma: {}", result.report.items[0].message);

        spec.holes.push(spec.holes[0].clone());
        spec.outline_fillet_rs.clear();
        spec.edge_fillet_r = Some(2.0);
        assert_eq!(spec.describe_in(Locale::De), "Eine 60 x 40 x 5 mm Platte mit 2 Bohrungen und 2 mm verrundeten Kanten");
        spec.holes.clear();
        spec.edge_fillet_r = None;
        assert_eq!(spec.describe_in(Locale::De), "Eine 60 x 40 x 5 mm Platte");
        let profile = ProfileSpec {
            corners: vec![[0.0, 0.0], [60.0, 0.0], [60.0, 40.0], [0.0, 40.0]],
            z: 5.0,
            holes: vec![],
            slots: vec![],
            bosses: vec![],
            pads: vec![],
            pockets: vec![],
            outline_pockets: Vec::new(),
            cutouts: vec![],
            arcs: Vec::new(),
            outline_fillet_rs: Vec::new(),
        };
        assert_eq!(profile.describe_in(Locale::De), "Eine 60 x 40 x 5 mm Platte mit einer 4-eckigen Kontur");
        assert_eq!(profile.describe(), "A 60 x 40 x 5 mm plate with a 4-sided outline");
    }

    #[test]
    fn mesh_mode_reports_nothing_lost() {
        let result = import(TETRA, "tetra.stl", &ImportOptions { mode: Mode::Mesh, ..ImportOptions::default() }).expect("imports");
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
            side_bosses: Vec::new(),
            slots: vec![],
            outline_fillet_rs: vec![4.0; 4],
            edge_fillet_r: None,
        };
        let bytes = stl::write_binary(&crate::synth::plate(&spec, 24, [-30.0, -20.0, 0.0]));
        let result = import(&bytes, "bracket.stl", &ImportOptions::default()).expect("imports");
        let json: serde_json::Value = serde_json::from_str(&result.design.to_json()).expect("valid");
        let types: Vec<&str> = json["features"].as_array().expect("features").iter().filter_map(|f| f["type"].as_str()).collect();
        assert_eq!(types, ["sketch", "extrude", "move", "hole", "fillet"]);
        assert_eq!(result.report.summary, "A 60 x 40 x 5 mm plate with 1 hole and 4 mm rounded corners");
        let codes: Vec<&str> = result.report.items.iter().map(|i| i.code).collect();
        assert_eq!(codes, ["revmesh.smoothed", "revmesh.deviation"]);
        // Roadmap 5.4: the per-face deviation lists the hole's facets bowing inside its circle.
        let deviation = &result.report.items[1];
        assert!(deviation.message.contains("hole 1: "), "{}", deviation.message);
        let hole_dev: f64 =
            deviation.message.split("hole 1: ").nth(1).and_then(|s| s.split(" mm").next()).and_then(|s| s.parse().ok()).expect("a number");
        assert!(hole_dev > 0.0 && hole_dev < 0.1, "the facets' sagitta, in mm: {hole_dev}");
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

    /// Roadmap 5.4: a stack's import report lists the per-band wall deviations, labelled
    /// `band <k> side <i>`; a combined part's lists them per extrusion (`extrude <a> side <i>`,
    /// `extrude <a> cut <n> side <i>`, `extrude <a> hole <n>`).
    #[test]
    fn stack_and_combined_imports_report_per_part_deviations() {
        let bytes = include_bytes!("../../../plugin/revmesh/tests/stack.stl");
        let result = import(bytes, "stack.stl", &ImportOptions::default()).expect("imports");
        let item = result.report.items.iter().find(|i| i.code == "revmesh.deviation").expect("a deviation item");
        assert!(item.message.contains("band 1 side 1"), "{}", item.message);
        assert!(item.message.contains("band 3 side 1"), "{}", item.message);

        let bytes = include_bytes!("../../../plugin/revmesh/tests/combined-cuts.stl");
        let result = import(bytes, "combined-cuts.stl", &ImportOptions::default()).expect("imports");
        let item = result.report.items.iter().find(|i| i.code == "revmesh.deviation").expect("a deviation item");
        assert!(item.message.contains("extrude x cut 1 side 1"), "{}", item.message);
        assert!(item.message.contains("extrude y cut 2 side 1"), "{}", item.message);
    }

    /// A box rounded all over reports its rounded surface's deviation (roadmap 5.4).
    #[test]
    fn rounded_box_import_reports_its_rounded_surface() {
        let mesh = crate::synth::rounded_box(60.0, 40.0, 30.0, 5.0, 16, [0.0; 3]);
        let bytes = stl::write_binary(&mesh);
        let result = import(&bytes, "rounded.stl", &ImportOptions::default()).expect("imports");
        let item = result.report.items.iter().find(|i| i.code == "revmesh.deviation").expect("a deviation item");
        assert!(item.message.contains("rounded surface"), "{}", item.message);
    }

    /// Counterbores and countersinks become SolidCraft's own hole kinds with mesh2cad's names; a
    /// counterbore on the bottom face is drilled up from there.
    #[test]
    fn stepped_holes_become_hole_kinds() {
        let bytes = include_bytes!("../../../plugin/revmesh/tests/stepped-holes.stl");
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
        let bytes = include_bytes!("../../../plugin/revmesh/tests/plate-patterns.stl");
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

    /// A plate standing along X imports as an outline on the YZ plane, with the hole's position
    /// parameters named after the world axes Y and Z, as mesh2cad's extrusion module names them.
    #[test]
    fn x_extruded_plate_imports_as_a_yz_profile() {
        use crate::recognize::{Axis, HoleSpec, PlateSpec};
        let spec = PlateSpec {
            x: 40.0,
            y: 20.0,
            z: 60.0,
            holes: vec![HoleSpec { x: 10.0, y: 5.0, d: 6.0, head: None, blind: None }],
            cutouts: vec![],
            pads: vec![],
            pockets: vec![],
            bosses: vec![],
            side_bosses: Vec::new(),
            slots: vec![],
            outline_fillet_rs: Vec::new(),
            edge_fillet_r: None,
        };
        let mesh = Axis::X.unrotate_mesh(&crate::synth::plate(&spec, 32, [3.0, 7.0, 11.0]));
        let result = import(&stl::write_binary(&mesh), "x-plate.stl", &ImportOptions::default()).expect("imports");
        let json: serde_json::Value = serde_json::from_str(&result.design.to_json()).expect("valid");
        assert_eq!(json["features"][0]["plane"]["name"], "YZ");
        let names: Vec<&str> = result.design.params.iter().filter(|p| !p.model).map(|p| p.name.as_str()).collect();
        // A rectangle's outline is fully axis-aligned, so only two independent sides drive it
        // (see `dimensioned_sides`).
        assert_eq!(names, ["extrude_len", "extrude_0", "hole_1_y", "hole_1_z", "hole_1_d", "outline_1_len", "outline_2_len"]);
        let hole = json["features"].as_array().expect("features").iter().find(|f| f["type"] == "hole").expect("a hole");
        assert_eq!(hole["direction"], serde_json::json!([-1.0, 0.0, 0.0]));
        assert_eq!(result.report.summary, "A 40 x 20 x 60 mm plate with a 4-sided outline and 1 hole");
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
            outline_pockets: Vec::new(),
            bosses: vec![],
            slots: vec![],
            arcs: Vec::new(),
            outline_fillet_rs: Vec::new(),
        };
        let bytes = stl::write_binary(&crate::synth::profile(&spec, 32, [20.0, 10.0, 3.0]));
        let result = import(&bytes, "l.stl", &ImportOptions::default()).expect("imports");
        let json: serde_json::Value = serde_json::from_str(&result.design.to_json()).expect("valid");
        let names: Vec<&str> = result.design.params.iter().filter(|p| !p.model).map(|p| p.name.as_str()).collect();
        // The L's outline is fully axis-aligned: the longest horizontal and vertical sides are
        // derived through loop closure, so only sides 2 to 5 drive (see `dimensioned_sides`).
        assert_eq!(
            names,
            [
                "extrude_len",
                "extrude_0",
                "hole_1_x",
                "hole_1_y",
                "hole_1_d",
                "outline_2_len",
                "outline_3_len",
                "outline_4_len",
                "outline_5_len"
            ]
        );
        assert_eq!(result.design.params[1].expr, "3 mm");
        assert_eq!(result.design.params[2].expr, "27.5 mm");
        assert_eq!(json["features"][2]["translate"], serde_json::json!(["20", "10", "extrude_0"]));
        assert_eq!(json["features"][3]["at"], serde_json::json!(["hole_1_x", "hole_1_y", "extrude_0 + extrude_len"]));
    }

    /// Keeps the mixed-corner-radii plate's plugin fixture and its expectations in step (roadmap
    /// 6.4): one fillet feature per radius group, `fillet_r_<g>`.
    #[test]
    fn plugin_test_mixed_radii_fixture() {
        let case: serde_json::Value =
            serde_json::from_str(include_str!("../../../plugin/revmesh/tests/mixed-radii.json")).expect("valid case");
        let bytes = include_bytes!("../../../plugin/revmesh/tests/mixed-radii.stl");
        let result = import(bytes, "mixed-radii.stl", &ImportOptions::default()).expect("imports");
        let Recognition::Plate { plate, .. } = recognize(&stl::parse(bytes).expect("parses")) else { panic!("not a plate") };
        assert_eq!(plate.outline_fillet_rs, vec![2.0, 3.0, 5.0, 4.0]);
        let expect = &case["expect"];
        assert_eq!(result.design.features.len() as u64, expect["reply"]["features"].as_u64().expect("count"));
        assert_eq!(result.report.summary, expect["reply"]["loss_report"]["summary"].as_str().expect("summary"));
        let names: Vec<&str> = result.design.params.iter().filter(|p| !p.model).map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"fillet_r_1") && names.contains(&"fillet_r_4") && !names.contains(&"fillet_r"));
        let volume = expect["model"]["total"]["volume_mm3"].as_f64().expect("volume");
        assert!((plate.volume() - volume).abs() < 1e-6 * volume, "{} vs {volume}", plate.volume());
    }

    /// Keeps the side-boss plate's plugin fixture and its expectations in step (roadmap 3.10):
    /// one joined cylinder on the +X wall, `boss_1_*` parameters.
    #[test]
    fn plugin_test_side_boss_fixture() {
        use crate::recognize::{SideBossSpec, Wall};
        let case: serde_json::Value =
            serde_json::from_str(include_str!("../../../plugin/revmesh/tests/side-boss.json")).expect("valid case");
        let bytes = include_bytes!("../../../plugin/revmesh/tests/side-boss.stl");
        let result = import(bytes, "side-boss.stl", &ImportOptions::default()).expect("imports");
        let Recognition::Plate { plate, .. } = recognize(&stl::parse(bytes).expect("parses")) else { panic!("not a plate") };
        assert_eq!(plate.side_bosses, [SideBossSpec { wall: Wall::PosX, at: [20.0, 3.0], d: 5.0, h: 5.0 }]);
        let expect = &case["expect"];
        assert_eq!(result.design.features.len() as u64, expect["reply"]["features"].as_u64().expect("count"));
        assert_eq!(result.report.summary, expect["reply"]["loss_report"]["summary"].as_str().expect("summary"));
        let names: Vec<&str> = result.design.params.iter().filter(|p| !p.model).map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"boss_1_d") && names.contains(&"boss_1_y") && names.contains(&"boss_1_z"));
        let design: serde_json::Value = serde_json::from_str(&result.design.to_json()).expect("valid");
        let cylinders: Vec<&serde_json::Value> =
            design["features"].as_array().expect("features").iter().filter(|f| f["type"] == "cylinder").collect();
        assert_eq!(cylinders.len(), 1);
        assert_eq!(cylinders[0]["axis"], serde_json::json!([1.0, 0.0, 0.0]));
        assert_eq!(cylinders[0]["base"], serde_json::json!(["base_cx + base_x / 2", "boss_1_y", "boss_1_z"]));
        let volume = expect["model"]["total"]["volume_mm3"].as_f64().expect("volume");
        assert!((plate.volume() - volume).abs() < 1e-6 * volume, "{} vs {volume}", plate.volume());
    }

    /// Keeps the standing plate's plugin fixture and its expectations in step.
    #[test]
    fn plugin_test_x_plate_fixture() {
        use crate::recognize::Axis;
        let case: serde_json::Value = serde_json::from_str(include_str!("../../../plugin/revmesh/tests/x-plate.json")).expect("valid case");
        let bytes = include_bytes!("../../../plugin/revmesh/tests/x-plate.stl");
        let result = import(bytes, "x-plate.stl", &ImportOptions::default()).expect("imports");
        let Recognition::Profile { profile, axis, .. } = recognize(&stl::parse(bytes).expect("parses")) else { panic!("not a profile") };
        assert_eq!(axis, Axis::X);
        let expect = &case["expect"];
        assert_eq!(result.design.features.len() as u64, expect["reply"]["features"].as_u64().expect("count"));
        assert_eq!(result.report.summary, expect["reply"]["loss_report"]["summary"].as_str().expect("summary"));
        let volume = expect["model"]["total"]["volume_mm3"].as_f64().expect("volume");
        assert!((profile.volume() - volume).abs() < 1e-6 * volume, "{} vs {volume}", profile.volume());
    }

    /// Keeps the combined extrusions' plugin fixtures and their expectations in step.
    #[test]
    fn plugin_test_combined_fixtures() {
        let cases: [(&str, &[u8]); 6] = [
            (include_str!("../../../plugin/revmesh/tests/combined.json"), include_bytes!("../../../plugin/revmesh/tests/combined.stl")),
            (include_str!("../../../plugin/revmesh/tests/combined-3.json"), include_bytes!("../../../plugin/revmesh/tests/combined-3.stl")),
            (
                include_str!("../../../plugin/revmesh/tests/combined-windows.json"),
                include_bytes!("../../../plugin/revmesh/tests/combined-windows.stl"),
            ),
            (
                include_str!("../../../plugin/revmesh/tests/combined-pocket.json"),
                include_bytes!("../../../plugin/revmesh/tests/combined-pocket.stl"),
            ),
            (
                include_str!("../../../plugin/revmesh/tests/combined-cuts.json"),
                include_bytes!("../../../plugin/revmesh/tests/combined-cuts.stl"),
            ),
            (
                include_str!("../../../plugin/revmesh/tests/combined-holes.json"),
                include_bytes!("../../../plugin/revmesh/tests/combined-holes.stl"),
            ),
        ];
        for (case, bytes) in cases {
            let case: serde_json::Value = serde_json::from_str(case).expect("valid case");
            let result = import(bytes, "combined.stl", &ImportOptions::default()).expect("imports");
            let Recognition::Combined { spec, .. } = recognize(&stl::parse(bytes).expect("parses")) else { panic!("not combined") };
            let expect = &case["expect"];
            assert_eq!(result.design.features.len() as u64, expect["reply"]["features"].as_u64().expect("count"));
            assert_eq!(result.report.summary, expect["reply"]["loss_report"]["summary"].as_str().expect("summary"));
            let volume = expect["model"]["total"]["volume_mm3"].as_f64().expect("volume");
            assert!((spec.volume() - volume).abs() < 1e-6 * volume, "{} vs {volume}", spec.volume());
        }
    }

    /// Keeps the stacks' plugin fixtures and their expectations in step.
    #[test]
    fn plugin_test_stack_fixtures() {
        let cases: [(&str, &[u8]); 2] = [
            (include_str!("../../../plugin/revmesh/tests/stack.json"), include_bytes!("../../../plugin/revmesh/tests/stack.stl")),
            (include_str!("../../../plugin/revmesh/tests/stack-y.json"), include_bytes!("../../../plugin/revmesh/tests/stack-y.stl")),
        ];
        for (case, bytes) in cases {
            let case: serde_json::Value = serde_json::from_str(case).expect("valid case");
            let result = import(bytes, "stack.stl", &ImportOptions::default()).expect("imports");
            let Recognition::Stack { spec, .. } = recognize(&stl::parse(bytes).expect("parses")) else { panic!("not a stack") };
            let expect = &case["expect"];
            assert_eq!(result.design.features.len() as u64, expect["reply"]["features"].as_u64().expect("count"));
            assert_eq!(result.report.summary, expect["reply"]["loss_report"]["summary"].as_str().expect("summary"));
            let volume = expect["model"]["total"]["volume_mm3"].as_f64().expect("volume");
            assert!((spec.volume() - volume).abs() < 1e-6 * volume, "{} vs {volume}", spec.volume());
        }
    }

    /// Keeps the rounded box's plugin fixture and its expectations in step.
    #[test]
    fn plugin_test_rounded_box_fixture() {
        let case: serde_json::Value =
            serde_json::from_str(include_str!("../../../plugin/revmesh/tests/rounded-box.json")).expect("valid case");
        let bytes = include_bytes!("../../../plugin/revmesh/tests/rounded-box.stl");
        let result = import(bytes, "rounded-box.stl", &ImportOptions::default()).expect("imports");
        let Recognition::Plate { plate, .. } = recognize(&stl::parse(bytes).expect("parses")) else { panic!("not a plate") };
        assert_eq!(plate.edge_fillet_r, Some(5.0));
        assert_eq!(plate.outline_fillet_rs, [] as [f64; 0]);
        let expect = &case["expect"];
        assert_eq!(result.design.features.len() as u64, expect["reply"]["features"].as_u64().expect("count"));
        assert_eq!(result.report.summary, expect["reply"]["loss_report"]["summary"].as_str().expect("summary"));
        let volume = expect["model"]["total"]["volume_mm3"].as_f64().expect("volume");
        assert!((plate.volume() - volume).abs() < 1e-6 * volume, "{} vs {volume}", plate.volume());
        let fillet = result.design.features.iter().find(|f| f.name == "Fillet1").expect("a fillet");
        let crate::design::FeatureKind::Fillet { edge_names, .. } = &fillet.kind else { panic!("not a fillet") };
        assert_eq!(edge_names.len(), 12, "all twelve edges are filleted");
    }

    /// Keeps the STL fixture of SolidCraft's plugin test and its expectations in step.
    #[test]
    fn plugin_test_plate_fixture() {
        let case: serde_json::Value =
            serde_json::from_str(include_str!("../../../plugin/revmesh/tests/plate-holes.json")).expect("valid case");
        let bytes = include_bytes!("../../../plugin/revmesh/tests/plate-holes.stl");
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
        let result = import(TETRA, "tetra.stl", &ImportOptions { mode: Mode::Mesh, ..ImportOptions::default() }).expect("imports");
        let mut streamed = Vec::new();
        result.design.write_json(&mut streamed).expect("writes");
        assert_eq!(String::from_utf8(streamed).expect("utf-8"), result.design.to_json());

        let mesh = Mesh::new(vec![[[0.1, 0.0, 0.0], [0.0, 0.2, 0.0], [0.0, 0.0, 0.3]]]);
        let json = import_mesh(&mesh, "t.stl", &ImportOptions { mode: Mode::Mesh, ..ImportOptions::default() }).design.to_json();
        assert!(json.contains(r#""positions":[0.1,0.0,0.0,0.0,0.2,0.0,0.0,0.0,0.3]"#), "{json}");
        assert!(json.contains(r#""triangles":[0,1,2]"#), "{json}");
    }

    #[test]
    fn broken_files_are_errors() {
        assert_eq!(import(b"solid e\nendsolid e\n", "e.stl", &ImportOptions::default()), Err(MeshError::Stl(crate::stl::StlError::Empty)));
    }

    /// Roadmap 7.2: OBJ and PLY files import through `import` by their name, exactly as STL does.
    #[test]
    fn obj_and_ply_files_import_like_stl() {
        let bytes = include_bytes!("../../../plugin/revmesh/tests/block-60x40x6.stl");
        let mesh = stl::parse(bytes).expect("parses");
        let stl_result = import(bytes, "block.stl", &ImportOptions::default()).expect("imports");
        for (writer, name) in [
            (crate::obj::write as fn(&Mesh) -> Vec<u8>, "block.obj"),
            (crate::ply::write_ascii as fn(&Mesh) -> Vec<u8>, "block.ply"),
            (crate::ply::write_binary as fn(&Mesh) -> Vec<u8>, "block.ply"),
        ] {
            let file = writer(&mesh);
            let result = import(&file, name, &ImportOptions::default()).expect("imports");
            assert_eq!(result.design.to_json(), stl_result.design.to_json(), "{name}");
            assert_eq!(result.report.summary, stl_result.report.summary, "{name}");
        }
    }

    /// Keeps the OBJ/PLY fixtures of SolidCraft's plugin test and their expectations in step
    /// (roadmap 7.2): every format case imports the block exactly as the STL case does.
    #[test]
    fn plugin_test_obj_ply_fixtures() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../plugin/revmesh/tests/");
        for file in ["block-60x40x6.obj", "block-60x40x6.ply", "block-60x40x6-bin.ply"] {
            let case: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(format!("{dir}{file}.json")).expect("case file")).expect("valid case");
            let bytes = std::fs::read(format!("{dir}{file}")).expect("fixture");
            let result = import(&bytes, file, &ImportOptions::default()).expect("imports");
            let expect = &case["expect"];
            assert_eq!(result.design.features.len() as u64, expect["reply"]["features"].as_u64().expect("count"), "{file}");
            assert_eq!(result.report.summary, expect["reply"]["loss_report"]["summary"].as_str().expect("summary"), "{file}");
            assert_eq!(result.report.items[0].subject, file.split('.').next().expect("a stem"), "{file}");
            let volume = expect["model"]["total"]["volume_mm3"].as_f64().expect("volume");
            let mesh = crate::format::parse(&bytes, file).expect("parses");
            assert!((mesh.volume() - volume).abs() < 1e-6 * volume, "{file}");
        }
        // The error cases: the message fragment each case expects is the localised reason.
        for file in ["obj-bad.obj", "ply-bad.ply"] {
            let stem = file.split('.').next().expect("a stem");
            let case: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(format!("{dir}{stem}.json")).expect("case file")).expect("valid case");
            let bytes = std::fs::read(format!("{dir}{file}")).expect("fixture");
            let err = import(&bytes, file, &ImportOptions::default()).expect_err("an error");
            let message = err.message(crate::Locale::En);
            let expected = case["expect"]["message"].as_str().expect("message");
            assert!(message.contains(expected), "{file}: {message} vs {expected}");
        }
    }

    #[test]
    fn body_names_follow_file_names() {
        assert_eq!(body_name("bracket.stl"), "bracket");
        assert_eq!(body_name("a.b.stl"), "a.b");
        assert_eq!(body_name(".stl"), "Mesh");
    }
}
