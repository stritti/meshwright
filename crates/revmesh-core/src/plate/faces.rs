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
    /// The wall facets' edges that do not run along the top face, seen from above, as the ends
    /// `a`, `b`, the middle's height `mid_z` (its plan is the midpoint of `a` and `b`) and the
    /// edge's full 3D length.
    pub(super) chords: Vec<(P2, P2, f64, f64)>,
}

/// What a facet is in the split between the two face levels: on the top face, on the bottom
/// face, a cap between them (facing up or down) or a wall. Shared by [`split_faces`] and the
/// wall-angle gate (`check_wall_angles` in `prism.rs`), so both see the same facet.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum FacetClass {
    Top,
    Bottom,
    Cap,
    Wall,
    /// A facet on a face level but facing the wrong way.
    Inward,
}

/// A facet measured against the face levels `min_z`..`top_z`.
pub(super) struct FacetInfo {
    /// Twice the facet's area (the normal's length).
    pub(super) len: f64,
    /// Whether the facet is a sliver flatter than the tolerance: its corners still join its
    /// neighbours as they should, but they can come out turned over once rounded to single
    /// precision, so only its direction is not trusted.
    pub(super) sliver: bool,
    /// The unit normal's vertical share.
    pub(super) nz: f64,
    /// How many corners lie on the top face.
    pub(super) on_top: usize,
    /// How many corners lie on the bottom face.
    pub(super) on_bottom: usize,
}

/// Measures `t` against the face levels `min_z`..`top_z` within `tol`.
pub(super) fn facet_info(t: &[Vec3; 3], min_z: f64, top_z: f64, tol: f64) -> FacetInfo {
    let [a, b, c] = *t;
    let normal = cross(sub(b, a), sub(c, a));
    let len = length(normal);
    let longest = [length(sub(b, a)), length(sub(c, b)), length(sub(a, c))].into_iter().fold(0.0, f64::max);
    let sliver = len == 0.0 || len / longest <= tol;
    let nz = if len == 0.0 { 0.0 } else { normal[2] / len };
    let on_top = t.iter().filter(|p| (p[2] - top_z).abs() <= tol).count();
    let on_bottom = t.iter().filter(|p| (p[2] - min_z).abs() <= tol).count();
    FacetInfo { len, sliver, nz, on_top, on_bottom }
}

impl FacetInfo {
    /// What [`split_faces`] sorts the facet into.
    pub(super) fn class(&self) -> FacetClass {
        if self.on_top + self.on_bottom < 3 {
            if !self.sliver && self.nz.abs() > CAP_NORMAL_MIN { FacetClass::Cap } else { FacetClass::Wall }
        } else {
            match self.on_top {
                3 if self.nz > 0.0 || self.sliver => FacetClass::Top,
                0 if self.nz < 0.0 || self.sliver => FacetClass::Bottom,
                0 | 3 => FacetClass::Inward,
                _ => FacetClass::Wall,
            }
        }
    }
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
                let mid_z = f64::midpoint(p[2], q[2]) - min[2];
                let len = length(sub(q, p));
                chords.push(if a < b { (a, b, mid_z, len) } else { (b, a, mid_z, len) });
            }
        }
    };
    for triangle in &mesh.triangles {
        let [a, b, c] = *triangle;
        let info = facet_info(triangle, min[2], top_z, tol);
        let class = info.class();
        if info.on_top + info.on_bottom < 3 {
            between.extend(triangle.iter().filter(|p| (p[2] - top_z).abs() > tol && (p[2] - min[2]).abs() > tol).map(&relative));
            // A face between the top and the bottom that faces up or down is no wall (a wall
            // band's facets lean a little, never this much): the caller allows it only on a
            // counterbore's floor or a countersink's cone.
            if class == FacetClass::Cap {
                caps.push(triangle.map(|p| relative(&p)));
                continue;
            }
            // A wall band between the faces; its corners on the bottom must lie under the walls
            // too, like those of a wall that runs all the way.
            rim.extend(triangle.iter().filter(|p| (p[2] - min[2]).abs() <= tol).map(|p| [p[0] - min[0], p[1] - min[1]]));
            wall_edges(triangle);
            continue;
        }
        if info.len == 0.0 {
            continue;
        }
        match class {
            FacetClass::Top => top.push(triangle.map(|p| [p[0] - min[0], p[1] - min[1]])),
            // Seen from above, the bottom face's facets turn the other way.
            FacetClass::Bottom => {
                bottom.push([[a[0] - min[0], a[1] - min[1]], [c[0] - min[0], c[1] - min[1]], [b[0] - min[0], b[1] - min[1]]]);
            }
            FacetClass::Inward => return Err("some faces point inwards".into()),
            // A wall from the top to the bottom: its top corners lie on the top face's boundary,
            // its bottom corners must lie under it (the caller checks; a sheared or drafted wall's
            // do not).
            FacetClass::Wall => {
                rim.extend(triangle.iter().filter(|p| (p[2] - min[2]).abs() <= tol).map(|p| [p[0] - min[0], p[1] - min[1]]));
                wall_edges(triangle);
            }
            FacetClass::Cap => unreachable!("a cap has fewer than three corners on the face levels"),
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
            .then(p.2.total_cmp(&q.2))
            .then(p.3.total_cmp(&q.3))
    });
    chords.dedup();
    Ok(Faces { top, bottom, between, caps, rim, chords })
}
