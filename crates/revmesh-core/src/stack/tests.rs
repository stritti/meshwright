use super::recognize_stack;

use crate::mesh::Mesh;
use crate::recognize::{Axis, Chamfer, Recognition, recognize};

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

/// Roadmap 5.4: a stack's walls are measured per band, labelled `band <k> side <i>` in the
/// stack's axis frame — along Z, and along X and Y when the part lies that way.
#[test]
fn deviations_are_reported_per_band() {
    for axis in [Axis::Z, Axis::X, Axis::Y] {
        let mesh = if axis == Axis::Z { stack() } else { axis.unrotate_mesh(&stack()) };
        let found = recognize(&mesh);
        let devs = crate::recognize::deviations(&mesh, &found);
        let labels: Vec<&str> = devs.iter().map(|(l, _)| l.as_str()).collect();
        assert!(labels.contains(&"band 1 side 1"), "{axis:?}: {labels:?}");
        assert!(labels.contains(&"band 2 side 1"), "{axis:?}: {labels:?}");
        assert!(labels.contains(&"band 3 side 1"), "{axis:?}: {labels:?}");
        // The fixture's walls are straight, so every sampled point lies on its wall.
        for (l, d) in &devs {
            assert!(*d < 0.01, "{axis:?} {l}: {d}");
        }
    }
}

/// Two copies of one prism, one on top of the other: three levels, but every band has the same
/// section (a 40 x 30 outline with a square through hole), so the bands merge into one.
#[test]
fn one_extrusion_split_at_a_level_is_no_stack() {
    let outline = [[0.0, 0.0], [40.0, 0.0], [40.0, 30.0], [0.0, 30.0]];
    let hole = vec![[15.0, 10.0], [25.0, 10.0], [25.0, 20.0], [15.0, 20.0]];
    let mut lower = crate::synth::prism(&outline, std::slice::from_ref(&hole), 5.0, [0.0; 3]);
    let upper = crate::synth::prism(&outline, &[hole], 5.0, [0.0, 0.0, 5.0]);
    lower.triangles.extend(upper.triangles);
    let err = recognize_stack(&lower).expect_err("one extrusion");
    assert!(err.ends_with("it is one extrusion"), "{err}");
}

/// A stray facet on the top level but outside the diamond lies on no band's wall or step.
#[test]
fn a_facet_on_no_band_is_no_stack() {
    let mut mesh = stack();
    mesh.triangles.push([[1.0, 1.0, 28.0], [3.0, 1.0, 28.0], [1.0, 3.0, 28.0]]);
    let err = recognize_stack(&mesh).expect_err("a stray facet");
    assert!(err.ends_with("a face lies on no band's wall or step"), "{err}");
}

/// Every facet still lies on a band, but one facet of the diamond's top is missing: the mesh no
/// longer holds the bands' volume, and the volume gate refuses it.
#[test]
fn a_volume_off_by_more_than_the_gate_is_no_stack() {
    let mut mesh = stack();
    let top = mesh.triangles.iter().position(|t| t.iter().all(|p| (p[2] - 28.0).abs() < 1e-6)).expect("a top facet");
    mesh.triangles.remove(top);
    let err = recognize_stack(&mesh).expect_err("volume off");
    assert!(err.contains("its bands hold"), "{err}");
}

/// The facet count is capped before any section is cut.
#[test]
fn too_many_facets_is_no_stack() {
    let facet = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
    let err = recognize_stack(&Mesh::new(vec![facet; 100_001])).expect_err("too many facets");
    assert!(err.contains("more than 100000 facets"), "{err}");
}

