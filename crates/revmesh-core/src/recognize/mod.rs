//! Feature recognition: finding the parametric part a mesh was made from.
//!
//! revmesh follows mesh2cad's approach and parameter names, one family of parts at a time.
//! Recognised so far: prisms along a principal axis — an outline of straight sides, optionally
//! with one radius on every corner parallel to the axis, with round holes — and boxes with all
//! twelve edges rounded with one radius (`edge_fillet_r`); see [`crate::plate`].
//! A rectangular outline extruded along Z becomes a [`PlateSpec`], driven by its sizes; any
//! other outline, and any outline extruded along X or Y, a [`ProfileSpec`]. Slots, counterbores
//! and pockets follow; whatever is not recognised stays a mesh.
//!
//! Like mesh2cad, which tries a prism along X, Y and Z, [`recognize`] tries each axis: the mesh
//! is rotated so the axis becomes Z (see [`Axis`]), the Z pipeline runs unchanged, and the result
//! is rotated back. Z is tried first, so an axis-aligned box keeps the `base_*` plate mesh2cad's
//! box vocabulary writes for it.
//!
//! When no prism fits, a part may still be the intersection of outlines extruded along two or
//! three axes (a [`CombinedSpec`], see `crate::combined`), as mesh2cad's boolean of extrusions
//! falls back to.

mod axis;
mod combined;
mod spec;
mod stack;
#[cfg(test)]
pub(crate) mod tests;
mod to_design;

pub use axis::Axis;
pub use combined::{CombinedHole, CombinedSpec, Cut, Extrusion};
pub use spec::{
    ArcSide, Blind, BossChamfer, BossSpec, CutoutSpec, HoleHead, HoleSpec, OutlinePocketSpec, PadSpec, PlateSpec, PocketSpec, ProfileSpec,
    SlotSpec,
};
pub use stack::{Band, StackSpec};

use std::collections::HashMap;

use crate::mesh::{Mesh, Vec3, cross, dot, length, sub};
use crate::plate::{WELD_GRID_TOL, tolerance};

