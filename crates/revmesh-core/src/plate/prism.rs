//! Recognising the prism itself: the pipeline over the faces' loops — outline, cut-outs,
//! prism and wall checks, and the recognition from the outline's corners.

use super::faces::{Faces, split_faces};
use super::geometry::{boundary_loops, distance, signed_area};
use super::holes::{BlindHole, Cutouts, FittedHole, FittedSlot, Stepped, axis_rectangle, polygon_cutout, round_hole, slot_shape};
use super::outline::outline_shape;
use super::walls::Walls;
use super::{AREA_MATCH_REL, CAP_NORMAL_MIN, CHORD_FACET_SPAN, MAX_VOLUME_ERROR, NOT_CLOSED, NOT_FLAT, P2, VOLUME_MATCH_REL, tolerance};
use crate::mesh::{Mesh, Vec3, cross, length, snap_mm, sub};
use crate::recognize::{Blind, HoleSpec, OutlinePocketSpec, PlateSpec, PocketSpec, ProfileSpec, Recognition, SlotSpec};

use super::holes::Footprint;
/// Looks for a plate in `mesh`: a prism over its top face, with holes, slots, cut-outs, pockets
/// and pads through or on it.
pub(super) fn recognize_prism(mesh: &Mesh, allow_steps: bool) -> Result<Recognition, String> {
    let bounds = mesh.bounds().ok_or("the mesh has no triangles")?;
    let size = bounds.size();
    let scale = size[0].max(size[1]).max(size[2]);
    let tol = tolerance(scale);
    if size.iter().any(|s| *s <= tol) {
        return Err("the mesh is flat".into());
    }
    let mesh_volume = mesh.volume();
    if mesh_volume <= 0.0 {
        return Err("the mesh is not closed or its faces point inwards".into());
    }

    let faces = split_faces(mesh, bounds.min, bounds.max[2], tol)?;
    // A step down from the top face is an up-facing facet level between the faces that shares
    // an edge with the top; the outline is the top face's and the steps' regions together.
    // Steps are recognised along Z only for now; along X or Y they would take parts that are
    // better written as an intersection of extrusions (roadmap 5.3).
    let steps = if allow_steps { step_levels(&faces, size[2], tol)? } else { Vec::new() };
    let face_loops = face_outlines(&faces, &steps, tol)?;
    let (corners, radius) = outline_shape(&face_loops.outline, tol)?;
    let rim = rim_fillet(&face_loops.outline, &corners, radius, size, tol);
    let mut cutouts = pair_cutouts(&faces, &face_loops, size[2], tol)?;
    add_steps(&mut cutouts, &steps, size[2], tol)?;
    check_prism(&faces, mesh_volume, size[2], &cutouts.stepped, &cutouts.blinds, rim.as_ref())?;
    check_walls(&faces, &corners, radius, &cutouts, scale, tol, rim.as_ref())?;
    let offset = bounds.min.map(snap_mm);
    recognition(&corners, radius, cutouts, size, offset, mesh_volume, rim.as_ref())
}

/// A box rounded all over (mesh2cad's `edge_fillet_r`): its rim radius and size, for the checks
/// against its surface — every point of it lies at distance `r` from the inner box.
pub(super) struct Rim {
    /// The radius all twelve edges are rounded with.
    pub(super) r: f64,
    /// The box's size along X, Y and Z.
    pub(super) size: Vec3,
}

impl Rim {
    /// Whether `p` (relative to the box's lowest corner) lies on the rounded box's surface, at
    /// distance `r` from the inner box within `tol + slack` — the twelve quarter cylinders, the
    /// eight sphere octants and the four flat side faces all are at exactly that distance.
    pub(super) fn holds(&self, p: Vec3, tol: f64, slack: f64) -> bool {
        let mut d2 = 0.0;
        for i in 0..3 {
            let (lo, hi) = (self.r, self.size[i] - self.r);
            let d = if p[i] < lo {
                lo - p[i]
            } else if p[i] > hi {
                p[i] - hi
            } else {
                0.0
            };
            d2 += d * d;
        }
        (d2.sqrt() - self.r).abs() <= tol + slack
    }

    /// The sagitta a chord of length `len` on the rounded surface bows inward by.
    pub(super) fn sagitta(&self, len: f64) -> f64 {
        let half = len / 2.0;
        if half < self.r { self.r - (self.r * self.r - half * half).sqrt() } else { self.r }
    }
}

/// The rim's rounding when the outline is the rectangle inset by one radius on every side: a box
/// rounded all over has its flat top and bottom faces inset by the radius, so the top face's
/// outline is that inner rectangle. Finely tessellated rims leak near-tangent facets into the cap
/// (their corners lie within the tolerance of its plane), which bulges the outline out by the
/// corner arcs' radius `radius`; the true inset is the bounding box's inset plus that radius.
pub(super) fn rim_fillet(outline: &[P2], corners: &[P2], radius: Option<f64>, size: Vec3, tol: f64) -> Option<Rim> {
    if corners.len() != 4 {
        return None;
    }
    let (mut x0, mut x1) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut y0, mut y1) = (f64::INFINITY, f64::NEG_INFINITY);
    for p in outline {
        x0 = x0.min(p[0]);
        x1 = x1.max(p[0]);
        y0 = y0.min(p[1]);
        y1 = y1.max(p[1]);
    }
    // The four corners must be the bounding box's corners, axis-aligned.
    let at = |v: f64, lo: f64, hi: f64| (v - lo).abs() <= tol || (v - hi).abs() <= tol;
    if !corners.iter().all(|c| at(c[0], x0, x1) && at(c[1], y0, y1)) {
        return None;
    }
    let insets = [x0, y0, size[0] - x1, size[1] - y1];
    let inset = insets.iter().sum::<f64>() / 4.0;
    if insets.iter().any(|&d| (d - inset).abs() > tol) || inset <= tol {
        return None;
    }
    let r = inset + radius.unwrap_or(0.0);
    if 2.0 * r >= size[0].min(size[1]).min(size[2]) {
        return None;
    }
    Some(Rim { r, size })
}

/// Whether `a` and `b` are within `rel` of each other, relative to the larger one.
pub(super) fn close(a: f64, b: f64, rel: f64) -> bool {
    (a - b).abs() <= rel * a.abs().max(b.abs())
}