/// The corner of the outline `ring` (counter-clockwise) at `i`, moved inward by `w`: the
/// intersection of its two sides' lines offset inward by `w`.
fn inset_corner(ring: &[[f64; 2]], i: usize, w: f64) -> [f64; 2] {
    let n = ring.len();
    let off = |a: [f64; 2], b: [f64; 2]| -> ([f64; 2], [f64; 2]) {
        let d = [b[0] - a[0], b[1] - a[1]];
        let len = d[0].hypot(d[1]);
        let m = [-d[1] / len, d[0] / len]; // inward (left of the CCW side)
        ([a[0] + m[0] * w, a[1] + m[1] * w], m)
    };
    let (p1, m1) = off(ring[(i + n - 1) % n], ring[i]);
    let (p2, m2) = off(ring[i], ring[(i + 1) % n]);
    let det = m1[0] * m2[1] - m1[1] * m2[0];
    let (r1, r2) = (p1[0] * m1[0] + p1[1] * m1[1], p2[0] * m2[0] + p2[1] * m2[1]);
    [(r1 * m2[1] - r2 * m1[1]) / det, (m1[0] * r2 - m2[0] * r1) / det]
}

/// A stack of two bands: a base `bw` x `bd` from z 0 to `h0`, and a top band over `outline`
/// (counter-clockwise) from `h0` to `h0 + h1`, its top edges all chamfered by `c` (roadmap 3.12).
/// When the chamfers meet (the inset outline degenerates), the top is a roof with no top face.
fn chamfered_stack(bw: f64, bd: f64, h0: f64, outline: &[[f64; 2]], h1: f64, c: Chamfer) -> Mesh {
    let big = [[0.0, 0.0], [bw, 0.0], [bw, bd], [0.0, bd]];
    let (from, to) = (h0, h0 + h1);
    let at = |p: [f64; 2], z: f64| [p[0], p[1], z];
    let mut t: Vec<[[f64; 3]; 3]> = Vec::new();
    // The base: bottom face, walls, and the step at `from` outside the top band's footprint.
    for [a, b, cc] in crate::synth::triangulate(&big, &[]) {
        t.push([at(a, 0.0), at(cc, 0.0), at(b, 0.0)]);
    }
    for i in 0..4 {
        let (a, b) = (big[i], big[(i + 1) % 4]);
        t.push([at(a, 0.0), at(b, 0.0), at(b, from)]);
        t.push([at(a, 0.0), at(b, from), at(a, from)]);
    }
    for [a, b, cc] in crate::synth::triangulate(&big, &[outline.to_vec()]) {
        t.push([at(a, from), at(b, from), at(cc, from)]);
    }
    // The top band's walls from `from` to `to - h`, then the chamfer facets up to the inset.
    let n = outline.len();
    if to - c.h > from + 1e-9 {
        for i in 0..n {
            let (a, b) = (outline[i], outline[(i + 1) % n]);
            t.push([at(a, from), at(b, from), at(b, to - c.h)]);
            t.push([at(a, from), at(b, to - c.h), at(a, to - c.h)]);
        }
    }
    let inset: Vec<[f64; 2]> = (0..n).map(|i| inset_corner(outline, i, c.w)).collect();
    let area = (0..n).map(|i| inset[i][0] * inset[(i + 1) % n][1] - inset[(i + 1) % n][0] * inset[i][1]).sum::<f64>() / 2.0;
    if area > 1e-6 {
        // A bevel: the chamfer quads from each wall's top edge to the inset edge, and the
        // remaining top face.
        for i in 0..n {
            let (a, b) = (outline[i], outline[(i + 1) % n]);
            let (ai, bi) = (inset[i], inset[(i + 1) % n]);
            t.push([at(a, to - c.h), at(b, to - c.h), at(bi, to)]);
            t.push([at(a, to - c.h), at(bi, to), at(ai, to)]);
        }
        for [a, b, cc] in crate::synth::triangulate(&inset, &[]) {
            t.push([at(a, to), at(b, to), at(cc, to)]);
        }
    } else {
        // A roof: the chamfer facets meet at the inset's centre point.
        let apex = [inset.iter().map(|p| p[0]).sum::<f64>() / n as f64, inset.iter().map(|p| p[1]).sum::<f64>() / n as f64];
        for i in 0..n {
            let (a, b) = (outline[i], outline[(i + 1) % n]);
            t.push([at(a, to - c.h), at(b, to - c.h), at(apex, to)]);
        }
    }
    Mesh::new(t)
}

