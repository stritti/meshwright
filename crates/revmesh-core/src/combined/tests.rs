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
    // The cut's outline is dimensioned too, as `cut_1_<i>_len` (its 5 and 6 mm sides).
    assert_eq!(value("cut_1_1_len"), Some("5 mm".into()));
    assert_eq!(value("cut_1_2_len"), Some("6 mm".into()));
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
        holes: vec![],
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
    // Each opening is drawn in its outline's sketch (the X outline's slanted side links the
    // sketch's X axis in as one construction line on top of its 5 + 4 curves).
    let json: serde_json::Value = serde_json::from_str(&spec.to_design("").to_json()).expect("json");
    let lines = |k: usize| json["features"][k]["sketch"]["curves"].as_array().expect("curves").len();
    assert_eq!([lines(0), lines(2), lines(4)], [6, 5 + 4 + 1, 4 + 4]);
    // ... and its extrusion takes the outline's loop (the openings its holes), not every region.
    assert_eq!(json["features"][1]["profiles"], serde_json::json!({"type": "all"}));
    assert_eq!(json["features"][3]["profiles"], serde_json::json!({"type": "curves", "loops": [["l1", "l2", "l3", "l4", "l5"]]}));
    // The outlines' and openings' sides are dimensioned, as `outline_<a>_<i>_len` and
    // `cutout_<n>_<i>_len`.
    let names: Vec<&str> = json["params"].as_array().expect("params").iter().map(|p| p["name"].as_str().expect("name")).collect();
    assert!(names.iter().any(|n| n.starts_with("outline_x_") && n.ends_with("_len")), "{names:?}");
    assert!(names.contains(&"cutout_1_1_len") && names.contains(&"cutout_2_1_len"), "the openings are dimensioned too: {names:?}");
}

/// The design: one sketch and extrusion per outline, the first a new body and the next
/// intersecting it, and a move to the part's place, driven by mesh2cad's names.
#[test]
fn writes_an_intersection_of_extrusions() {
    let Recognition::Combined { spec, .. } = recognize(&combined()) else { panic!("not combined") };
    let json: serde_json::Value = serde_json::from_str(&spec.to_design_at("", [1.0, 2.0, 3.0]).to_json()).expect("json");
    let names: Vec<&str> = json["params"].as_array().expect("params").iter().map(|p| p["name"].as_str().expect("name")).collect();
    // mesh2cad's names for the extrusions come first; the outlines' driving sides follow, as
    // `outline_z_<i>_len` for the plate seen from above and `outline_x_<i>_len` for the ramp,
    // its slanted side carrying `outline_x_<i>_angle`.
    assert_eq!(&names[..4], ["extrude_z_len", "extrude_z_0", "extrude_x_len", "extrude_x_0"]);
    assert!(names.iter().any(|n| n.starts_with("outline_z_") && n.ends_with("_len")), "{names:?}");
    assert!(names.iter().any(|n| n.starts_with("outline_x_") && n.ends_with("_len")), "{names:?}");
    assert!(names.iter().any(|n| n.starts_with("outline_x_") && n.ends_with("_angle")), "{names:?}");
    let features = json["features"].as_array().expect("features");
    let constraints = |k: usize| features[k]["sketch"]["constraints"].as_array().expect("constraints");
    assert!(constraints(0).iter().filter(|c| c["type"] == "length").count() >= 2, "the Z outline's driving sides carry Lengths");
    assert_eq!(constraints(2).iter().filter(|c| c["type"] == "angle").count(), 1, "the ramp's slanted side carries the Angle");
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
        holes: vec![],
    };
    assert!((spec.volume() - 950.0 * 60.0).abs() < 1e-6);
}