/// The two faces' boundary loops, and the top face's single outline among them.
pub(super) struct FaceLoops {
    /// The top face's single counter-clockwise outline, the steps' regions included.
    pub(super) outline: Vec<P2>,
    /// The top face's boundary loops (the steps' holes included): the outline and the cut-outs'.
    pub(super) loops: Vec<Vec<P2>>,
    /// The bottom face's boundary loops.
    pub(super) bottom_loops: Vec<Vec<P2>>,
}

/// How many distinct levels a step down may have; more means a slanted or curved surface, no
/// stepful plate.
const MAX_STEP_LEVELS: usize = 8;

/// The up-facing facet levels strictly between the bottom and the top face whose region shares
/// an edge with the top's on which a vertical wall stands — a step down from the top face (a
/// pocket's floor shares no edge with the top, and a slanted surface's facets are caps whose
/// edges no wall stands on). Each level's height and its triangles seen from above.
#[allow(clippy::type_complexity)] // a level's height and its triangles
fn step_levels(faces: &Faces, height: f64, tol: f64) -> Result<Vec<(f64, Vec<[P2; 3]>)>, String> {
    let mut levels: Vec<(f64, Vec<[P2; 3]>)> = Vec::new();
    for cap in &faces.caps {
        let n = cross(sub(cap[1], cap[0]), sub(cap[2], cap[0]));
        let len = length(n);
        if len == 0.0 || n[2] / len <= CAP_NORMAL_MIN {
            continue; // down-facing (a pocket's or counterbore's floor) or a sliver
        }
        let z = (cap[0][2] + cap[1][2] + cap[2][2]) / 3.0;
        if z <= tol || z >= height - tol {
            continue; // facets leaked into the caps near the faces (a rounded rim)
        }
        let flat = cap.map(|p| [p[0], p[1]]);
        if let Some((_, ts)) = levels.iter_mut().find(|(lz, _)| (*lz - z).abs() <= tol) {
            ts.push(flat);
        } else {
            if levels.len() >= MAX_STEP_LEVELS {
                return Ok(Vec::new());
            }
            levels.push((z, vec![flat]));
        }
    }
    if levels.is_empty() {
        return Ok(levels);
    }
    // Every wall facet's edge off the top face is a chord; a vertical wall stands on it.
    let bits = |p: P2| (p[0].to_bits(), p[1].to_bits());
    let mut wall_bases: std::collections::HashSet<(u64, u64, u64, u64)> = std::collections::HashSet::new();
    for (a, b, _, _) in &faces.chords {
        let (ka, kb) = (bits(*a), bits(*b));
        wall_bases.insert((ka.0, ka.1, kb.0, kb.1));
        wall_bases.insert((kb.0, kb.1, ka.0, ka.1));
    }
    let edges_of = |ring: &[P2]| (0..ring.len()).map(|i| (ring[i], ring[(i + 1) % ring.len()])).collect::<Vec<_>>();
    let same_edge = |(a, b): (P2, P2), (c, d): (P2, P2)| {
        (distance(a, c) <= tol && distance(b, d) <= tol) || (distance(a, d) <= tol && distance(b, c) <= tol)
    };
    // A level is a step when its outer loop shares an edge with the top face's boundary (or with
    // a step already found, for a second level down) and a wall stands on that edge.
    let top_outer = boundary_loops(&faces.top, tol)?.into_iter().find(|l| signed_area(l) > 0.0).ok_or("the top face has no outline")?;
    let mut union_edges = edges_of(&top_outer);
    let mut steps = Vec::new();
    let mut remaining = levels;
    loop {
        let mut attached = false;
        remaining.retain(|(z, ts)| {
            let Some(outer) = boundary_loops(ts, tol).ok().and_then(|ls| ls.into_iter().find(|l| signed_area(l) > 0.0)) else {
                return true;
            };
            let shared = edges_of(&outer).into_iter().filter(|e| union_edges.iter().any(|u| same_edge(*e, *u))).collect::<Vec<_>>();
            let on_wall = shared.iter().any(|&(a, b)| {
                let (ka, kb) = (bits(a), bits(b));
                wall_bases.contains(&(ka.0, ka.1, kb.0, kb.1))
            });
            if on_wall {
                union_edges.extend(edges_of(&outer));
                steps.push((*z, ts.clone()));
                attached = true;
                false
            } else {
                true
            }
        });
        if !attached {
            break;
        }
    }
    Ok(steps)
}

/// The faces' boundary loops and the top face's single outline among them (the steps' regions
/// included in the top), checked against the bottom face's outline.
pub(super) fn face_outlines(faces: &Faces, steps: &[(f64, Vec<[P2; 3]>)], tol: f64) -> Result<FaceLoops, String> {
    let mut top = faces.top.clone();
    for (_, ts) in steps {
        top.extend(ts.iter().copied());
    }
    let loops = boundary_loops(&top, tol)?;
    let bottom_loops = boundary_loops(&faces.bottom, tol)?;
    let mut outlines = loops.iter().filter(|l| signed_area(l) > 0.0);
    let outline = outlines.next().ok_or("the top face has no outline")?.clone();
    if outlines.next().is_some() {
        return Err("the top face has more than one outline (several separate parts are not recognised yet)".into());
    }
    let bottom_outline = bottom_loops.iter().find(|l| signed_area(l) > 0.0).ok_or("the bottom face has no outline")?;
    // An exporter may mesh the top and bottom faces apart, with their own corners on round
    // edges: their outlines must match to 0.01%. A draft changes that by far more.
    if !close(signed_area(&outline), signed_area(bottom_outline), AREA_MATCH_REL) {
        return Err(NOT_CLOSED.into());
    }
    Ok(FaceLoops { outline, loops, bottom_loops })
}

/// The holes, slots, cut-outs and pockets a plate has, from the two faces' boundary loops.
pub(super) struct PlateCutouts {
    /// The through holes' circles, as fitted: the owning hole's index in the sorted `holes`
    /// (the design numbers them `hole_<n>`), the centre and the radius.
    pub(super) circles: Vec<(usize, P2, f64)>,
    /// The loops the walls stand on, as meshed: the outline and the cut-outs'.
    pub(super) wall_loops: Vec<Vec<P2>>,
    /// The slots' two end arcs (the slot's index in the sorted `slots`, centre, radius, corner,
    /// reach), bounded round walls the wall check uses too.
    pub(super) slot_arcs: Vec<(usize, P2, f64, P2, f64)>,
    /// Every hole, sorted from left to right.
    pub(super) holes: Vec<HoleSpec>,
    /// The straight slots, in mesh2cad's order.
    pub(super) slots: Vec<SlotSpec>,
    /// The polygonal cut-outs, each counter-clockwise from its lowest corner, the leftmost of
    /// those, sorted by that corner.
    pub(super) cutouts: Vec<Vec<P2>>,
    /// The rectangular pockets, sorted from left to right.
    pub(super) pockets: Vec<PocketSpec>,
    /// The pockets of any other outline of straight sides, sorted by their first corner.
    pub(super) outline_pockets: Vec<OutlinePocketSpec>,
    /// The stepped holes (a counterbore or a countersink).
    pub(super) stepped: Vec<Stepped>,
    /// The blind holes and the pockets' mouths.
    pub(super) blinds: Vec<BlindHole>,
}

