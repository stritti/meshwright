//! Recognising a plate: an outline of straight sides, optionally with one radius on every
//! vertical corner, extruded along Z, with round vertical through holes.
//!
//! Written independently from mesh2cad's published description (a Z extrusion with one radius on
//! every vertical corner of the outline, through holes, the parameter names `base_x`,
//! `hole_<n>_d` and `outline_fillet_r`); no mesh2cad code was used for it.
//!
//! The mesh is taken as exported by a CAD tool: corners of the true part's facets lie on its
//! surfaces, so a round hole is a polygon whose corners lie on a circle. The steps are:
//!
//! 1. Every corner lies on the bottom or the top face, every side wall is vertical and the top
//!    and bottom faces have the same area as the volume demands: the mesh is a straight prism.
//! 2. The boundary of the top face is split into closed loops. The one loop running
//!    counter-clockwise is the outline, the clockwise ones are holes.
//! 3. The outline must be made of straight sides, its corners all sharp or all rounded with the
//!    same radius (a rounded corner is a run of facet corners on one circle that meets the sides
//!    on either side of it tangentially); each hole must be round. A rectangular outline becomes
//!    a [`PlateSpec`], any other a [`ProfileSpec`].
//! 4. Each hole meets the top and the bottom face in circles about one centre. Where they
//!    differ, the wider one is the mouth of a counterbore (corners on its wide wall down to a
//!    flat floor) or a countersink (a cone down to where the bore's corners begin); every corner
//!    and every facet that faces up or down between the faces must then lie on that surface.
//! 5. The plate rebuilt from those numbers must have the mesh's volume within 1%, the check
//!    mesh2cad documents for its scripts; otherwise the mesh is kept.

#![allow(clippy::many_single_char_names)] // points a, b, c, sizes x, y, z and radii r, as in the geometry

use std::collections::{BTreeMap, HashMap};
use std::f64::consts::PI;

use crate::mesh::{Mesh, Vec3, cross, length, sub};
use crate::recognize::{HoleHead, HoleSpec, PlateSpec, ProfileSpec, Recognition};

/// Largest relative difference between the rebuilt plate's volume and the mesh's.
pub const MAX_VOLUME_ERROR: f64 = 0.01;

/// Fewest corners a polygon needs to be taken for a round hole. Six would turn hexagonal nut
/// traps into round holes.
pub const MIN_HOLE_SEGMENTS: usize = 8;

/// Largest share of its circle one facet of a round hole may span (50°): eight facets are the
/// fewest, and the flat side of a D-shaped cut-out spans far more.
const MAX_HOLE_STEP: f64 = 50.0 * PI / 180.0;

/// Largest step round a hole's rim, as a multiple of the median step.
const MAX_STEP_RATIO: f64 = 2.5;

/// Largest turn between two facets of a rounded corner (50°): a quarter circle needs at least two
/// facets, and a hexagon's corners (60°) stay corners.
const MAX_FACET_TURN: f64 = 50.0 * PI / 180.0;

/// Largest turn between facets of a curved side (30°): finer than a twelve-sided outline.
const MAX_CURVE_TURN: f64 = 30.0 * PI / 180.0;

/// Recognised sizes are rounded to micrometres (this many steps per millimetre): STL stores
/// single-precision floats, so a 35 mm hole position comes back as 35.0000019…, which nobody
/// wants in the parameter panel.
const SNAP_PER_MM: f64 = 1000.0;

/// How far a corner may lie from where the recogniser expects it, in millimetres, for a part
/// whose largest size is `scale`. STL coordinates have about seven significant digits.
fn tolerance(scale: f64) -> f64 {
    (scale * 1e-5).max(1e-3)
}

fn snap(value: f64) -> f64 {
    let snapped = (value * SNAP_PER_MM).round() / SNAP_PER_MM;
    if snapped == 0.0 { 0.0 } else { snapped }
}

type P2 = [f64; 2];

const NOT_FLAT: &str = "it is not a flat plate: some corners lie between its top and bottom faces \
                        (steps, pockets, bosses and blind holes are not recognised yet)";

/// A hole's rounded parameters, its circle as fitted, and the index of its loop.
type FittedHole = (HoleSpec, (P2, f64), usize);

/// What [`split_faces`] sorts a mesh into, in coordinates relative to its lowest corner.
struct Faces {
    /// The top face's triangles, seen from above.
    top: Vec<[P2; 3]>,
    /// The bottom face's triangles, seen from above (counter-clockwise, as the top's).
    bottom: Vec<[P2; 3]>,
    /// Every corner between the top and the bottom face.
    between: Vec<Vec3>,
    /// Facets between the faces that are no walls (they face up or down): the floor of a
    /// counterbore, the cone of a countersink, or a step or pocket.
    caps: Vec<[Vec3; 3]>,
    /// The bottom corners of the walls.
    rim: Vec<P2>,
    /// The wall facets' edges that do not run along the top face.
    chords: Vec<(P2, P2)>,
}

/// A hole whose mouth is wider than its bore, at the top or the bottom face, in coordinates
/// relative to the plate's lowest corner.
#[derive(Debug, Clone, Copy)]
struct Stepped {
    centre: P2,
    bore_r: f64,
    head_r: f64,
    /// The face the mouth opens on, and which way is into the plate from it (-1 down from the
    /// top, +1 up from the bottom).
    face_z: f64,
    inward: f64,
    /// How deep the mouth goes: to a counterbore's floor, or to where a countersink's cone meets
    /// the bore.
    depth: f64,
    counterbore: bool,
}

impl Stepped {
    fn reaches(&self, p: P2, tol: f64) -> bool {
        distance(p, self.centre) <= self.head_r + tol
    }

    /// Whether `p` lies on the hole's surface: its bore, its counterbore's wall or floor, or its
    /// countersink's cone.
    fn holds(&self, p: Vec3, tol: f64) -> bool {
        let r = distance([p[0], p[1]], self.centre);
        let depth = (p[2] - self.face_z) * self.inward;
        if (r - self.bore_r).abs() <= tol && depth >= -tol {
            return true;
        }
        if depth < -tol || depth > self.depth + tol {
            return false;
        }
        if self.counterbore {
            (r - self.head_r).abs() <= tol || ((depth - self.depth).abs() <= tol && r >= self.bore_r - tol && r <= self.head_r + tol)
        } else {
            let cone = self.head_r - depth * (self.head_r - self.bore_r) / self.depth;
            (r - cone).abs() <= tol
        }
    }

