//! Sorting a mesh into its top and bottom faces and the walls between them.

use super::geometry::distance;
use crate::mesh::{Mesh, Vec3, cross, length, sub};

use super::{CAP_NORMAL_MIN, P2};
/// What [`split_faces`] sorts a mesh into, in coordinates relative to its lowest corner.
pub(super) struct Faces {
    /// The top face's triangles, seen from above.
    pub(super) top: Vec<[P2; 3]>,
    /// The bottom face's triangles, seen from above (counter-clockwise, as the top's).
    pub(super) bottom: Vec<[P2; 3]>,
    /// Every corner between the top and the bottom face.
    pub(super) between: Vec<Vec3>,
    /// Facets between the faces that are no walls (they face up or down): the floor of a
    /// counterbore, the cone of a countersink, or a step or pocket.
    pub(super) caps: Vec<[Vec3; 3]>,
    /// The bottom corners of the walls.
    pub(super) rim: Vec<P2>,
    /// The wall facets' edges that do not run along the top face, seen from above, with each
    /// edge's middle.
    pub(super) chords: Vec<(P2, P2, Vec3)>,
}

/// Sorts the triangles into the top face (returned in 2D, relative to `min`), the bottom face
/// (only its area is needed) and vertical walls, and refuses anything else. Also returns every
/// corner between the top and the bottom (in 2D, relative to `min`).
///
/// A wall may have corners between the top and the bottom: some exporters (SolidCraft's own
/// among them) mesh a tall wall in several bands, whose facets lean a little as their corners
/// lie on a curved wall at different heights and angles. The caller checks that those corners
/// lie on the walls; anywhere else they are a step, a pocket or a boss.
pub(super) fn split_faces(mesh: &Mesh, min: Vec3, top_z: f64, tol: f64) -> Result<Faces, String> {
    let mut top = Vec::new();
    let mut bottom = Vec::new();
    let mut between = Vec::new();
    let mut caps = Vec::new();
    let mut rim = Vec::new();
    let relative = |p: &Vec3| [p[0] - min[0], p[1] - min[1], p[2] - min[2]];
    let mut chords = Vec::new();
    let flat = |p: &Vec3| [p[0] - min[0], p[1] - min[1]];
    // A wall facet's edges that do not run along the top face: seen from above they must follow
    // the walls between their corners too, or a facet could cut across a corner.
    let mut wall_edges = |t: &crate::mesh::Triangle| {
        for k in 0..3 {
            let (p, q) = (t[k], t[(k + 1) % 3]);
            let on_top = |v: Vec3| (v[2] - top_z).abs() <= tol;
            let (a, b) = (flat(&p), flat(&q));
            if !(on_top(p) && on_top(q)) && distance(a, b) > tol {
                let middle = relative(&[f64::midpoint(p[0], q[0]), f64::midpoint(p[1], q[1]), f64::midpoint(p[2], q[2])]);
                chords.push(if a < b { (a, b, middle) } else { (b, a, middle) });
            }
        }
    };
    for triangle in &mesh.triangles {
        let [a, b, c] = *triangle;
        let normal = cross(sub(b, a), sub(c, a));
        let len = length(normal);
        // A sliver flatter than the tolerance can come out turned over once its corners are
        // rounded to single precision; its corners still join its neighbours as they should, so
        // only its direction is not trusted.
        let longest = [length(sub(b, a)), length(sub(c, b)), length(sub(a, c))].into_iter().fold(0.0, f64::max);
        let sliver = len == 0.0 || len / longest <= tol;
        let nz = if len == 0.0 { 0.0 } else { normal[2] / len };
        let on_top = triangle.iter().filter(|p| (p[2] - top_z).abs() <= tol).count();
        let on_bottom = triangle.iter().filter(|p| (p[2] - min[2]).abs() <= tol).count();
        if on_top + on_bottom < 3 {
            between.extend(triangle.iter().filter(|p| (p[2] - top_z).abs() > tol && (p[2] - min[2]).abs() > tol).map(&relative));
            // A face between the top and the bottom that faces up or down is no wall (a wall
            // band's facets lean a little, never this much): the caller allows it only on a
            // counterbore's floor or a countersink's cone.
            if !sliver && nz.abs() > CAP_NORMAL_MIN {
                caps.push(triangle.map(|p| relative(&p)));
                continue;
            }
            // A wall band between the faces; its corners on the bottom must lie under the walls
            // too, like those of a wall that runs all the way.
            rim.extend(triangle.iter().filter(|p| (p[2] - min[2]).abs() <= tol).map(|p| [p[0] - min[0], p[1] - min[1]]));
            wall_edges(triangle);
            continue;
        }
        if len == 0.0 {
            continue;
        }
        match on_top {
            3 if nz > 0.0 || sliver => top.push(triangle.map(|p| [p[0] - min[0], p[1] - min[1]])),
            // Seen from above, the bottom face's facets turn the other way.
            0 if nz < 0.0 || sliver => {
                bottom.push([[a[0] - min[0], a[1] - min[1]], [c[0] - min[0], c[1] - min[1]], [b[0] - min[0], b[1] - min[1]]]);
            }
            0 | 3 => return Err("some faces point inwards".into()),
            // A wall from the top to the bottom: its top corners lie on the top face's boundary,
            // its bottom corners must lie under it (the caller checks; a sheared or drafted wall's
            // do not).
            _ => {
                rim.extend(triangle.iter().filter(|p| (p[2] - min[2]).abs() <= tol).map(|p| [p[0] - min[0], p[1] - min[1]]));
                wall_edges(triangle);
            }
        }
    }
    // A corner or an edge is shared by several facets; it needs checking once.
    rim.sort_by(|p, q| p[0].total_cmp(&q[0]).then(p[1].total_cmp(&q[1])));
    rim.dedup();
    between.sort_by(|p, q| p[0].total_cmp(&q[0]).then(p[1].total_cmp(&q[1])).then(p[2].total_cmp(&q[2])));
    between.dedup();
    chords.sort_by(|p, q| {
        p.0[0]
            .total_cmp(&q.0[0])
            .then(p.0[1].total_cmp(&q.0[1]))
            .then(p.1[0].total_cmp(&q.1[0]))
            .then(p.1[1].total_cmp(&q.1[1]))
            .then(p.2[2].total_cmp(&q.2[2]))
    });
    chords.dedup();
    Ok(Faces { top, bottom, between, caps, rim, chords })
}
