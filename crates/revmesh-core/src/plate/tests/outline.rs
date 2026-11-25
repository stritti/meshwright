use std::f64::consts::PI;

use crate::plate::P2;
use crate::plate::geometry::distance;
use crate::recognize::{ArcSide, HoleSpec, PlateSpec, ProfileSpec, recognize};
use crate::synth;

use super::{banded, ell, found, found_profile, reason, spec};

#[test]
fn recognises_finely_faceted_corners() {
    let expected = spec(&[(40.0, 25.0, 10.0)], Some(0.5));
    assert_eq!(found(&synth::plate(&expected, 720, [0.0; 3])).0, expected);
}

/// Single-precision coordinates (binary STL) put noise on finely faceted corners that a
/// circle through three neighbouring corners magnifies; the whole arc must be fitted.
#[test]
fn recognises_fine_corners_from_single_precision_files() {
    for (r, segments) in [(2.0, 128), (2.0, 256), (0.5, 128), (5.0, 720)] {
        let expected = spec(&[(40.0, 25.0, 10.0)], Some(r));
        let mesh = synth::plate(&expected, segments, [0.0; 3]);
        let reread = crate::stl::parse(&crate::stl::write_binary(&mesh)).expect("round trip");
        assert_eq!(found(&reread).0, expected, "radius {r}, {segments} segments");
        let l = ell(Some(r));
        let mesh = synth::profile(&l, segments, [12.5, 7.0, 0.0]);
        let reread = crate::stl::parse(&crate::stl::write_binary(&mesh)).expect("round trip");
        assert_eq!(found_profile(&reread).0, l, "L, radius {r}, {segments} segments");
    }
}

#[test]
fn recognises_an_l_shaped_outline_with_inside_and_outside_corners() {
    for r in [None, Some(2.0)] {
        let expected = ell(r);
        let (profile, offset) = found_profile(&synth::profile(&expected, 32, [0.0; 3]));
        assert_eq!(profile, expected, "radius {r:?}");
        assert_eq!(offset, [0.0; 3]);
    }
}

#[test]
fn recognises_an_offset_l_from_a_single_precision_file() {
    let expected = ell(Some(3.0));
    let mesh = synth::profile(&expected, 48, [100.0, -20.0, 0.0]);
    let reread = crate::stl::parse(&crate::stl::write_binary(&mesh)).expect("round trip");
    let (profile, offset) = found_profile(&reread);
    assert_eq!(profile, expected);
    assert_eq!(offset, [100.0, -20.0, 0.0]);
}

#[test]
fn recognises_slanted_sides() {
    let expected = ProfileSpec {
        corners: vec![[0.0, 0.0], [50.0, 0.0], [60.0, 20.0], [25.0, 35.0], [0.0, 20.0]],
        z: 4.0,
        holes: vec![HoleSpec { x: 25.0, y: 15.0, d: 8.0, head: None, blind: None }],
        cutouts: vec![],
        pads: vec![],
        pockets: vec![],
        outline_pockets: Vec::new(),
        bosses: vec![],
        slots: vec![],
        arcs: Vec::new(),
        outline_fillet_r: Some(3.0),
    };
    let mesh = synth::profile(&expected, 64, [0.0; 3]);
    let reread = crate::stl::parse(&crate::stl::write_binary(&mesh)).expect("round trip");
    assert_eq!(found_profile(&reread).0, expected);
}

/// Roadmap 5.2: a side of the outline that is a circular arc is recognised as an arc side —
/// here the top of a 60 x 40 mm plate bulging out as a semicircle of radius 30 about its middle.
#[test]
fn recognises_an_arc_side_in_the_outline() {
    let expected = ProfileSpec {
        outline_pockets: vec![],
        corners: vec![[0.0, 0.0], [60.0, 0.0], [60.0, 40.0], [0.0, 40.0]],
        z: 5.0,
        holes: vec![],
        slots: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![],
        cutouts: vec![],
        arcs: vec![None, None, Some(ArcSide { centre: [30.0, 40.0], r: 30.0 }), None],
        outline_fillet_r: None,
    };
    for segments in [4, 32] {
        let mesh = synth::profile(&expected, segments, [0.0; 3]);
        let reread = crate::stl::parse(&crate::stl::write_binary(&mesh)).expect("round trip");
        let (profile, offset) = found_profile(&reread);
        assert_eq!(profile, expected, "{segments} segments");
        assert_eq!(offset, [0.0; 3]);
    }
}