    /// Whether the facet `cap` (facing up or down) is part of the mouth: a piece of the
    /// counterbore's annular floor, or a facet of the countersink's cone. Its corners lying on
    /// the hole is not enough: a disk across the bore has them all on the bore's circle.
    fn holds_cap(&self, cap: &[Vec3; 3], tol: f64) -> bool {
        let depth = |p: &Vec3| (p[2] - self.face_z) * self.inward;
        let centroid = [(cap[0][0] + cap[1][0] + cap[2][0]) / 3.0, (cap[0][1] + cap[1][1] + cap[2][1]) / 3.0];
        let r = distance(centroid, self.centre);
        if self.counterbore {
            cap.iter().all(|p| (depth(p) - self.depth).abs() <= tol) && r >= self.bore_r - tol && r <= self.head_r + tol
        } else {
            let (low, high) = cap.iter().map(depth).fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), d| (lo.min(d), hi.max(d)));
            cap.iter().all(|p| self.holds(*p, tol) && depth(p) <= self.depth + tol) && high - low > tol
        }
    }

    fn head(&self) -> HoleHead {
        let bottom = self.inward > 0.0;
        let d = snap(2.0 * self.head_r);
        if self.counterbore {
            HoleHead::Counterbore { d, depth: snap(self.depth), bottom }
        } else {
            let angle = 2.0 * ((self.head_r - self.bore_r) / self.depth).atan().to_degrees();
            // A whole degree when that moves the cone's edge by no more than a micrometre, else
            // a hundredth of one.
            let whole = angle.round();
            let angle = if ((whole - angle).to_radians() * self.depth).abs() <= 1e-3 { whole } else { (angle * 100.0).round() / 100.0 };
            HoleHead::Countersink { d, angle, bottom }
        }
    }
}

/// Looks for a plate in `mesh`. `Err` carries the reason in the user's words.
#[allow(clippy::too_many_lines)] // one recognition, its checks in order
pub(crate) fn recognize_plate(mesh: &Mesh) -> Result<Recognition, String> {
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
    let close = |a: f64, b: f64, rel: f64| (a - b).abs() <= rel * a.abs().max(b.abs());
    let loops = boundary_loops(&faces.top, tol)?;
    let bottom_loops = boundary_loops(&faces.bottom, tol)?;
    let mut outlines = loops.iter().filter(|l| signed_area(l) > 0.0);
    let outline = outlines.next().ok_or("the top face has no outline")?;
    if outlines.next().is_some() {
        return Err("the top face has more than one outline (several separate parts are not recognised yet)".into());
    }
    let bottom_outline = bottom_loops.iter().find(|l| signed_area(l) > 0.0).ok_or("the bottom face has no outline")?;
    // An exporter may mesh the top and bottom faces apart, with their own corners on round
    // edges: their outlines must match to 0.01%. A draft changes that by far more.
    if !close(signed_area(outline), signed_area(bottom_outline), 1e-4) {
        return Err("the mesh is not closed, or its top and bottom faces differ".into());
    }
    let (corners, radius) = outline_shape(outline, tol)?;

    // Each hole meets the top and the bottom face in a circle about one centre (the circles as
    // fitted; the parameters are rounded). Where the two differ, the wider one is the mouth of a
    // counterbore or a countersink.
    let fit = |loops: &[Vec<P2>]| -> Result<Vec<FittedHole>, String> {
        loops.iter().enumerate().filter(|(_, l)| signed_area(l) < 0.0).map(|(k, l)| round_hole(l, tol).map(|(h, c)| (h, c, k))).collect()
    };
    let (tops, bottoms) = (fit(&loops)?, fit(&bottom_loops)?);
    if tops.len() != bottoms.len() {
        return Err("a hole does not go through (blind holes are not recognised yet)".into());
    }
    let mut circles = Vec::new();
    let mut wall_loops = vec![outline.clone()];
    let mut holes = Vec::new();
    let mut stepped = Vec::new();
    for (hole, (centre, top_r), loop_index) in &tops {
        let (_, (bottom_centre, bottom_r), _) = bottoms
            .iter()
            .find(|(_, (c, _), _)| distance(*c, *centre) <= tol)
            .ok_or("a hole does not go straight through (slanted and blind holes are not recognised yet)")?;
        if (top_r - bottom_r).abs() <= tol {
            circles.push((*centre, *top_r));
            wall_loops.push(loops[*loop_index].clone());
            holes.push(hole.clone());
            continue;
        }
        let centre = [f64::midpoint(centre[0], bottom_centre[0]), f64::midpoint(centre[1], bottom_centre[1])];
        let on_top = top_r > bottom_r;
        let (head_r, bore_r) = if on_top { (*top_r, *bottom_r) } else { (*bottom_r, *top_r) };
        let (face_z, inward) = if on_top { (size[2], -1.0) } else { (0.0, 1.0) };
        let mut s = Stepped { centre, bore_r, head_r, face_z, inward, depth: 0.0, counterbore: false };
        // Its corners between the faces tell the kind: a counterbore has some on its wide wall,
        // down to its floor; a countersink's cone meets the bore at the bore's corners nearest
        // the face.
        let inside = |p: &Vec3| p[2] > tol && p[2] < size[2] - tol;
        let mine: Vec<Vec3> = faces
            .between
            .iter()
            .chain(faces.caps.iter().flatten())
            .copied()
            .filter(|p| inside(p) && s.reaches([p[0], p[1]], tol))
            .collect();
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
        if s.depth <= tol || s.depth >= size[2] - tol {
            return Err("a hole's counterbore or countersink is as deep as the plate (stepped holes are not recognised yet)".into());
        }
        holes.push(HoleSpec { x: snap(centre[0]), y: snap(centre[1]), d: snap(2.0 * bore_r), head: Some(s.head()) });
        stepped.push(s);
    }
    holes.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));

    // Without stepped holes the mesh must be a prism over its top face: the two faces match, and
    // walls meshed in bands bulge only a little between the top face's corners (0.1%). A step
    // changes either by far more.
    if stepped.is_empty() {
        let area = |t: &[[P2; 3]]| t.iter().map(|[a, b, c]| signed_area(&[*a, *b, *c])).sum::<f64>();
        let top_area = area(&faces.top);
        if !close(top_area, area(&faces.bottom), 1e-4) || !close(top_area * size[2], mesh_volume, 1e-3) {
            return Err("the mesh is not closed, or its top and bottom faces differ".into());
        }
    }
    // A facet facing up or down between the faces must belong to a counterbore's floor or a
    // countersink's cone; anywhere else it is a floor or a ledge (of a blind hole, a step or a
    // pocket), even when its corners lie on a hole's circle.
    if !faces.caps.iter().all(|cap| stepped.iter().any(|s| s.holds_cap(cap, tol))) {
        return Err(NOT_FLAT.into());
    }
    // Corners between the top and the bottom face, and the bottom ends of the walls, must lie on
    // the walls the top face's boundary stands for, or on a stepped hole's surface: anywhere else
    // they are a step, a pocket, a boss, or a wall that leans (a sheared prism has the same areas
    // and volume as a straight one).
    let walls = Walls::new(&corners, radius, &circles, &wall_loops, scale, tol);
    let in_step = |p: Vec3| stepped.iter().any(|s| s.holds(p, tol));
    if faces.between.iter().any(|p| !walls.holds([p[0], p[1]]) && !in_step(*p)) {
        return Err(NOT_FLAT.into());
    }
    let reaches_step = |p: P2| stepped.iter().any(|s| s.reaches(p, tol));
    if faces.rim.iter().any(|p| !walls.holds(*p) && !in_step([p[0], p[1], 0.0]))
        || faces.chords.iter().any(|&(a, b)| (!reaches_step(a) || !reaches_step(b)) && !walls.holds_chord(a, b))
    {
        return Err("some side walls are not vertical (drafts, chamfers and slanted walls are not recognised yet)".into());
    }

    let offset = bounds.min.map(|c| if c.abs() <= tol { 0.0 } else { snap(c) });
    let (z, outline_fillet_r) = (snap(size[2]), radius.map(snap));
    let box_corners = [[0.0, 0.0], [size[0], 0.0], [size[0], size[1]], [0.0, size[1]]];
    let rectangle =
        corners.len() == 4 && box_corners.iter().all(|q| corners.iter().any(|c| (c[0] - q[0]).abs() <= tol && (c[1] - q[1]).abs() <= tol));
    let (found, volume) = if rectangle {
        let plate = PlateSpec { x: snap(size[0]), y: snap(size[1]), z, holes, outline_fillet_r };
        let volume = plate.volume();
        (Recognition::Plate { plate, offset }, volume)
    } else {
        let mut corners: Vec<P2> = corners.iter().map(|c| c.map(snap)).collect();
        // Start at the lowest corner, the leftmost of those, so that line l1 is predictable.
        let first =
            (0..corners.len()).min_by(|&a, &b| corners[a][1].total_cmp(&corners[b][1]).then(corners[a][0].total_cmp(&corners[b][0])));
        corners.rotate_left(first.unwrap_or(0));
        let profile = ProfileSpec { corners, z, holes, outline_fillet_r };
        let volume = profile.volume();
        (Recognition::Profile { profile, offset }, volume)
    };
    let error = (volume - mesh_volume) / mesh_volume;
    if error.abs() > MAX_VOLUME_ERROR {
        return Err(format!("the rebuilt plate would differ from the mesh by {:.1}% in volume", error.abs() * 100.0));
    }
    Ok(found)
}