/// Roadmap 3.12: a stack whose top band's top edges carry one chamfer — a bevel here (w = h = 2
/// on a 30 x 20 top) — is recognised, its volume takes the chamfer off exactly, and the design
/// writes it as one `Chamfer1`.
#[test]
fn recognises_a_bevel_on_the_top_band() {
    let outline = [[15.0, 10.0], [45.0, 10.0], [45.0, 30.0], [15.0, 30.0]];
    let mesh = chamfered_stack(60.0, 40.0, 10.0, &outline, 10.0, Chamfer { w: 2.0, h: 2.0 });
    let (spec, offset) = recognize_stack(&mesh).expect("a stack");
    assert_eq!(offset, [0.0; 3]);
    assert_eq!(spec.bands.len(), 2);
    assert_eq!(spec.bands[1].chamfer, Some(Chamfer { w: 2.0, h: 2.0 }));
    // 60*40*10 + 30*20*10 less the chamfer: h*(P*w/2 - K*w^2/3) with P = 100, K = 4.
    let removed = 2.0 * (100.0 * 2.0 / 2.0 - 4.0 * 4.0 / 3.0);
    assert!((spec.volume() - (24_000.0 + 6_000.0 - removed)).abs() < 1e-9, "{}", spec.volume());
    assert!((spec.volume() - mesh.volume()).abs() < 1e-6, "{} vs {}", spec.volume(), mesh.volume());
    assert_eq!(spec.describe(Axis::Z), "A 60 x 40 x 20 mm part of 2 outlines stacked along Z with a 2 x 2 mm chamfer on its top");
    // The design: one Chamfer1 on the top band's top edges, driven by `chamfer` (a 45° one).
    let json: serde_json::Value = serde_json::from_str(&spec.to_design_on("", [0.0; 3], Axis::Z).to_json()).expect("json");
    let chamfer = json["features"].as_array().expect("features").iter().find(|f| f["name"] == "Chamfer1").expect("Chamfer1");
    assert_eq!(chamfer["type"], "chamfer");
    assert_eq!(chamfer["distance"], "chamfer");
    assert!(chamfer.get("distance2").is_none());
    assert_eq!(chamfer["edge_names"].as_array().expect("names").len(), 4);
    assert_eq!(chamfer["edge_names"][0], "F4:side:l1|F4:top");
    let value = |name: &str| json["params"].as_array().expect("params").iter().find(|p| p["name"] == name).map(|p| p["expr"].clone());
    assert_eq!(value("chamfer"), Some("2 mm".into()));
    // The deviation report measures the chamfer faces too.
    let found = Recognition::Stack { spec: spec.clone(), offset, axis: Axis::Z };
    let devs = crate::recognize::deviations(&mesh, &found);
    let labels: Vec<&str> = devs.iter().map(|(l, _)| l.as_str()).collect();
    assert!(labels.iter().any(|l| l.starts_with("band 2 chamfer")), "{labels:?}");
    for (l, d) in &devs {
        assert!(*d < 0.01, "{l}: {d}");
    }
}

