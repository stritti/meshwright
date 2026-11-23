use super::{DesignBuilder, Extent, FORMAT, FeatureKind, Operation, Plane, Profiles, fmt_number, rectangle_sketch};

#[test]
fn numbers_drop_trailing_zero() {
    assert_eq!(fmt_number(60.0), "60");
    assert_eq!(fmt_number(2.5), "2.5");
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
