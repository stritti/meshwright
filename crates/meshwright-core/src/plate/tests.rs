use super::geometry::{distance, fit_circle};
use super::walls::Walls;
use crate::mesh::{Vec3, cross, sub};
use crate::recognize::recognize;
use crate::recognize::{Blind, BossSpec, HoleHead, HoleSpec, PadSpec, PlateSpec, PocketSpec, ProfileSpec, Recognition, SlotSpec};
use crate::synth;
use std::f64::consts::PI;

use super::P2;
use crate::mesh::Mesh;
fn spec(holes: &[(f64, f64, f64)], r: Option<f64>) -> PlateSpec {
    PlateSpec {
        x: 80.0,
        y: 50.0,
        z: 5.0,
        holes: holes.iter().map(|&(x, y, d)| HoleSpec { x, y, d, head: None, blind: None }).collect(),
        cutouts: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![],
        slots: vec![],
        outline_fillet_r: r,
    }
}

/// An L: 60 along X and 15 deep, with a 15 wide leg up to 40, one hole in each arm.
fn ell(r: Option<f64>) -> ProfileSpec {
    ProfileSpec {
        corners: vec![[0.0, 0.0], [60.0, 0.0], [60.0, 15.0], [15.0, 15.0], [15.0, 40.0], [0.0, 40.0]],
        z: 5.0,
        holes: vec![
            HoleSpec { x: 7.5, y: 30.0, d: 5.0, head: None, blind: None },
            HoleSpec { x: 45.0, y: 7.5, d: 6.0, head: None, blind: None },
        ],
        cutouts: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![],
        slots: vec![],
        outline_fillet_r: r,
    }
}

fn found(mesh: &Mesh) -> (PlateSpec, Vec3) {
    match recognize(mesh) {
        Recognition::Plate { plate, offset } => (plate, offset),
        other => panic!("not recognised as a rectangular plate: {other:?}"),
    }
}

fn found_profile(mesh: &Mesh) -> (ProfileSpec, Vec3) {
    match recognize(mesh) {
        Recognition::Profile { profile, offset } => (profile, offset),
        other => panic!("not recognised as a profile: {other:?}"),
    }
}

fn reason(mesh: &Mesh) -> String {
    match recognize(mesh) {
        Recognition::Unrecognized { reason } => reason,
        other => panic!("unexpectedly recognised {other:?}"),
    }
}

#[test]
fn fits_a_circle_exactly() {
    let ring: Vec<P2> = (0..12).map(|i| f64::from(i) * PI / 6.0).map(|a| [3.0 + 2.0 * a.cos(), -1.0 + 2.0 * a.sin()]).collect();
    let ([x, y], r) = fit_circle(&ring).expect("a circle");
    assert!((x - 3.0).abs() < 1e-12 && (y + 1.0).abs() < 1e-12 && (r - 2.0).abs() < 1e-12);
}

#[test]
fn recognises_holes_and_rounded_corners() {
    let expected = spec(&[(10.0, 10.0, 5.0), (40.0, 25.0, 12.0), (70.0, 38.5, 6.6)], Some(4.0));
    let (plate, offset) = found(&synth::plate(&expected, 32, [0.0; 3]));
    assert_eq!(plate, expected);
    assert_eq!(offset, [0.0; 3]);
}

#[test]
fn recognises_sharp_corners_and_coarse_holes() {
    let expected = spec(&[(20.0, 25.0, 8.0), (60.0, 25.0, 8.0)], None);
    assert_eq!(found(&synth::plate(&expected, 12, [0.0; 3])).0, expected);
}

#[test]
fn survives_single_precision_files() {
    let expected = spec(&[(12.5, 30.0, 3.4), (55.0, 20.0, 9.0)], Some(2.5));
    let mesh = synth::plate(&expected, 48, [0.0; 3]);
    let reread = crate::stl::parse(&crate::stl::write_binary(&mesh)).expect("round trip");
    assert_eq!(found(&reread).0, expected);
}

#[test]
fn moves_an_offset_plate_to_the_origin() {
    let expected = spec(&[(40.0, 25.0, 10.0)], Some(5.0));
    let (plate, offset) = found(&synth::plate(&expected, 32, [-40.0, -25.0, 3.0]));
    assert_eq!(plate, expected);
    assert_eq!(offset, [-40.0, -25.0, 3.0]);
}

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
        bosses: vec![],
        slots: vec![],
        outline_fillet_r: Some(3.0),
    };
    let mesh = synth::profile(&expected, 64, [0.0; 3]);
    let reread = crate::stl::parse(&crate::stl::write_binary(&mesh)).expect("round trip");
    assert_eq!(found_profile(&reread).0, expected);
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
            bosses: vec![],
            slots: vec![],
            outline_fillet_r: None
        }
    );
}

#[test]
fn round_outline_is_not_a_polygon() {
    let circle: Vec<P2> = (0..64).map(|i| f64::from(i) * PI / 32.0).map(|a| [30.0 + 30.0 * a.cos(), 30.0 + 30.0 * a.sin()]).collect();
    assert!(reason(&synth::prism(&circle, &[], 5.0, [0.0; 3])).contains("curved sides"));
}

/// A D-shaped cut-out (half a circle closed by its diameter) has every corner on a circle;
/// it must not become a round hole.
#[test]
fn d_shaped_cut_out_is_not_a_hole() {
    let outline = [[0.0, 0.0], [80.0, 0.0], [80.0, 50.0], [0.0, 50.0]];
    let half: Vec<P2> = (0..=32).map(|i| f64::from(i) * PI / 32.0).map(|a| [40.0 + 2.0 * a.cos(), 25.0 + 2.0 * a.sin()]).collect();
    assert!(reason(&synth::prism(&outline, &[half], 5.0, [0.0; 3])).contains("not round"));
    // A shaft's D-hole: a full circle with a flat cutting off a 74° arc.
    let flat = (0.8_f64).acos();
    let mut shaft: Vec<P2> = (0..=27)
        .map(|i| -PI + flat + f64::from(i) * (2.0 * PI - 2.0 * flat) / 27.0)
        .map(|a| [40.0 + 2.5 * a.cos(), 25.0 + 2.5 * a.sin()])
        .collect();
    shaft.rotate_left(14);
    assert!(reason(&synth::prism(&outline, &[shaft], 5.0, [0.0; 3])).contains("not round"));
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
        assert!(reason(&synth::prism(&ellipse, &[], 5.0, [0.0; 3])).contains("curved sides"), "{corners} corners");
    }
    assert!(started.elapsed().as_secs_f64() < 5.0, "took {:?}", started.elapsed());
}

/// A hole of six facets is no round hole but a hexagonal cut-out, rebuilt as it is.
#[test]
fn hexagonal_cut_out_is_not_a_hole() {
    let (plate, _) = found_profile(&synth::plate(&spec(&[(40.0, 25.0, 10.0)], None), 6, [0.0; 3]));
    assert_eq!(plate.holes, []);
    assert_eq!(plate.cutouts.len(), 1);
    assert_eq!(plate.cutouts[0].len(), 6);
    assert_eq!(plate.cutouts[0][0], [37.5, 20.67]);
}

/// Splits every wall facet at the middle of its vertical edge, moved `inward` mm towards
/// the plate's middle, as an exporter that meshes walls in bands would (with `inward` 0).
#[allow(clippy::float_cmp)] // the generator puts a wall's corners exactly above each other
fn banded(mesh: &Mesh, inward: f64, middle: P2) -> Mesh {
    let mut out = Vec::new();
    for t in &mesh.triangles {
        let vertical_edge = (0..3).find(|&k| {
            let (p, q) = (t[k], t[(k + 1) % 3]);
            p[0] == q[0] && p[1] == q[1] && p[2] != q[2]
        });
        let Some(k) = vertical_edge else {
            out.push(*t);
            continue;
        };
        let (p, q, r) = (t[k], t[(k + 1) % 3], t[(k + 2) % 3]);
        let toward = [middle[0] - p[0], middle[1] - p[1]];
        let len = toward[0].hypot(toward[1]);
        let m = [p[0] + toward[0] / len * inward, p[1] + toward[1] / len * inward, f64::midpoint(p[2], q[2])];
        out.push([p, m, r]);
        out.push([m, q, r]);
    }
    Mesh::new(out)
}

#[test]
fn recognises_walls_meshed_in_bands() {
    let expected = spec(&[(20.0, 25.0, 8.0), (60.0, 25.0, 6.0)], Some(4.0));
    let mesh = banded(&synth::plate(&expected, 32, [0.0; 3]), 0.0, [40.0, 25.0]);
    assert_eq!(found(&mesh).0, expected);
}

#[test]
fn a_dent_in_a_banded_wall_is_no_plate() {
    let mesh = banded(&synth::plate(&spec(&[(20.0, 25.0, 8.0)], None), 32, [0.0; 3]), 0.5, [40.0, 25.0]);
    let why = reason(&mesh);
    assert!(why.contains("not a flat plate") || why.contains("faces differ"), "{why}");
}