/// SolidCraft's plate under a ramp with a Ø6 hole through it along Z, a Ø4 one along X and a
/// Ø4 blind hole 10 deep into its front along Y: SolidCraft meshes the bores' walls with staggered
/// corners, tilted a little to every axis.
#[test]
fn recognises_round_holes() {
    let mesh = fixture(include_bytes!("../../../../plugin/revmesh/tests/combined-holes.stl"));
    let (spec, offset) = recognize_combined(&mesh).expect("combined");
    assert_eq!(offset, [0.0; 3]);
    let holes: Vec<(Axis, [f64; 2], f64, f64, f64)> = spec.holes.iter().map(|h| (h.axis, h.centre, h.d, h.from, h.to)).collect();
    assert_eq!(
        holes,
        [(Axis::X, [12.0, 15.0], 4.0, 0.0, 60.0), (Axis::Y, [10.0, -8.0], 4.0, 0.0, 10.0), (Axis::Z, [50.0, 5.0], 6.0, 0.0, 30.0)]
    );
    // The outlines are the plate's and the ramp's again, their openings now holes.
    assert_eq!(spec.extrusions.iter().map(|e| (e.axis, e.cutouts.len())).collect::<Vec<_>>(), [(Axis::Z, 0), (Axis::X, 0)]);
    assert_eq!(spec.cuts.len(), 0);
    // 37 000 less π 3² 30, π 2² 60 and π 2² 10, as SolidCraft measures it.
    assert!((spec.volume() - 35_272.124_040_525_61).abs() < 1e-3, "{}", spec.volume());
    assert_eq!(spec.describe(), "A 60 x 40 x 30 mm part of outlines extruded along X and Z with 3 holes");
    // Cylinders cut out of the part in its own frame, driven by mesh2cad's names.
    let json: serde_json::Value = serde_json::from_str(&spec.to_design_at("", [1.0, 2.0, 3.0]).to_json()).expect("json");
    let features = json["features"].as_array().expect("features");
    let cylinders: Vec<&serde_json::Value> = features.iter().filter(|f| f["type"] == "cylinder").collect();
    assert_eq!(cylinders.len(), 3);
    assert_eq!(cylinders[0]["base"], serde_json::json!(["-1", "hole_1_y - 2", "hole_1_z - extrude_z_0"]));
    assert_eq!(cylinders[0]["height"], "extrude_x_len + 2");
    assert_eq!(cylinders[1]["base"], serde_json::json!(["hole_2_x - extrude_x_0", "-1", "hole_2_z - extrude_z_0"]));
    assert_eq!(cylinders[1]["axis"], serde_json::json!([0.0, 1.0, 0.0]));
    assert_eq!(cylinders[1]["height"], "hole_2_depth + 1");
    assert_eq!(cylinders[2]["radius"], "hole_3_d / 2");
    assert!(cylinders.iter().all(|c| c["operation"] == "cut"));
    let value = |name: &str| json["params"].as_array().expect("params").iter().find(|p| p["name"] == name).map(|p| p["expr"].clone());
    assert_eq!(value("hole_2_depth"), Some("10 mm".into()));
    assert_eq!(value("hole_2_z"), Some("11 mm".into()));
    assert_eq!(value("hole_3_x"), Some("51 mm".into()));
    assert_eq!(features.last().expect("move")["type"], "move");
}

/// A blind hole opening on the far face along its axis is drilled from that face, against it.
#[test]
fn blind_holes_from_the_far_face() {
    let spec = CombinedSpec {
        size: [20.0, 10.0, 10.0],
        extrusions: vec![crate::recognize::Extrusion {
            axis: Axis::Z,
            outline: vec![[0.0, 0.0], [20.0, 0.0], [20.0, 10.0], [0.0, 10.0]],
            cutouts: vec![],
        }],
        cuts: vec![],
        holes: vec![crate::recognize::CombinedHole { axis: Axis::Z, centre: [5.0, 5.0], d: 2.0, from: 6.0, to: 10.0 }],
    };
    // 2000 less π 1² 4, exactly: the part is as deep all over the hole.
    assert!((spec.volume() - (2000.0 - std::f64::consts::PI * 4.0)).abs() < 1e-9, "{}", spec.volume());
    let json: serde_json::Value = serde_json::from_str(&spec.to_design("").to_json()).expect("json");
    let hole = json["features"].as_array().expect("features").iter().find(|f| f["type"] == "cylinder").expect("hole").clone();
    assert_eq!(hole["base"][2], "extrude_z_len + 1");
    assert_eq!(hole["axis"], serde_json::json!([0.0, 0.0, -1.0]));
}

