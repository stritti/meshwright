//! Recognising the prism itself: the pipeline over the faces' loops — outline, cut-outs,
//! prism and wall checks, and the recognition from the outline's corners.

use super::faces::{Faces, split_faces};
use super::geometry::{boundary_loops, distance, signed_area};
use super::holes::{BlindHole, Cutouts, FittedHole, FittedSlot, Stepped, axis_rectangle, polygon_cutout, round_hole, slot_shape};
use super::outline::outline_shape;
use super::walls::Walls;
use super::{AREA_MATCH_REL, CHORD_FACET_SPAN, MAX_VOLUME_ERROR, NOT_CLOSED, NOT_FLAT, P2, VOLUME_MATCH_REL, tolerance};
use crate::mesh::{Mesh, Vec3, snap_mm};
use crate::recognize::{Blind, HoleSpec, PlateSpec, PocketSpec, ProfileSpec, Recognition, SlotSpec};

use super::holes::Footprint;
/// Looks for a plate in `mesh`: a prism over its top face, with holes, slots, cut-outs, pockets
/// and pads through or on it.
pub(super) fn recognize_prism(mesh: &Mesh) -> Result<Recognition, String> {
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
    let face_loops = face_outlines(&faces, tol)?;
    let (corners, radius) = outline_shape(&face_loops.outline, tol)?;
    let cutouts = pair_cutouts(&faces, &face_loops, size[2], tol)?;
    check_prism(&faces, mesh_volume, size[2], &cutouts.stepped, &cutouts.blinds)?;
    check_walls(&faces, &corners, radius, &cutouts, scale, tol)?;
    let offset = bounds.min.map(snap_mm);
    recognition(&corners, radius, cutouts, size, offset, tol, mesh_volume)
}

/// Whether `a` and `b` are within `rel` of each other, relative to the larger one.
pub(super) fn close(a: f64, b: f64, rel: f64) -> bool {
    (a - b).abs() <= rel * a.abs().max(b.abs())
}

/// The two faces' boundary loops, and the top face's single outline among them.
pub(super) struct FaceLoops {
    /// The top face's single counter-clockwise outline.
    pub(super) outline: Vec<P2>,
    /// The top face's boundary loops: the outline and the cut-outs'.
    pub(super) loops: Vec<Vec<P2>>,
    /// The bottom face's boundary loops.
    pub(super) bottom_loops: Vec<Vec<P2>>,
}

