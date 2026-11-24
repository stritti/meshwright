use super::recognize_combined;

use crate::mesh::Mesh;
use crate::recognize::{Axis, CombinedSpec, Recognition, recognize};
use crate::stl;

fn fixture(bytes: &[u8]) -> Mesh {
    stl::parse(bytes).expect("parses")
}

fn combined() -> Mesh {
    fixture(include_bytes!("../../../../plugin/revmesh/tests/combined.stl"))
}

fn combined_3() -> Mesh {
    fixture(include_bytes!("../../../../plugin/revmesh/tests/combined-3.stl"))
}

const L: [[f64; 2]; 6] = [[0.0, 0.0], [60.0, 0.0], [60.0, 15.0], [20.0, 15.0], [20.0, 40.0], [0.0, 40.0]];
const RAMP: [[f64; 2]; 5] = [[0.0, 0.0], [40.0, 0.0], [40.0, 10.0], [15.0, 30.0], [0.0, 30.0]];

/// The outline's corners from its lowest-then-leftmost one on, so loops compare whatever corner
/// they start at.
fn from_lowest(outline: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let k = (0..outline.len())
        .min_by(|&a, &b| outline[a][1].total_cmp(&outline[b][1]).then(outline[a][0].total_cmp(&outline[b][0])))
        .expect("corners");
    outline[k..].iter().chain(&outline[..k]).copied().collect()
}

/// SolidCraft's L-shaped plate under a ramp: the plate seen from above, the ramp from the side.
#[test]
fn recognises_a_plate_under_a_ramp() {
    let found = recognize(&combined());
    let Recognition::Combined { spec, offset } = found else { panic!("not combined: {found:?}") };
    assert_eq!(offset, [0.0; 3]);
    assert_eq!(spec.size, [60.0, 40.0, 30.0]);
    assert_eq!(spec.extrusions.iter().map(|e| e.axis).collect::<Vec<_>>(), [Axis::Z, Axis::X]);
    assert_eq!(from_lowest(spec.outline(Axis::Z).expect("plan")), L);
    assert_eq!(from_lowest(spec.outline(Axis::X).expect("side")), RAMP);
    assert!((spec.volume() - 37_000.0).abs() < 1e-6, "{}", spec.volume());
    assert_eq!(spec.describe(), "A 60 x 40 x 30 mm part of outlines extruded along X and Z");
}

/// With a chamfered front seen along Y as well: three outlines (the front's second coordinate is
/// −Z on the XZ plane).
#[test]
fn recognises_three_outlines() {
    let (spec, _) = recognize_combined(&combined_3()).expect("combined");
    assert_eq!(spec.extrusions.len(), 3);
    assert_eq!(from_lowest(spec.outline(Axis::Y).expect("front")), [[0.0, -30.0], [48.0, -30.0], [60.0, -20.0], [60.0, 0.0], [0.0, 0.0]]);
    assert!((spec.volume() - 36_100.0).abs() < 1e-6, "{}", spec.volume());
    assert_eq!(spec.describe(), "A 60 x 40 x 30 mm part of outlines extruded along X, Y and Z");
}

/// Where the part is in the file is kept.
#[test]
fn keeps_where_the_part_is() {
    let moved = Mesh::new(combined().triangles.iter().map(|t| t.map(|p| [p[0] + 5.0, p[1] - 3.0, p[2] + 12.5])).collect());
    let (spec, offset) = recognize_combined(&moved).expect("combined");
    assert_eq!(offset, [5.0, -3.0, 12.5]);
    assert_eq!(spec.extrusions.len(), 2);
}

/// A part so thin that rounding its sizes to micrometres changes its volume by more than 1% is
/// refused, though the change lies within the tolerance over its surface.
#[test]
fn a_thin_part_keeps_its_volume() {
    let thin = Mesh::new(combined().triangles.iter().map(|t| t.map(|p| [p[0], p[1], p[2] * 0.0104 / 30.0])).collect());
    let why = recognize_combined(&thin).expect_err("refused");
    assert!(why.contains("the outlines' intersection and its cuts hold"), "{why}");
}