/// Sorts the triangles into the top face (returned in 2D, relative to `min`), the bottom face
/// (only its area is needed) and vertical walls, and refuses anything else. Also returns every
/// corner between the top and the bottom (in 2D, relative to `min`).
///
/// A wall may have corners between the top and the bottom: some exporters (SolidCraft's own
/// among them) mesh a tall wall in several bands, whose facets lean a little as their corners
/// lie on a curved wall at different heights and angles. The caller checks that those corners
/// lie on the walls; anywhere else they are a step, a pocket or a boss.
fn split_faces(mesh: &Mesh, min: Vec3, top_z: f64, tol: f64) -> Result<Faces, String> {
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
                chords.push(if a < b { (a, b) } else { (b, a) });
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
            if !sliver && nz.abs() > 0.5 {
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
        p.0[0].total_cmp(&q.0[0]).then(p.0[1].total_cmp(&q.0[1])).then(p.1[0].total_cmp(&q.1[0])).then(p.1[1].total_cmp(&q.1[1]))
    });
    chords.dedup();
    Ok(Faces { top, bottom, between, caps, rim, chords })
}

/// One piece of a plate's walls, seen from above.
enum Wall {
    /// A straight wall, or a facet of the top face's boundary as meshed.
    Segment(P2, P2),
    /// A rounded corner: on the circle about `centre` of radius `r`, no farther from the sharp
    /// `corner` than its tangent points (`t`); the rest of the circle lies farther away.
    Arc { centre: P2, r: f64, corner: P2, t: f64 },
    /// A hole's bore, as fitted (not as its rounded parameters).
    Circle(P2, f64),
}

/// The walls of a recognised plate, indexed on a grid so that checking a corner against them
/// costs the same however many corners and holes the plate has.
struct Walls {
    pieces: Vec<Wall>,
    cells: HashMap<(i64, i64), Vec<usize>>,
    cell: f64,
    tol: f64,
}