/// Roadmap 3.12: a roof — the chamfers of all top edges meet at a point, so the top has no face
/// (a 20 x 20 top, w = 10, h = 5) — is recognised through the box's top as the virtual level.
#[test]
fn recognises_a_roof_on_the_top_band() {
    let outline = [[20.0, 10.0], [40.0, 10.0], [40.0, 30.0], [20.0, 30.0]];
    let mesh = chamfered_stack(60.0, 40.0, 10.0, &outline, 10.0, Chamfer { w: 10.0, h: 5.0 });
    let (spec, _) = recognize_stack(&mesh).expect("a stack");
    assert_eq!(spec.bands.len(), 2);
    assert_eq!(spec.bands[1].chamfer, Some(Chamfer { w: 10.0, h: 5.0 }));
    // The top slice 5 high less the pyramid to the apex: 20*20*5 - (1/3)*400*5.
    let want = 24_000.0 + 20.0 * 20.0 * 10.0 - (20.0 * 20.0 * 5.0 - 400.0 * 5.0 / 3.0);
    assert!((spec.volume() - want).abs() < 1e-9, "{}", spec.volume());
    assert!((spec.volume() - mesh.volume()).abs() < 1e-6, "{} vs {}", spec.volume(), mesh.volume());
    // The same roof lying along X is found there too, chamfer included.
    let lying = Axis::X.unrotate_mesh(&mesh);
    let (lying, _) = recognize_stack(&Axis::X.rotate_mesh(&lying)).expect("a stack along X");
    assert_eq!(lying.bands.last().and_then(|b| b.chamfer), Some(Chamfer { w: 10.0, h: 5.0 }));
    assert!((lying.volume() - want).abs() < 1e-9);
    // Not 45°, so the design drives it with `chamfer_w` and `chamfer_h`.
    let json: serde_json::Value = serde_json::from_str(&spec.to_design_on("", [0.0; 3], Axis::Z).to_json()).expect("json");
    let chamfer = json["features"].as_array().expect("features").iter().find(|f| f["name"] == "Chamfer1").expect("Chamfer1");
    assert_eq!(chamfer["distance"], "chamfer_w");
    assert_eq!(chamfer["distance2"], "chamfer_h");
    let value = |name: &str| json["params"].as_array().expect("params").iter().find(|p| p["name"] == name).map(|p| p["expr"].clone());
    assert_eq!(value("chamfer_w"), Some("10 mm".into()));
    assert_eq!(value("chamfer_h"), Some("5 mm".into()));
}

/// A chamfered top on a band whose outline is slanted in the profile plane (a diamond) has faces
/// tilted to every axis, so the combined recogniser refuses it and the whole `recognize` run
/// reaches the stack recogniser with the chamfer (roadmap 3.12).
#[test]
fn a_chamfered_diamond_top_is_a_stack() {
    let diamond = [[39.0, 20.0], [30.0, 29.0], [21.0, 20.0], [30.0, 11.0]];
    let mesh = chamfered_stack(60.0, 40.0, 10.0, &diamond, 10.0, Chamfer { w: 2.0, h: 2.0 });
    let found = recognize(&mesh);
    let Recognition::Stack { spec, axis, .. } = found else { panic!("not a stack: {found:?}") };
    assert_eq!(axis, Axis::Z);
    assert_eq!(spec.bands[1].chamfer, Some(Chamfer { w: 2.0, h: 2.0 }));
    // 60*40*10 + (18*18/2)*10 less the chamfer of the diamond's top (P = 4*9*sqrt(2), K = 4).
    let p = 4.0 * 9.0 * 2.0f64.sqrt();
    let removed = 2.0 * (p * 2.0 / 2.0 - 4.0 * 4.0 / 3.0);
    let want = 24_000.0 + 162.0 * 10.0 - removed;
    assert!((spec.volume() - want).abs() < 1e-9, "{}", spec.volume());
    assert!((spec.volume() - mesh.volume()).abs() < 1e-6, "{} vs {}", spec.volume(), mesh.volume());
}