/// A 10 x 6 x 5 pocket in the top leaves the silhouettes as they were. It is a cut along each
/// axis (closed at both ends along X and Y, open through the top along Z); as mesh2cad, the
/// longest wins: along X from 5 to 15, its outline the pocket's side in (y, z) up to the top.
#[test]
fn recognises_a_pocket_as_a_cut() {
    let mesh = fixture(include_bytes!("../../../../plugin/revmesh/tests/combined-pocket.stl"));
    let (spec, _) = recognize_combined(&mesh).expect("combined");
    let [cut] = spec.cuts.as_slice() else { panic!("{:?}", spec.cuts) };
    assert_eq!((cut.axis, cut.from, cut.to), (Axis::X, 5.0, 15.0));
    assert_eq!(from_lowest(&cut.outline), [[5.0, 25.0], [11.0, 25.0], [11.0, 30.0], [5.0, 30.0]]);
    assert!((spec.volume() - 36_700.0).abs() < 1e-6, "{}", spec.volume());
    assert_eq!(spec.describe(), "A 60 x 40 x 30 mm part of outlines extruded along X and Z with 1 pocket");
    // Its sketch on YZ, cut from its start: cut_1_0 in the file, less where the part starts.
    let json: serde_json::Value = serde_json::from_str(&spec.to_design_at("", [2.0, 0.0, 0.0]).to_json()).expect("json");
    let cut = &json["features"][5];
    assert_eq!((cut["type"].as_str(), cut["operation"].as_str()), (Some("extrude"), Some("cut")));
    assert_eq!(json["features"][4]["plane"]["name"], "YZ");
    assert_eq!(cut["extent"]["distance"], "cut_1_len");
    assert_eq!(cut["extent"]["start_offset"], "cut_1_0 - extrude_x_0");
    let value = |name: &str| json["params"].as_array().expect("params").iter().find(|p| p["name"] == name).map(|p| p["expr"].clone());
    assert_eq!(value("cut_1_0"), Some("7 mm".into()));
    assert_eq!(value("cut_1_len"), Some("10 mm".into()));
}

/// A cut along an axis without an outline starts at a number: where the part starts along it.
#[test]
fn a_cut_along_a_plain_axis_starts_at_a_number() {
    let block = crate::recognize::tests::block(20.0, 10.0, 10.0, [0.0; 3]);
    let base = CombinedSpec {
        size: [20.0, 10.0, 10.0],
        extrusions: vec![crate::recognize::Extrusion {
            axis: Axis::Z,
            outline: vec![[0.0, 0.0], [20.0, 0.0], [20.0, 10.0], [0.0, 10.0]],
            cutouts: vec![],
        }],
        cuts: vec![crate::recognize::Cut {
            axis: Axis::Y,
            outline: vec![[2.0, -10.0], [6.0, -10.0], [6.0, -6.0], [2.0, -6.0]],
            from: 0.0,
            to: 3.0,
        }],
    };
    assert!((base.volume() - (block.volume() - 4.0 * 4.0 * 3.0)).abs() < 1e-9, "{}", base.volume());
    for (y, start) in [(0.0, "cut_1_0"), (-4.5, "cut_1_0 + 4.5"), (7.0, "cut_1_0 - 7")] {
        let json: serde_json::Value = serde_json::from_str(&base.to_design_at("", [0.0, y, 0.0]).to_json()).expect("json");
        assert_eq!(json["features"][3]["extent"]["start_offset"], start);
    }
}

/// SolidCraft's plate under a ramp with a box cut into its end along X and a triangular notch cut
/// into its front along Y (opening towards −Y: the cut runs from the face to its floor).
#[test]
fn recognises_cuts_along_x_and_y() {
    let mesh = fixture(include_bytes!("../../../../plugin/revmesh/tests/combined-cuts.stl"));
    let (spec, _) = recognize_combined(&mesh).expect("combined");
    let spans: Vec<(Axis, f64, f64, usize)> = spec.cuts.iter().map(|c| (c.axis, c.from, c.to, c.outline.len())).collect();
    assert_eq!(spans, [(Axis::X, 45.0, 60.0, 4), (Axis::Y, 0.0, 8.0, 3)]);
    assert!((spec.volume() - 35_800.0).abs() < 1e-6, "{}", spec.volume());
}

/// A pocket whose floor slopes along X is a cut along Y (its floor is parallel to Y), its outline
/// following the slope; one whose floor is warped has faces tilted to every axis.
#[test]
fn sloped_and_warped_pocket_floors() {
    let pocket = || fixture(include_bytes!("../../../../plugin/revmesh/tests/combined-pocket.stl"));
    let mut sloped = pocket();
    for p in sloped.triangles.iter_mut().flatten() {
        if (p[2] - 25.0).abs() < 1e-4 && p[0] > 14.0 {
            p[2] -= 0.5;
        }
    }
    let (spec, _) = recognize_combined(&sloped).expect("combined");
    let [cut] = spec.cuts.as_slice() else { panic!("{:?}", spec.cuts) };
    assert_eq!((cut.axis, cut.from, cut.to), (Axis::Y, 5.0, 11.0));
    assert!((spec.volume() - sloped.volume()).abs() < 1e-3, "{} vs {}", spec.volume(), sloped.volume());
    let mut warped = pocket();
    let mut moved = false;
    for p in warped.triangles.iter_mut().flatten() {
        if (p[2] - 25.0).abs() < 1e-4 && p[0] > 14.0 && p[1] > 10.0 {
            p[2] -= 0.5;
            moved = true;
        }
    }
    assert!(moved);
    assert!(recognize_combined(&warped).is_err());
}

/// A box is bounded by its sides only, and a single extrusion is a prism (found before).
#[test]
fn boxes_and_prisms_are_no_combination() {
    let block = crate::recognize::tests::block(10.0, 20.0, 5.0, [0.0; 3]);
    assert!(recognize_combined(&block).is_err());
    assert!(matches!(recognize(&block), Recognition::Plate { .. }));
}