/// Fits the cut-outs of the two faces' boundary loops into the plate's, pairing each with the
/// same cut-out on the other face. A cut-out that is not round may be a straight slot or a
/// polygon of straight sides; either must go straight through, unless it is a pocket's mouth.
pub(super) fn pair_cutouts(faces: &Faces, face_loops: &FaceLoops, height: f64, tol: f64) -> Result<PlateCutouts, String> {
    let (tops, top_slots, top_polygons) = fit_loops(&face_loops.loops, tol)?;
    let (bottoms, bottom_slots, bottom_polygons) = fit_loops(&face_loops.bottom_loops, tol)?;
    let mut out = PlateCutouts {
        circles: Vec::new(),
        wall_loops: vec![face_loops.outline.clone()],
        slot_arcs: Vec::new(),
        holes: Vec::new(),
        slots: Vec::new(),
        cutouts: Vec::new(),
        pockets: Vec::new(),
        outline_pockets: Vec::new(),
        stepped: Vec::new(),
        blinds: Vec::new(),
    };
    let pocket_mouths = pair_polygons(faces, &top_polygons, &bottom_polygons, face_loops, &mut out, height, tol)?;
    pair_slots(&top_slots, &bottom_slots, face_loops, &mut out, tol)?;
    let mut mouths: Vec<(Option<&HoleSpec>, BlindHole)> = pocket_mouths.into_iter().map(|b| (None, b)).collect();
    mouths.extend(pair_through_holes(faces, &tops, &bottoms, face_loops, &mut out, height, tol)?);
    floor_mouths(faces, mouths, &mut out, height, tol)?;
    Ok(out)
}

/// Fits the round holes, the straight slots and the polygonal cut-outs in one face's boundary
/// loops.
pub(super) fn fit_loops(loops: &[Vec<P2>], tol: f64) -> Result<Cutouts, String> {
    let (mut holes, mut slots, mut polygons) = (Vec::new(), Vec::new(), Vec::new());
    for (k, l) in loops.iter().enumerate().filter(|(_, l)| signed_area(l) < 0.0) {
        match round_hole(l, tol) {
            Ok((h, c)) => holes.push((h, c, k)),
            Err(why) => {
                if let Some((spec, geometry)) = slot_shape(l, tol) {
                    slots.push((spec, geometry, k));
                } else {
                    polygons.push((polygon_cutout(l, tol).ok_or(why)?, k));
                }
            }
        }
    }
    Ok((holes, slots, polygons))
}

/// The polygonal cut-outs. A cut-out on both faces goes through; a polygon on one face only is
/// a pocket's mouth (a rectangle along X and Y, or any other outline of straight sides), its
/// floor found with the blind holes'. Returns the pockets' mouths. Each through cut-out is
/// written counter-clockwise from its lowest corner, the leftmost of those.
pub(super) fn pair_polygons(
    faces: &Faces,
    top: &[(Vec<P2>, usize)],
    bottom: &[(Vec<P2>, usize)],
    face_loops: &FaceLoops,
    out: &mut PlateCutouts,
    height: f64,
    tol: f64,
) -> Result<Vec<BlindHole>, String> {
    let loops = &face_loops.loops;
    let bottom_loops = &face_loops.bottom_loops;
    let same_polygon = |a: &[P2], b: &[P2]| a.len() == b.len() && a.iter().all(|p| b.iter().any(|q| distance(*p, *q) <= tol));
    let pocket = |ring: &[P2], face_z: f64, inward: f64| -> Result<BlindHole, String> {
        let foot = match axis_rectangle(ring, tol) {
            Some((lo, hi)) => Footprint::Rect { lo, hi },
            None => Footprint::Polygon { ring: pocket_corners(ring, tol)? },
        };
        Ok(BlindHole { foot, ring: ring.to_vec(), face_z, inward, depth: 0.0 })
    };
    // Two rectangles matching on the faces are two pockets when floors lie between them, facets
    // facing up or down inside the rectangle.
    let floored = |ring: &[P2]| {
        axis_rectangle(ring, tol).is_some_and(|(lo, hi)| {
            let foot = Footprint::Rect { lo, hi };
            faces.caps.iter().any(|cap| cap.iter().all(|p| p[2] > tol && p[2] < height - tol && foot.contains([p[0], p[1]], tol)))
        })
    };
    let mut pocket_mouths = Vec::new();
    let mut paired = vec![false; bottom.len()];
    for (c, k) in top {
        match bottom.iter().position(|(other, _)| same_polygon(c, other)) {
            Some(j) if floored(&loops[*k]) => {
                paired[j] = true;
                pocket_mouths.push(pocket(&loops[*k], height, -1.0)?);
                pocket_mouths.push(pocket(&bottom_loops[bottom[j].1], 0.0, 1.0)?);
            }
            Some(j) => {
                paired[j] = true;
                out.wall_loops.push(loops[*k].clone());
                let mut c: Vec<P2> = c.iter().rev().map(|p| p.map(snap_mm)).collect();
                let first = (0..c.len()).min_by(|&a, &b| c[a][1].total_cmp(&c[b][1]).then(c[a][0].total_cmp(&c[b][0])));
                c.rotate_left(first.unwrap_or(0));
                out.cutouts.push(c);
            }
            None => pocket_mouths.push(pocket(&loops[*k], height, -1.0)?),
        }
    }
    for ((_, k), _) in bottom.iter().zip(&paired).filter(|(_, paired)| !**paired) {
        pocket_mouths.push(pocket(&bottom_loops[*k], 0.0, 1.0)?);
    }
    out.cutouts.sort_by(|a, b| a[0][1].total_cmp(&b[0][1]).then(a[0][0].total_cmp(&b[0][0])));
    Ok(pocket_mouths)
}