/// A 200 x 200 x 5 mm plate with a 2 mm blind hole 2.5 mm deep, its floor a fan of the rim's
/// corners: every corner between the faces lies on the hole's circle, and the floor is a floor.
#[test]
fn fanned_blind_floors_are_floors() {
    let (z, floor) = (5.0, 2.5);
    let outline = [[0.0, 0.0], [200.0, 0.0], [200.0, 200.0], [0.0, 200.0]];
    let ring = synth::rings(&[HoleSpec { x: 100.0, y: 100.0, d: 2.0, head: None, blind: None }], 16).remove(0);
    let lift = |p: P2, h: f64| [p[0], p[1], h];
    let mut t: Vec<crate::mesh::Triangle> = Vec::new();
    for [a, b, c] in synth::triangulate(&outline, std::slice::from_ref(&ring)) {
        t.push([lift(a, z), lift(b, z), lift(c, z)]);
    }
    for [a, b, c] in synth::triangulate(&outline, &[]) {
        t.push([lift(a, 0.0), lift(c, 0.0), lift(b, 0.0)]);
    }
    for i in 0..4 {
        let (a, b) = (outline[i], outline[(i + 1) % 4]);
        t.push([lift(a, 0.0), lift(b, 0.0), lift(b, z)]);
        t.push([lift(a, 0.0), lift(b, z), lift(a, z)]);
    }
    let m = ring.len();
    for i in 0..m {
        // The bore, seen from inside the hole, runs clockwise round the rim.
        let (a, b) = (ring[(i + 1) % m], ring[i]);
        t.push([lift(a, floor), lift(b, floor), lift(b, z)]);
        t.push([lift(a, floor), lift(b, z), lift(a, z)]);
    }
    for i in 1..m - 1 {
        t.push([lift(ring[0], floor), lift(ring[i], floor), lift(ring[i + 1], floor)]);
    }
    let mesh = Mesh::new(t);
    assert!(mesh.volume() > 0.0);
    let (plate, _) = found(&mesh);
    assert_eq!(plate.holes[0].blind, Some(Blind { depth: 2.5, bottom: false }));
    // Its floor's fan is no through hole's wall.
    assert!((plate.volume() - mesh.volume()).abs() / mesh.volume() < 1e-5);
}

/// A banded wall corner moved out to where the sharp corner would be is no wall: straight
/// sides end at their arcs' tangent points.
#[test]
#[allow(clippy::float_cmp)] // the same corner, as the generator wrote it into every facet
fn a_corner_out_of_its_arc_is_no_wall() {
    let spec = PlateSpec {
        x: 80.0,
        y: 50.0,
        z: 5.0,
        holes: vec![],
        cutouts: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![],
        slots: vec![],
        outline_fillet_r: Some(1.0),
    };
    let mut mesh = banded(&synth::plate(&spec, 32, [0.0; 3]), 0.0, [40.0, 25.0]);
    // The band corner nearest the bottom-left arc's middle goes to the sharp corner (0, 0).
    let arc_middle = [1.0 - std::f64::consts::FRAC_1_SQRT_2, 1.0 - std::f64::consts::FRAC_1_SQRT_2, 2.5];
    let nearest = mesh
        .triangles
        .iter()
        .flatten()
        .copied()
        .filter(|p| (p[2] - 2.5).abs() < 1e-9)
        .min_by(|p, q| {
            distance([p[0], p[1]], [arc_middle[0], arc_middle[1]]).total_cmp(&distance([q[0], q[1]], [arc_middle[0], arc_middle[1]]))
        })
        .expect("band corners");
    for t in &mut mesh.triangles {
        for p in t.iter_mut() {
            if *p == nearest {
                *p = [0.0, 0.0, 2.5];
            }
        }
    }
    let why = reason(&mesh);
    assert!(why.contains("not a flat plate") || why.contains("faces differ"), "{why}");
}

/// A solid between a top face (`outline` with `holes`) and a bottom face made of the same
/// corners moved by `shift`, walls joining them corner to corner: a sheared prism.
#[allow(clippy::float_cmp)] // corners compared as the generator wrote them
fn sheared(outline: &[P2], holes: &[Vec<P2>], z: f64, shift_outline: P2, shift_holes: P2) -> Mesh {
    let at = |p: P2, s: P2, h: f64| [p[0] + s[0], p[1] + s[1], h];
    let mut t: Vec<crate::mesh::Triangle> = Vec::new();
    for [a, b, c] in synth::triangulate(outline, holes) {
        t.push([at(a, [0.0; 2], z), at(b, [0.0; 2], z), at(c, [0.0; 2], z)]);
    }
    // The bottom face: the same triangulation, its outline and holes moved apart.
    let moved = |p: P2| {
        if holes.iter().flatten().any(|q| *q == p) { shift_holes } else { shift_outline }
    };
    for [a, b, c] in synth::triangulate(outline, holes) {
        t.push([at(a, moved(a), 0.0), at(c, moved(c), 0.0), at(b, moved(b), 0.0)]);
    }
    let reversed: Vec<Vec<P2>> = holes.iter().map(|r| r.iter().rev().copied().collect()).collect();
    for (ring, s) in std::iter::once((outline, shift_outline)).chain(reversed.iter().map(|r| (r.as_slice(), shift_holes))) {
        for i in 0..ring.len() {
            let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
            t.push([at(a, s, 0.0), at(b, s, 0.0), at(b, [0.0; 2], z)]);
            t.push([at(a, s, 0.0), at(b, [0.0; 2], z), at(a, [0.0; 2], z)]);
        }
    }
    Mesh::new(t)
}

/// Shearing keeps the faces' areas and the volume of a prism, so only the walls tell.
#[test]
#[allow(clippy::float_cmp)] // corners compared as the generator wrote them
fn sheared_walls_are_not_vertical() {
    let outline = [[0.0, 0.0], [80.0, 0.0], [80.0, 50.0], [0.0, 50.0]];
    let ring = synth::rings(&[HoleSpec { x: 40.0, y: 25.0, d: 8.0, head: None, blind: None }], 32);
    let leaning_hole = sheared(&outline, &ring, 5.0, [0.0; 2], [0.5, 0.0]);
    let why = reason(&leaning_hole);
    assert!(why.contains("not vertical") || why.contains("straight through"), "{why}");
    let leaning_block = sheared(&outline, &[], 5.0, [0.5, 0.0], [0.0; 2]);
    let why = reason(&leaning_block);
    assert!(why.contains("not vertical") || why.contains("faces differ"), "{why}");
}

/// A banded wall whose bottom band is shifted: the band corners lie on the wall, the bottom
/// ones do not.
#[test]
#[allow(clippy::float_cmp)] // corners compared as the generator wrote them
fn a_shifted_bottom_band_is_not_vertical() {
    let spec = PlateSpec {
        x: 80.0,
        y: 50.0,
        z: 5.0,
        holes: vec![],
        cutouts: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![],
        slots: vec![],
        outline_fillet_r: None,
    };
    let mut mesh = banded(&synth::plate(&spec, 32, [0.0; 3]), 0.0, [40.0, 25.0]);
    // Shift the whole bottom 0.5 mm along X: the mesh stays closed, the faces keep their
    // areas and the volume stays that of a prism, so only the walls' bottom corners tell.
    for t in &mut mesh.triangles {
        for p in t.iter_mut() {
            if p[2] == 0.0 {
                p[0] += 0.5;
            }
        }
    }
    let why = reason(&mesh);
    assert!(why.contains("not vertical"), "{why}");
}

/// A diamond's long diagonal sides go into the cells along them only, not into every cell
/// of their bounding boxes (about 256 x 256 here).
#[test]
fn diagonal_walls_fill_few_cells() {
    let diamond = [[50.0, 0.0], [100.0, 50.0], [50.0, 100.0], [0.0, 50.0]];
    let walls = Walls::new(&diamond, None, &[], &[], &[], 100.0, 1e-3);
    assert!(walls.cells.len() < 10_000, "{} cells", walls.cells.len());
    assert!(walls.holds([75.0, 25.0]) && walls.holds([25.0, 75.0]));
    assert!(!walls.holds([50.0, 50.0]) && !walls.holds([75.0, 26.0]));
}

/// Arcs that stop 0.045 radians short of their sides meet them at a kink too small for the
/// angle check next to long sides, but their circles miss the sides by about 0.02 mm.
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
    reason(&mesh);
}