/// A stadium outline — two straight sides and a semicircle arc side at each end — is
/// recognised as a profile of four corners with two arc sides (the profile sits in its
/// bounding box's frame, so the left bulge to x = -10 puts the origin at [-10, 10, 0]).
#[test]
fn recognises_a_stadium_outline_as_two_arc_sides() {
    let shape = ProfileSpec {
        outline_pockets: vec![],
        corners: vec![[0.0, 10.0], [40.0, 10.0], [40.0, 30.0], [0.0, 30.0]],
        z: 4.0,
        holes: vec![],
        slots: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![],
        cutouts: vec![],
        arcs: vec![None, Some(ArcSide { centre: [40.0, 20.0], r: 10.0 }), None, Some(ArcSide { centre: [0.0, 20.0], r: 10.0 })],
        outline_fillet_r: None,
    };
    let mesh = synth::profile(&shape, 16, [0.0; 3]);
    let reread = crate::stl::parse(&crate::stl::write_binary(&mesh)).expect("round trip");
    let (profile, offset) = found_profile(&reread);
    assert_eq!(offset, [-10.0, 10.0, 0.0]);
    let expected = ProfileSpec {
        outline_pockets: vec![],
        corners: vec![[10.0, 0.0], [50.0, 0.0], [50.0, 20.0], [10.0, 20.0]],
        z: 4.0,
        holes: vec![],
        slots: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![],
        cutouts: vec![],
        arcs: vec![None, Some(ArcSide { centre: [50.0, 10.0], r: 10.0 }), None, Some(ArcSide { centre: [10.0, 10.0], r: 10.0 })],
        outline_fillet_r: None,
    };
    assert_eq!(profile, expected);
}

#[test]
fn chamfered_corners_make_an_eight_sided_outline() {
    // One facet per quarter circle is a 45° chamfer: an exact eight-sided outline.
    let (profile, _) = found_profile(&synth::plate(&spec(&[], Some(5.0)), 4, [0.0; 3]));
    let corners = vec![[5.0, 0.0], [75.0, 0.0], [80.0, 5.0], [80.0, 45.0], [75.0, 50.0], [5.0, 50.0], [0.0, 45.0], [0.0, 5.0]];
    assert_eq!(
        profile,
        ProfileSpec {
            corners,
            z: 5.0,
            holes: vec![],
            cutouts: vec![],
            pads: vec![],
            pockets: vec![],
            outline_pockets: Vec::new(),
            bosses: vec![],
            slots: vec![],
            arcs: Vec::new(),
            outline_fillet_r: None
        }
    );
}

#[test]
fn round_outline_is_not_a_polygon() {
    let circle: Vec<P2> = (0..64).map(|i| f64::from(i) * PI / 32.0).map(|a| [30.0 + 30.0 * a.cos(), 30.0 + 30.0 * a.sin()]).collect();
    assert!(reason(&synth::prism(&circle, &[], 5.0, [0.0; 3])).contains("round all the way round"));
}

/// An outline that is one long curve must be refused quickly, however finely it is faceted.
#[test]
fn long_curves_are_refused_quickly() {
    let started = std::time::Instant::now();
    for corners in [2_000, 8_000] {
        let ellipse: Vec<P2> = (0..corners)
            .map(|i| 2.0 * PI * f64::from(i) / f64::from(corners))
            .map(|a| [50.0 + 50.0 * a.cos(), 25.0 + 25.0 * a.sin()])
            .collect();
        assert!(reason(&synth::prism(&ellipse, &[], 5.0, [0.0; 3])).contains("not a circular arc"), "{corners} corners");
    }
    assert!(started.elapsed().as_secs_f64() < 5.0, "took {:?}", started.elapsed());
}

