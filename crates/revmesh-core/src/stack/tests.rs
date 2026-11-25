use super::recognize_stack;

use crate::mesh::Mesh;
use crate::recognize::{Axis, Recognition, recognize};

fn stack() -> Mesh {
    crate::stl::parse(include_bytes!("../../../../plugin/revmesh/tests/stack.stl")).expect("parses")
}

/// The outline's corners from its lowest-then-leftmost one on.
fn from_lowest(outline: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let k = (0..outline.len())
        .min_by(|&a, &b| outline[a][1].total_cmp(&outline[b][1]).then(outline[a][0].total_cmp(&outline[b][0])))
        .expect("corners");
    outline[k..].iter().chain(&outline[..k]).copied().collect()
}

/// SolidCraft's stack: a 60 x 40 base 10 high, an L on it 10 high, and a diamond 8 high on top
/// reaching past the L (no intersection of outlines less straight cuts: its walls are slanted).
#[test]
fn recognises_a_stack_of_three() {
    let found = recognize(&stack());
    let Recognition::Stack { spec, offset, axis } = found else { panic!("not a stack: {found:?}") };
    assert_eq!((axis, offset), (Axis::Z, [0.0; 3]));
    let spans: Vec<(f64, f64)> = spec.bands.iter().map(|b| (b.from, b.to)).collect();
    assert_eq!(spans, [(0.0, 10.0), (10.0, 20.0), (20.0, 28.0)]);
    assert_eq!(from_lowest(&spec.bands[1].outline), [[5.0, 5.0], [45.0, 5.0], [45.0, 20.0], [25.0, 20.0], [25.0, 35.0], [5.0, 35.0]]);
    assert_eq!(from_lowest(&spec.bands[2].outline), [[40.0, 4.0], [49.0, 13.0], [40.0, 22.0], [31.0, 13.0]]);
    assert!((spec.volume() - 34_296.0).abs() < 1e-9, "{}", spec.volume());
    assert_eq!(spec.describe(axis), "A 60 x 40 x 28 mm part of 3 outlines stacked along Z");
}

/// The design: per band a sketch and an extrusion from where the band below ends, the first a
/// new body and the next joined to it, driven by mesh2cad's names.
#[test]
fn writes_joined_extrusions() {
    let Recognition::Stack { spec, axis, .. } = recognize(&stack()) else { panic!("not a stack") };
    let json: serde_json::Value = serde_json::from_str(&spec.to_design_on("", [1.0, 2.0, 3.0], axis).to_json()).expect("json");
    let params: Vec<(&str, &str)> = json["params"]
        .as_array()
        .expect("params")
        .iter()
        .map(|p| (p["name"].as_str().expect("name"), p["expr"].as_str().expect("expr")))
        .collect();
    // The extrusions keep mesh2cad's names, first.
    let extrude: Vec<(&str, &str)> = params.iter().copied().filter(|(n, _)| n.starts_with("extrude_")).collect();
    assert_eq!(
        extrude,
        [
            ("extrude_z1_len", "10 mm"),
            ("extrude_z1_0", "3 mm"),
            ("extrude_z2_len", "10 mm"),
            ("extrude_z2_0", "extrude_z1_0 + extrude_z1_len"),
            ("extrude_z3_len", "8 mm"),
            ("extrude_z3_0", "extrude_z2_0 + extrude_z2_len"),
        ]
    );
    // Each band's outline is dimensioned side by side, as `outline_<k>_<i>_len` (and
    // `outline_<k>_<i>_angle` on the diamond's longest slanted side).
    let user = |name: &str| params.iter().find(|(n, _)| *n == name).map(|(_, e)| *e);
    assert_eq!(user("outline_1_1_len"), Some("60 mm"));
    assert_eq!(user("outline_1_2_len"), Some("40 mm"));
    assert_eq!(user("outline_2_1_len"), Some("20 mm"));
    assert_eq!(user("outline_2_6_len"), Some("15 mm"));
    assert_eq!(user("outline_3_1_len"), Some("12.7279 mm"));
    assert_eq!(user("outline_3_4_angle"), Some("225 deg"));
    let features = json["features"].as_array().expect("features");
    // The sketches carry the Length/Angle constraints behind those parameters.
    let constraints = |k: usize| features[k]["sketch"]["constraints"].as_array().expect("constraints");
    let count = |k: usize, kind: &str| constraints(k).iter().filter(|c| c["type"] == kind).count();
    assert_eq!([count(0, "length"), count(2, "length"), count(4, "length")], [2, 4, 2]);
    assert_eq!([count(0, "angle"), count(2, "angle"), count(4, "angle")], [0, 0, 1]);
    let ops: Vec<&str> = features.iter().filter_map(|f| f["operation"].as_str()).collect();
    assert_eq!(ops, ["new_body", "join", "join"]);
    assert_eq!(features[1]["extent"].get("start_offset"), None);
    assert_eq!(features[3]["extent"]["start_offset"], "extrude_z2_0 - extrude_z1_0");
    assert_eq!(features[6]["translate"], serde_json::json!(["1", "2", "extrude_z1_0"]));
}

/// The same stack lying along X and along Y is found along that axis.
#[test]
fn stacks_along_x_and_y() {
    for axis in [Axis::X, Axis::Y] {
        let mesh = axis.unrotate_mesh(&stack());
        let found = recognize(&mesh);
        let Recognition::Stack { spec, axis: along, .. } = found else { panic!("not a stack along {axis:?}: {found:?}") };
        assert_eq!(along, axis);
        assert_eq!(spec.bands.len(), 3);
        assert!((spec.volume() - 34_296.0).abs() < 1e-9);
    }
}

/// A plate (one band) is no stack, nor a part with a slanted face, nor one whose step is lifted
/// off the band below (a face on no band's wall or step).
#[test]
fn what_is_no_stack() {
    let plate = crate::recognize::tests::block(10.0, 20.0, 5.0, [0.0; 3]);
    assert!(recognize_stack(&plate).is_err());
    assert!(recognize_stack(&crate::mesh::tests::tetrahedron(10.0)).is_err());
    let mut lifted = stack();
    for p in lifted.triangles.iter_mut().flatten() {
        if (p[2] - 20.0).abs() < 1e-4 && p[0] > 48.0 {
            p[2] = 20.5;
        }
    }
    assert!(recognize_stack(&lifted).is_err());
}

/// The stack with a block on top instead of the diamond is a stack too, but the intersection of
/// its outlines along X and Y less a cut along X is found first, as mesh2cad writes it.
#[test]
fn a_stack_of_blocks_is_an_intersection_first() {
    let steps = crate::stl::parse(include_bytes!("../../../../plugin/revmesh/tests/combined-steps.stl")).expect("parses");
    let (spec, _) = recognize_stack(&steps).expect("stack");
    assert_eq!(spec.bands.len(), 3);
    let found = recognize(&steps);
    let Recognition::Combined { spec, .. } = found else { panic!("not combined: {found:?}") };
    let [cut] = spec.cuts.as_slice() else { panic!("{:?}", spec.cuts) };
    assert_eq!((cut.axis, cut.from, cut.to), (Axis::X, 25.0, 60.0));
    assert!((spec.volume() - 35_000.0).abs() < 1e-6);
}