/// A 200 x 200 x 5 plate whose bottom corner is clipped from (1, 0, 0) to (0, 1, 0), a slanted
/// facet joining them to (0, 0, 5): its corners all lie on the sides, its bottom edge does not.
#[test]
fn a_facet_across_a_corner_is_no_wall() {
    let (s, z) = (200.0, 5.0);
    let mut t: Vec<crate::mesh::Triangle> = vec![
        // Top and bottom.
        [[0.0, 0.0, z], [s, 0.0, z], [s, s, z]],
        [[0.0, 0.0, z], [s, s, z], [0.0, s, z]],
        [[1.0, 0.0, 0.0], [s, 0.0, 0.0], [s, s, 0.0]],
        [[1.0, 0.0, 0.0], [s, s, 0.0], [0.0, s, 0.0]],
        [[1.0, 0.0, 0.0], [0.0, s, 0.0], [0.0, 1.0, 0.0]],
        // Walls, the clipped corner among them.
        [[1.0, 0.0, 0.0], [s, 0.0, 0.0], [s, 0.0, z]],
        [[1.0, 0.0, 0.0], [s, 0.0, z], [0.0, 0.0, z]],
        [[s, 0.0, 0.0], [s, s, 0.0], [s, s, z]],
        [[s, 0.0, 0.0], [s, s, z], [s, 0.0, z]],
        [[s, s, 0.0], [0.0, s, 0.0], [0.0, s, z]],
        [[s, s, 0.0], [0.0, s, z], [s, s, z]],
        [[0.0, s, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, z]],
        [[0.0, s, 0.0], [0.0, 0.0, z], [0.0, s, z]],
        [[0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, z]],
    ];
    // Turn every facet to face away from the middle (the solid is convex).
    let middle = [s / 2.0, s / 2.0, z / 2.0];
    for tri in &mut t {
        let n = cross(sub(tri[1], tri[0]), sub(tri[2], tri[0]));
        let centre = [
            (tri[0][0] + tri[1][0] + tri[2][0]) / 3.0,
            (tri[0][1] + tri[1][1] + tri[2][1]) / 3.0,
            (tri[0][2] + tri[1][2] + tri[2][2]) / 3.0,
        ];
        if crate::mesh::dot(n, sub(centre, middle)) < 0.0 {
            tri.swap(1, 2);
        }
    }
    let mesh = Mesh::new(t);
    assert!(mesh.volume() > 0.0);
    let why = reason(&mesh);
    assert!(why.contains("not vertical"), "{why}");
}

/// A large 8-sided bore whose bottom rim is turned half a step against its top rim: the
/// wall's bottom edges are chords whose middles lie 3 mm inside the circle, cells away from
/// it, and still follow the wall.
#[test]
fn chords_of_independently_meshed_rims_follow_the_bore() {
    let (s, z, r) = (400.0, 5.0, 40.0);
    let outline = [[0.0, 0.0], [s, 0.0], [s, s], [0.0, s]];
    let ring = |phase: f64| -> Vec<P2> {
        (0..8).map(|i| (f64::from(i) + phase) * PI / 4.0).map(|a| [200.0 + r * a.cos(), 200.0 + r * a.sin()]).collect()
    };
    let (top_ring, bottom_ring) = (ring(0.0), ring(0.5));
    let lift = |p: P2, h: f64| [p[0], p[1], h];
    let mut t: Vec<crate::mesh::Triangle> = Vec::new();
    for [a, b, c] in synth::triangulate(&outline, std::slice::from_ref(&top_ring)) {
        t.push([lift(a, z), lift(b, z), lift(c, z)]);
    }
    for [a, b, c] in synth::triangulate(&outline, std::slice::from_ref(&bottom_ring)) {
        t.push([lift(a, 0.0), lift(c, 0.0), lift(b, 0.0)]);
    }
    for i in 0..4 {
        let (a, b) = (outline[i], outline[(i + 1) % 4]);
        t.push([lift(a, 0.0), lift(b, 0.0), lift(b, z)]);
        t.push([lift(a, 0.0), lift(b, z), lift(a, z)]);
    }
    for i in 0..8 {
        let (t0, t1, b0) = (top_ring[i], top_ring[(i + 1) % 8], bottom_ring[i]);
        let b_prev = bottom_ring[(i + 7) % 8];
        t.push([lift(t0, z), lift(b0, 0.0), lift(t1, z)]);
        t.push([lift(t0, z), lift(b_prev, 0.0), lift(b0, 0.0)]);
    }
    // Hole walls face the hole's axis, the others away from the plate's middle.
    for tri in &mut t {
        let n = cross(sub(tri[1], tri[0]), sub(tri[2], tri[0]));
        let centre = [
            (tri[0][0] + tri[1][0] + tri[2][0]) / 3.0,
            (tri[0][1] + tri[1][1] + tri[2][1]) / 3.0,
            (tri[0][2] + tri[1][2] + tri[2][2]) / 3.0,
        ];
        let in_bore = tri.iter().all(|p| ((p[0] - 200.0).hypot(p[1] - 200.0) - r).abs() < 1e-6);
        let away = if in_bore { [200.0 - centre[0], 200.0 - centre[1], 0.0] } else { sub(centre, [200.0, 200.0, z / 2.0]) };
        if (in_bore || n[2].abs() < 0.5) && crate::mesh::dot(n, away) < 0.0 {
            tri.swap(1, 2);
        }
    }
    let mesh = Mesh::new(t);
    assert!(mesh.volume() > 0.0);
    let walls = {
        let (plate, _) = found(&mesh);
        plate.holes
    };
    assert_eq!(walls.len(), 1);
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
    assert!(why.contains("curved sides"), "{why}");
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
    };
    assert_eq!(found(&synth::plate(&expected, 32, [0.0; 3])).0, expected);
}

/// Far from the origin single precision leaves hole corners about 0.2 µm off their circle;
/// rounding the hole's numbers must not add to that when the walls are checked.
#[test]
fn plates_far_from_the_origin() {
    let expected = spec(&[(20.0, 25.0, 8.0), (60.0, 25.0, 5.0)], Some(3.0));
    let mesh = synth::plate(&expected, 48, [4000.0, 4000.0, 0.0]);
    let reread = crate::stl::parse(&crate::stl::write_binary(&mesh)).expect("round trip");
    let (plate, offset) = found(&reread);
    assert_eq!(offset, [4000.0, 4000.0, 0.0]);
    assert_eq!(plate.holes.len(), 2);
}

/// A 70 x 40 x 6 plate exported from SolidCraft 0.45 with a 16 x 20 rectangular and a
/// hexagonal cut-out (7 mm to its corners) and an Ø5 hole.
#[test]
fn recognises_polygonal_cut_outs() {
    let mesh = crate::stl::parse(include_bytes!("../../../../plugin/meshwright/tests/cutouts.stl")).expect("parses");
    let (plate, _) = found_profile(&mesh);
    assert_eq!(plate.corners, [[0.0, 0.0], [70.0, 0.0], [70.0, 40.0], [0.0, 40.0]]);
    assert_eq!(plate.cutouts.len(), 2);
    assert_eq!(plate.cutouts[0], [[8.0, 10.0], [24.0, 10.0], [24.0, 30.0], [8.0, 30.0]]);
    assert_eq!(plate.cutouts[1].len(), 6);
    // SolidCraft measured 13,998.356 mm³; the hexagon's corners are rounded to 0.01 mm.
    assert!((plate.volume() - 13_998.356).abs() < 0.05, "{}", plate.volume());
}

/// A 70 x 40 x 6 plate exported from SolidCraft 0.45 with a 16 x 12 x 4 pad on its top face, a
/// 20 x 14 x 3 pocket in the top face, a 12 x 8 x 2 one in the bottom face and an Ø4 hole.
#[test]
fn recognises_pads_and_pockets() {
    let mesh = crate::stl::parse(include_bytes!("../../../../plugin/meshwright/tests/pockets.stl")).expect("parses");
    let (plate, _) = found(&mesh);
    assert_eq!(plate.pads, [PadSpec { x: 20.0, y: 20.0, w: 16.0, l: 12.0, h: 4.0 }]);
    let pocket = |x: f64, y: f64, w: f64, l: f64, depth: f64, bottom: bool| PocketSpec { x, y, w, l, depth, bottom };
    assert_eq!(plate.pockets, [pocket(50.0, 15.0, 20.0, 14.0, 3.0, false), pocket(50.0, 30.0, 12.0, 8.0, 2.0, true)]);
    // SolidCraft measured 16,460.602 mm³ for the part it exported.
    assert!((plate.volume() - 16_460.602).abs() < 0.001, "{}", plate.volume());
}