/// The faces' boundary loops and the top face's single outline among them, checked against the
/// bottom face's outline.
pub(super) fn face_outlines(faces: &Faces, tol: f64) -> Result<FaceLoops, String> {
    let loops = boundary_loops(&faces.top, tol)?;
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
    /// The through holes' circles, as fitted.
    pub(super) circles: Vec<(P2, f64)>,
    /// The loops the walls stand on, as meshed: the outline and the cut-outs'.
    pub(super) wall_loops: Vec<Vec<P2>>,
    /// The slots' two end arcs (centre, radius, corner, reach), bounded round walls the wall
    /// check uses too.
    pub(super) slot_arcs: Vec<(P2, f64, P2, f64)>,
    /// Every hole, sorted from left to right.
    pub(super) holes: Vec<HoleSpec>,
    /// The straight slots, in mesh2cad's order.
    pub(super) slots: Vec<SlotSpec>,
    /// The polygonal cut-outs, each counter-clockwise from its lowest corner, the leftmost of
    /// those, sorted by that corner.
    pub(super) cutouts: Vec<Vec<P2>>,
    /// The rectangular pockets, sorted from left to right.
    pub(super) pockets: Vec<PocketSpec>,
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

/// The polygonal cut-outs. A cut-out on both faces goes through; a rectangle along X and Y on
/// one face only is a pocket's mouth, its floor found with the blind holes'. Returns the
/// pockets' mouths. Each through cut-out is written counter-clockwise from its lowest corner,
/// the leftmost of those.
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
        let (lo, hi) = axis_rectangle(ring, tol)
            .ok_or("a cut-out does not go straight through (pockets other than rectangles and slanted cut-outs are not recognised yet)")?;
        Ok(BlindHole { foot: Footprint::Rect { lo, hi }, ring: ring.to_vec(), face_z, inward, depth: 0.0 })
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
    for (spec, g, k) in top {
        out.slot_arcs.extend(g.halves());
        out.wall_loops.push(face_loops.loops[*k].clone());
        out.slots.push(spec.clone());
    }
    out.slots.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)).then(a.angle.total_cmp(&b.angle)).then(a.w.total_cmp(&b.w)));
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
            out.circles.push((*centre, *top_r));
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
        match (hole, b.foot) {
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
            (None, Footprint::Round { .. }) => unreachable!("a round mouth is a hole's"),
        }
        out.blinds.push(b);
    }
    out.holes.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    out.pockets.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    Ok(())
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
    let mut s = Stepped { centre, bore_r, head_r, bore, mouth, face_z, inward, depth: 0.0, counterbore: false };
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
/// changes either by far more.
pub(super) fn check_prism(faces: &Faces, mesh_volume: f64, height: f64, stepped: &[Stepped], blinds: &[BlindHole]) -> Result<(), String> {
    if stepped.is_empty() && blinds.is_empty() {
        let area = |t: &[[P2; 3]]| t.iter().map(|[a, b, c]| signed_area(&[*a, *b, *c])).sum::<f64>();
        let top_area = area(&faces.top);
        if !close(top_area, area(&faces.bottom), AREA_MATCH_REL) || !close(top_area * height, mesh_volume, VOLUME_MATCH_REL) {
            return Err(NOT_CLOSED.into());
        }
    }
    Ok(())
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
) -> Result<(), String> {
    // A facet facing up or down between the faces must belong to a counterbore's floor or a
    // countersink's cone; anywhere else it is a floor or a ledge (of a blind hole, a step or a
    // pocket), even when its corners lie on a hole's circle.
    if !faces
        .caps
        .iter()
        .all(|cap| cutouts.stepped.iter().any(|s| s.holds_cap(cap, tol)) || cutouts.blinds.iter().any(|b| b.holds_cap(cap, tol)))
    {
        return Err(NOT_FLAT.into());
    }
    let walls = Walls::new(corners, radius, &cutouts.circles, &cutouts.slot_arcs, &cutouts.wall_loops, scale, tol);
    let in_step = |p: Vec3| cutouts.stepped.iter().any(|s| s.holds(p, tol)) || cutouts.blinds.iter().any(|b| b.holds(p, tol));
    if faces.between.iter().any(|p| !walls.holds([p[0], p[1]]) && !in_step(*p)) {
        return Err(NOT_FLAT.into());
    }
    // A wall facet's edge in a stepped hole's mouth must follow the hole's surface too: its
    // middle within the sagitta of a chord two facets long, or the facet cuts across the bore.
    // (Pockets facing each other share a footprint: the edge must follow one of them.)
    let follows = |&(a, b, middle): &(P2, P2, Vec3)| {
        let mut steps = cutouts.stepped.iter().filter(|s| s.reaches(a, tol) && s.reaches(b, tol)).peekable();
        let mut blind = cutouts.blinds.iter().filter(|h| h.reaches(a, tol) && h.reaches(b, tol)).peekable();
        if steps.peek().is_some() {
            steps.any(|s| s.holds(middle, tol + CHORD_FACET_SPAN * s.sagitta()))
        } else if blind.peek().is_some() {
            blind.any(|h| h.holds(middle, tol + CHORD_FACET_SPAN * h.sagitta()))
        } else {
            walls.holds_chord(a, b)
        }
    };
    let on_blind_wall = |p: &P2| cutouts.blinds.iter().any(|b| b.foot.on_wall(*p, &b.ring, tol));
    if faces.rim.iter().any(|p| !walls.holds(*p) && !in_step([p[0], p[1], 0.0]) && !on_blind_wall(p)) || !faces.chords.iter().all(follows) {
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
    tol: f64,
    mesh_volume: f64,
) -> Result<Recognition, String> {
    let (z, outline_fillet_r) = (snap_mm(size[2]), radius.map(snap_mm));
    let box_corners = [[0.0, 0.0], [size[0], 0.0], [size[0], size[1]], [0.0, size[1]]];
    let PlateCutouts { holes, slots, cutouts, pockets, .. } = cutouts;
    // A rectangle with cut-outs is an outline, as mesh2cad writes it (`extrude_len`, the
    // outline's corners and each cut-out's).
    let rectangle = cutouts.is_empty()
        && corners.len() == 4
        && box_corners.iter().all(|q| corners.iter().any(|c| (c[0] - q[0]).abs() <= tol && (c[1] - q[1]).abs() <= tol));
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
        };
        let volume = plate.volume();
        (Recognition::Plate { plate, offset }, volume)
    } else {
        let mut corners: Vec<P2> = corners.iter().map(|c| c.map(snap_mm)).collect();
        // Start at the lowest corner, the leftmost of those, so that line l1 is predictable.
        let first =
            (0..corners.len()).min_by(|&a, &b| corners[a][1].total_cmp(&corners[b][1]).then(corners[a][0].total_cmp(&corners[b][0])));
        corners.rotate_left(first.unwrap_or(0));
        let profile = ProfileSpec { corners, z, holes, slots, pads: vec![], pockets, bosses: vec![], cutouts, outline_fillet_r };
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