impl Walls {
    /// The walls of the outline `corners` (rounded with `radius`), of the holes' `circles` and,
    /// as meshed, of the top face's boundary `loops` (a facet's chord of an arc is no wall of the
    /// true part, but its corners and its subdivisions lie on it).
    fn new(corners: &[P2], radius: Option<f64>, circles: &[(P2, f64)], loops: &[Vec<P2>], size: f64, tol: f64) -> Self {
        let n = corners.len();
        let r = radius.unwrap_or(0.0);
        let unit = |p: P2, q: P2| {
            let len = distance(p, q);
            [(q[0] - p[0]) / len, (q[1] - p[1]) / len]
        };
        // Per corner: how far its arc's tangent points lie from it along the sides (0 when
        // sharp), and the arc's centre (inside the turn, `r` from both sides).
        let rounds: Vec<(f64, P2)> = (0..n)
            .map(|i| {
                let (a, c, b) = (corners[(i + n - 1) % n], corners[i], corners[(i + 1) % n]);
                let (d1, d2) = (unit(a, c), unit(c, b));
                let turn = (d1[0] * d2[1] - d1[1] * d2[0]).atan2(d1[0] * d2[0] + d1[1] * d2[1]);
                let t = r * (turn.abs() / 2.0).tan();
                let start = [c[0] - d1[0] * t, c[1] - d1[1] * t];
                let inward = r * turn.signum();
                (t, [start[0] - d1[1] * inward, start[1] + d1[0] * inward])
            })
            .collect();
        let mut walls = Vec::new();
        for i in 0..n {
            let (c, b) = (corners[i], corners[(i + 1) % n]);
            let d = unit(c, b);
            // The side's straight part, between the tangent points of its two corners' arcs.
            let (t0, t1) = (rounds[i].0, rounds[(i + 1) % n].0);
            walls.push(Wall::Segment([c[0] + d[0] * t0, c[1] + d[1] * t0], [b[0] - d[0] * t1, b[1] - d[1] * t1]));
            if r > 0.0 {
                walls.push(Wall::Arc { centre: rounds[i].1, r, corner: c, t: rounds[i].0 });
            }
        }
        walls.extend(circles.iter().map(|&(c, r)| Wall::Circle(c, r)));
        for ring in loops {
            walls.extend((0..ring.len()).map(|i| Wall::Segment(ring[i], ring[(i + 1) % ring.len()])));
        }
        // About 256 cells across the plate, never finer than a few tolerances.
        let cell = (size / 256.0).max(4.0 * tol);
        // Each wall goes into the cells along it, not into every cell of its bounding box (a long
        // diagonal side or a large circle would fill thousands): points on it half a cell apart,
        // each with the cells around it, cover every cell within a tolerance of it.
        let mut cells: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
        for (k, wall) in walls.iter().enumerate() {
            #[allow(clippy::cast_sign_loss)] // counts of half cells along a wall, never negative
            let samples: Vec<P2> = match *wall {
                Wall::Segment(a, b) => {
                    let steps = (distance(a, b) / (0.5 * cell)).ceil().max(1.0) as usize;
                    (0..=steps).map(|s| s as f64 / steps as f64).map(|t| [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])]).collect()
                }
                Wall::Arc { centre, r, .. } | Wall::Circle(centre, r) => {
                    let steps = ((2.0 * PI * r) / (0.5 * cell)).ceil().max(8.0) as usize;
                    (0..steps)
                        .map(|s| 2.0 * PI * s as f64 / steps as f64)
                        .map(|a| [centre[0] + r * a.cos(), centre[1] + r * a.sin()])
                        .collect()
                }
            };
            for p in samples {
                let (x, y) = ((p[0] / cell).floor() as i64, (p[1] / cell).floor() as i64);
                for key in
                    [(x - 1, y - 1), (x, y - 1), (x + 1, y - 1), (x - 1, y), (x, y), (x + 1, y), (x - 1, y + 1), (x, y + 1), (x + 1, y + 1)]
                {
                    let list = cells.entry(key).or_default();
                    if list.last() != Some(&k) {
                        list.push(k);
                    }
                }
            }
        }
        Self { pieces: walls, cells, cell, tol }
    }

    /// Whether the straight edge from `a` to `b` (both on the walls, relative to the plate's
    /// corner) follows a wall: its middle lies on a side or the meshed boundary, or within its
    /// own sagitta of a circle (a chord of a round wall bows inward by `len² / 8r`).
    fn holds_chord(&self, a: P2, b: P2) -> bool {
        let p = [f64::midpoint(a[0], b[0]), f64::midpoint(a[1], b[1])];
        let half2 = distance(a, b).powi(2) / 4.0;
        let tol = self.tol;
        // The middle of a chord lies `r - sqrt(r² - (len/2)²)` inside its circle, maybe cells away
        // from it; the walls are looked up at the chord's ends, which lie on them.
        let on_circle = |centre: P2, r: f64| r * r > half2 && (distance(p, centre) - (r * r - half2).sqrt()).abs() <= tol;
        let follows = |k: usize| match self.pieces[k] {
            Wall::Segment(s, e) => segment_distance(p, s, e) <= tol,
            Wall::Arc { centre, r, corner, t } => on_circle(centre, r) && distance(p, corner) <= t + tol,
            Wall::Circle(centre, r) => on_circle(centre, r),
        };
        [a, b, p].iter().any(|q| {
            let key = ((q[0] / self.cell).floor() as i64, (q[1] / self.cell).floor() as i64);
            self.cells.get(&key).is_some_and(|candidates| candidates.iter().any(|&k| follows(k)))
        })
    }

    /// Whether `p` (relative to the plate's corner) lies on one of the walls.
    fn holds(&self, p: P2) -> bool {
        let key = ((p[0] / self.cell).floor() as i64, (p[1] / self.cell).floor() as i64);
        let tol = self.tol;
        self.cells.get(&key).is_some_and(|candidates| {
            candidates.iter().any(|&k| match self.pieces[k] {
                Wall::Segment(a, b) => segment_distance(p, a, b) <= tol,
                Wall::Arc { centre, r, corner, t } => (distance(p, centre) - r).abs() <= tol && distance(p, corner) <= t + tol,
                Wall::Circle(centre, r) => (distance(p, centre) - r).abs() <= tol,
            })
        })
    }
}

/// The distance from `p` to the line through `a` and `b`.
fn line_distance(p: P2, a: P2, b: P2) -> f64 {
    let (u, w) = ([b[0] - a[0], b[1] - a[1]], [p[0] - a[0], p[1] - a[1]]);
    (u[0] * w[1] - u[1] * w[0]).abs() / u[0].hypot(u[1])
}

