//! Recognising a plate: an outline of straight sides, optionally with a radius on every
//! vertical corner (each corner its own), extruded along Z, with round vertical through holes.
//!
//! Written independently from mesh2cad's published description (a Z extrusion with one radius on
//! every vertical corner of the outline, through holes, the parameter names `base_x`,
//! `hole_<n>_d` and `outline_fillet_rs`); no mesh2cad code was used for it.
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
//!    a [`PlateSpec`](crate::recognize::PlateSpec), any other a
//!    [`ProfileSpec`](crate::recognize::ProfileSpec).
//! 4. Each hole meets the top and the bottom face in circles about one centre. Where they
//!    differ, the wider one is the mouth of a counterbore (corners on its wide wall down to a
//!    flat floor) or a countersink (a cone down to where the bore's corners begin); every corner
//!    and every facet that faces up or down between the faces must then lie on that surface.
//! 5. A rectangular outline inset from the bounding box by one radius on every side is a box
//!    rounded all over (mesh2cad's `edge_fillet_r`): its top and bottom faces are the inner
//!    rectangle, and every corner between the faces must lie on the rounded surface, at that
//!    radius from the inner box.
//! 6. The plate rebuilt from those numbers must have the mesh's volume within 1%, the check
//!    mesh2cad documents for its scripts; otherwise the mesh is kept.
//!
//! The module is split by concern: `geometry` (2D helpers), `faces` (top/bottom face and
//! wall split), `walls` (the wall index), `outline` (straight sides and rounded corners),
//! `holes` (holes, slots, cut-outs, pockets), `bosses` (what stands on the top face) and
//! `prism` (the recognition pipeline). Only `recognize_plate` is the entry point.

#![allow(clippy::many_single_char_names)] // points a, b, c, sizes x, y, z and radii r, as in the geometry

mod bosses;
mod faces;
mod geometry;
pub(crate) use geometry::{Rng, fit_circle};
mod holes;
mod outline;
mod prism;
#[cfg(test)]
mod tests;
mod walls;

pub(crate) use outline::{edge_fillet_removed, fillet_loss};

use std::f64::consts::PI;

pub(crate) use bosses::strip_side_bosses;
use bosses::{Shape, strip_bosses};
use holes::Footprint;
use prism::recognize_prism;
pub(crate) use prism::{face_deviations, walls_deviations};

use crate::mesh::{Mesh, snap_mm};
use crate::recognize::{BossChamfer, BossSpec, PadSpec, Recognition, Wall};

/// The draft of a boss `h` high (as written) narrowing from radius `foot` (as written) to the
/// measured `top`, in degrees as a parameter: a whole degree, else a hundredth of one, else the
/// angle as it is, the first whose top radius as SolidCraft computes it (`foot - h * tan(angle)`)
/// lies within a micrometre of `top` and is positive. `None`: no angle does.
fn draft_angle(foot: f64, top: f64, h: f64) -> Option<f64> {
    let degrees = (foot - top).atan2(h).to_degrees();
    let top_at = |d: f64| foot - h * d.to_radians().tan();
    [degrees.round(), (degrees * 100.0).round() / 100.0, degrees].into_iter().find(|d| (top_at(*d) - top).abs() <= 1e-3 && top_at(*d) > 0.0)
}

/// Largest relative difference between the rebuilt plate's volume and the mesh's.
pub const MAX_VOLUME_ERROR: f64 = 0.01;

/// Fewest corners a polygon needs to be taken for a round slot's end (slot ends stay at eight:
/// a slot with coarser ends reads as a polygon cut-out).
pub const MIN_HOLE_SEGMENTS: usize = 8;

/// Fewest inliers of a rim loop's fixed-seed RANSAC circle fit for it to read as a round hole
/// (roadmap 6.2), and they must be a strict majority of the loop with the excluded corners within
/// one facet's sagitta of the circle; in a face's loops fewer than `MIN_HOLE_SEGMENTS` corners
/// stay polygonal cut-outs anyway, and the ring's own edges keep the `MAX_HOLE_STEP` gate.
const MIN_RING_SEGMENTS: usize = 6;

/// Largest share of its circle one facet of a RANSAC-fitted rim may span (65°, twice over for
/// the gap a dropped outlier leaves): six facets are the fewest, a pentagon's 72° are too
/// coarse, and the flat side of a D-shaped cut-out spans far more.
const MAX_RING_STEP: f64 = 65.0 * PI / 180.0;

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

/// How far a corner may lie from its expected place, relative to the part's size, and at least.
const REL_TOLERANCE: f64 = 1e-5;
const MIN_TOLERANCE: f64 = 1e-3;