/// A 20 x 20 ramp, 20 high at X = 0 and 10 at X = 20, with a radius-9 bore through it along Z of
/// `sides` facets.
fn ramp_with_bore(sides: u32) -> Mesh {
    use std::f64::consts::TAU;
    let square = [[0.0, 0.0], [20.0, 0.0], [20.0, 20.0], [0.0, 20.0]];
    let bore: Vec<[f64; 2]> =
        (0..sides).map(|i| f64::from(i) * TAU / f64::from(sides)).map(|t| [10.0 + 9.0 * t.cos(), 10.0 + 9.0 * t.sin()]).collect();
    let mut mesh = crate::synth::prism(&square, &[bore], 20.0, [0.0; 3]);
    for p in mesh.triangles.iter_mut().flatten() {
        if p[2] > 0.0 {
            p[2] = 20.0 - p[0] / 2.0;
        }
    }
    mesh
}

/// A bore ending in the ramp's slope is no round hole yet (seen across it, it covers no
/// rectangle): it stays the opening its facets make, exactly.
#[test]
fn a_bore_into_a_slope_stays_an_opening() {
    let mesh = ramp_with_bore(12);
    let (spec, _) = recognize_combined(&mesh).expect("combined");
    assert_eq!(spec.holes.len(), 0);
    assert_eq!(spec.outline(Axis::Z).map(<[[f64; 2]]>::len), Some(4));
    assert_eq!(spec.extrusions.iter().map(|e| e.cutouts.iter().map(Vec::len).sum::<usize>()).sum::<usize>(), 12);
    // (Its corners rounded to micrometres, as every outline's are.)
    assert!((spec.volume() - mesh.volume()).abs() < 0.2, "{} vs {}", spec.volume(), mesh.volume());
}

/// A closed 20 x 20 x 10 box with a radius-9 blind hole of `sides` facets drilled 9 deep into its
/// top: walls from the floor at Z = 1 up to the top.
fn box_with_blind_bore(sides: u32) -> Mesh {
    use std::f64::consts::TAU;
    let square = [[0.0, 0.0], [20.0, 0.0], [20.0, 20.0], [0.0, 20.0]];
    let bore: Vec<[f64; 2]> =
        (0..sides).map(|i| f64::from(i) * TAU / f64::from(sides)).map(|t| [10.0 + 9.0 * t.cos(), 10.0 + 9.0 * t.sin()]).collect();
    let at = |p: [f64; 2], z: f64| [p[0], p[1], z];
    let mut t = Vec::new();
    for [a, b, c] in crate::synth::triangulate(&square, std::slice::from_ref(&bore)) {
        t.push([at(a, 10.0), at(b, 10.0), at(c, 10.0)]);
    }
    for [a, b, c] in crate::synth::triangulate(&square, &[]) {
        t.push([at(a, 0.0), at(c, 0.0), at(b, 0.0)]);
    }
    for i in 0..4 {
        let (a, b) = (square[i], square[(i + 1) % 4]);
        t.extend([[at(a, 0.0), at(b, 0.0), at(b, 10.0)], [at(a, 0.0), at(b, 10.0), at(a, 10.0)]]);
    }
    for [a, b, c] in crate::synth::triangulate(&bore, &[]) {
        t.push([at(a, 1.0), at(b, 1.0), at(c, 1.0)]);
    }
    let n = bore.len();
    for i in 0..n {
        let (p, q) = (bore[(i + 1) % n], bore[i]);
        t.extend([[at(p, 1.0), at(q, 1.0), at(q, 10.0)], [at(p, 1.0), at(q, 10.0), at(p, 10.0)]]);
    }
    Mesh::new(t)
}