/// Each step as a pocket cut from the top face down to the step's level: the step region's
/// outline (which shares edges with the plate's outline) as an outline pocket, and a blind over
/// it so the wall checks hold the step's wall and its level's facets.
fn add_steps(cutouts: &mut PlateCutouts, steps: &[(f64, Vec<[P2; 3]>)], height: f64, tol: f64) -> Result<(), String> {
    for (z, ts) in steps {
        for l in boundary_loops(ts, tol)?.iter().filter(|l| signed_area(l) > 0.0) {
            let corners = pocket_corners(l, tol)?;
            let depth = height - z;
            cutouts.outline_pockets.push(OutlinePocketSpec { corners: corners.clone(), depth: snap_mm(depth), bottom: false });
            cutouts.blinds.push(BlindHole {
                foot: Footprint::Polygon { ring: corners },
                ring: l.clone(),
                face_z: height,
                inward: -1.0,
                depth,
            });
        }
    }
    cutouts.outline_pockets.sort_by(|a, b| a.corners[0][1].total_cmp(&b.corners[0][1]).then(a.corners[0][0].total_cmp(&b.corners[0][0])));
    Ok(())
}

/// A pocket mouth's polygon of straight sides: counter-clockwise from its lowest corner, the
/// leftmost of those, its corners snapped.
fn pocket_corners(ring: &[P2], tol: f64) -> Result<Vec<P2>, String> {
    let mut corners = polygon_cutout(ring, tol)
        .ok_or("a cut-out does not go straight through (pockets of curved sides and slanted cut-outs are not recognised yet)")?;
    if signed_area(&corners) < 0.0 {
        corners.reverse();
    }
    let mut corners: Vec<P2> = corners.iter().map(|p| p.map(snap_mm)).collect();
    let first = (0..corners.len()).min_by(|&a, &b| corners[a][1].total_cmp(&corners[b][1]).then(corners[a][0].total_cmp(&corners[b][0])));
    corners.rotate_left(first.unwrap_or(0));
    Ok(corners)
}

/// The straight slots, which must go straight through. In mesh2cad's order: by Y, then X, angle
/// and width.
pub(super) fn pair_slots(
    top: &[FittedSlot],
    bottom: &[FittedSlot],
    face_loops: &FaceLoops,
    out: &mut PlateCutouts,
    tol: f64,
) -> Result<(), String> {
    if top.len() != bottom.len() || !top.iter().all(|(_, g, _)| bottom.iter().any(|(_, other, _)| g.matches(other, tol))) {
        return Err("a slot does not go straight through (blind and slanted slots are not recognised yet)".into());
    }
    // Sort the slots with their geometry and loop, in mesh2cad's order, so each end arc keeps
    // the number its slot gets in the design (`slot_<n>`).
    let mut fitted = top.to_vec();
    fitted.sort_by(|a, b| {
        a.0.y.total_cmp(&b.0.y).then(a.0.x.total_cmp(&b.0.x)).then(a.0.angle.total_cmp(&b.0.angle)).then(a.0.w.total_cmp(&b.0.w))
    });
    for (i, (spec, g, k)) in fitted.into_iter().enumerate() {
        out.slot_arcs.extend(g.halves().into_iter().map(|(centre, r, corner, t)| (i, centre, r, corner, t)));
        out.wall_loops.push(face_loops.loops[k].clone());
        out.slots.push(spec);
    }
    Ok(())
}

/// Pairs each circle on the top face with the circle about the same centre on the bottom face
/// (the circles as fitted; the parameters are rounded). Where the two differ, the wider one is
/// the mouth of a counterbore or a countersink. Returns the mouths of holes that go through
/// neither face: the blind holes'.
pub(super) fn pair_through_holes<'a>(
    faces: &Faces,
    tops: &'a [FittedHole],
    bottoms: &'a [FittedHole],
    face_loops: &FaceLoops,
    out: &mut PlateCutouts,
    height: f64,
    tol: f64,
) -> Result<Vec<(Option<&'a HoleSpec>, BlindHole)>, String> {
    let loops = &face_loops.loops;
    let bottom_loops = &face_loops.bottom_loops;
    let mut mouths = Vec::new();
    let mut paired = vec![false; bottoms.len()];
    for (hole, (centre, top_r), loop_index) in tops {
        let Some(j) = bottoms.iter().position(|(_, (c, _), _)| distance(*c, *centre) <= tol) else {
            mouths.push((Some(hole), blind_mouth(*centre, *top_r, &loops[*loop_index], height, -1.0)));
            continue;
        };
        paired[j] = true;
        let (_, (bottom_centre, bottom_r), bottom_index) = &bottoms[j];
        if (top_r - bottom_r).abs() <= tol {
            out.circles.push((0, *centre, *top_r)); // the owning index is set once `holes` is sorted
            out.wall_loops.push(loops[*loop_index].clone());
            out.holes.push(hole.clone());
            continue;
        }
        let (spec, s) = stepped_hole(
            faces,
            (*centre, *top_r),
            (*bottom_centre, *bottom_r),
            (&loops[*loop_index], &bottom_loops[*bottom_index]),
            height,
            tol,
        )?;
        out.holes.push(spec);
        out.stepped.push(s);
    }
    for ((hole, (centre, r), k), _) in bottoms.iter().zip(&paired).filter(|(_, paired)| !**paired) {
        mouths.push((Some(hole), blind_mouth(*centre, *r, &bottom_loops[*k], 0.0, 1.0)));
    }
    Ok(mouths)
}

/// A blind hole's round mouth on one face only; its depth is found from the facets within it.
pub(super) fn blind_mouth(centre: P2, r: f64, ring: &[P2], face_z: f64, inward: f64) -> BlindHole {
    BlindHole { foot: Footprint::Round { centre, r }, ring: ring.to_vec(), face_z, inward, depth: 0.0 }
}

