use super::{DesignBuilder, Extent, FORMAT, FeatureKind, MeshData, Operation, Plane, Profiles, fmt_number, rectangle_sketch};
use crate::mesh::Mesh;

#[test]
fn numbers_drop_trailing_zero() {
    assert_eq!(fmt_number(60.0), "60");
    assert_eq!(fmt_number(2.5), "2.5");
}

/// Roadmap 7.3: a mesh coordinate that survives a single-precision round trip is written as one
/// (the precision STL stores it in, so the design stays small), anything finer keeps its full
/// double-precision value, so parts far from the origin lose nothing in the mesh fallback.
#[test]
fn mesh_coordinates_keep_double_precision_beyond_f32() {
    let stl_precision = f64::from(0.1_f32); // 0.10000000149011612 as a double
    let mesh = Mesh::new(vec![[[stl_precision, 0.0, 0.0], [1_000_000.001, 0.0, 0.0], [0.5, 0.0, 0.0]]]);
    let json = serde_json::to_string(&MeshData::new("Body1", mesh)).expect("serializes");
    assert!(json.contains(r#""positions":[0.1,0.0,0.0,1000000.001,0.0,0.0,0.5,0.0,0.0]"#), "{json}");
}

#[test]
fn builder_numbers_features_and_reserves_bodies() {
    let mut b = DesignBuilder::new("t");
    let first =
        b.feature("Sketch1", vec![], FeatureKind::Sketch { plane: Plane::Origin { name: "XY" }, sketch: rectangle_sketch(1.0, 1.0) });
    let second = b.feature(
        "Extrude1",
        vec!["Body1".into()],
        FeatureKind::Extrude { sketch: first, profiles: Profiles::All, extent: Extent::positive("1"), operation: Operation::NewBody },
    );
    let design = b.build();
    assert_eq!((first, second, design.next_id), (1, 2, 3));
    assert_eq!(design.reserved_body_names, ["Body1"]);
    let json: serde_json::Value = serde_json::from_str(&design.to_json()).expect("valid JSON");
    assert_eq!(json["format"], FORMAT);
    assert_eq!(json["features"][1]["type"], "extrude");
    assert_eq!(json["features"][1]["profiles"]["type"], "all");
    assert!(json["features"][0].get("body_names").is_none());
}