/// Arcs that stop 0.045 radians short of their sides meet them at a kink too small for the
/// angle check next to long sides, and their circles miss the sides by about 0.02 mm, so they
/// are no fillets — but as arc sides with sharp corners between them they are recognised.
#[test]
fn arcs_short_of_their_sides_are_no_fillets() {
    let (x, y, r, gap, facets) = (80.0, 50.0, 20.0, 0.045, 128);
    let centres = [[x - r, r, -PI / 2.0], [x - r, y - r, 0.0], [r, y - r, PI / 2.0], [r, r, PI]];
    let mut outline = Vec::new();
    for [cx, cy, start] in centres {
        for k in 0..=facets {
            let a = start + gap + (PI / 2.0 - 2.0 * gap) * f64::from(k) / f64::from(facets);
            outline.push([cx + r * a.cos(), cy + r * a.sin()]);
        }
    }
    let mesh = synth::prism(&outline, &[], 5.0, [0.0; 3]);
    let (profile, _) = found_profile(&mesh);
    // Not fillets (no corner radius), but arc sides with sharp corners between them.
    assert_eq!(profile.outline_fillet_r, None);
    assert_eq!(profile.corners.len(), 8);
    let arcs: Vec<ArcSide> = profile.arc_sides().iter().flatten().copied().collect();
    assert_eq!(arcs.len(), 4);
    assert!(arcs.iter().all(|a| (a.r - r).abs() < 0.001), "{arcs:?}");
}

/// Small rounds of many facets next to long sides, in single precision.
#[test]
fn small_fine_rounds() {
    let expected = PlateSpec {
        x: 80.0,
        y: 50.0,
        z: 5.0,
        holes: vec![],
        cutouts: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![],
        slots: vec![],
        outline_fillet_r: Some(0.5),
        edge_fillet_r: None,
    };
    let mesh = crate::stl::parse(&crate::stl::write_binary(&synth::plate(&expected, 96, [0.0; 3]))).expect("round trip");
    assert_eq!(found(&mesh).0, expected);
}

/// A cog of 2,000 sharp corners with walls in bands is checked in one pass over its corners.
#[test]
fn many_cornered_banded_outlines_are_quick() {
    let n = 2000;
    let cog: Vec<P2> = (0..n)
        .map(|i| {
            let a = 2.0 * PI * f64::from(i) / f64::from(n);
            let r = if i % 2 == 0 { 40.0 } else { 38.0 };
            [50.0 + r * a.cos(), 50.0 + r * a.sin()]
        })
        .collect();
    let mesh = banded(&synth::prism(&cog, &[], 5.0, [0.0; 3]), 0.0, [50.0, 50.0]);
    let started = std::time::Instant::now();
    let _ = recognize(&mesh);
    assert!(started.elapsed() < std::time::Duration::from_secs(2), "{:?}", started.elapsed());
}

/// Extra corners in the middle of the outline's facets (top and bottom alike) change nothing.
#[test]
fn subdivided_outline_facets() {
    let expected = PlateSpec {
        x: 80.0,
        y: 50.0,
        z: 5.0,
        holes: vec![],
        cutouts: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![],
        slots: vec![],
        outline_fillet_r: Some(3.0),
        edge_fillet_r: None,
    };
    let outline = synth::rounded(&[[0.0, 0.0], [80.0, 0.0], [80.0, 50.0], [0.0, 50.0]], Some(3.0), 32);
    let mut split = Vec::new();
    for i in 0..outline.len() {
        let (a, b) = (outline[i], outline[(i + 1) % outline.len()]);
        if distance(a, b) > 0.0 {
            split.push(a);
            split.push([f64::midpoint(a[0], b[0]), f64::midpoint(a[1], b[1])]);
        }
    }
    assert_eq!(found(&synth::prism(&split, &[], 5.0, [0.0; 3])).0, expected);
}

/// A round outline of 4,000 facets alternating 1 : 3 in length is scanned once, not from every
/// corner (which took seconds).
#[test]
fn uneven_round_outlines_are_refused_quickly() {
    let steps: Vec<f64> = (0..4000).map(|i| if i % 2 == 0 { 1.0 } else { 3.0 }).collect();
    let total: f64 = steps.iter().sum();
    let mut angle = 0.0_f64;
    let circle: Vec<P2> = steps
        .iter()
        .map(|s| {
            let p = [20.0 + 20.0 * angle.cos(), 20.0 + 20.0 * angle.sin()];
            angle += 2.0 * PI * s / total;
            p
        })
        .collect();
    let started = std::time::Instant::now();
    let why = reason(&synth::prism(&circle, &[], 5.0, [0.0; 3]));
    assert!(why.contains("round all the way round"), "{why}");
    assert!(started.elapsed() < std::time::Duration::from_secs(2), "{:?}", started.elapsed());
}