/// A coarsely faceted bore holds less than its true circle: with 12 sides 5.7% less, past the 1%
/// a recognised part keeps; with 64 sides the hole is recognised.
#[test]
fn coarse_bores_keep_the_mesh() {
    let coarse = box_with_blind_bore(12);
    assert!(coarse.volume() > 0.0);
    let why = recognize_combined(&coarse).expect_err("refused");
    assert!(why.contains("too coarsely faceted"), "{why}");
    let (spec, _) = recognize_combined(&box_with_blind_bore(64)).expect("combined");
    let holes: Vec<_> = spec.holes.iter().map(|h| (h.axis, h.centre, h.d, h.from, h.to)).collect();
    assert_eq!(holes, [(Axis::Z, [10.0, 10.0], 18.0, 1.0, 10.0)]);
}

/// Roadmap 5.4: a combined part's walls are measured per extrusion, labelled `extrude <a> side
/// <i>`, its blind cuts' `extrude <a> cut <n> side <i>`, its holes' `extrude <a> hole <n>` — in
/// each extrusion's own axis frame.
#[test]
fn deviations_are_reported_per_extrusion() {
    let mesh = combined();
    let found = recognize(&mesh);
    let devs = crate::recognize::deviations(&mesh, &found);
    let labels: Vec<&str> = devs.iter().map(|(l, _)| l.as_str()).collect();
    assert!(labels.iter().any(|l| l.starts_with("extrude z side")), "{labels:?}");
    assert!(labels.iter().any(|l| l.starts_with("extrude x side")), "{labels:?}");
    for (l, d) in &devs {
        assert!(*d < 0.01, "{l}: {d}");
    }

    // A box cut along X and a triangular notch along Y are reported as cut walls.
    let mesh = fixture(include_bytes!("../../../../plugin/revmesh/tests/combined-cuts.stl"));
    let found = recognize(&mesh);
    let devs = crate::recognize::deviations(&mesh, &found);
    let labels: Vec<&str> = devs.iter().map(|(l, _)| l.as_str()).collect();
    assert!(labels.iter().any(|l| l.starts_with("extrude x cut 1 side")), "{labels:?}");
    assert!(labels.iter().any(|l| l.starts_with("extrude y cut 2 side")), "the Y cut is cut 2 design-wide: {labels:?}");

    // Openings are numbered design-wide across axes (`cutout_<n>_*`), so the report's
    // `cut-out <n>` labels match the editable parameters: the Y opening is cut-out 2 here.
    let mesh = fixture(include_bytes!("../../../../plugin/revmesh/tests/combined-windows.stl"));
    let found = recognize(&mesh);
    let devs = crate::recognize::deviations(&mesh, &found);
    let labels: Vec<&str> = devs.iter().map(|(l, _)| l.as_str()).collect();
    assert!(labels.iter().any(|l| l.starts_with("extrude x cut-out 1, side")), "{labels:?}");
    assert!(labels.iter().any(|l| l.starts_with("extrude y cut-out 2, side")), "{labels:?}");

    // The staggered, slightly tilted bore corners of combined-holes.stl are measured against
    // their circles: the holes' labels carry a real deviation, not zero.
    let mesh = fixture(include_bytes!("../../../../plugin/revmesh/tests/combined-holes.stl"));
    let found = recognize(&mesh);
    let devs = crate::recognize::deviations(&mesh, &found);
    let labels: Vec<&str> = devs.iter().map(|(l, _)| l.as_str()).collect();
    assert!(labels.iter().any(|l| l.starts_with("extrude z hole")), "{labels:?}");
    let hole = devs.iter().find(|(l, _)| l.starts_with("extrude z hole")).expect("a z hole");
    assert!(hole.1 > 0.0 && hole.1 < 0.1, "the bore's tilt: {hole:?}");
}