/// A chamfer on only some of the top edges, a slanted face below the top and a chamfer on a
/// non-convex top are no stack (roadmap 3.12 stays exact-or-nothing).
#[test]
fn partial_or_wrong_chamfers_are_no_stack() {
    use crate::combined::silhouette::segment_distance;
    let outline = [[15.0, 10.0], [45.0, 10.0], [45.0, 30.0], [15.0, 30.0]];
    // Only three edges chamfered: drop the chamfer facets whose bottom corners lie on side 2.
    let mut mesh = chamfered_stack(60.0, 40.0, 10.0, &outline, 10.0, Chamfer { w: 2.0, h: 2.0 });
    let (a2, b2) = (outline[2], outline[3]);
    mesh.triangles.retain(|t| {
        let zs: Vec<f64> = t.iter().map(|p| p[2]).collect();
        let chamfer = zs.iter().any(|z| (*z - 18.0).abs() < 1e-9) && zs.iter().any(|z| (*z - 20.0).abs() < 1e-9);
        if !chamfer {
            return true;
        }
        let on_side2 = t.iter().filter(|p| (p[2] - 18.0).abs() < 1e-9).map(|p| [p[0], p[1]]).all(|p| segment_distance(p, a2, b2) < 1e-9);
        !on_side2
    });
    let err = recognize_stack(&mesh).expect_err("partial chamfer");
    assert!(err.contains("leaves an edge sharp"), "{err}");
    // A slanted face that does not reach the box's top.
    let mut low = chamfered_stack(60.0, 40.0, 10.0, &outline, 10.0, Chamfer { w: 2.0, h: 2.0 });
    low.triangles.push([[20.0, 15.0, 12.0], [30.0, 15.0, 12.0], [25.0, 20.0, 14.0]]);
    let err = recognize_stack(&low).expect_err("slanted below the top");
    assert!(err.contains("below its top"), "{err}");
    // A chamfer on a non-convex (L-shaped) top.
    let l = [[10.0, 10.0], [50.0, 10.0], [50.0, 20.0], [25.0, 20.0], [25.0, 35.0], [10.0, 35.0]];
    let mesh = chamfered_stack(60.0, 40.0, 10.0, &l, 10.0, Chamfer { w: 2.0, h: 2.0 });
    let err = recognize_stack(&mesh).expect_err("non-convex top");
    assert!(err.contains("not convex"), "{err}");
}

/// A closed mesh with no face square to the axis at all (a tetrahedron whose every slanted
/// facet touches the top) leaves the levels empty: refused with a reason, not a panic on
/// `levels[0]` (review finding on roadmap 3.12).
#[test]
fn a_mesh_with_no_levels_is_no_stack_not_a_panic() {
    let mesh = Mesh::new(vec![
        [[0.0, 0.0, 0.0], [0.0, 2.0, 2.0], [2.0, 0.0, 0.0]],
        [[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 0.0, 2.0]],
        [[0.0, 0.0, 0.0], [0.0, 0.0, 2.0], [0.0, 2.0, 2.0]],
        [[2.0, 0.0, 0.0], [0.0, 2.0, 2.0], [0.0, 0.0, 2.0]],
    ]);
    let err = recognize_stack(&mesh).expect_err("no levels");
    assert!(err.contains("does not step along its axis"), "{err}");
    let found = recognize(&mesh);
    assert!(!matches!(found, Recognition::Stack { .. }), "{found:?}");
}

/// The chamfer's volume coefficient at a corner is the tangent of half the *exterior* angle
/// (Steiner), not its cotangent: on an equilateral triangle each corner contributes tan(60°)
/// = sqrt(3), so K = 3*sqrt(3) — a rectangle's 90° turns hide the difference (review finding).
#[test]
fn a_chamfer_on_a_triangular_top_takes_the_exterior_tangent() {
    let side = 24.0;
    let height = side * 3.0f64.sqrt() / 2.0;
    let outline = [[8.0, 8.0], [32.0, 8.0], [20.0, 8.0 + height]];
    let mesh = chamfered_stack(40.0, 40.0, 10.0, &outline, 10.0, Chamfer { w: 2.0, h: 2.0 });
    // With the cotangent instead of the tangent the chamfer would be 9.3 mm³ too big and the
    // volume gate would refuse the stack.
    let (spec, _) = recognize_stack(&mesh).expect("a stack");
    assert_eq!(spec.bands.len(), 2);
    assert_eq!(spec.bands[1].chamfer, Some(Chamfer { w: 2.0, h: 2.0 }));
    // The mesh's own volume: h*(P*w/2 - K*w^2/3) with P = 3*side and K = 3*sqrt(3) (tan 60°).
    let removed = 2.0 * (3.0 * side * 2.0 / 2.0 - 3.0 * 3.0f64.sqrt() * 4.0 / 3.0);
    let want = 40.0 * 40.0 * 10.0 + side * height / 2.0 * 10.0 - removed;
    assert!((mesh.volume() - want).abs() < 1e-9, "{} vs {want}", mesh.volume());
    // The recognised outline's apex is snapped to micrometres, so allow that slack.
    assert!((spec.volume() - mesh.volume()).abs() < 0.1, "{} vs {}", spec.volume(), mesh.volume());
}