/// The distance from `p` to the segment from `a` to `b`.
fn segment_distance(p: P2, a: P2, b: P2) -> f64 {
    let (u, w) = ([b[0] - a[0], b[1] - a[1]], [p[0] - a[0], p[1] - a[1]]);
    let len2 = u[0] * u[0] + u[1] * u[1];
    let t = if len2 == 0.0 { 0.0 } else { ((w[0] * u[0] + w[1] * u[1]) / len2).clamp(0.0, 1.0) };
    distance(p, [a[0] + t * u[0], a[1] + t * u[1]])
}

/// The closed boundary loops of a set of counter-clockwise triangles: outlines run
/// counter-clockwise, holes clockwise.
fn boundary_loops(triangles: &[[P2; 3]], tol: f64) -> Result<Vec<Vec<P2>>, String> {
    // Corners that are the same point in the file can differ in the last bit after the offset
    // was subtracted, so they are welded on a grid well below the tolerance.
    let grid = tol / 16.0;
    let mut ids: HashMap<(i64, i64), usize> = HashMap::new();
    let mut points: Vec<P2> = Vec::new();
    let mut id = |p: P2| {
        let key = ((p[0] / grid).round() as i64, (p[1] / grid).round() as i64);
        *ids.entry(key).or_insert_with(|| {
            points.push(p);
            points.len() - 1
        })
    };
    let mut edges: BTreeMap<(usize, usize), usize> = BTreeMap::new();
    for t in triangles {
        let [a, b, c] = t.map(&mut id);
        if a == b || b == c || c == a {
            continue;
        }
        for edge in [(a, b), (b, c), (c, a)] {
            *edges.entry(edge).or_default() += 1;
        }
    }
    let mut next: BTreeMap<usize, usize> = BTreeMap::new();
    for (&(a, b), &count) in &edges {
        if count > 1 {
            return Err("the top face overlaps itself".into());
        }
        if !edges.contains_key(&(b, a)) && next.insert(a, b).is_some() {
            return Err("the top face touches itself at a corner".into());
        }
    }
    let mut loops = Vec::new();
    while let Some((&start, _)) = next.first_key_value() {
        let mut current = start;
        let mut ring = Vec::new();
        while let Some(to) = next.remove(&current) {
            ring.push(points[current]);
            current = to;
        }
        if current != start || ring.len() < 3 {
            return Err("the top face has an open edge (the mesh has a gap)".into());
        }
        loops.push(ring);
    }
    Ok(loops)
}

/// The shoelace area of a closed polygon, positive when counter-clockwise.
fn signed_area(ring: &[P2]) -> f64 {
    let n = ring.len();
    (0..n)
        .map(|i| {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            a[0] * b[1] - b[0] * a[1]
        })
        .sum::<f64>()
        / 2.0
}