/// The floors of the blind holes' and the pockets' mouths: the nearest facets facing up or down
/// within each footprint (beyond them may lie the floor of a pocket facing it from the other
/// face). That they are flat and all at one depth is checked with the other facets below.
pub(super) fn floor_mouths(
    faces: &Faces,
    mouths: Vec<(Option<&HoleSpec>, BlindHole)>,
    out: &mut PlateCutouts,
    height: f64,
    tol: f64,
) -> Result<(), String> {
    for (hole, mut b) in mouths {
        let (face_z, inward) = (b.face_z, b.inward);
        let inside = |p: &Vec3| p[2] > tol && p[2] < height - tol;
        b.depth = faces
            .caps
            .iter()
            .filter(|cap| cap.iter().all(|p| inside(p) && b.reaches([p[0], p[1]], tol)))
            .flatten()
            .map(|p| (p[2] - face_z) * inward)
            .reduce(f64::min)
            .ok_or(
                "a hole neither goes straight through nor ends in a flat floor (slanted holes and drill points are not recognised yet)",
            )?;
        match (hole, &b.foot) {
            (Some(hole), _) => {
                out.holes.push(HoleSpec { blind: Some(Blind { depth: snap_mm(b.depth), bottom: inward > 0.0 }), ..hole.clone() });
            }
            (None, Footprint::Rect { lo, hi }) => {
                let [x, y] = b.foot.middle().map(snap_mm);
                out.pockets.push(PocketSpec {
                    x,
                    y,
                    w: snap_mm(hi[0] - lo[0]),
                    l: snap_mm(hi[1] - lo[1]),
                    depth: snap_mm(b.depth),
                    bottom: inward > 0.0,
                });
            }
            (None, Footprint::Polygon { ring }) => {
                out.outline_pockets.push(OutlinePocketSpec { corners: ring.clone(), depth: snap_mm(b.depth), bottom: inward > 0.0 });
            }
            (None, Footprint::Round { .. }) => unreachable!("a round mouth is a hole's"),
        }
        out.blinds.push(b);
    }
    out.holes.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    // Each through hole's circle carries its hole's index in the sorted holes, so the wall
    // labels name the design's `hole_<n>`, not the order the circles were fitted in.
    for c in &mut out.circles {
        let best = out
            .holes
            .iter()
            .enumerate()
            .map(|(i, h)| (i, distance([h.x, h.y], c.1)))
            .filter(|(_, d)| *d <= tol)
            .min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((i, _)) = best {
            c.0 = i;
        }
    }
    out.pockets.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    out.outline_pockets.sort_by(|a, b| a.corners[0][1].total_cmp(&b.corners[0][1]).then(a.corners[0][0].total_cmp(&b.corners[0][0])));
    Ok(())
}

/// Whether the mouth of the stepped hole `s` (its corners between the faces `mine`) is a round of
/// its rim: a quarter circle about (mouth radius, as deep as the mouth is wider than the bore),
/// meshed in steps along it no coarser than a round hole's (a round in one step has no corners
/// between the face and the bore: it is a 90° countersink, which looks the same). Every corner
/// between lies on it, or off it by no more than its facets do (a corner an exporter added
/// within one): that allowance, the sagitta of its largest step along the quarter circle and
/// of the loops' facets round the axis, is returned.
fn rounded_rim(s: &Stepped, mine: &[Vec3], tol: f64) -> Option<(f64, f64)> {
    let depth = |p: &Vec3| (p[2] - s.face_z) * s.inward;
    let r = |p: &Vec3| distance([p[0], p[1]], s.centre);
    let between: Vec<&Vec3> =
        mine.iter().filter(|p| depth(p) > tol && depth(p) < s.head_r - s.bore_r + tol && r(p) > s.bore_r + tol).collect();
    // The round's radius from its corners: one `a` out from the bore and `d` down from the face
    // lies on the quarter circle of radius `a + d + sqrt(2ad)`.
    let samples: Vec<f64> = between.iter().map(|p| (r(p) - s.bore_r, depth(p))).map(|(a, d)| a + d + (2.0 * a * d).sqrt()).collect();
    if samples.is_empty() {
        return None;
    }
    let f = samples.iter().sum::<f64>() / samples.len() as f64;
    // The mouth as measured agrees with it: as wide, or narrower by no more than the round's
    // first facets can lie within the tolerance of the face (rising by `tol` at most, they reach
    // `sqrt(2 f tol)` in from the mouth).
    let head_r = s.bore_r + f;
    let narrower = head_r - s.head_r;
    if narrower < -tol || narrower > (2.0 * f * tol).sqrt() + tol {
        return None;
    }
    let off = |p: &Vec3| ((r(p) - head_r).hypot(depth(p) - f) - f).abs();
    // The steps along the quarter circle between its own corners (those on it), from the face
    // (at -90° about its centre) to the bore (at 180°).
    let along = |p: &Vec3| (depth(p) - f).atan2(r(p) - head_r).rem_euclid(2.0 * std::f64::consts::PI);
    let mut angles: Vec<f64> = between.iter().filter(|p| off(p) <= tol).map(|p| along(p)).collect();
    angles.extend([1.5 * std::f64::consts::PI, std::f64::consts::PI]);
    angles.sort_by(f64::total_cmp);
    let step = angles.windows(2).map(|w| w[1] - w[0]).fold(0.0, f64::max);
    if between.is_empty() || step > super::MAX_HOLE_STEP + 1e-9 {
        return None;
    }
    let round_the_axis =
        (super::geometry::sagitta(&s.mouth, s.head_r) / s.head_r).max(super::geometry::sagitta(&s.bore, s.bore_r) / s.bore_r);
    let slack = f * (1.0 - (step / 2.0).cos()) + head_r * round_the_axis;
    // It meets the bore where the bore's corners nearest the face are, at its own depth (a
    // flatter cone meets it higher up, though its corners may fit a round as wide): the
    // shallowest of them lies as deep as the round is at its radius.
    let bore_in = s.bore_r - super::geometry::sagitta(&s.bore, s.bore_r) - tol;
    let round_at = |p: &Vec3| f - (f * f - (head_r - r(p)).powi(2)).max(0.0).sqrt();
    let meets =
        mine.iter().filter(|p| depth(p) > tol && r(p) >= bore_in && r(p) <= s.bore_r + tol).min_by(|a, b| depth(a).total_cmp(&depth(b)));
    if meets.is_none_or(|p| (depth(p) - round_at(p)).abs() > slack + tol) {
        return None;
    }
    between.iter().all(|p| off(p) <= slack + tol).then_some((f, slack))
}

