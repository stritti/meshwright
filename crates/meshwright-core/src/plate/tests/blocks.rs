use crate::mesh::Mesh;
use crate::plate::P2;
use crate::recognize::{PadSpec, PocketSpec};
use crate::synth;

use super::{found, reason};

/// A 70 x 40 x 6 plate exported from SolidCraft 0.45 with a 16 x 12 x 4 pad on its top face, a
/// 20 x 14 x 3 pocket in the top face, a 12 x 8 x 2 one in the bottom face and an Ø4 hole.
#[test]
fn recognises_pads_and_pockets() {
    let mesh = crate::stl::parse(include_bytes!("../../../../../plugin/meshwright/tests/pockets.stl")).expect("parses");
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
    let mut mesh = crate::stl::parse(include_bytes!("../../../../../plugin/meshwright/tests/pockets.stl")).expect("parses");
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