/// Reads `outline` (counter-clockwise) as a polygon of straight sides whose corners are all
/// sharp or all rounded with one radius. Returns the corners as if they were sharp, and the
/// radius.
#[allow(clippy::too_many_lines)] // one scan, read best in one piece
fn outline_shape(outline: &[P2], tol: f64) -> Result<(Vec<P2>, Option<f64>), String> {
    let mut v = merge_collinear(outline);
    let n = v.len();
    if n < 3 {
        return Err("the outline is degenerate".into());
    }
    // Start right after the longest edge: it is a straight side, so no rounded corner runs across
    // the start of the list.
    let longest = (0..n).max_by(|&a, &b| distance(v[a], v[(a + 1) % n]).total_cmp(&distance(v[b], v[(b + 1) % n]))).unwrap_or(0);
    v.rotate_left((longest + 1) % n);
    let len = |i: usize| distance(v[i % n], v[(i + 1) % n]);
    let turns: Vec<f64> = (0..n)
        .map(|i| {
            let (a, b, c) = (v[(i + n - 1) % n], v[i], v[(i + 1) % n]);
            let (u, w) = ([b[0] - a[0], b[1] - a[1]], [c[0] - b[0], c[1] - b[1]]);
            (u[0] * w[1] - u[1] * w[0]).atan2(u[0] * w[0] + u[1] * w[1])
        })
        .collect();
    let turn = |i: usize| turns[i % n];
    // Facet `j` carries on a run of facets that started with facet `first`: about as long, and
    // turning the same small way as the run's first turn, at corner `first + 1`.
    let carries_on = |first: usize, j: usize| {
        (len(j) - len(first)).abs() <= 0.25 * len(first) && turn(j).abs() <= MAX_FACET_TURN && turn(j) * turn(first + 1) > 0.0
    };

    // A rounded corner is a run of facets of about equal length that each turn a little the
    // same way, whose corners lie on one circle (fitted to all of them, which averages out the
    // noise of single-precision files) and which meets the sides on either side tangentially.
    // It can only start where a side ends, never inside a run, so a long curve is scanned once.
    let mut arcs: Vec<(usize, usize, f64)> = Vec::new(); // first and last facet corner, radius
    let mut i = 0;
    while i + 2 < n {
        if i > 0 && carries_on(i - 1, i) && turn(i).abs() >= 0.75 * turn(i + 1).abs() {
            i += 1;
            continue;
        }
        let before = v[(i + n - 1) % n];
        let mut fit = CircleSums::new(v[i]);
        fit.add(v[i]);
        fit.add(v[i + 1]);
        let mut found = None;
        let mut end = i + 1;
        // Some exporters (SolidCraft among them) start or end an arc with a facet of another
        // length; a facet that turns the same small way and whose far corner keeps the run's
        // first and last corners on one circle carries the run on too.
        let on_fit = |fit: &CircleSums, end: usize| {
            let same_way = turn(end).abs() <= MAX_FACET_TURN && (end == i + 1 || turn(end) * turn(i + 1) > 0.0) && turn(i + 1) != 0.0;
            same_way && {
                let mut with = fit.clone();
                with.add(v[(end + 1) % n]);
                with.circle()
                    .is_some_and(|(centre, r)| [v[i], v[end], v[(end + 1) % n]].iter().all(|p| (distance(*p, centre) - r).abs() <= tol))
            }
        };
        while end + 1 < n && (carries_on(i, end) || on_fit(&fit, end)) {
            end += 1;
            fit.add(v[end]);
            let Some((centre, r)) = fit.circle() else { continue };
            // Each side is held to its own length: a chord of the circle turns from the tangent by
            // half its own angle, a tangent side (however short) by nothing. Measured against a
            // neighbouring facet instead, a shorter facet of the arc itself could pass for a side.
            // The angle alone allows a long side to miss the circle by more than the tolerance:
            // the circle must also touch each side's line, its centre `r` from it.
            let touches = |a: P2, b: P2| (line_distance(centre, a, b) - r).abs() <= tol;
            if tangent(before, v[i], centre, len(i + n - 1) / (2.0 * r))
                && tangent(v[(end + 1) % n], v[end], centre, len(end) / (2.0 * r))
                && touches(before, v[i])
                && touches(v[end], v[(end + 1) % n])
                && v[i..=end].iter().all(|p| (distance(*p, centre) - r).abs() <= tol)
            {
                found = Some((end, r));
                break;
            }
        }
        if let Some((last, r)) = found {
            arcs.push((i, last, r));
            i = last;
        } else {
            // A run of three or more facets on one circle that met no tangent side is a curve
            // (or a round outline), not scanned again from inside it, so a long curve is scanned
            // once however uneven its facets. An arc can only start inside the run where a side
            // meets the circle tangentially: the scan goes on at the first such corner, or after
            // the run when it has none.
            let on_one_circle = |(centre, r): (P2, f64)| v[i..=end].iter().all(|p| (distance(*p, centre) - r).abs() <= tol);
            // A curve's facets are of a size; a long straight side and the first facets of a
            // small round also fit one (huge) circle within the tolerance, and must not be
            // skipped.
            let (short, long) = (i..end).map(len).fold((f64::INFINITY, 0.0_f64), |(lo, hi), l| (lo.min(l), hi.max(l)));
            i = match fit.circle() {
                Some((centre, r)) if end > i + 2 && long <= 4.0 * short && on_one_circle((centre, r)) => {
                    (i + 1..end).find(|&j| tangent(v[j - 1], v[j], centre, len(j - 1) / (2.0 * r))).unwrap_or(end)
                }
                _ => i + 1,
            };
        }
    }
    let mut on_arc = vec![false; n];
    for &(first, last, _) in &arcs {
        on_arc[first..=last].iter_mut().for_each(|c| *c = true);
    }

    // Four or more short facets in a row, of one length and each turning a little the same way,
    // that are no rounded corner make a curved side (or a round outline).
    let mut run = 0;
    for j in 1..=n {
        // Facets of uneven length count too, as some exporters space a curve's corners unevenly.
        let gentle = !on_arc[j % n]
            && !on_arc[j - 1]
            && turn(j).abs() <= MAX_CURVE_TURN
            && turn(j) * turn(j - 1) > 0.0
            && (carries_on(j - 1, j) || len(j) <= 3.0 * len(j - 1) && len(j - 1) <= 3.0 * len(j));
        run = if gentle { run + 1 } else { 0 };
        if run >= 3 {
            return Err("the outline has curved sides (only straight sides and rounded corners are recognised so far)".into());
        }
    }

    // A rounded corner becomes the point where its two sides meet; sharp corners stay.
    let mut corners = Vec::new();
    let mut k = 0;
    while k < n {
        if let Some(&(first, last, _)) = arcs.iter().find(|a| a.0 == k) {
            let corner = intersect(v[(first + n - 1) % n], v[first], v[last], v[(last + 1) % n])
                .ok_or("a corner is rounded all the way round (slot-shaped ends are not recognised yet)")?;
            corners.push(corner);
            k = last + 1;
        } else {
            corners.push(v[k]);
            k += 1;
        }
    }
    if corners.len() < 3 {
        return Err("the outline is degenerate".into());
    }
    if arcs.is_empty() {
        return Ok((corners, None));
    }
    if arcs.len() != corners.len() {
        return Err("only some corners of the outline are rounded (mixed corners are not recognised yet)".into());
    }
    let (low, high) = arcs.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), a| (lo.min(a.2), hi.max(a.2)));
    if high - low > tol {
        return Err("the corners are rounded with different radii (mixed radii are not recognised yet)".into());
    }
    Ok((corners, Some(arcs.iter().map(|a| a.2).sum::<f64>() / arcs.len() as f64)))
}

/// Running sums for a least-squares circle (Kåsa's algebraic fit) through points added one at a
/// time, around a nearby origin to keep the sums small.
#[derive(Clone)]
struct CircleSums {
    origin: P2,
    n: f64,
    x: f64,
    y: f64,
    xx: f64,
    xy: f64,
    yy: f64,
    xz: f64,
    yz: f64,
    z: f64,
}

impl CircleSums {
    fn new(origin: P2) -> Self {
        Self { origin, n: 0.0, x: 0.0, y: 0.0, xx: 0.0, xy: 0.0, yy: 0.0, xz: 0.0, yz: 0.0, z: 0.0 }
    }

    fn add(&mut self, p: P2) {
        let (x, y) = (p[0] - self.origin[0], p[1] - self.origin[1]);
        let z = x * x + y * y;
        self.n += 1.0;
        self.x += x;
        self.y += y;
        self.xx += x * x;
        self.xy += x * y;
        self.yy += y * y;
        self.xz += x * z;
        self.yz += y * z;
        self.z += z;
    }

    /// The circle x² + y² + D x + E y + F = 0 closest to the points, as centre and radius.
    fn circle(&self) -> Option<(P2, f64)> {
        // Around the points' mean the system splits: F follows from D and E.
        let (mx, my) = (self.x / self.n, self.y / self.n);
        let (sxx, sxy, syy) = (self.xx - self.n * mx * mx, self.xy - self.n * mx * my, self.yy - self.n * my * my);
        let (rx, ry) = (self.xz - mx * self.z, self.yz - my * self.z);
        let det = sxx * syy - sxy * sxy;
        if det.abs() <= f64::EPSILON * (sxx * syy).max(f64::MIN_POSITIVE) {
            return None;
        }
        let d = (-rx * syy + ry * sxy) / det;
        let e = (-ry * sxx + rx * sxy) / det;
        let f = -(self.z + d * self.x + e * self.y) / self.n;
        let (cx, cy) = (-d / 2.0, -e / 2.0);
        let r2 = cx * cx + cy * cy - f;
        (r2 > 0.0).then(|| ([self.origin[0] + cx, self.origin[1] + cy], r2.sqrt()))
    }
}