/// A facet tilted to every axis is no side of an extrusion along one.
#[test]
fn tilted_faces_are_refused() {
    let tetra = crate::mesh::tests::tetrahedron(10.0);
    assert_eq!(recognize_combined(&tetra), Err(super::TILTED.into()));
}

/// A prism with an opening is one outline with a cut-out, no combination (the prisms find it).
#[test]
fn a_prism_with_an_opening_is_no_combination() {
    let ring = crate::synth::prism(&L, &[vec![[5.0, 5.0], [10.0, 5.0], [10.0, 10.0], [5.0, 10.0]]], 30.0, [0.0; 3]);
    let why = recognize_combined(&ring).expect_err("refused");
    assert!(why.contains("two axes"), "{why}");
    assert!(matches!(recognize(&ring), Recognition::Profile { .. }));
}

/// SolidCraft's plate under a ramp with a window through it along Y and one along X: each is an
/// opening in that axis's outline (the box's side along Y), and where they cross both take it.
#[test]
fn recognises_windows_through_the_outlines() {
    let mesh = fixture(include_bytes!("../../../../plugin/revmesh/tests/combined-windows.stl"));
    let (spec, _) = recognize_combined(&mesh).expect("combined");
    assert_eq!(spec.extrusions.iter().map(|e| e.axis).collect::<Vec<_>>(), [Axis::Z, Axis::X, Axis::Y]);
    let cutouts =
        |axis: Axis| spec.extrusions.iter().find(|e| e.axis == axis).map(|e| e.cutouts.iter().map(|c| from_lowest(c)).collect::<Vec<_>>());
    assert_eq!(cutouts(Axis::Z), Some(vec![]));
    assert_eq!(cutouts(Axis::X), Some(vec![vec![[5.0, 2.0], [9.0, 2.0], [9.0, 6.0], [5.0, 6.0]]]));
    // Along Y the sketch's second coordinate is −Z.
    assert_eq!(cutouts(Axis::Y), Some(vec![vec![[30.0, -8.0], [40.0, -8.0], [40.0, -3.0], [30.0, -3.0]]]));
    assert_eq!(spec.outline(Axis::Y).map(<[[f64; 2]]>::len), Some(4));
    assert!((spec.volume() - 35_410.0).abs() < 1e-6, "{}", spec.volume());
    assert_eq!(spec.describe(), "A 60 x 40 x 30 mm part of outlines extruded along X, Y and Z with 2 cut-outs");
    // Each opening is drawn in its outline's sketch.
    let json: serde_json::Value = serde_json::from_str(&spec.to_design("").to_json()).expect("json");
    let lines = |k: usize| json["features"][k]["sketch"]["curves"].as_array().expect("curves").len();
    assert_eq!([lines(0), lines(2), lines(4)], [6, 5 + 4, 4 + 4]);
    // ... and its extrusion takes the outline's loop (the openings its holes), not every region.
    assert_eq!(json["features"][1]["profiles"], serde_json::json!({"type": "all"}));
    assert_eq!(json["features"][3]["profiles"], serde_json::json!({"type": "curves", "loops": [["l1", "l2", "l3", "l4", "l5"]]}));
}

/// The design: one sketch and extrusion per outline, the first a new body and the next
/// intersecting it, and a move to the part's place, driven by mesh2cad's names.
#[test]
fn writes_an_intersection_of_extrusions() {
    let Recognition::Combined { spec, .. } = recognize(&combined()) else { panic!("not combined") };
    let json: serde_json::Value = serde_json::from_str(&spec.to_design_at("", [1.0, 2.0, 3.0]).to_json()).expect("json");
    let names: Vec<&str> = json["params"].as_array().expect("params").iter().map(|p| p["name"].as_str().expect("name")).collect();
    assert_eq!(names, ["extrude_z_len", "extrude_z_0", "extrude_x_len", "extrude_x_0"]);
    let features = json["features"].as_array().expect("features");
    let kinds: Vec<(&str, &str)> =
        features.iter().map(|f| (f["type"].as_str().expect("type"), f["operation"].as_str().unwrap_or(""))).collect();
    assert_eq!(kinds, [("sketch", ""), ("extrude", "new_body"), ("sketch", ""), ("extrude", "intersect"), ("move", "")]);
    assert_eq!(features[0]["plane"]["name"], "XY");
    assert_eq!(features[2]["plane"]["name"], "YZ");
    assert_eq!(features[3]["extent"]["distance"], "extrude_x_len");
    assert_eq!(features[4]["translate"], serde_json::json!(["extrude_x_0", "2", "extrude_z_0"]));
}

/// The spec's volume follows the outlines it holds.
#[test]
fn volume_of_a_spec() {
    let spec = CombinedSpec {
        size: [60.0, 40.0, 30.0],
        extrusions: vec![crate::recognize::Extrusion { axis: Axis::X, outline: RAMP.to_vec(), cutouts: vec![] }],
        cuts: vec![],
    };
    assert!((spec.volume() - 950.0 * 60.0).abs() < 1e-6);
}