/// The per-face surface deviation report for a recognised part (roadmap 5.4): a plate's or a
/// profile's walls measured in the Z frame (another axis's profile is rotated into it), a
/// combined part's per extrusion ("extrude z side 1", "extrude z cut 1 side 2", "extrude z hole
/// 1"), a stack's per band ("band 2 side 1"). Empty for an unrecognised mesh.
pub(crate) fn deviations(mesh: &Mesh, found: &Recognition) -> Vec<(String, f64)> {
    match found {
        Recognition::Plate { .. } => crate::plate::face_deviations(mesh),
        Recognition::Profile { axis, .. } => crate::plate::face_deviations(&axis.rotate_mesh(mesh)),
        Recognition::Combined { spec, .. } => {
            // The combined spec's frame: the bounding box's corner at the origin in world
            // coordinates, then rotated per axis (along Y the profile's second coordinate is
            // −Z). An axis with no outline of its own (one that only a cut or a hole runs
            // along) gets the box's side as the outline, so the cut's and hole's walls are found.
            let bounds = mesh.bounds().expect("recognised meshes are not empty");
            let in_frame =
                |axis: Axis| Mesh::new(mesh.triangles.iter().map(|t| t.map(|p| axis.rotate(crate::mesh::sub(p, bounds.min)))).collect());
            let box_side = |axis: Axis| {
                let [u, v] = axis.profile_axes();
                let lo = if axis.profile_sign(1) < 0.0 { -spec.size[v] } else { 0.0 };
                vec![[0.0, lo], [spec.size[u], lo], [spec.size[u], lo + spec.size[v]], [0.0, lo + spec.size[v]]]
            };
            let mut axes: Vec<Axis> = spec.extrusions.iter().map(|e| e.axis).collect();
            for extra in spec.cuts.iter().map(|c| c.axis).chain(spec.holes.iter().map(|h| h.axis)) {
                if !axes.contains(&extra) {
                    axes.push(extra);
                }
            }
            let mut out = Vec::new();
            for axis in axes {
                let extrusion = spec.extrusions.iter().find(|e| e.axis == axis);
                let outline: Vec<[f64; 2]> = extrusion.map_or_else(|| box_side(axis), |e| e.outline.clone());
                // The design numbers `cutout_<n>_*` across all extrusions in spec order.
                let cutout_base: usize = spec.extrusions.iter().take_while(|e| e.axis != axis).map(|e| e.cutouts.len()).sum();
                let cutouts: Vec<CutoutSpec> = extrusion
                    .map_or_else(Vec::new, |e| e.cutouts.iter().map(|c| CutoutSpec { corners: c.clone(), arcs: Vec::new() }).collect());
                // The cut's design-wide number travels with it, so the report's `cut <n>` matches
                // the design's `cut_<n>_*` parameters across axes.
                let cuts: Vec<(usize, Vec<[f64; 2]>)> =
                    spec.cuts.iter().enumerate().filter(|(_, c)| c.axis == axis).map(|(i, c)| (i, c.outline.clone())).collect();
                let circles: Vec<(usize, [f64; 2], f64)> =
                    spec.holes.iter().enumerate().filter(|(_, h)| h.axis == axis).map(|(i, h)| (i, h.centre, h.d / 2.0)).collect();
                let prefix = format!("extrude {}", axis.letter().to_ascii_lowercase());
                out.extend(crate::plate::walls_deviations(
                    &in_frame(axis),
                    &outline,
                    &cutouts,
                    &cuts,
                    &circles,
                    cutout_base,
                    (0.0, spec.size[axis.index()]),
                    &prefix,
                ));
            }
            out
        }
        Recognition::Stack { spec, axis, .. } => {
            // The stack spec's frame: the mesh rotated into the axis's frame, rebased to that
            // frame's lowest corner (as `recognize_stack` does).
            let rotated = axis.rotate_mesh(mesh);
            let min = rotated.bounds().expect("recognised meshes are not empty").min;
            let in_frame = Mesh::new(rotated.triangles.iter().map(|t| t.map(|p| crate::mesh::sub(p, min))).collect());
            let mut out = Vec::new();
            for (k, band) in spec.bands.iter().enumerate() {
                let cutouts: Vec<CutoutSpec> = band.cutouts.iter().map(|c| CutoutSpec { corners: c.clone(), arcs: Vec::new() }).collect();
                // The design numbers `cutout_<n>_*` across all bands, from the lowest up.
                let cutout_base: usize = spec.bands.iter().take(k).map(|b| b.cutouts.len()).sum();
                let prefix = format!("band {}", k + 1);
                out.extend(crate::plate::walls_deviations(
                    &in_frame,
                    &band.outline,
                    &cutouts,
                    &[],
                    &[],
                    cutout_base,
                    (band.from, band.to),
                    &prefix,
                ));
            }
            out
        }
        Recognition::Unrecognized { .. } => Vec::new(),
    }
}

/// What recognition found.
#[derive(Debug, Clone, PartialEq)]
pub enum Recognition {
    /// A rectangular plate extruded along Z, possibly with holes and rounded corners. (A
    /// rectangle extruded along X or Y is a [`Recognition::Profile`], as mesh2cad writes it.)
    Plate {
        /// The plate, its corner at the origin.
        plate: PlateSpec,
        /// Where the plate's corner was in the file; the design puts it at the origin.
        offset: Vec3,
    },
    /// An outline of straight sides extruded along `axis`, possibly with holes and rounded
    /// corners.
    Profile {
        /// The profile, its bounding box's corner at the origin, in the axis's profile
        /// coordinates (the sketch coordinates on the axis's plane).
        profile: ProfileSpec,
        /// Where that corner was in the file; the design puts it at the origin.
        offset: Vec3,
        /// The axis the outline is extruded along.
        axis: Axis,
    },
    /// The intersection of outlines extruded along two or three axes.
    Combined {
        /// The part, its bounding box's corner at the origin.
        spec: CombinedSpec,
        /// Where that corner was in the file; the design puts it at the origin.
        offset: Vec3,
    },
    /// Outlines stacked along `axis`, each extruded over its own span, joined.
    Stack {
        /// The stack, in its axis's frame, its bounding box's corner at the origin there.
        spec: StackSpec,
        /// Where that corner is in the file (the design puts it at the origin).
        offset: Vec3,
        /// The axis the outlines are stacked along.
        axis: Axis,
    },
    /// Nothing the recognisers know; the mesh is kept as it is.
    Unrecognized {
        /// Why, in the user's words.
        reason: String,
    },
}