/// An exporter may tessellate a chamfer face into rows of coplanar triangles whose lower rows
/// do not reach the top: the plane as a whole does, and the chamfer is recognised still (review
/// finding on roadmap 3.12).
#[test]
fn a_tessellated_chamfer_is_still_recognised() {
    let outline = [[15.0, 10.0], [45.0, 10.0], [45.0, 30.0], [15.0, 30.0]];
    let mesh = chamfered_stack(60.0, 40.0, 10.0, &outline, 10.0, Chamfer { w: 2.0, h: 2.0 });
    let (spec, _) = recognize_stack(&mesh).expect("a stack");
    let mid = |a: [f64; 3], b: [f64; 3]| [0, 1, 2].map(|i| f64::midpoint(a[i], b[i]));
    let subdivided = Mesh::new(
        mesh.triangles
            .iter()
            .flat_map(|t| {
                let (ab, bc, ca) = (mid(t[0], t[1]), mid(t[1], t[2]), mid(t[2], t[0]));
                [[t[0], ab, ca], [ab, t[1], bc], [ca, bc, t[2]], [ab, bc, ca]]
            })
            .collect(),
    );
    let (found, _) = recognize_stack(&subdivided).expect("still a stack");
    assert_eq!(found.bands.len(), spec.bands.len());
    assert_eq!(found.bands[1].chamfer, Some(Chamfer { w: 2.0, h: 2.0 }));
    assert!((found.volume() - mesh.volume()).abs() < 1e-6, "{} vs {}", found.volume(), mesh.volume());
}

/// Many distinct slanted planes (a finely tessellated curved surface) are refused at a cap
/// instead of being grouped one by one — the grouping stays linear in the facet count (review
/// finding on roadmap 3.12).
#[test]
fn too_many_slanted_planes_are_no_stack() {
    let outline = [[15.0, 10.0], [45.0, 10.0], [45.0, 30.0], [15.0, 30.0]];
    let mut mesh = chamfered_stack(60.0, 40.0, 10.0, &outline, 10.0, Chamfer { w: 2.0, h: 2.0 });
    // 300 stray slanted triangles fanned around a point, each in its own plane.
    for k in 0..300 {
        let a = f64::from(k) * std::f64::consts::TAU / 300.0;
        mesh.triangles.push([[30.0, 20.0, 20.0], [30.0 + a.cos(), 20.0 + a.sin(), 19.0], [30.0 - a.sin(), 20.0 + a.cos(), 19.0]]);
    }
    let err = recognize_stack(&mesh).expect_err("too many slanted planes");
    assert!(err.contains("more than 256 slanted faces"), "{err}");
}