/// Drops facet corners that lie on a straight line between their neighbours.
fn merge_collinear(ring: &[P2]) -> Vec<P2> {
    let n = ring.len();
    (0..n)
        .filter(|&i| {
            let (a, b, c) = (ring[(i + n - 1) % n], ring[i], ring[(i + 1) % n]);
            let (u, w) = ([b[0] - a[0], b[1] - a[1]], [c[0] - b[0], c[1] - b[1]]);
            let sin = (u[0] * w[1] - u[1] * w[0]) / (u[0].hypot(u[1]) * w[0].hypot(w[1]));
            !(sin.abs() <= 1e-4 && u[0] * w[0] + u[1] * w[1] > 0.0)
        })
        .map(|i| ring[i])
        .collect()
}

fn distance(a: P2, b: P2) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

/// Whether the side from `from` to `at` touches the circle around `centre` at `at`. A facet of
/// the circle turns away from the tangent by half its angle, whose sine is `sin_half`; half of
/// that is the allowance, so an arc's own corners are never taken for its ends.
fn tangent(from: P2, at: P2, centre: P2, sin_half: f64) -> bool {
    let side = [at[0] - from[0], at[1] - from[1]];
    let radius = [at[0] - centre[0], at[1] - centre[1]];
    let cos = (side[0] * radius[0] + side[1] * radius[1]) / (side[0].hypot(side[1]) * radius[0].hypot(radius[1]));
    cos.abs() <= (0.5 * sin_half).min(0.05)
}

/// Where the line through `a` and `b` meets the line through `c` and `d`.
fn intersect(a: P2, b: P2, c: P2, d: P2) -> Option<P2> {
    let (u, w) = ([b[0] - a[0], b[1] - a[1]], [d[0] - c[0], d[1] - c[1]]);
    let den = u[0] * w[1] - u[1] * w[0];
    if den.abs() <= 1e-9 * u[0].hypot(u[1]) * w[0].hypot(w[1]) {
        return None;
    }
    let t = ((c[0] - a[0]) * w[1] - (c[1] - a[1]) * w[0]) / den;
    Some([a[0] + t * u[0], a[1] + t * u[1]])
}

/// Fits a circle to a hole's corners and checks that they all lie on it.
fn round_hole(ring: &[P2], tol: f64) -> Result<(HoleSpec, (P2, f64)), String> {
    if ring.len() < MIN_HOLE_SEGMENTS {
        return Err(format!(
            "a cut-out has only {} corners, so it is taken for a polygon, not a round hole (polygonal cut-outs are not recognised yet)",
            ring.len()
        ));
    }
    let not_round = || "a cut-out is not round (slots, D-shapes and other cut-outs are not recognised yet)".to_owned();
    let ([cx, cy], r) = fit_circle(ring).ok_or_else(not_round)?;
    if ring.iter().any(|p| ((p[0] - cx).hypot(p[1] - cy) - r).abs() > tol) {
        return Err(not_round());
    }
    // Corners on a circle are not enough: a D-shaped cut-out has them too. Its facets must go
    // once round the circle in small, even steps; a flat side is one long step.
    let n = ring.len();
    let mut steps: Vec<f64> = (0..n)
        .map(|i| {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            let step = (b[1] - cy).atan2(b[0] - cx) - (a[1] - cy).atan2(a[0] - cx);
            (step + PI).rem_euclid(2.0 * PI) - PI
        })
        .collect();
    let around: f64 = steps.iter().sum();
    for step in &mut steps {
        *step = step.abs();
    }
    let mut sorted = steps.clone();
    sorted.sort_by(f64::total_cmp);
    let median = sorted[n / 2];
    // Some exporters (SolidCraft among them) space a rim's corners unevenly, a step up to about
    // twice another; a flat side is several steps long.
    if (around.abs() - 2.0 * PI).abs() > 1e-6 || steps.iter().any(|s| *s > MAX_HOLE_STEP || *s > MAX_STEP_RATIO * median) {
        return Err(not_round());
    }
    Ok((HoleSpec { x: snap(cx), y: snap(cy), d: snap(2.0 * r), head: None }, ([cx, cy], r)))
}

/// The least-squares circle through `points` (Kåsa's algebraic fit), as centre and radius.
#[allow(clippy::many_single_char_names)] // the textbook symbols
fn fit_circle(points: &[P2]) -> Option<(P2, f64)> {
    let n = points.len() as f64;
    let mean = [points.iter().map(|p| p[0]).sum::<f64>() / n, points.iter().map(|p| p[1]).sum::<f64>() / n];
    // In coordinates around the mean, solve for x² + y² + D x + E y + F = 0.
    let (mut sxx, mut sxy, mut syy, mut sxz, mut syz, mut sz) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    for p in points {
        let (u, v) = (p[0] - mean[0], p[1] - mean[1]);
        let z = u * u + v * v;
        sxx += u * u;
        sxy += u * v;
        syy += v * v;
        sxz += u * z;
        syz += v * z;
        sz += z;
    }
    // Around the mean Σu = Σv = 0, so F = -Σz / n and D, E follow from a 2 by 2 system.
    let det = sxx * syy - sxy * sxy;
    if det.abs() <= f64::EPSILON * (sxx * syy).max(f64::MIN_POSITIVE) {
        return None;
    }
    let d = (-sxz * syy + syz * sxy) / det;
    let e = (-syz * sxx + sxz * sxy) / det;
    let (u, v) = (-d / 2.0, -e / 2.0);
    let r2 = u * u + v * v + sz / n;
    (r2 > 0.0).then(|| ([mean[0] + u, mean[1] + v], r2.sqrt()))
}

/// The area the four corners of a rectangle lose to roundings of radius `r`.
pub(crate) fn corner_loss(r: f64) -> f64 {
    (4.0 - PI) * r * r
}