/// Two pockets of one footprint in the top and the bottom face, a web between their floors.
#[test]
fn pockets_facing_each_other() {
    let outline = [[0.0, 0.0], [60.0, 0.0], [60.0, 40.0], [0.0, 40.0]];
    let rect = vec![[20.0, 10.0], [40.0, 10.0], [40.0, 30.0], [20.0, 30.0]];
    let at = |p: P2, z: f64| [p[0], p[1], z];
    let mut t = Vec::new();
    for [a, b, c] in synth::triangulate(&outline, std::slice::from_ref(&rect)) {
        t.push([at(a, 6.0), at(b, 6.0), at(c, 6.0)]);
        t.push([at(a, 0.0), at(c, 0.0), at(b, 0.0)]);
    }
    for i in 0..4 {
        let (a, b) = (outline[i], outline[(i + 1) % 4]);
        t.extend([[at(a, 0.0), at(b, 0.0), at(b, 6.0)], [at(a, 0.0), at(b, 6.0), at(a, 6.0)]]);
    }
    // The top pocket's floor at Z = 4 faces up, the bottom one's at Z = 2 down.
    let [a, b, c, d] = [rect[0], rect[1], rect[2], rect[3]];
    t.extend([[at(a, 4.0), at(b, 4.0), at(c, 4.0)], [at(a, 4.0), at(c, 4.0), at(d, 4.0)]]);
    t.extend([[at(a, 2.0), at(c, 2.0), at(b, 2.0)], [at(a, 2.0), at(d, 2.0), at(c, 2.0)]]);
    for i in 0..4 {
        let (p, q) = (rect[(i + 1) % 4], rect[i]);
        t.extend([[at(p, 4.0), at(q, 4.0), at(q, 6.0)], [at(p, 4.0), at(q, 6.0), at(p, 6.0)]]);
        t.extend([[at(p, 0.0), at(q, 0.0), at(q, 2.0)], [at(p, 0.0), at(q, 2.0), at(p, 2.0)]]);
    }
    let mesh = Mesh::new(t);
    assert!(mesh.volume() > 0.0);
    let (plate, _) = found(&mesh);
    let pocket = |depth: f64, bottom: bool| PocketSpec { x: 30.0, y: 20.0, w: 20.0, l: 20.0, depth, bottom };
    let mut pockets = plate.pockets.clone();
    pockets.sort_by_key(|p| p.bottom);
    assert_eq!(pockets, [pocket(2.0, false), pocket(2.0, true)]);
    assert!((plate.volume() - mesh.volume()).abs() < 1e-6);
}

/// A pocket's floor turned off level, and a triangle sunk into the top face, are no pockets.
#[test]
fn pockets_need_flat_floors_and_rectangles() {
    let mut mesh = crate::stl::parse(include_bytes!("../../../../plugin/meshwright/tests/pockets.stl")).expect("parses");
    for t in &mut mesh.triangles {
        for p in t.iter_mut() {
            if (p[2] - 3.0).abs() < 1e-4 && p[0] > 55.0 && p[1] < 22.0 {
                p[2] -= 0.4;
            }
        }
    }
    reason(&mesh);
    let outline = [[0.0, 0.0], [60.0, 0.0], [60.0, 40.0], [0.0, 40.0]];
    let triangle = vec![[20.0, 10.0], [40.0, 10.0], [30.0, 25.0]];
    let at = |p: P2, z: f64| [p[0], p[1], z];
    let mut t = Vec::new();
    for [a, b, c] in synth::triangulate(&outline, std::slice::from_ref(&triangle)) {
        t.push([at(a, 6.0), at(b, 6.0), at(c, 6.0)]);
    }
    for [a, b, c] in synth::triangulate(&outline, &[]) {
        t.push([at(a, 0.0), at(c, 0.0), at(b, 0.0)]);
    }
    for i in 0..4 {
        let (a, b) = (outline[i], outline[(i + 1) % 4]);
        t.extend([[at(a, 0.0), at(b, 0.0), at(b, 6.0)], [at(a, 0.0), at(b, 6.0), at(a, 6.0)]]);
    }
    let [a, b, c] = [triangle[0], triangle[1], triangle[2]];
    t.push([at(a, 3.0), at(b, 3.0), at(c, 3.0)]);
    for i in 0..3 {
        let (p, q) = (triangle[(i + 1) % 3], triangle[i]);
        t.extend([[at(p, 3.0), at(q, 3.0), at(q, 6.0)], [at(p, 3.0), at(q, 6.0), at(p, 6.0)]]);
    }
    let why = reason(&Mesh::new(t));
    assert!(why.contains("pockets other than rectangles"), "{why}");
}

/// A 70 x 40 x 6 plate exported from SolidCraft 0.45 with an Ø10 x 5 and an Ø8 x 3 boss on
/// its top face and an Ø5 hole.
#[test]
fn recognises_bosses() {
    let mesh = crate::stl::parse(include_bytes!("../../../../plugin/meshwright/tests/bosses.stl")).expect("parses");
    let (plate, _) = found(&mesh);
    let boss = |x: f64, y: f64, d: f64, h: f64| BossSpec { x, y, d, h };
    assert_eq!(plate.bosses, [boss(20.0, 20.0, 10.0, 5.0), boss(50.0, 25.0, 8.0, 3.0)]);
    assert_eq!((plate.z, plate.holes.len()), (6.0, 1));
    // SolidCraft measured 17,225.686 mm³ for the part it exported.
    assert!((plate.volume() - 17_225.686).abs() < 0.001, "{}", plate.volume());
}

/// A 60 x 40 x 6 plate with a solid standing on its top face over the loop `foot` (`n`
/// corners): `walls` gives each corner of the foot its place on the solid's top, at height
/// `h` above the plate.
fn standing(foot: &[P2], top: &dyn Fn(P2) -> P2, h: f64) -> Mesh {
    let outline = [[0.0, 0.0], [60.0, 0.0], [60.0, 40.0], [0.0, 40.0]];
    let at = |p: P2, z: f64| [p[0], p[1], z];
    let mut t = Vec::new();
    for [a, b, c] in synth::triangulate(&outline, &[foot.to_vec()]) {
        t.push([at(a, 6.0), at(b, 6.0), at(c, 6.0)]);
    }
    for [a, b, c] in synth::triangulate(&outline, &[]) {
        t.push([at(a, 0.0), at(c, 0.0), at(b, 0.0)]);
    }
    for i in 0..4 {
        let (a, b) = (outline[i], outline[(i + 1) % 4]);
        t.extend([[at(a, 0.0), at(b, 0.0), at(b, 6.0)], [at(a, 0.0), at(b, 6.0), at(a, 6.0)]]);
    }
    let roof: Vec<P2> = foot.iter().map(|p| top(*p)).collect();
    for [a, b, c] in synth::triangulate(&roof, &[]) {
        t.push([at(a, 6.0 + h), at(b, 6.0 + h), at(c, 6.0 + h)]);
    }
    let n = foot.len();
    for i in 0..n {
        let (a, b) = (foot[i], foot[(i + 1) % n]);
        let (c, d) = (roof[(i + 1) % n], roof[i]);
        t.extend([[at(a, 6.0), at(b, 6.0), at(c, 6.0 + h)], [at(a, 6.0), at(c, 6.0 + h), at(d, 6.0 + h)]]);
    }
    Mesh::new(t)
}

fn circle(centre: P2, r: f64, n: usize) -> Vec<P2> {
    (0..n).map(|i| 2.0 * PI * i as f64 / n as f64).map(|a| [centre[0] + r * a.cos(), centre[1] + r * a.sin()]).collect()
}

/// A straight round boss of a synthetic mesh; a drafted one, a twisted one and a block that
/// is no rectangle along X and Y are refused (a square block along them is a pad).
#[test]
fn bosses_must_be_straight_and_round() {
    let foot = circle([20.0, 20.0], 5.0, 32);
    let (plate, _) = found(&standing(&foot, &|p| p, 4.0));
    assert_eq!(plate.bosses, [BossSpec { x: 20.0, y: 20.0, d: 10.0, h: 4.0 }]);
    let draft = |p: P2| [20.0 + (p[0] - 20.0) * 0.9, 20.0 + (p[1] - 20.0) * 0.9];
    let (sin, cos) = (PI / 4.0).sin_cos();
    let twist = |p: P2| [20.0 + (p[0] - 20.0) * cos - (p[1] - 20.0) * sin, 20.0 + (p[0] - 20.0) * sin + (p[1] - 20.0) * cos];
    let block = [[15.0, 15.0], [25.0, 15.0], [25.0, 25.0], [15.0, 25.0]];
    let (plate, _) = found(&standing(&block, &|p| p, 4.0));
    assert_eq!(plate.pads, [PadSpec { x: 20.0, y: 20.0, w: 10.0, l: 10.0, h: 4.0 }]);
    let diamond = [[20.0, 14.0], [26.0, 20.0], [20.0, 26.0], [14.0, 20.0]];
    let pad_draft = |p: P2| [20.0 + (p[0] - 20.0) * 0.9, 20.0 + (p[1] - 20.0) * 0.9];
    for mesh in
        [standing(&foot, &draft, 4.0), standing(&foot, &twist, 4.0), standing(&diamond, &|p| p, 4.0), standing(&block, &pad_draft, 4.0)]
    {
        reason(&mesh);
    }
}