/// A hole whose top and bottom circles differ in radius: the wider one is the mouth of a
/// counterbore or a countersink, told apart by the corners between the faces.
pub(super) fn stepped_hole(
    faces: &Faces,
    top: (P2, f64),
    bottom: (P2, f64),
    loops: (&[P2], &[P2]),
    height: f64,
    tol: f64,
) -> Result<(HoleSpec, Stepped), String> {
    let (top_centre, top_r) = top;
    let (bottom_centre, bottom_r) = bottom;
    let centre = [f64::midpoint(top_centre[0], bottom_centre[0]), f64::midpoint(top_centre[1], bottom_centre[1])];
    let on_top = top_r > bottom_r;
    let (head_r, bore_r) = if on_top { (top_r, bottom_r) } else { (bottom_r, top_r) };
    let (face_z, inward) = if on_top { (height, -1.0) } else { (0.0, 1.0) };
    let (mouth, bore) = if on_top { (loops.0.to_vec(), loops.1.to_vec()) } else { (loops.1.to_vec(), loops.0.to_vec()) };
    let mut s =
        Stepped { centre, bore_r, head_r, bore, mouth, face_z, inward, depth: 0.0, counterbore: false, rounded: false, round_slack: 0.0 };
    // Its corners between the faces tell the kind: a counterbore has some on its wide wall,
    // down to its floor; a countersink's cone meets the bore at the bore's corners nearest
    // the face.
    let inside = |p: &Vec3| p[2] > tol && p[2] < height - tol;
    let mine: Vec<Vec3> =
        faces.between.iter().chain(faces.caps.iter().flatten()).copied().filter(|p| inside(p) && s.reaches([p[0], p[1]], tol)).collect();
    let depth = |p: &Vec3| (p[2] - face_z) * inward;
    let r = |p: &Vec3| distance([p[0], p[1]], centre);
    if let Some(floor) = mine.iter().filter(|p| (r(p) - head_r).abs() <= tol).map(depth).reduce(f64::max) {
        s.counterbore = true;
        s.depth = floor;
    } else if let Some((f, slack)) = rounded_rim(&s, &mine, tol) {
        // A round of the rim of radius `f`, as deep: the mouth is the bore widened by it (its
        // first, nearly flat facets may lie within the tolerance of the face, the face's loop a
        // little narrower).
        s.rounded = true;
        s.round_slack = slack;
        s.head_r = bore_r + f;
        s.depth = f;
    } else {
        s.depth = mine
            .iter()
            .filter(|p| (r(p) - bore_r).abs() <= tol)
            .map(depth)
            .reduce(f64::min)
            .ok_or("a hole narrows all the way through (tapered holes are not recognised yet)")?;
    }
    if s.depth <= tol || s.depth >= height - tol {
        return Err("a hole's counterbore or countersink is as deep as the plate (stepped holes are not recognised yet)".into());
    }
    let spec = HoleSpec { x: snap_mm(centre[0]), y: snap_mm(centre[1]), d: snap_mm(2.0 * bore_r), head: Some(s.head()), blind: None };
    Ok((spec, s))
}

/// Without stepped holes the mesh must be a prism over its top face: the two faces match, and
/// walls meshed in bands bulge only a little between the top face's corners (0.1%). A step
/// changes either by far more. A box rounded all over (`rim`) is smaller than its top face times
/// the height; its volume is checked against the rounded box in [`recognition`].
pub(super) fn check_prism(
    faces: &Faces,
    mesh_volume: f64,
    height: f64,
    stepped: &[Stepped],
    blinds: &[BlindHole],
    rim: Option<&Rim>,
) -> Result<(), String> {
    if stepped.is_empty() && blinds.is_empty() {
        let area = |t: &[[P2; 3]]| t.iter().map(|[a, b, c]| signed_area(&[*a, *b, *c])).sum::<f64>();
        let top_area = area(&faces.top);
        if !close(top_area, area(&faces.bottom), AREA_MATCH_REL)
            || (rim.is_none() && !close(top_area * height, mesh_volume, VOLUME_MATCH_REL))
        {
            return Err(NOT_CLOSED.into());
        }
    }
    Ok(())
}

/// The wall index over the outline's sides and the cut-outs' walls, as `check_walls` checks
/// them: on a rounded rim the top outline is the inner rectangle, which lies inside the rounded
/// box, so its segments must not count as walls — only the cut-outs' walls stay.
fn walls_of(corners: &[P2], radius: Option<f64>, cutouts: &PlateCutouts, scale: f64, tol: f64, rim: Option<&Rim>) -> Walls {
    let (wall_corners, wall_radius, wall_loops): (&[P2], Option<f64>, &[Vec<P2>]) =
        if rim.is_some() { (&[], None, &cutouts.wall_loops[1..]) } else { (corners, radius, &cutouts.wall_loops[..]) };
    Walls::new(wall_corners, wall_radius, &cutouts.circles, &cutouts.slot_arcs, wall_loops, &[], &slot_sides_of(cutouts), scale, tol)
}

/// Each slot's two straight sides as segments — the slot's index in the sorted `slots` (the
/// design's `slot_<n>`) and one side's start and end — so their walls can be indexed like the
/// cut-outs'.
fn slot_sides_of(cutouts: &PlateCutouts) -> Vec<(usize, P2, P2)> {
    let mut out = Vec::new();
    for (i, s) in cutouts.slots.iter().enumerate() {
        let (sin, cos) = s.angle.to_radians().sin_cos();
        let (u, v) = ([cos, sin], [-sin, cos]);
        let (half, r) = (s.len / 2.0, s.w / 2.0);
        let at = |sgn: f64, off: f64| [s.x + sgn * half * u[0] + off * v[0], s.y + sgn * half * u[1] + off * v[1]];
        out.push((i, at(1.0, -r), at(-1.0, -r)));
        out.push((i, at(1.0, r), at(-1.0, r)));
    }
    out
}