/// How far the top and bottom faces' areas and the prism volume may differ relatively, for faces
/// meshed apart.
const AREA_MATCH_REL: f64 = 1e-4;
const VOLUME_MATCH_REL: f64 = 1e-3;

/// A facet of a run on one circle may differ from the run's first facet's length by this share.
const CARRY_LEN_SHARE: f64 = 0.25;

/// A corner whose turn is at least this share of the next corner's continues the run.
const CARRY_TURN_SHARE: f64 = 0.75;

/// Facets of a curve span at most this ratio of longest to shortest.
const CURVE_LEN_SPREAD: f64 = 4.0;

/// Facets of a gentle run (a possible curved side) span at most this ratio.
const GENTLE_LEN_SPREAD: f64 = 3.0;

/// A gentle run of at most this many facets whose circle fit fails is a short polygonal run: its
/// sides stay straight edges. A longer run that fits no circle is a non-circular curve and is
/// refused.
const SHORT_POLYGONAL_RUN: usize = 8;

/// Sine below which two sides count as one straight line when facet corners are merged.
const COLLINEAR_SIN: f64 = 1e-4;

/// Relative denominator below which two lines count as parallel.
const PARALLEL_DENOM: f64 = 1e-9;

/// How far the steps round a hole's rim may add up to a full circle.
const FULL_TURN_TOL: f64 = 1e-6;

/// A facet between the faces counts as a cap when its normal is at least this vertical (60°).
const CAP_NORMAL_MIN: f64 = 0.5;

/// Cells across the plate when the walls are indexed, and the finest cell in tolerances.
const WALL_CELLS_ACROSS: f64 = 256.0;
const WALL_CELL_MIN_TOL: f64 = 4.0;

/// Walls are sampled along themselves this many halves of a cell apart.
const WALL_SAMPLE_HALVES: f64 = 0.5;

/// The grid that welds boundary corners, in tolerances.
pub(crate) const WELD_GRID_TOL: f64 = 16.0;

/// A wall edge's middle may lie this many facets' sagittas inside its circle (a two-facet chord).
const CHORD_FACET_SPAN: f64 = 4.0;

/// A countersink angle rounds to a whole degree when that moves its edge by at most this much.
const ANGLE_WHOLE_TOL: f64 = 1e-3;

/// The finest angle a countersink keeps, in hundredths of a degree.
const ANGLE_HUNDREDTHS: f64 = 100.0;

/// How far a corner may lie from where the recogniser expects it, in millimetres, for a part
/// whose largest size is `scale`. STL coordinates have about seven significant digits.
pub(crate) fn tolerance(scale: f64) -> f64 {
    (scale * REL_TOLERANCE).max(MIN_TOLERANCE)
}

type P2 = [f64; 2];

const NOT_FLAT: &str = "it is not a flat plate: some corners lie between its top and bottom faces \
                        and belong to no step, pocket or hole (not recognised yet)";

const NOT_CLOSED: &str = "the mesh is not closed, or its top and bottom faces differ";