/// Corners rounded almost all the way along the short sides leave short straight sides
/// between the arcs, whose far corners lie within the tolerance of the circle.
#[test]
fn short_sides_between_large_rounds() {
    let expected = PlateSpec {
        x: 80.0,
        y: 50.0,
        z: 5.0,
        holes: vec![],
        cutouts: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![],
        slots: vec![],
        outline_fillet_r: Some(24.95),
        edge_fillet_r: None,
    };
    assert_eq!(found(&synth::plate(&expected, 32, [0.0; 3])).0, expected);
}

/// An arc side joined to a slanted side at a turn greater than the curve limit still spans all
/// its facets: the semicircular top here meets its right side at about 37 degrees, and the
/// outline keeps its four corners (codex finding on #68).
#[test]
fn arc_run_starts_after_a_sharp_junction() {
    let expected = ProfileSpec {
        outline_pockets: vec![],
        corners: vec![[0.0, 0.0], [60.0, 0.0], [90.0, 40.0], [30.0, 40.0]],
        z: 5.0,
        holes: vec![],
        slots: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![],
        cutouts: vec![],
        arcs: vec![None, None, Some(ArcSide { centre: [60.0, 40.0], r: 30.0 }), None],
        outline_fillet_r: None,
    };
    let mesh = synth::profile(&expected, 16, [0.0; 3]);
    let (profile, _) = found_profile(&mesh);
    assert_eq!(profile, expected);
}

/// A chord and its quarter-circle arc close a valid two-corner outline (a quarter pie). The
/// chord's ends touch the bounding box's lowest corner, so the profile sits in a frame at
/// [20, 20, 0].
#[test]
fn chord_and_arc_close_a_two_corner_outline() {
    let (cx, cy, r) = (20.0, 20.0, 20.0);
    let shape = ProfileSpec {
        outline_pockets: vec![],
        corners: vec![[cx + r, cy], [cx, cy + r]],
        z: 4.0,
        holes: vec![],
        slots: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![],
        cutouts: vec![],
        arcs: vec![Some(ArcSide { centre: [cx, cy], r }), None],
        outline_fillet_r: None,
    };
    let mesh = synth::profile(&shape, 16, [0.0; 3]);
    let (profile, offset) = found_profile(&mesh);
    assert_eq!(offset, [cx, cy, 0.0]);
    let expected = ProfileSpec {
        outline_pockets: vec![],
        corners: vec![[r, 0.0], [0.0, r]],
        z: 4.0,
        holes: vec![],
        slots: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![],
        cutouts: vec![],
        arcs: vec![Some(ArcSide { centre: [0.0, 0.0], r }), None],
        outline_fillet_r: None,
    };
    assert_eq!(profile, expected);
    assert!((profile.volume() - mesh.volume()).abs() / mesh.volume() < 0.01, "{} vs {}", profile.volume(), mesh.volume());
}

/// A banded arc wall (its facets split at mid-height, corners between the faces on the fitted
/// circle) passes the wall checks through the arc side's fitted circle.
#[test]
fn banded_arc_walls_pass_the_wall_checks() {
    let spec = ProfileSpec {
        outline_pockets: vec![],
        corners: vec![[0.0, 0.0], [60.0, 0.0], [60.0, 40.0], [0.0, 40.0]],
        z: 5.0,
        holes: vec![],
        slots: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![],
        cutouts: vec![],
        arcs: vec![None, None, Some(ArcSide { centre: [30.0, 40.0], r: 30.0 }), None],
        outline_fillet_r: None,
    };
    let mesh = banded(&synth::profile(&spec, 8, [0.0; 3]), 0.0, [30.0, 20.0]);
    let (profile, _) = found_profile(&mesh);
    assert_eq!(profile, spec);
}