/// A boss whose top is larger than what is left of the plate's top face.
#[test]
fn bosses_larger_than_the_plate_face() {
    let outline = [[0.0, 0.0], [40.0, 0.0], [40.0, 40.0], [0.0, 40.0]];
    let foot = circle([20.0, 20.0], 17.0, 48);
    let mut mesh = standing(&foot, &|p| p, 4.0);
    // `standing` makes a 60 x 40 plate: cut it to 40 x 40 by rebuilding with that outline.
    let at = |p: P2, z: f64| [p[0], p[1], z];
    let mut t: Vec<crate::mesh::Triangle> = mesh.triangles.iter().filter(|t| t.iter().any(|p| p[2] > 6.0 + 1e-9)).copied().collect();
    for [a, b, c] in synth::triangulate(&outline, std::slice::from_ref(&foot)) {
        t.push([at(a, 6.0), at(b, 6.0), at(c, 6.0)]);
    }
    for [a, b, c] in synth::triangulate(&outline, &[]) {
        t.push([at(a, 0.0), at(c, 0.0), at(b, 0.0)]);
    }
    for i in 0..4 {
        let (a, b) = (outline[i], outline[(i + 1) % 4]);
        t.extend([[at(a, 0.0), at(b, 0.0), at(b, 6.0)], [at(a, 0.0), at(b, 6.0), at(a, 6.0)]]);
    }
    mesh = Mesh::new(t);
    let (plate, _) = found(&mesh);
    assert_eq!(plate.bosses, [BossSpec { x: 20.0, y: 20.0, d: 34.0, h: 4.0 }]);
}

#[test]
fn split_boss_footprints() {
    let foot = circle([20.0, 20.0], 5.0, 32);
    let n = foot.len();
    let split: Vec<P2> = (0..n)
        .flat_map(|i| [foot[i], [f64::midpoint(foot[i][0], foot[(i + 1) % n][0]), f64::midpoint(foot[i][1], foot[(i + 1) % n][1])]])
        .collect();
    let (plate, _) = found(&standing(&split, &|p| p, 4.0));
    assert_eq!(plate.bosses, [BossSpec { x: 20.0, y: 20.0, d: 10.0, h: 4.0 }]);
}

/// An 80 x 50 x 8 plate exported from SolidCraft 0.45 with three slots (along X, along Y and
/// at 30°) and a hole.
#[test]
fn recognises_slots() {
    let mesh = crate::stl::parse(include_bytes!("../../../../plugin/meshwright/tests/slots.stl")).expect("parses");
    let (plate, _) = found(&mesh);
    let slot = |x: f64, y: f64, len: f64, w: f64, angle: f64| SlotSpec { x, y, len, w, angle };
    assert_eq!(plate.slots, [slot(25.0, 15.0, 20.0, 6.0, 0.0), slot(62.0, 30.0, 15.0, 8.0, 90.0), slot(30.0, 36.0, 16.0, 5.0, 30.0)]);
    assert_eq!(plate.holes.len(), 1);
    // SolidCraft measured 28,497.522 mm³ for the part it exported.
    assert!((plate.volume() - 28_497.522).abs() < 0.001, "{}", plate.volume());
}

/// A slot's loop: `n` facets on each round end, counter-clockwise.
fn stadium(centre: P2, len: f64, w: f64, angle: f64, n: usize) -> Vec<P2> {
    let (sin, cos) = angle.to_radians().sin_cos();
    let mut ring = Vec::new();
    for (end, start) in [(len / 2.0, -PI / 2.0), (-len / 2.0, PI / 2.0)] {
        for i in 0..=n {
            let a = start + PI * i as f64 / n as f64;
            let (s, t) = (end + w / 2.0 * a.cos(), w / 2.0 * a.sin());
            ring.push([centre[0] + s * cos - t * sin, centre[1] + s * sin + t * cos]);
        }
    }
    ring
}

fn plate_with_cutout(cutout: Vec<P2>) -> Mesh {
    synth::prism(&[[0.0, 0.0], [80.0, 0.0], [80.0, 50.0], [0.0, 50.0]], &[cutout], 6.0, [0.0; 3])
}

/// Slots of a synthetic mesh at any angle, finely or coarsely faceted.
#[test]
fn synthetic_slots() {
    for (angle, n) in [(0.0, 16), (0.0, 15), (90.0, 8), (90.0, 9), (30.0, 12), (30.0, 13), (137.5, 24), (137.5, 7)] {
        let (plate, _) = found(&plate_with_cutout(stadium([40.0, 25.0], 18.0, 7.0, angle, n)));
        assert_eq!(plate.slots, [SlotSpec { x: 40.0, y: 25.0, len: 18.0, w: 7.0, angle }], "{angle} {n}");
    }
}

/// Cut-outs that look like slots but are not: rounded ends narrower than the slot, ends of
/// three facets, an ellipse, and a slot whose ends differ in width.
#[test]
fn near_slots_are_refused() {
    let rounded = synth::rounded(&[[30.0, 22.0], [50.0, 22.0], [50.0, 28.0], [30.0, 28.0]], Some(1.5), 32);
    let ellipse: Vec<P2> = (0..48).map(|i| 2.0 * PI * f64::from(i) / 48.0).map(|a| [40.0 + 12.0 * a.cos(), 25.0 + 4.0 * a.sin()]).collect();
    let mut uneven = stadium([40.0, 25.0], 18.0, 7.0, 0.0, 16);
    for p in &mut uneven[..17] {
        p[1] = 25.0 + (p[1] - 25.0) * 1.2;
    }
    // Codex's: long facets from each side across to a short tip of the end.
    let full = stadium([40.0, 25.0], 18.0, 7.0, 0.0, 16);
    let mut cut: Vec<P2> = Vec::new();
    for (end, sign) in [(&full[..17], -1.0), (&full[17..], 1.0)] {
        for k in [0, 6, 7, 8, 9, 10, 16] {
            let mut p = end[k];
            if k == 0 || k == 16 {
                p[0] += sign * 4.0;
            }
            cut.push(p);
        }
    }
    // (A slot with ends of three facets is an octagonal cut-out, see `polygonal_cut_outs`.)
    for cutout in [rounded, ellipse, uneven, cut] {
        let why = reason(&plate_with_cutout(cutout));
        assert!(why.contains("not round") || why.contains("corners"), "{why}");
    }
}

/// Ends faceted more finely below the axis than above it: the middle comes from the straight
/// sides, not the loop's centroid.
#[test]
fn unevenly_faceted_slot_ends() {
    let full = stadium([40.0, 25.0], 18.0, 7.0, 0.0, 16);
    // Every other corner above the axis left out, the straight sides' ends kept.
    let uneven: Vec<P2> =
        full.iter().enumerate().filter(|(i, p)| p[1] <= 25.0 || p[1] >= 28.5 - 1e-9 || i % 2 == 0).map(|(_, p)| *p).collect();
    assert!(uneven.len() < full.len() - 6);
    let (plate, _) = found(&plate_with_cutout(uneven));
    assert_eq!(plate.slots, [SlotSpec { x: 40.0, y: 25.0, len: 18.0, w: 7.0, angle: 0.0 }]);
}

/// A slot's wall meshed in bands with one corner of a band folded into the slot is no wall:
/// the inner half of an end's circle lies inside the slot.
#[test]
#[allow(clippy::float_cmp)] // corners compared as the generator wrote them
fn a_slot_wall_folded_inward_is_refused() {
    let mesh = plate_with_cutout(stadium([40.0, 25.0], 18.0, 7.0, 0.0, 16));
    // Split every wall facet at Z = 3, then reflect the band corner of the right end's
    // second rim corner across X = 49 (the end's centre).
    let ring = stadium([40.0, 25.0], 18.0, 7.0, 0.0, 16);
    let target = ring[1];
    let banded = banded(&mesh, 0.0, [40.0, 25.0]);
    let mut moved = 0;
    let triangles = banded
        .triangles
        .iter()
        .map(|t| {
            t.map(|p| {
                if p[2] == 3.0 && (p[0] - target[0]).abs() < 1e-6 && (p[1] - target[1]).abs() < 1e-6 {
                    moved += 1;
                    [2.0 * 49.0 - p[0], p[1], p[2]]
                } else {
                    p
                }
            })
        })
        .collect();
    assert!(moved > 0);
    reason(&Mesh::new(triangles));
}

