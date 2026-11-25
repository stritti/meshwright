use crate::mesh::{Mesh, cross, sub};
use crate::plate::P2;
use crate::plate::geometry::distance;
use crate::plate::walls::Walls;
use crate::recognize::{Axis, HoleSpec, PlateSpec, Recognition, recognize};
use crate::synth;

use super::{banded, found, reason, spec};

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
        edge_fillet_r: None,
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

/// Shearing keeps the faces' areas and the volume of a prism, so only the walls tell. A hole
/// slanted by the shear goes through no face straight, and stays a mesh. A block sheared in X
/// over Z is still a straight prism along Y (its XZ cross-section does not change), so it is
/// recognised along Y with the parallelogram as its outline.
#[test]
#[allow(clippy::float_cmp)] // corners compared as the generator wrote them
fn sheared_walls_are_not_vertical() {
    let outline = [[0.0, 0.0], [80.0, 0.0], [80.0, 50.0], [0.0, 50.0]];
    let ring = synth::rings(&[HoleSpec { x: 40.0, y: 25.0, d: 8.0, head: None, blind: None }], 32);
    let leaning_hole = sheared(&outline, &ring, 5.0, [0.0; 2], [0.5, 0.0]);
    let why = reason(&leaning_hole);
    assert!(why.contains("not vertical") || why.contains("straight through"), "{why}");
    let leaning_block = sheared(&outline, &[], 5.0, [0.5, 0.0], [0.0; 2]);
    let Recognition::Profile { profile, offset, axis } = recognize(&leaning_block) else {
        panic!("not recognised: {:?}", recognize(&leaning_block))
    };
    assert_eq!(axis, Axis::Y);
    assert_eq!(profile.corners, [[0.0, 0.0], [80.0, 0.0], [80.5, 5.0], [0.5, 5.0]]);
    assert_eq!(profile.z, 50.0);
    assert_eq!(offset, [0.0, 0.0, 5.0]);
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
        edge_fillet_r: None,
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
    let walls = Walls::new(&diamond, None, &[], &[], &[], &[], &[], 100.0, 1e-3);
    assert!(walls.cells.len() < 10_000, "{} cells", walls.cells.len());
    assert!(walls.holds([75.0, 25.0]) && walls.holds([25.0, 75.0]));
    assert!(!walls.holds([50.0, 50.0]) && !walls.holds([75.0, 26.0]));
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