/// The mesh's per-face surface deviation from the recognised plate: each wall piece's label with
/// the farthest its sampled mesh points lie from it (a facet chord of a round wall bows inward by
/// its sagitta; planar faces lie exactly on theirs). Empty when the mesh is no Z prism, or when
/// none of its walls is indexed (a box rounded all over, whose rounded surfaces are not
/// measured). Best effort: any pipeline error yields no report rather than failing the import.
pub(crate) fn face_deviations(mesh: &Mesh) -> Vec<(String, f64)> {
    let run = || -> Result<Vec<(String, f64)>, String> {
        let bounds = mesh.bounds().ok_or("the mesh has no triangles")?;
        let size = bounds.size();
        let scale = size[0].max(size[1]).max(size[2]);
        let tol = tolerance(scale);
        if size.iter().any(|s| *s <= tol) {
            return Err("the mesh is flat".into());
        }
        let faces = split_faces(mesh, bounds.min, bounds.max[2], tol)?;
        let steps = step_levels(&faces, size[2], tol)?;
        let face_loops = face_outlines(&faces, &steps, tol)?;
        let (corners, radius) = outline_shape(&face_loops.outline, tol)?;
        let rim = rim_fillet(&face_loops.outline, &corners, radius, size, tol);
        let cutouts = pair_cutouts(&faces, &face_loops, size[2], tol)?;
        // Against the ideal walls only, not the meshed boundary loops (a facet's own chord would
        // measure zero): outline sides and corner rounds, the holes' circles, the slots' arcs
        // and straight sides, the cut-outs' straight sides.
        let (wall_corners, wall_radius): (&[P2], Option<f64>) = if rim.is_some() { (&[], None) } else { (&corners, radius) };
        let walls = Walls::new(
            wall_corners,
            wall_radius,
            &cutouts.circles,
            &cutouts.slot_arcs,
            &[],
            &cutouts.cutouts,
            &slot_sides_of(&cutouts),
            scale,
            tol,
        );
        // No wall of the part is indexed (a box rounded all over): no deviation to report.
        if walls.pieces.is_empty() {
            return Ok(Vec::new());
        }
        let mut worst: std::collections::BTreeMap<String, f64> = std::collections::BTreeMap::new();
        let mut note = |label: &str, d: f64| {
            let e = worst.entry(label.to_owned()).or_insert(0.0);
            if d > *e {
                *e = d;
            }
        };
        // The faces themselves lie in their planes by construction.
        note("top face", 0.0);
        note("bottom face", 0.0);
        // Wall corners and chord middles: their distance to the nearest ideal wall. `faces`
        // keeps every point relative to the mesh's lowest corner, as the walls are.
        // Samples of unsupported features (blind or stepped holes, whose surfaces are not
        // indexed) are excluded rather than attributed to distant outer walls.
        let in_unsupported = |p: Vec3| cutouts.stepped.iter().any(|s| s.holds(p, tol)) || cutouts.blinds.iter().any(|b| b.holds(p, tol));
        let footprint_unsupported = |q: P2| {
            cutouts.stepped.iter().any(|s| distance(q, s.centre) <= s.head_r + tol)
                || cutouts.blinds.iter().any(|b| b.foot.contains(q, tol))
        };
        for p in &faces.between {
            if in_unsupported(*p) {
                continue;
            }
            if let Some((d, k)) = walls.deviation([p[0], p[1]], &[]) {
                note(&walls.labels[k], d);
            }
        }
        for p in &faces.rim {
            if let Some((d, k)) = walls.deviation(*p, &[]) {
                note(&walls.labels[k], d);
            }
        }
        let mut note_chord = |a: P2, b: P2, middle: P2| {
            // A chord of an unsupported feature (a blind or stepped hole's bore or mouth) has
            // no indexed surface: skip it rather than attributing it to a distant outer wall.
            if footprint_unsupported(a) || footprint_unsupported(middle) {
                return;
            }
            for p in [a, b] {
                if let Some((d, k)) = walls.deviation(p, &[]) {
                    note(&walls.labels[k], d);
                }
            }
            // The chord's middle is measured against the wall owning its ends (a coarse round
            // wall's middle lies several cells inside its circle, so the wall is found through
            // the ends), not against a neighbouring feature that happens to lie nearer.
            if let Some((d, k)) = walls.deviation_chord(a, b, middle) {
                note(&walls.labels[k], d);
            }
        };
        for &(a, b, middle, _) in &faces.chords {
            note_chord(a, b, [middle[0], middle[1]]);
        }
        // The top boundary loops' chords too: `split_faces` skips edges lying wholly on the top
        // face, so a coarsely tessellated top rim's chords would otherwise not be sampled.
        for ring in std::iter::once(&face_loops.outline).chain(face_loops.loops.iter()) {
            for i in 0..ring.len() {
                let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
                if distance(a, b) > tol {
                    note_chord(a, b, [f64::midpoint(a[0], b[0]), f64::midpoint(a[1], b[1])]);
                }
            }
        }
        Ok(worst.into_iter().collect())
    };
    run().unwrap_or_default()
}

/// The facets and corners between the faces must belong to the walls or to a stepped or blind
/// hole: anywhere else they are a step, a pocket, a boss, or a wall that leans (a sheared prism
/// has the same areas and volume as a straight one).
pub(super) fn check_walls(
    faces: &Faces,
    corners: &[P2],
    radius: Option<f64>,
    cutouts: &PlateCutouts,
    scale: f64,
    tol: f64,
    rim: Option<&Rim>,
) -> Result<(), String> {
    let on_rim = |p: Vec3, slack: f64| rim.is_some_and(|rim| rim.holds(p, tol, slack));
    // The sagitta a facet's own longest edge bows the rounded surface inward by.
    let sagitta_of = |cap: &[Vec3; 3]| {
        let edge = |a: Vec3, b: Vec3| ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
        let l = edge(cap[0], cap[1]).max(edge(cap[1], cap[2])).max(edge(cap[2], cap[0]));
        rim.map(|rim| rim.sagitta(l))
    };
    // A facet facing up or down between the faces must belong to a counterbore's floor, a
    // countersink's cone or the rounded rim; anywhere else it is a floor or a ledge (of a blind
    // hole, a step or a pocket), even when its corners lie on a hole's circle.
    let cap_on_rim = |cap: &[Vec3; 3]| {
        cap.iter().all(|p| on_rim(*p, 0.0))
            && sagitta_of(cap).is_some_and(|s| {
                let middle = [
                    (cap[0][0] + cap[1][0] + cap[2][0]) / 3.0,
                    (cap[0][1] + cap[1][1] + cap[2][1]) / 3.0,
                    (cap[0][2] + cap[1][2] + cap[2][2]) / 3.0,
                ];
                on_rim(middle, CHORD_FACET_SPAN * s)
            })
    };
    if !faces.caps.iter().all(|cap| {
        cutouts.stepped.iter().any(|s| s.holds_cap(cap, tol)) || cutouts.blinds.iter().any(|b| b.holds_cap(cap, tol)) || cap_on_rim(cap)
    }) {
        return Err(NOT_FLAT.into());
    }
    let walls = walls_of(corners, radius, cutouts, scale, tol, rim);
    let in_step = |p: Vec3| cutouts.stepped.iter().any(|s| s.holds(p, tol)) || cutouts.blinds.iter().any(|b| b.holds(p, tol));
    if faces.between.iter().any(|p| !walls.holds([p[0], p[1]]) && !in_step(*p) && !on_rim(*p, 0.0)) {
        return Err(NOT_FLAT.into());
    }
    // A wall facet's edge in a stepped hole's mouth must follow the hole's surface too: its
    // middle within the sagitta of a chord two facets long, or the facet cuts across the bore.
    // (Pockets facing each other share a footprint: the edge must follow one of them.) On the
    // rounded rim the middle may bow inward by its chord's sagitta, as on any round wall.
    let follows = |&(a, b, middle, len): &(P2, P2, Vec3, f64)| {
        let mut steps = cutouts.stepped.iter().filter(|s| s.reaches(a, tol) && s.reaches(b, tol)).peekable();
        let mut blind = cutouts.blinds.iter().filter(|h| h.reaches(a, tol) && h.reaches(b, tol)).peekable();
        if steps.peek().is_some() {
            steps.any(|s| s.holds(middle, tol + CHORD_FACET_SPAN * s.sagitta()))
        } else if blind.peek().is_some() {
            // A step's ring shares edges with the plate's outline, so an outline wall's chord
            // can reach the step's corners too: it still follows the outline wall.
            blind.any(|h| h.holds(middle, tol + CHORD_FACET_SPAN * h.sagitta())) || walls.holds_chord(a, b)
        } else {
            walls.holds_chord(a, b) || rim.is_some_and(|rim| on_rim(middle, CHORD_FACET_SPAN * rim.sagitta(len)))
        }
    };
    let on_blind_wall = |p: &P2| cutouts.blinds.iter().any(|b| b.foot.on_wall(*p, &b.ring, tol));
    // A wall's bottom corner on a rounded rim lies on the bottom cap, which is part of the
    // rounded surface (at z = 0 every point of the inner rectangle is at distance `r`).
    if faces.rim.iter().any(|p| !walls.holds(*p) && !on_rim([p[0], p[1], 0.0], 0.0) && !in_step([p[0], p[1], 0.0]) && !on_blind_wall(p))
        || !faces.chords.iter().all(follows)
    {
        return Err("some side walls are not vertical (drafts, chamfers and slanted walls are not recognised yet)".into());
    }
    Ok(())
}