/// Top and bottom rims meshed apart, with eight facets per end on top and seven below: a
/// wall facet's edge across an end's tip still follows the end.
#[test]
fn slot_rims_meshed_apart() {
    let outline = [[0.0, 0.0], [80.0, 0.0], [80.0, 50.0], [0.0, 50.0]];
    let (top, bottom) = (stadium([40.0, 25.0], 18.0, 7.0, 0.0, 8), stadium([40.0, 25.0], 18.0, 7.0, 0.0, 7));
    let mut t: Vec<crate::mesh::Triangle> = Vec::new();
    let at = |p: P2, h: f64| [p[0], p[1], h];
    for [a, b, c] in synth::triangulate(&outline, std::slice::from_ref(&top)) {
        t.push([at(a, 6.0), at(b, 6.0), at(c, 6.0)]);
    }
    for [a, b, c] in synth::triangulate(&outline, std::slice::from_ref(&bottom)) {
        t.push([at(a, 0.0), at(c, 0.0), at(b, 0.0)]);
    }
    for i in 0..4 {
        let (a, b) = (outline[i], outline[(i + 1) % 4]);
        t.extend([[at(a, 0.0), at(b, 0.0), at(b, 6.0)], [at(a, 0.0), at(b, 6.0), at(a, 6.0)]]);
    }
    // The slot's wall: walk both rims clockwise (as the top face's boundary runs), ordered
    // by their angle about the middle, joining them by triangles as they advance.
    let clockwise = |ring: &[P2]| -> Vec<P2> {
        let turn = |p: &P2| (-(p[1] - 25.0).atan2(p[0] - 40.0)).rem_euclid(2.0 * PI);
        let mut out: Vec<P2> = ring.to_vec();
        out.sort_by(|a, b| turn(a).total_cmp(&turn(b)));
        out
    };
    let (lower, upper) = (clockwise(&bottom), clockwise(&top));
    let turn = |p: P2| (-(p[1] - 25.0).atan2(p[0] - 40.0)).rem_euclid(2.0 * PI);
    let (nl, nu) = (lower.len(), upper.len());
    let (mut i, mut j) = (0, 0);
    while i < nl || j < nu {
        let (l0, l1) = (lower[i % nl], lower[(i + 1) % nl]);
        let (u0, u1) = (upper[j % nu], upper[(j + 1) % nu]);
        let next_low = if i + 1 >= nl { 2.0 * PI + turn(lower[0]) } else { turn(l1) };
        let next_up = if j + 1 >= nu { 2.0 * PI + turn(upper[0]) } else { turn(u1) };
        if j >= nu || (i < nl && next_low <= next_up) {
            t.push([at(l0, 0.0), at(l1, 0.0), at(u0, 6.0)]);
            i += 1;
        } else {
            t.push([at(l0, 0.0), at(u1, 6.0), at(u0, 6.0)]);
            j += 1;
        }
    }
    let mesh = Mesh::new(t);
    assert!(mesh.volume() > 0.0);
    let (plate, _) = found(&mesh);
    assert_eq!(plate.slots, [SlotSpec { x: 40.0, y: 25.0, len: 18.0, w: 7.0, angle: 0.0 }]);
}

/// Every facet of a slot's rim split in the middle: the new corners lie on the facets'
/// chords, not the circles, and the slot is the same.
#[test]
fn split_slot_facets() {
    let ring = stadium([40.0, 25.0], 18.0, 7.0, 0.0, 16);
    let n = ring.len();
    let split: Vec<P2> = (0..n)
        .flat_map(|i| [ring[i], [f64::midpoint(ring[i][0], ring[(i + 1) % n][0]), f64::midpoint(ring[i][1], ring[(i + 1) % n][1])]])
        .collect();
    let (plate, _) = found(&plate_with_cutout(split));
    assert_eq!(plate.slots, [SlotSpec { x: 40.0, y: 25.0, len: 18.0, w: 7.0, angle: 0.0 }]);
}

/// Cut-outs of straight sides: a rectangle, a triangle, an L-shape, and a slot whose ends
/// are three facets each. A curve of many gentle corners is no polygon.
#[test]
fn polygonal_cut_outs() {
    let rectangle = vec![[10.0, 10.0], [25.0, 10.0], [25.0, 30.0], [10.0, 30.0]];
    let triangle = vec![[40.0, 10.0], [60.0, 12.0], [48.0, 30.0]];
    let ell = vec![[30.0, 35.0], [50.0, 35.0], [50.0, 40.0], [35.0, 40.0], [35.0, 45.0], [30.0, 45.0]];
    let outline = [[0.0, 0.0], [80.0, 0.0], [80.0, 50.0], [0.0, 50.0]];
    let mesh = synth::prism(&outline, &[rectangle.clone(), triangle.clone(), ell.clone()], 6.0, [0.0; 3]);
    let (plate, _) = found_profile(&mesh);
    assert_eq!(plate.cutouts, [rectangle, triangle, ell]);
    assert!((plate.volume() - mesh.volume()).abs() < 1e-6);
    let (plate, _) = found_profile(&plate_with_cutout(stadium([40.0, 25.0], 18.0, 7.0, 0.0, 3)));
    assert_eq!(plate.cutouts[0].len(), 8);
    let gentle: Vec<P2> = (0..30).map(|i| 2.0 * PI * f64::from(i) / 30.0).map(|a| [40.0 + 12.0 * a.cos(), 25.0 + 6.0 * a.sin()]).collect();
    reason(&plate_with_cutout(gentle));
}

/// A slot through the top face that does not reach the bottom face is no through slot.
#[test]
fn a_slot_must_go_through() {
    let mut mesh = plate_with_cutout(stadium([40.0, 25.0], 18.0, 7.0, 0.0, 16));
    // Widen the slot's bottom loop by a tenth: still a slot, but the walls lean.
    for t in &mut mesh.triangles {
        for p in t.iter_mut() {
            if p[2] == 0.0 && (p[1] - 25.0).abs() < 3.6 && (p[0] - 40.0).abs() < 12.6 {
                p[0] = 40.0 + (p[0] - 40.0) * 1.1;
                p[1] = 25.0 + (p[1] - 25.0) * 1.1;
            }
        }
    }
    let why = reason(&mesh);
    assert!(why.contains("slot does not go straight through") || why.contains("faces differ"), "{why}");
}

/// A 60 x 40 x 8 plate exported from SolidCraft 0.45 with blind holes from the top (Ø6 x 4,
/// Ø3 x 6) and the bottom (Ø8 x 2.5) and an Ø5 hole through.
#[test]
fn recognises_blind_holes() {
    let mesh = crate::stl::parse(include_bytes!("../../../../plugin/meshwright/tests/blind-holes.stl")).expect("parses");
    let (plate, _) = found(&mesh);
    let blind = |depth: f64, bottom: bool| Some(Blind { depth, bottom });
    let holes: Vec<(f64, f64, f64, Option<Blind>)> = plate.holes.iter().map(|h| (h.x, h.y, h.d, h.blind)).collect();
    assert_eq!(
        holes,
        [
            (15.0, 20.0, 6.0, blind(4.0, false)),
            (30.0, 20.0, 5.0, None),
            (45.0, 20.0, 8.0, blind(2.5, true)),
            (45.0, 32.0, 3.0, blind(6.0, false))
        ]
    );
    // SolidCraft measured 18,761.748 mm³ for the part it exported.
    assert!((plate.volume() - 18_761.748).abs() < 0.001, "{}", plate.volume());
}

/// A blind hole's floor turned 0.5 mm off level is no floor.
#[test]
fn a_tilted_blind_floor_is_refused() {
    let mut mesh = crate::stl::parse(include_bytes!("../../../../plugin/meshwright/tests/blind-holes.stl")).expect("parses");
    // Lower the floor corners of the Ø6 hole (at 15, 20; floor 4 mm down) on one side.
    for t in &mut mesh.triangles {
        for p in t.iter_mut() {
            if (p[2] - 4.0).abs() < 1e-4 && p[0] > 15.0 && (p[0] - 15.0).hypot(p[1] - 20.0) < 3.1 {
                p[2] -= 0.5;
            }
        }
    }
    reason(&mesh);
}

/// A closed 60 x 40 x 8 plate with an Ø6 blind hole of `n` corners at (15, 20) from the top,
/// its wall `depth` deep and its end a flat floor or, with `tip`, a cone that far deeper.
fn blind_plate(depth: f64, tip: f64, n: usize) -> Mesh {
    let ring = synth::rings(&[HoleSpec { x: 15.0, y: 20.0, d: 6.0, head: None, blind: None }], n).remove(0);
    let (z, floor) = (8.0, 8.0 - depth);
    let outline = [[0.0, 0.0], [60.0, 0.0], [60.0, 40.0], [0.0, 40.0]];
    let at = |p: P2, h: f64| [p[0], p[1], h];
    let mut triangles = Vec::new();
    for [a, b, c] in synth::triangulate(&outline, std::slice::from_ref(&ring)) {
        triangles.push([at(a, z), at(b, z), at(c, z)]);
    }
    for [a, b, c] in synth::triangulate(&outline, &[]) {
        triangles.push([at(a, 0.0), at(c, 0.0), at(b, 0.0)]);
    }
    if tip > 0.0 {
        let apex = [15.0, 20.0, floor - tip];
        for i in 0..n {
            triangles.push([at(ring[i], floor), at(ring[(i + 1) % n], floor), apex]);
        }
    } else {
        for [a, b, c] in synth::triangulate(&ring, &[]) {
            triangles.push([at(a, floor), at(b, floor), at(c, floor)]);
        }
    }
    let mut wall = |ring: &[P2], lo: f64, hi: f64| {
        let m = ring.len();
        for i in 0..m {
            let (a, b) = (ring[i], ring[(i + 1) % m]);
            triangles.extend([[at(a, lo), at(b, lo), at(b, hi)], [at(a, lo), at(b, hi), at(a, hi)]]);
        }
    };
    wall(&outline, 0.0, z);
    wall(&ring.iter().rev().copied().collect::<Vec<P2>>(), floor, z);
    Mesh::new(triangles)
}