/// An asymmetric chamfer on a stack along X or Y: at an edge whose side face points up in world
/// space that side is the up-most face, so its chamfer feature swaps `distance` and
/// `distance2` (review finding on roadmap 3.12).
#[test]
fn an_asymmetric_chamfer_along_x_swaps_the_distances() {
    let outline = [[15.0, 10.0], [45.0, 10.0], [45.0, 30.0], [15.0, 30.0]];
    let mesh = chamfered_stack(60.0, 40.0, 10.0, &outline, 10.0, Chamfer { w: 3.0, h: 2.0 });
    let lying = Axis::X.unrotate_mesh(&mesh);
    let (spec, frame_at) = recognize_stack(&Axis::X.rotate_mesh(&lying)).expect("a stack along X");
    assert_eq!(spec.bands[1].chamfer, Some(Chamfer { w: 3.0, h: 2.0 }));
    let json: serde_json::Value =
        serde_json::from_str(&spec.to_design_on("", Axis::X.unrotate(frame_at), Axis::X).to_json()).expect("json");
    let chamfers: Vec<&serde_json::Value> =
        json["features"].as_array().expect("features").iter().filter(|f| f["type"] == "chamfer").collect();
    assert_eq!(chamfers.len(), 2, "{chamfers:?}");
    let c1 = chamfers.iter().find(|c| c["name"] == "Chamfer1").expect("Chamfer1");
    assert_eq!(c1["distance"], "chamfer_w");
    assert_eq!(c1["distance2"], "chamfer_h");
    assert_eq!(c1["edge_names"].as_array().expect("names").len(), 3);
    let c2 = chamfers.iter().find(|c| c["name"] == "Chamfer2").expect("Chamfer2");
    assert_eq!(c2["distance"], "chamfer_h");
    assert_eq!(c2["distance2"], "chamfer_w");
    assert_eq!(c2["edge_names"].as_array().expect("names").len(), 1);
    // A Z stack keeps one Chamfer1 with the extrusion's top face first.
    let (spec_z, at_z) = recognize_stack(&mesh).expect("a stack along Z");
    let json: serde_json::Value = serde_json::from_str(&spec_z.to_design_on("", at_z, Axis::Z).to_json()).expect("json");
    let chamfers: Vec<&serde_json::Value> =
        json["features"].as_array().expect("features").iter().filter(|f| f["type"] == "chamfer").collect();
    assert_eq!(chamfers.len(), 1);
    assert_eq!(chamfers[0]["name"], "Chamfer1");
    assert_eq!(chamfers[0]["distance"], "chamfer_w");
    assert_eq!(chamfers[0]["distance2"], "chamfer_h");
    assert_eq!(chamfers[0]["edge_names"].as_array().expect("names").len(), 4);
}

/// A roof whose chamfer height equals the top band's height consumes its walls entirely: the
/// top band has no wall facet, and its footprint comes from the chamfer planes' bottom edges
/// (review finding on roadmap 3.12).
#[test]
fn a_roof_that_consumes_the_top_bands_walls_is_a_stack() {
    let outline = [[20.0, 10.0], [40.0, 10.0], [40.0, 30.0], [20.0, 30.0]];
    // A 60 x 40 x 10 base with a 20 x 20 pyramid on top rising the band's whole height.
    let mesh = chamfered_stack(60.0, 40.0, 10.0, &outline, 10.0, Chamfer { w: 10.0, h: 10.0 });
    let (spec, _) = recognize_stack(&mesh).expect("a stack");
    assert_eq!(spec.bands.len(), 2);
    assert_eq!(spec.bands[1].outline.len(), 4);
    assert_eq!(spec.bands[1].chamfer, Some(Chamfer { w: 10.0, h: 10.0 }));
    // 24000 plus the pyramid's 400*10/3.
    let want = 24_000.0 + 400.0 * 10.0 / 3.0;
    assert!((spec.volume() - want).abs() < 1e-9, "{} vs {want}", spec.volume());
    assert!((spec.volume() - mesh.volume()).abs() < 1e-6, "{} vs {}", spec.volume(), mesh.volume());
    // Lying along X, a symmetric roof is one chamfer feature over all edges — splitting the
    // edges would let the first chamfer consume the top face before the second applies (review
    // finding).
    let lying = Axis::X.unrotate_mesh(&mesh);
    let (spec_x, frame_at) = recognize_stack(&Axis::X.rotate_mesh(&lying)).expect("a stack along X");
    let json: serde_json::Value =
        serde_json::from_str(&spec_x.to_design_on("", Axis::X.unrotate(frame_at), Axis::X).to_json()).expect("json");
    let chamfers: Vec<&serde_json::Value> =
        json["features"].as_array().expect("features").iter().filter(|f| f["type"] == "chamfer").collect();
    assert_eq!(chamfers.len(), 1, "{chamfers:?}");
    assert_eq!(chamfers[0]["name"], "Chamfer1");
    assert_eq!(chamfers[0]["distance"], "chamfer");
    assert!(chamfers[0].get("distance2").is_none());
    assert_eq!(chamfers[0]["edge_names"].as_array().expect("names").len(), 4);
}