/// The recognition from the outline's corners: a rectangular outline without cut-outs becomes a
/// [`PlateSpec`], any other outline (or one with cut-outs) a [`ProfileSpec`]; the rebuilt part
/// must match the mesh's volume within [`MAX_VOLUME_ERROR`].
pub(super) fn recognition(
    corners: &[P2],
    radius: Option<f64>,
    cutouts: PlateCutouts,
    size: Vec3,
    offset: Vec3,
    mesh_volume: f64,
    rim: Option<&Rim>,
) -> Result<Recognition, String> {
    let tol = tolerance(size[0].max(size[1]).max(size[2]));
    // On a rounded rim the outline's own corner arcs are the leaked rim facets, not a fillet.
    let (z, outline_fillet_r) = (snap_mm(size[2]), if rim.is_some() { None } else { radius.map(snap_mm) });
    let box_corners = [[0.0, 0.0], [size[0], 0.0], [size[0], size[1]], [0.0, size[1]]];
    let PlateCutouts { mut holes, slots, cutouts, pockets, outline_pockets, .. } = cutouts;
    // A through hole in a pocket's or step's region goes only up to that level, not through the
    // whole plate: from the bottom face it is blind, as deep as the level lies above it. A blind
    // hole already measured beneath the region keeps its own depth. A hole whose counterbore,
    // countersink or round opens on the level would be drilled from the plate's top face, not
    // from the level, so it is not recognised: the mesh is kept.
    for h in &mut holes {
        if let Some(p) = outline_pockets.iter().find(|p| !p.bottom && p.contains([h.x, h.y], tol)) {
            if h.head.is_some_and(|head| !head.bottom()) {
                return Err("a hole with a counterbore, countersink or round opening on a step's level is not recognised yet".into());
            }
            if h.blind.is_none() {
                h.blind = Some(Blind { depth: snap_mm(size[2] - p.depth), bottom: true });
            }
        }
    }
    // A rectangle with cut-outs is an outline, as mesh2cad writes it (`extrude_len`, the
    // outline's corners and each cut-out's). The rectangle is the box itself, or — when the rim is
    // rounded — the top face's inner rectangle inset by the radius (rim_fillet checked the shape).
    let matches = |want: &[[f64; 2]; 4]| {
        corners.len() == 4 && want.iter().all(|q| corners.iter().any(|c| (c[0] - q[0]).abs() <= tol && (c[1] - q[1]).abs() <= tol))
    };
    // A pocket of any outline but a rectangle also turns the plate into a profile: its sketch
    // is the pocket's, not the rectangle's.
    let rectangle = cutouts.is_empty() && outline_pockets.is_empty() && corners.len() == 4 && (matches(&box_corners) || rim.is_some());
    if rim.is_some() && !rectangle {
        return Err("a box with all edges rounded and cut-outs is not recognised yet".into());
    }
    let (found, volume) = if rectangle {
        let plate = PlateSpec {
            x: snap_mm(size[0]),
            y: snap_mm(size[1]),
            z,
            holes,
            slots,
            pads: vec![],
            pockets,
            bosses: vec![],
            cutouts,
            outline_fillet_r,
            edge_fillet_r: rim.map(|rim| snap_mm(rim.r)),
        };
        let volume = plate.volume();
        (Recognition::Plate { plate, offset }, volume)
    } else {
        let mut corners: Vec<P2> = corners.iter().map(|c| c.map(snap_mm)).collect();
        // Start at the lowest corner, the leftmost of those, so that line l1 is predictable.
        let first =
            (0..corners.len()).min_by(|&a, &b| corners[a][1].total_cmp(&corners[b][1]).then(corners[a][0].total_cmp(&corners[b][0])));
        corners.rotate_left(first.unwrap_or(0));
        let profile =
            ProfileSpec { corners, z, holes, slots, pads: vec![], pockets, outline_pockets, bosses: vec![], cutouts, outline_fillet_r };
        let volume = profile.volume();
        // The pipeline runs in a frame where the extrusion axis is Z.
        (Recognition::Profile { profile, offset, axis: crate::recognize::Axis::Z }, volume)
    };
    let error = (volume - mesh_volume) / mesh_volume;
    if error.abs() > MAX_VOLUME_ERROR {
        return Err(format!("the rebuilt plate would differ from the mesh by {:.1}% in volume", error.abs() * 100.0));
    }
    Ok(found)
}