/// A blind hole of a synthetic mesh, flat-bottomed; one with a drill point is refused.
#[test]
fn blind_holes_need_flat_floors() {
    let (plate, _) = found(&blind_plate(4.0, 0.0, 32));
    assert_eq!(plate.holes, [HoleSpec { x: 15.0, y: 20.0, d: 6.0, head: None, blind: Some(Blind { depth: 4.0, bottom: false }) }]);
    let why = reason(&blind_plate(4.0, 1.8, 32));
    assert!(why.contains("not a flat plate") || why.contains("flat floor"), "{why}");
}

/// A 60 x 40 x 8 plate exported from SolidCraft 0.45 with a counterbore and a countersink on
/// the top face and a counterbore on the bottom face.
#[test]
fn recognises_counterbores_and_countersinks() {
    let mesh = crate::stl::parse(include_bytes!("../../../../plugin/meshwright/tests/stepped-holes.stl")).expect("parses");
    let (plate, _) = found(&mesh);
    let heads: Vec<(f64, f64, f64, Option<HoleHead>)> = plate.holes.iter().map(|h| (h.x, h.y, h.d, h.head)).collect();
    assert_eq!(
        heads,
        [
            (15.0, 20.0, 5.5, Some(HoleHead::Counterbore { d: 10.0, depth: 3.0, bottom: false })),
            (30.0, 32.0, 4.0, Some(HoleHead::Counterbore { d: 8.0, depth: 2.5, bottom: true })),
            (45.0, 20.0, 6.0, Some(HoleHead::Countersink { d: 12.0, angle: 90.0, bottom: false })),
        ]
    );
    // SolidCraft measured 18,311.518 mm³ for the part it exported.
    assert!((plate.volume() - 18_311.518).abs() < 0.001, "{}", plate.volume());
}

/// A counterbore's floor turned 0.5 mm off level is no floor.
#[test]
fn a_tilted_counterbore_floor_is_refused() {
    let mut mesh = crate::stl::parse(include_bytes!("../../../../plugin/meshwright/tests/stepped-holes.stl")).expect("parses");
    // Lift the floor corners of the top counterbore (at 15, 20; floor 3 mm down) on one side.
    for t in &mut mesh.triangles {
        for p in t.iter_mut() {
            if (p[2] - 5.0).abs() < 1e-4 && p[0] > 15.0 && (p[0] - 15.0).hypot(p[1] - 20.0) < 5.1 {
                p[2] += 0.5;
            }
        }
    }
    reason(&mesh);
}

/// A 1 mm web left in the counterbored bore (between Z = 2 and Z = 3, two blind holes meeting
/// it from both faces) is no through hole, although every corner of its two disks lies on the
/// bore's circle. Its volume is below the 1 % the volume check would notice.
#[test]
fn a_web_across_a_counterbored_bore_is_refused() {
    let mut mesh = crate::stl::parse(include_bytes!("../../../../plugin/meshwright/tests/stepped-holes.stl")).expect("parses");
    let centre = [15.0, 20.0];
    let mut rim: Vec<P2> = Vec::new();
    for p in mesh.triangles.iter().flatten() {
        let q = [p[0], p[1]];
        if (distance(q, centre) - 2.75).abs() < 1e-3 && !rim.iter().any(|r| distance(*r, q) < 1e-6) {
            rim.push(q);
        }
    }
    rim.sort_by(|a, b| (a[1] - centre[1]).atan2(a[0] - centre[0]).total_cmp(&(b[1] - centre[1]).atan2(b[0] - centre[0])));
    assert!(rim.len() > 8, "{}", rim.len());
    for i in 1..rim.len() - 1 {
        let (a, b, c) = (rim[0], rim[i], rim[i + 1]);
        mesh.triangles.push([[a[0], a[1], 3.0], [b[0], b[1], 3.0], [c[0], c[1], 3.0]]);
        mesh.triangles.push([[a[0], a[1], 2.0], [c[0], c[1], 2.0], [b[0], b[1], 2.0]]);
    }
    reason(&mesh);
}

/// A closed 60 x 60 x 8 plate with one hole at (30, 30): an Ø`bore` bore and an
/// Ø`head` x `depth` counterbore from the top, each a polygon of `n` corners. With `split`,
/// the first facet pair of the bore's and of the counterbore's wall is split where the two
/// meet, as an exporter refining a mesh would: the new corner lies on the chord. `twist` turns
/// the bore's upper rim (at the floor) by that angle about the hole's axis.
fn counterbored(bore: f64, head: f64, depth: f64, n: usize, split: bool, twist: f64) -> Mesh {
    let ring = |d: f64| synth::rings(&[HoleSpec { x: 30.0, y: 30.0, d, head: None, blind: None }], n).remove(0);
    let (bore_ring, head_ring) = (ring(bore), ring(head));
    let (sin, cos) = twist.sin_cos();
    let turned: Vec<P2> = bore_ring
        .iter()
        .map(|p| [30.0 + (p[0] - 30.0) * cos - (p[1] - 30.0) * sin, 30.0 + (p[0] - 30.0) * sin + (p[1] - 30.0) * cos])
        .collect();
    let (z, floor) = (8.0, 8.0 - depth);
    let outline = [[0.0, 0.0], [60.0, 0.0], [60.0, 60.0], [0.0, 60.0]];
    let at = |p: P2, h: f64| [p[0], p[1], h];
    let mut triangles = Vec::new();
    for [a, b, c] in synth::triangulate(&outline, std::slice::from_ref(&head_ring)) {
        triangles.push([at(a, z), at(b, z), at(c, z)]);
    }
    for [a, b, c] in synth::triangulate(&outline, std::slice::from_ref(&bore_ring)) {
        triangles.push([at(a, 0.0), at(c, 0.0), at(b, 0.0)]);
    }
    for [a, b, c] in synth::triangulate(&head_ring, std::slice::from_ref(&turned)) {
        triangles.push([at(a, floor), at(b, floor), at(c, floor)]);
    }
    // A wall from `lower` up to `upper`, two loops of corners that match one for one.
    let mut wall = |lower: &[P2], upper: &[P2], lo: f64, hi: f64, split: bool| {
        let n = lower.len();
        for i in 0..n {
            let (a, b, c, d) = (at(lower[i], lo), at(lower[(i + 1) % n], lo), at(upper[(i + 1) % n], hi), at(upper[i], hi));
            if split && i == 0 {
                let m = [f64::midpoint(a[0], c[0]), f64::midpoint(a[1], c[1]), f64::midpoint(lo, hi)];
                triangles.extend([[a, b, m], [m, b, c], [a, m, d], [m, c, d]]);
            } else {
                triangles.extend([[a, b, c], [a, c, d]]);
            }
        }
    };
    wall(&outline, &outline, 0.0, z, false);
    let reversed = |r: &[P2]| r.iter().rev().copied().collect::<Vec<P2>>();
    wall(&reversed(&bore_ring), &reversed(&turned), 0.0, floor, split);
    wall(&reversed(&head_ring), &reversed(&head_ring), floor, z, split);
    Mesh::new(triangles)
}

fn counterbore_of(mesh: &Mesh) -> Option<HoleHead> {
    let (plate, _) = found(mesh);
    assert_eq!(plate.holes.len(), 1);
    plate.holes[0].head
}

/// The floor of a counterbore only 0.1 mm wider than its 16-cornered bore reaches inside the
/// bore's circle, between the corners of its loop.
#[test]
fn narrow_counterbores_over_coarse_bores() {
    assert_eq!(
        counterbore_of(&counterbored(10.0, 10.2, 3.0, 16, false, 0.0)),
        Some(HoleHead::Counterbore { d: 10.2, depth: 3.0, bottom: false })
    );
}

/// Corners an exporter adds within the facets of a counterbored hole's walls lie on the walls'
/// chords, not on their circles, as with a plain hole.
#[test]
fn subdivided_walls_of_counterbored_holes() {
    let expected = Some(HoleHead::Counterbore { d: 11.0, depth: 3.0, bottom: false });
    assert_eq!(counterbore_of(&counterbored(10.0, 11.0, 3.0, 16, false, 0.0)), expected);
    let split = counterbored(10.0, 11.0, 3.0, 16, true, 0.0);
    assert!((split.volume() - counterbored(10.0, 11.0, 3.0, 16, false, 0.0).volume()).abs() < 1e-9);
    assert_eq!(counterbore_of(&split), expected);
}

/// A counterbored bore whose upper rim is turned 60° against its lower one before the wall
/// joins them narrows in the middle: its facets cut across the bore, although every corner
/// lies on the bore's circle. Turned half a step it is still a bore, as exporters mesh it.
#[test]
fn a_twisted_counterbored_bore_is_refused() {
    let why = reason(&counterbored(5.5, 10.0, 3.0, 32, false, 60_f64.to_radians()));
    assert!(why.contains("not vertical"), "{why}");
    let half_step = Some(HoleHead::Counterbore { d: 10.0, depth: 3.0, bottom: false });
    assert_eq!(counterbore_of(&counterbored(5.5, 10.0, 3.0, 32, false, PI / 32.0)), half_step);
}