/// A rotated X or Y attempt holds a second copy of the mesh plus the face and wall indexes on
/// top of it while the original is still alive: measured at up to ~420 bytes per triangle peak
/// on a densely banded mesh (a closed cylinder with 128 circumferential segments and 1,300
/// axial bands — 333,056 triangles — peaked at 133 MiB, over the 128 MiB the manifest asks
/// for), which would trap the component before it can fall back to a mesh body. Above this
/// many triangles only Z is tried (it needs no copy), so a too-large part along X or Y is
/// imported as a mesh body instead.
const MAX_ROTATED_TRIANGLES: usize = 200_000;

/// A supporting line of one or more non-standard edges: a point on it, its canonical unit
/// direction, and the bad segments along it (merged where they overlap or touch). Coverage is
/// checked only inside the segments, so a gap between two of them — a stretch of the line with
/// no edge at all — is fine.
struct Line {
    pa: Vec3,
    u: Vec3,
    segments: Vec<(f64, f64)>,
    grid: f64,
}

/// Sweeps one line: every position inside a bad segment must be covered by exactly one facet in
/// each direction (a T-junction's sub-edges cover it; a fin's double coverage or an open edge's
/// none do not balance). `bucketed` holds every directed edge by its line key.
#[allow(clippy::too_many_lines, clippy::many_single_char_names)] // points a, b, p, q, as in the geometry
fn sweep_line(line: &Line, bucketed: &[([i64; 6], u32)], directed: &[(u32, u32)], coords: &[Vec3], grid: f64) -> Result<(), String> {
    let k = line_key_of(line);
    let (outer_lo, outer_hi) =
        line.segments.iter().copied().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), (a, b)| (lo.min(a), hi.max(b)));
    // Gather the directed edges on this line: the buckets within one step of the key.
    let mut events: Vec<(f64, i8, i8)> = Vec::new();
    for du in [-1i64, 0, 1] {
        for dv in [-1, 0, 1] {
            for dw in [-1, 0, 1] {
                for dc in [-1i64, 0, 1] {
                    for dd in [-1, 0, 1] {
                        for de in [-1, 0, 1] {
                            let nk = [k[0] + du, k[1] + dv, k[2] + dw, k[3] + dc, k[4] + dd, k[5] + de];
                            let range = bucketed.partition_point(|(bk, _)| *bk < nk)..bucketed.partition_point(|(bk, _)| *bk <= nk);
                            for &(_, ei) in &bucketed[range] {
                                let (a, b) = directed[ei as usize];
                                let (p, q) = (coords[a as usize], coords[b as usize]);
                                let off = |p: Vec3| length(cross(sub(p, line.pa), line.u));
                                if off(p) > grid || off(q) > grid {
                                    continue;
                                }
                                let (s, e) = (dot(sub(p, line.pa), line.u), dot(sub(q, line.pa), line.u));
                                let (lo, hi) = if s <= e { (s, e) } else { (e, s) };
                                if hi < outer_lo - grid || lo > outer_hi + grid {
                                    continue;
                                }
                                if dot(sub(q, p), line.u) > 0.0 {
                                    events.push((lo, 1, 0));
                                    events.push((hi, -1, 0));
                                } else {
                                    events.push((lo, 0, 1));
                                    events.push((hi, 0, -1));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    events.sort_by(|a, b| a.0.total_cmp(&b.0));
    // Each bad segment on its own: every stretch inside it must be covered exactly once each
    // way. A gap between two segments has no edge at all, so it is not checked. Both `events`
    // and `segments` are sorted, so one two-pointer sweep over the events covers all segments:
    // the event index only moves forward, O(events + segments) in all.
    let (mut cf, mut cb) = (0i32, 0i32);
    let mut ei = 0usize;
    for &(s_lo, s_hi) in &line.segments {
        // Events before the segment: their stretches lie in earlier segments or gaps, already
        // checked or not to be checked — only their counts carry over.
        while ei < events.len() && events[ei].0 < s_lo {
            cf += i32::from(events[ei].1);
            cb += i32::from(events[ei].2);
            ei += 1;
        }
        let mut prev = s_lo;
        while ei < events.len() && events[ei].0 <= s_hi {
            let pos = events[ei].0;
            if pos > prev && (cf != 1 || cb != 1) {
                return Err("the mesh is not a closed surface: an edge belongs to more or fewer than two faces".into());
            }
            cf += i32::from(events[ei].1);
            cb += i32::from(events[ei].2);
            prev = pos;
            ei += 1;
        }
        if prev < s_hi && (cf != 1 || cb != 1) {
            return Err("the mesh is not a closed surface: an edge belongs to more or fewer than two faces".into());
        }
    }
    Ok(())
}

/// The line key of `line` (see [`line_key`]).
fn line_key_of(line: &Line) -> [i64; 6] {
    line_key(line.pa, [line.pa[0] + line.u[0], line.pa[1] + line.u[1], line.pa[2] + line.u[2]], 4.0 * line.grid)
}

/// The canonical unit direction of the edge `pa`→`pb`: normalised, and sign-fixed on its
/// largest-magnitude component, so float noise on the other components (an edge meant to be
/// axis-aligned, perturbed by 1e-8) cannot flip the sign and with it the line key. When two
/// components are nearly equal in magnitude, the ±1 bucket search still finds both signs.
fn unit_dir(pa: Vec3, pb: Vec3) -> Vec3 {
    let ab = sub(pb, pa);
    let len = length(ab);
    let mut u = [ab[0] / len, ab[1] / len, ab[2] / len];
    let i = (0..3).max_by(|&a, &b| u[a].abs().total_cmp(&u[b].abs())).unwrap_or(0);
    if u[i] < 0.0 {
        for c in &mut u {
            *c = -*c;
        }
    }
    u
}

/// A line's key: canonical unit direction (largest-magnitude component positive) and the offset
/// cross(p, u), quantised — collinear edges within the weld grid `grid` land in the same or a
/// neighbouring bucket.
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
fn line_key(pa: Vec3, pb: Vec3, grid: f64) -> [i64; 6] {
    let u = unit_dir(pa, pb);
    let c = cross(pa, u);
    [
        (u[0] * 20.0).round() as i64,
        (u[1] * 20.0).round() as i64,
        (u[2] * 20.0).round() as i64,
        (c[0] / grid).round() as i64,
        (c[1] / grid).round() as i64,
        (c[2] / grid).round() as i64,
    ]
}

/// Topology gate (roadmap 6.6, `ComplexGen`'s closed face loops): every edge of the mesh must be
/// covered by exactly one facet in each direction, so the recognised faces close into loops.
/// Corners are welded on a grid well below the tolerance, as `boundary_loops` does in 2D. An
/// exporter's T-junction (a corner lying on another facet's edge) still counts: the segment is
/// then covered by the sub-edges. Edges that are not the usual one-each-way pair are grouped by
/// their supporting line, and each line is swept once, so the gate costs O(edges log edges), not
/// quadratic on T-junction meshes. Sort-based throughout: no per-triangle index beyond a few
/// vectors, freed before recognition builds its own.
#[allow(clippy::many_single_char_names)] // points a, b, p, the cell indices c, as in the geometry
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss, clippy::cast_lossless)] // the packed keys are bounded: < 2^21 per axis, < 2^31 point ids
fn closed_loops(mesh: &Mesh, tol: f64) -> Result<(), String> {
    let Some(bounds) = mesh.bounds() else { return Ok(()) };
    let grid = tol / WELD_GRID_TOL;
    // Each axis' cell index fits in 21 bits: the part is at most 1e5 tolerances wide and the
    // weld grid is 16 tolerances fine, so at most ~1.6e6 cells per axis.
    let key = |p: Vec3| -> u64 {
        let c = |k: usize| (((p[k] - bounds.min[k]) / grid).round() as i64) as u64 & 0x1F_FFFF;
        c(0) << 42 | c(1) << 21 | c(2)
    };
    let mut points: Vec<u64> = mesh.triangles.iter().flatten().map(|p| key(*p)).collect();
    points.sort_unstable();
    points.dedup();
    let id = |p: Vec3| points.partition_point(|&x| x < key(p)) as u32;
    let mut directed: Vec<(u32, u32)> = Vec::with_capacity(mesh.triangles.len() * 3);
    for t in &mesh.triangles {
        let ids = t.map(id);
        if ids[0] == ids[1] || ids[1] == ids[2] || ids[2] == ids[0] {
            continue; // a degenerate facet welds its corners together; it carries no edge
        }
        directed.extend([(ids[0], ids[1]), (ids[1], ids[2]), (ids[2], ids[0])]);
    }
    // Per unordered edge: how many facets run each way (bit 0: lo->hi, bit 1: hi->lo).
    let mut by_edge: Vec<u64> = directed
        .iter()
        .map(|&(a, b)| if a < b { (u64::from(a) << 33) | (u64::from(b) << 1) } else { (u64::from(b) << 33) | (u64::from(a) << 1) | 1 })
        .collect();
    by_edge.sort_unstable();
    let mut bad: Vec<(u32, u32)> = Vec::new();
    let mut i = 0;
    while i < by_edge.len() {
        let edge = by_edge[i] & !1;
        let (mut f, mut r) = (0u64, 0u64);
        while i < by_edge.len() && by_edge[i] & !1 == edge {
            if by_edge[i] & 1 == 0 {
                f += 1;
            } else {
                r += 1;
            }
            i += 1;
        }
        if f != 1 || r != 1 {
            bad.push(((edge >> 33) as u32, (edge >> 1 & 0x7FFF_FFFF) as u32));
        }
    }
    if bad.is_empty() {
        return Ok(());
    }
    // Welded points' coordinates.
    let mut v: Vec<(u64, Vec3)> = mesh.triangles.iter().flatten().map(|&p| (key(p), p)).collect();
    v.sort_unstable_by_key(|(k, _)| *k);
    v.dedup_by_key(|(k, _)| *k);
    let coords: Vec<Vec3> = v.into_iter().map(|(_, p)| p).collect();
    // A line's key: canonical unit direction (see [`unit_dir`]) and the offset cross(p, u),
    // quantised to the weld grid. Collinear edges within the grid land in the same or a
    // neighbouring bucket.
    // Bucket every directed edge by its line key, once.
    let mut bucketed: Vec<([i64; 6], u32)> =
        directed.iter().enumerate().map(|(i, &(a, b))| (line_key(coords[a as usize], coords[b as usize], 4.0 * grid), i as u32)).collect();
    bucketed.sort_unstable_by_key(|(k, _)| *k);
    // Group the bad segments by their line; each line is swept once below.
    let mut lines: Vec<Line> = Vec::new();
    let mut line_at: HashMap<[i64; 6], usize> = HashMap::new();
    for &(a, b) in &bad {
        let (pa, pb) = (coords[a as usize], coords[b as usize]);
        let k = line_key(pa, pb, 4.0 * grid);
        let idx = *line_at.entry(k).or_insert_with(|| {
            lines.push(Line { pa, u: unit_dir(pa, pb), segments: Vec::new(), grid });
            lines.len() - 1
        });
        let line = &mut lines[idx];
        let (s, e) = (dot(sub(pa, line.pa), line.u), dot(sub(pb, line.pa), line.u));
        line.segments.push((s.min(e), s.max(e)));
    }
    // Merge each line's segments where they overlap or touch, so coverage is checked per
    // separate bad stretch, not across the gaps between them.
    for line in &mut lines {
        line.segments.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));
        let mut merged: Vec<(f64, f64)> = Vec::with_capacity(line.segments.len());
        for &(lo, hi) in &line.segments {
            if let Some(last) = merged.last_mut()
                && lo <= last.1 + line.grid
            {
                last.1 = last.1.max(hi);
                continue;
            }
            merged.push((lo, hi));
        }
        line.segments = merged;
    }
    for line in &lines {
        sweep_line(line, &bucketed, &directed, &coords, grid)?;
    }
    Ok(())
}

/// Looks for a part the recognisers know in `mesh`: a prism along Z, then along X, then along Y
/// (each but the first on the mesh rotated, see [`Axis`]), then an intersection of extrusions.
#[must_use]
pub fn recognize(mesh: &Mesh) -> Recognition {
    recognize_with(mesh, MAX_ROTATED_TRIANGLES)
}

fn recognize_with(mesh: &Mesh, max_rotated_triangles: usize) -> Recognition {
    // Topology gate (roadmap 6.6, ComplexGen's closed face loops): every edge must belong to
    // exactly two facets, in opposite directions, so the recognised faces close into loops.
    // Refused before any axis is tried; a twisted or non-manifold fake never reaches the
    // volume gate.
    let scale = mesh.bounds().map_or(0.0, |b| b.size().into_iter().fold(0.0, f64::max));
    if let Err(why) = closed_loops(mesh, tolerance(scale)) {
        return Recognition::Unrecognized { reason: why };
    }
    let mut reason = String::new();
    for axis in Axis::ALL {
        if axis != Axis::Z && mesh.triangles.len() > max_rotated_triangles {
            continue;
        }
        let rotated;
        let source = if axis == Axis::Z {
            mesh
        } else {
            rotated = axis.rotate_mesh(mesh);
            &rotated
        };
        match crate::plate::recognize_plate(source, axis == Axis::Z) {
            Ok(Recognition::Plate { plate, offset }) => {
                let offset = axis.unrotate(offset).map(crate::mesh::snap_mm);
                if axis == Axis::Z {
                    return Recognition::Plate { plate, offset };
                }
                if plate.edge_fillet_r.is_some() {
                    // A box rounded all over is written as a Z plate (mesh2cad's box vocabulary
                    // is a Z extrusion's); Z was tried first and would have found it.
                    if reason.is_empty() {
                        reason = "a box with all edges rounded along another axis".into();
                    }
                    continue;
                }
                // Along X or Y a rectangle is written as an outline (`extrude_len` and its
                // corners), the way mesh2cad's extrusion module writes it.
                return Recognition::Profile { profile: plate.into_profile(), offset, axis };
            }
            Ok(Recognition::Profile { profile, offset, .. }) => {
                return Recognition::Profile { profile, offset: axis.unrotate(offset).map(crate::mesh::snap_mm), axis };
            }
            Ok(Recognition::Unrecognized { .. } | Recognition::Combined { .. } | Recognition::Stack { .. }) => {
                unreachable!("recognize_plate finds prisms or errs")
            }
            // The first axis's reason is the one reported: Z reads most naturally, and it is the
            // reason every mesh got before X and Y were tried.
            Err(why) => {
                if reason.is_empty() {
                    reason = why;
                }
            }
        }
    }
    // The prisms' reason stays the one reported: a part that is no intersection of extrusions
    // either is most often a prism with something the recognisers do not know yet.
    if let Ok((spec, offset)) = crate::combined::recognize_combined(mesh) {
        return Recognition::Combined { spec, offset };
    }
    // As mesh2cad, a union of partial extrusions when the intersection misses. A part whose
    // faces all lie square to the axes is a stack along each of them: the fewest bands win, the
    // first of Z, X and Y on a tie.
    let mut best: Option<Recognition> = None;
    for axis in Axis::ALL {
        if axis != Axis::Z && mesh.triangles.len() > max_rotated_triangles {
            continue;
        }
        let rotated;
        let source = if axis == Axis::Z {
            mesh
        } else {
            rotated = axis.rotate_mesh(mesh);
            &rotated
        };
        if let Ok((spec, offset)) = crate::stack::recognize_stack(source) {
            let fewer = match &best {
                Some(Recognition::Stack { spec: b, .. }) => spec.bands.len() < b.bands.len(),
                _ => true,
            };
            if fewer {
                best = Some(Recognition::Stack { spec, offset: axis.unrotate(offset).map(crate::mesh::snap_mm), axis });
            }
        }
    }
    best.unwrap_or(Recognition::Unrecognized { reason })
}