/// The area the corner `c` of a counter-clockwise outline (coming from `a`, going on to `b`)
/// loses to a rounding of radius `r`; negative for an inside corner, which gains it.
pub(crate) fn fillet_loss(a: P2, c: P2, b: P2, r: f64) -> f64 {
    let (u, w) = ([c[0] - a[0], c[1] - a[1]], [b[0] - c[0], b[1] - c[1]]);
    let turn = (u[0] * w[1] - u[1] * w[0]).atan2(u[0] * w[0] + u[1] * w[1]);
    turn.signum() * r * r * ((turn.abs() / 2.0).tan() - turn.abs() / 2.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recognize::recognize;
    use crate::synth;

    fn spec(holes: &[(f64, f64, f64)], r: Option<f64>) -> PlateSpec {
        PlateSpec {
            x: 80.0,
            y: 50.0,
            z: 5.0,
            holes: holes.iter().map(|&(x, y, d)| HoleSpec { x, y, d, head: None }).collect(),
            outline_fillet_r: r,
        }
    }

    /// An L: 60 along X and 15 deep, with a 15 wide leg up to 40, one hole in each arm.
    fn ell(r: Option<f64>) -> ProfileSpec {
        ProfileSpec {
            corners: vec![[0.0, 0.0], [60.0, 0.0], [60.0, 15.0], [15.0, 15.0], [15.0, 40.0], [0.0, 40.0]],
            z: 5.0,
            holes: vec![HoleSpec { x: 7.5, y: 30.0, d: 5.0, head: None }, HoleSpec { x: 45.0, y: 7.5, d: 6.0, head: None }],
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
            holes: vec![HoleSpec { x: 25.0, y: 15.0, d: 8.0, head: None }],
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
        assert_eq!(profile, ProfileSpec { corners, z: 5.0, holes: vec![], outline_fillet_r: None });
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

    #[test]
    fn hexagonal_cut_out_is_not_a_hole() {
        assert!(reason(&synth::plate(&spec(&[(40.0, 25.0, 10.0)], None), 6, [0.0; 3])).contains("6 corners"));
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
    /// corners: every corner between the faces lies on the hole's circle, but the floor is no wall.
    #[test]
    fn a_blind_hole_is_no_through_hole() {
        let (z, floor) = (5.0, 2.5);
        let outline = [[0.0, 0.0], [200.0, 0.0], [200.0, 200.0], [0.0, 200.0]];
        let ring = synth::rings(&[HoleSpec { x: 100.0, y: 100.0, d: 2.0, head: None }], 16).remove(0);
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
        let why = reason(&mesh);
        assert!(why.contains("not a flat plate") || why.contains("does not go through"), "{why}");
    }

    /// A banded wall corner moved out to where the sharp corner would be is no wall: straight
    /// sides end at their arcs' tangent points.
    #[test]
    #[allow(clippy::float_cmp)] // the same corner, as the generator wrote it into every facet
    fn a_corner_out_of_its_arc_is_no_wall() {
        let spec = PlateSpec { x: 80.0, y: 50.0, z: 5.0, holes: vec![], outline_fillet_r: Some(1.0) };
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
        let ring = synth::rings(&[HoleSpec { x: 40.0, y: 25.0, d: 8.0, head: None }], 32);
        let leaning_hole = sheared(&outline, &ring, 5.0, [0.0; 2], [0.5, 0.0]);
        let why = reason(&leaning_hole);
        assert!(why.contains("not vertical") || why.contains("does not go straight through"), "{why}");
        let leaning_block = sheared(&outline, &[], 5.0, [0.5, 0.0], [0.0; 2]);
        let why = reason(&leaning_block);
        assert!(why.contains("not vertical") || why.contains("faces differ"), "{why}");
    }

    /// A banded wall whose bottom band is shifted: the band corners lie on the wall, the bottom
    /// ones do not.
    #[test]
    #[allow(clippy::float_cmp)] // corners compared as the generator wrote them
    fn a_shifted_bottom_band_is_not_vertical() {
        let spec = PlateSpec { x: 80.0, y: 50.0, z: 5.0, holes: vec![], outline_fillet_r: None };
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
        let walls = Walls::new(&diamond, None, &[], &[], 100.0, 1e-3);
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
        let expected = PlateSpec { x: 80.0, y: 50.0, z: 5.0, holes: vec![], outline_fillet_r: Some(0.5) };
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
        let expected = PlateSpec { x: 80.0, y: 50.0, z: 5.0, holes: vec![], outline_fillet_r: Some(3.0) };
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
        let expected = PlateSpec { x: 80.0, y: 50.0, z: 5.0, holes: vec![], outline_fillet_r: Some(24.95) };
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

    /// A 60 x 40 x 8 plate exported from SolidCraft 0.45 with a counterbore and a countersink on
    /// the top face and a counterbore on the bottom face.
    #[test]
    fn recognises_counterbores_and_countersinks() {
        let mesh = crate::stl::parse(include_bytes!("../../../plugin/meshwright/tests/stepped-holes.stl")).expect("parses");
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
        let mut mesh = crate::stl::parse(include_bytes!("../../../plugin/meshwright/tests/stepped-holes.stl")).expect("parses");
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
        let mut mesh = crate::stl::parse(include_bytes!("../../../plugin/meshwright/tests/stepped-holes.stl")).expect("parses");
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

    /// An L-bracket exported from SolidCraft 0.45 (`file.export_stl`, standard quality): walls in
    /// bands, top and bottom faces meshed apart, arcs and rims of uneven facets.
    #[test]
    fn recognises_solidcraft_exports() {
        let mesh = crate::stl::parse(include_bytes!("../../../plugin/meshwright/tests/solidcraft-l-bracket.stl")).expect("parses");
        let Recognition::Profile { profile, offset } = recognize(&mesh) else { panic!("not recognised: {:?}", recognize(&mesh)) };
        assert_eq!(offset, [20.0, 10.0, 0.0]);
        assert_eq!(profile.corners, [[0.0, 0.0], [60.0, 0.0], [60.0, 15.0], [15.0, 15.0], [15.0, 40.0], [0.0, 40.0]]);
        assert_eq!(profile.holes, [HoleSpec { x: 7.5, y: 30.0, d: 5.0, head: None }, HoleSpec { x: 45.0, y: 7.5, d: 6.0, head: None }]);
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
}