/// A closed 60 x 60 x 8 plate with an Ø5.5 bore at (30, 30) under an Ø10 90° countersink, each
/// loop of `n` corners, the bore (both its rims) turned by `twist` against the mouth.
fn countersunk(n: usize, twist: f64) -> Mesh {
    let ring = |d: f64, turn: f64| -> Vec<P2> {
        (0..n).map(|i| 2.0 * PI * i as f64 / n as f64 + turn).map(|a| [30.0 + d / 2.0 * a.cos(), 30.0 + d / 2.0 * a.sin()]).collect()
    };
    let (bore, mouth) = (ring(5.5, twist), ring(10.0, 0.0));
    let (z, cone) = (8.0, 8.0 - 2.25);
    let outline = [[0.0, 0.0], [60.0, 0.0], [60.0, 60.0], [0.0, 60.0]];
    let at = |p: P2, h: f64| [p[0], p[1], h];
    let mut triangles = Vec::new();
    for [a, b, c] in synth::triangulate(&outline, std::slice::from_ref(&mouth)) {
        triangles.push([at(a, z), at(b, z), at(c, z)]);
    }
    for [a, b, c] in synth::triangulate(&outline, std::slice::from_ref(&bore)) {
        triangles.push([at(a, 0.0), at(c, 0.0), at(b, 0.0)]);
    }
    let mut wall = |lower: &[P2], upper: &[P2], lo: f64, hi: f64| {
        let m = lower.len();
        for i in 0..m {
            let (a, b, c, d) = (at(lower[i], lo), at(lower[(i + 1) % m], lo), at(upper[(i + 1) % m], hi), at(upper[i], hi));
            triangles.extend([[a, b, c], [a, c, d]]);
        }
    };
    wall(&outline, &outline, 0.0, z);
    let reversed = |r: &[P2]| r.iter().rev().copied().collect::<Vec<P2>>();
    wall(&reversed(&bore), &reversed(&bore), 0.0, cone);
    wall(&reversed(&bore), &reversed(&mouth), cone, z);
    Mesh::new(triangles)
}

/// A countersink's cone joining a mouth and a bore turned a quarter turn apart narrows the
/// hole: its facets cut inside the cone, although their corners lie on it.
#[test]
fn a_twisted_countersink_is_refused() {
    let head = Some(HoleHead::Countersink { d: 10.0, angle: 90.0, bottom: false });
    assert_eq!(counterbore_of(&countersunk(32, 0.0)), head);
    assert_eq!(counterbore_of(&countersunk(32, PI / 32.0)), head);
    reason(&countersunk(32, PI / 2.0));
}

/// A facet of a countersink's cone split into three at its centroid leaves a corner on the
/// facet, inside the cone.
#[test]
fn split_countersink_facets() {
    let mut mesh = crate::stl::parse(include_bytes!("../../../../plugin/meshwright/tests/stepped-holes.stl")).expect("parses");
    let on_cone = |t: &[Vec3; 3]| {
        t.iter().all(|p| (2.9..6.1).contains(&(p[0] - 45.0).hypot(p[1] - 20.0)) && p[2] > 4.9) && t.iter().any(|p| (p[2] - 8.0).abs() > 0.1)
    };
    let k = mesh.triangles.iter().position(|t| on_cone(t) && t.iter().any(|p| (p[2] - 8.0).abs() < 1e-4)).expect("a cone facet");
    let [a, b, c] = mesh.triangles.swap_remove(k);
    let m = [(a[0] + b[0] + c[0]) / 3.0, (a[1] + b[1] + c[1]) / 3.0, (a[2] + b[2] + c[2]) / 3.0];
    mesh.triangles.extend([[a, b, m], [b, c, m], [c, a, m]]);
    assert_eq!(found(&mesh).0.holes.len(), 3);
}

/// A rim edge of a hole's bottom loop split in the middle (both facets on it) leaves a corner
/// on the edge, inside the circle: the rim stays round.
#[test]
#[allow(clippy::float_cmp)] // the edge corners are the generator output, bit for bit
fn split_hole_rims_stay_round() {
    let expected = PlateSpec {
        x: 60.0,
        y: 60.0,
        z: 8.0,
        holes: vec![HoleSpec { x: 30.0, y: 30.0, d: 10.0, head: None, blind: None }],
        cutouts: vec![],
        pads: vec![],
        pockets: vec![],
        bosses: vec![],
        slots: vec![],
        outline_fillet_r: None,
    };
    let mut mesh = synth::plate(&expected, 16, [0.0; 3]);
    let ring = synth::rings(&expected.holes, 16).remove(0);
    let (p, q) = ([ring[0][0], ring[0][1], 0.0], [ring[1][0], ring[1][1], 0.0]);
    let m = [f64::midpoint(p[0], q[0]), f64::midpoint(p[1], q[1]), 0.0];
    let mut split = 0;
    mesh.triangles = mesh
        .triangles
        .iter()
        .flat_map(|t| {
            // The facet's corners from the edge's start round to the edge's end, if it has it.
            (0..3).find(|&k| (t[k] == p && t[(k + 1) % 3] == q) || (t[k] == q && t[(k + 1) % 3] == p)).map_or_else(
                || vec![*t],
                |k| {
                    split += 1;
                    let (a, b, c) = (t[k], t[(k + 1) % 3], t[(k + 2) % 3]);
                    vec![[a, m, c], [m, b, c]]
                },
            )
        })
        .collect();
    assert_eq!(split, 2);
    assert_eq!(found(&mesh).0, expected);
}

/// An L-bracket exported from SolidCraft 0.45 (`file.export_stl`, standard quality): walls in
/// bands, top and bottom faces meshed apart, arcs and rims of uneven facets.
#[test]
fn recognises_solidcraft_exports() {
    let mesh = crate::stl::parse(include_bytes!("../../../../plugin/meshwright/tests/solidcraft-l-bracket.stl")).expect("parses");
    let Recognition::Profile { profile, offset } = recognize(&mesh) else { panic!("not recognised: {:?}", recognize(&mesh)) };
    assert_eq!(offset, [20.0, 10.0, 0.0]);
    assert_eq!(profile.corners, [[0.0, 0.0], [60.0, 0.0], [60.0, 15.0], [15.0, 15.0], [15.0, 40.0], [0.0, 40.0]]);
    assert_eq!(
        profile.holes,
        [HoleSpec { x: 7.5, y: 30.0, d: 5.0, head: None, blind: None }, HoleSpec { x: 45.0, y: 7.5, d: 6.0, head: None, blind: None }]
    );
    assert_eq!((profile.z, profile.outline_fillet_r), (5.0, Some(2.0)));
}

#[test]
fn standing_plate_is_not_recognised() {
    let mesh = synth::plate(&spec(&[(40.0, 25.0, 10.0)], None), 32, [0.0; 3]);
    let turned = Mesh::new(mesh.triangles.iter().map(|t| t.map(|p| [p[0], p[2], -p[1]])).collect());
    // Seen along Z it is a strip with a hole through its side: no prism over its top face.
    let why = reason(&turned);
    assert!(why.contains("not a flat plate") || why.contains("faces differ"), "{why}");
}

#[test]
fn mesh_with_a_gap_is_not_recognised() {
    let mut mesh = synth::plate(&spec(&[(40.0, 25.0, 10.0)], Some(3.0)), 32, [0.0; 3]);
    mesh.triangles.remove(0);
    reason(&mesh);
}

#[test]
fn synthetic_volumes_match_the_specs() {
    let s = spec(&[(40.0, 25.0, 10.0)], Some(5.0));
    let mesh = synth::plate(&s, 128, [0.0; 3]);
    assert!((mesh.volume() - s.volume()).abs() / s.volume() < 1e-3);
    let l = ell(Some(2.0));
    let mesh = synth::profile(&l, 128, [0.0; 3]);
    assert!((mesh.volume() - l.volume()).abs() / l.volume() < 1e-3);
}

#[test]
#[allow(clippy::float_cmp)] // the generator puts every top corner at exactly the same height
fn synthetic_top_faces_are_not_folded() {
    let plate = synth::plate(&spec(&[(10.0, 10.0, 5.0), (40.0, 25.0, 12.0), (70.0, 38.5, 6.6)], Some(4.0)), 32, [0.0; 3]);
    let profile = synth::profile(&ell(Some(2.0)), 32, [0.0; 3]);
    for mesh in [plate, profile] {
        for [a, b, c] in &mesh.triangles {
            let n = cross(sub(*b, *a), sub(*c, *a));
            if a[2] == b[2] && b[2] == c[2] {
                assert!(n[2].abs() > 1e-9, "degenerate cap triangle");
                assert_eq!(n[2] > 0.0, a[2] > 0.0, "cap triangle facing the wrong way");
            }
        }
    }
}