/// Looks for a plate in `mesh`, possibly with round bosses on its top face or on a side wall
/// (roadmap 3.10). `Err` carries the reason in the user's words.
pub(crate) fn recognize_plate(mesh: &Mesh, allow_steps: bool) -> Result<Recognition, String> {
    let why = match recognize_prism(mesh, allow_steps) {
        Ok(found) => return Ok(found),
        Err(why) => why,
    };
    // Something may stand on a side wall: strip it there (the wall rotated to face up) and
    // recognise what lies behind it, which may have top bosses of its own. Only in the Z frame
    // and below the rotated-mesh size cap — along X or Y a side boss is the top boss of the
    // rotated attempt, which the caller tries anyway.
    if allow_steps
        && mesh.triangles.len() <= crate::recognize::MAX_ROTATED_TRIANGLES
        && let Some((stripped, mut side_bosses)) = strip_side_bosses(mesh)
        && let Ok(mut recognised) = recognize_plate(&stripped, allow_steps)
        && let Recognition::Plate { plate, offset } = &mut recognised
        // A boss standing on a side boss (a stepped one) cannot be written, one cylinder at a
        // time: refuse the part instead of dropping the inner boss.
        && plate.side_bosses.is_empty()
    {
        // The bosses were found in the file's frame; the spec wants them relative to the plate's
        // corner, as the top bosses' are — the writer adds the offset again.
        for b in &mut side_bosses {
            let (across, up) = match b.wall {
                Wall::PosX | Wall::NegX => (offset[1], offset[2]),
                Wall::PosY | Wall::NegY => (offset[0], offset[2]),
            };
            b.at = [snap_mm(b.at[0] - across), snap_mm(b.at[1] - up)];
        }
        plate.side_bosses = side_bosses;
        let error = (plate.volume() - mesh.volume()) / mesh.volume();
        if error.abs() > MAX_VOLUME_ERROR {
            return Err(format!("the rebuilt plate would differ from the mesh by {:.1}% in volume", error.abs() * 100.0));
        }
        return Ok(recognised);
    }
    // Something may stand on the top face: the plate is then what lies below it, its top face
    // closed over the bosses' footprints.
    let Some((plate, found, _)) = strip_bosses(mesh)? else { return Err(why) };
    let mut recognised = recognize_prism(&plate, allow_steps)?;
    let (offset, top) = match &recognised {
        Recognition::Plate { plate, offset } => (*offset, offset[2] + plate.z),
        Recognition::Profile { profile, offset, .. } => (*offset, offset[2] + profile.z),
        Recognition::Unrecognized { .. } | Recognition::Combined { .. } | Recognition::Stack { .. } | Recognition::Slanted { .. } => {
            return Err(why);
        }
    };
    let (mut bosses, mut pads) = (Vec::new(), Vec::new());
    for b in &found {
        let [x, y] = [snap_mm(b.foot.middle()[0] - offset[0]), snap_mm(b.foot.middle()[1] - offset[1])];
        let h = snap_mm(b.top - top);
        match b.foot {
            Footprint::Round { r, .. } => {
                let mut boss = BossSpec { x, y, d: snap_mm(2.0 * r), h, fillet: None, draft_deg: None, chamfer: None };
                match b.shape {
                    Shape::Straight => {}
                    // A round foot widens the footprint by its radius beyond the cylinder.
                    Shape::Fillet => (boss.d, boss.fillet) = (snap_mm(2.0 * b.wall_r), Some(snap_mm(b.fillet))),
                    // Of the sizes as written: the cone SolidCraft builds from them.
                    Shape::Draft => {
                        boss.draft_deg = Some(
                            draft_angle(boss.d / 2.0, b.wall_r, boss.h)
                                .ok_or("a drafted boss's top would not lie where it is, or not at all (it is kept as a mesh)")?,
                        );
                    }
                    Shape::Chamfer { w, h: down } => boss.chamfer = Some(BossChamfer { w: snap_mm(w), h: snap_mm(down) }),
                }
                bosses.push(boss);
            }
            Footprint::Rect { lo, hi } => pads.push(PadSpec { x, y, w: snap_mm(hi[0] - lo[0]), l: snap_mm(hi[1] - lo[1]), h }),
            // Only round bosses and rectangular pads stand above the top face.
            Footprint::Polygon { .. } => unreachable!("a polygon footprint above the top face is no boss"),
        }
    }
    bosses.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    pads.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    let volume = match &mut recognised {
        Recognition::Plate { plate, .. } => {
            (plate.bosses, plate.pads) = (bosses, pads);
            plate.volume()
        }
        Recognition::Profile { profile, .. } => {
            (profile.bosses, profile.pads) = (bosses, pads);
            profile.volume()
        }
        Recognition::Unrecognized { .. } | Recognition::Combined { .. } | Recognition::Stack { .. } | Recognition::Slanted { .. } => {
            unreachable!("handled above")
        }
    };
    let error = (volume - mesh.volume()) / mesh.volume();
    if error.abs() > MAX_VOLUME_ERROR {
        return Err(format!("the rebuilt plate would differ from the mesh by {:.1}% in volume", error.abs() * 100.0));
    }
    Ok(recognised)
}

#[cfg(test)]
mod draft_tests {
    /// A short, steep draft keeps the precision that keeps its top where it is: a whole degree
    /// would move a 0.1 mm boss's top radius from 1 mm to below zero.
    #[test]
    fn steep_drafts_keep_their_top() {
        let angle = super::draft_angle(5.0, 1.0, 0.1).expect("an angle");
        let top = 5.0 - 0.1 * angle.to_radians().tan();
        assert!((top - 1.0).abs() <= 1e-3, "{angle} gives {top}");
        assert_eq!(super::draft_angle(6.0, 6.0 - 8.0 * 5.0_f64.to_radians().tan(), 8.0), Some(5.0));
        // Measured 0.1006 high but written 0.101: the angle is worked out for the written height,
        // so the top SolidCraft builds stays where it was, above zero.
        let angle = super::draft_angle(50.0, 0.1, 0.101).expect("an angle");
        assert!(50.0 - 0.101 * angle.to_radians().tan() > 0.0);
    }
}