/// Snapping can shave an arc's radius below half the distance of its snapped endpoints, which
/// would leave the sketch's Diameter constraint unsolvable; the radius is kept large enough.
#[test]
fn arc_radius_covers_its_snapped_chord() {
    let spec = ProfileSpec {
        outline_pockets: vec![],
        corners: vec![[0.0, 0.0], [60.0008, 0.0], [60.0008, 40.0], [0.0, 40.0]],
        z: 5.0,
        holes: vec![],
        slots: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![],
        cutouts: vec![],
        arcs: vec![None, None, Some(ArcSide { centre: [30.0004, 40.0], r: 30.0004 }), None],
        outline_fillet_r: None,
    };
    let mesh = synth::profile(&spec, 32, [0.0; 3]);
    let (profile, _) = found_profile(&mesh);
    let arc = profile.arc_sides()[2].expect("arc side");
    let chord = distance(profile.corners[2], profile.corners[3]);
    assert!(2.0 * arc.r >= chord - 1e-9, "diameter {} < chord {chord}", 2.0 * arc.r);
}

/// A concave arc side (a notch) sweeps clockwise around its circle; the sketch arcs are
/// counter-clockwise, so emitting it would rebuild the complementary bulge and the volume gate
/// would still pass — the run is refused instead.
#[test]
fn a_concave_arc_side_is_refused() {
    let (cx, cy, r) = (50.0, 100.0, 1.0);
    let seg = 32;
    let mut outline: Vec<P2> = vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [cx + r, cy]];
    for k in 1..seg {
        let t = -PI * f64::from(k) / f64::from(seg); // clockwise from angle 0 to -pi: the notch dips down
        outline.push([cx + r * t.cos(), cy + r * t.sin()]);
    }
    outline.push([cx - r, cy]);
    outline.push([0.0, 100.0]);
    let mesh = synth::prism(&outline, &[], 4.0, [0.0; 3]);
    let why = reason(&mesh);
    assert!(why.contains("concave arc side"), "{why}");
}

/// Straight sides about as long as the arc's facets adjoin a gentle junction and would be carried
/// into the run; their far corners lie off the circle, so the run is trimmed to the arc before the
/// fit (a 60 x 5 base under a radius-30 semicircle in 32 facets is an arc-sided outline, not
/// "not a circular arc").
#[test]
fn adjoining_short_straight_sides_are_trimmed_from_the_arc_fit() {
    let expected = ProfileSpec {
        outline_pockets: vec![],
        corners: vec![[0.0, 0.0], [60.0, 0.0], [60.0, 5.0], [0.0, 5.0]],
        z: 4.0,
        holes: vec![],
        slots: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![],
        cutouts: vec![],
        arcs: vec![None, None, Some(ArcSide { centre: [30.0, 5.0], r: 30.0 }), None],
        outline_fillet_r: None,
    };
    let mesh = synth::profile(&expected, 16, [0.0; 3]);
    let (profile, offset) = found_profile(&mesh);
    assert_eq!(offset, [0.0; 3]);
    assert_eq!(profile, expected);
}

/// The wall checks need the arc side's exact fitted circle: snapping centre and radius
/// independently would move the wall by more than the tolerance on a small part. Only the output
/// parameters snap.
#[test]
fn arc_sides_keep_the_exact_fit_and_only_the_params_snap() {
    use super::super::outline::outline_shape;
    let (cx, cy, r) = (5.00049, 5.00049, 5.00049);
    let seg = 32;
    let a = [cx + r, cy];
    let b = [cx - r, cy];
    let mut outline: Vec<P2> = vec![[b[0], 0.0], [a[0], 0.0], a];
    for k in 1..seg {
        let t = PI * f64::from(k) / f64::from(seg);
        outline.push([cx + r * t.cos(), cy + r * t.sin()]);
    }
    outline.push(b);
    let (_, _, side_arcs) = outline_shape(&outline, 0.001).expect("an arc side");
    let arc = side_arcs.iter().flatten().next().expect("one arc side");
    assert!(distance(arc.centre, [cx, cy]) < 1e-9 && (arc.r - r).abs() < 1e-9, "the exact fit: {arc:?}");
    assert!(arc.centre[0] - 5.0 > 1e-4, "the fitted centre goes out unsnapped: {arc:?}");
    let mesh = synth::prism(&outline, &[], 4.0, [0.0; 3]);
    let (profile, offset) = found_profile(&mesh);
    assert_eq!(offset, [0.0; 3]);
    let arcs: Vec<ArcSide> = profile.arc_sides().iter().flatten().copied().collect();
    // Centre and radius snap to 1 um, and the radius covers half the snapped chord (10.001 / 2).
    assert_eq!(arcs, vec![ArcSide { centre: [5.0, 5.0], r: 5.0005 }], "the params snap to 1 um");
}
