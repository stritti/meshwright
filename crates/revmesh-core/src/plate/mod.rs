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
//!    a [`PlateSpec`](crate::recognize::PlateSpec), any other a
//!    [`ProfileSpec`](crate::recognize::ProfileSpec).
//! 4. Each hole meets the top and the bottom face in circles about one centre. Where they
//!    differ, the wider one is the mouth of a counterbore (corners on its wide wall down to a
//!    flat floor) or a countersink (a cone down to where the bore's corners begin); every corner
//!    and every facet that faces up or down between the faces must then lie on that surface.
//! 5. The plate rebuilt from those numbers must have the mesh's volume within 1%, the check
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
mod holes;
mod outline;
mod prism;
#[cfg(test)]
mod tests;
mod walls;

pub(crate) use outline::{corner_loss, fillet_loss};

use std::f64::consts::PI;

use bosses::strip_bosses;
use holes::Footprint;
use prism::recognize_prism;

use crate::mesh::{Mesh, snap_mm};
use crate::recognize::{BossSpec, PadSpec, Recognition};

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
const WELD_GRID_TOL: f64 = 16.0;

/// A wall edge's middle may lie this many facets' sagittas inside its circle (a two-facet chord).
const CHORD_FACET_SPAN: f64 = 4.0;

/// A countersink angle rounds to a whole degree when that moves its edge by at most this much.
const ANGLE_WHOLE_TOL: f64 = 1e-3;

/// The finest angle a countersink keeps, in hundredths of a degree.
const ANGLE_HUNDREDTHS: f64 = 100.0;

/// How far a corner may lie from where the recogniser expects it, in millimetres, for a part
/// whose largest size is `scale`. STL coordinates have about seven significant digits.
fn tolerance(scale: f64) -> f64 {
    (scale * REL_TOLERANCE).max(MIN_TOLERANCE)
}

type P2 = [f64; 2];

const NOT_FLAT: &str = "it is not a flat plate: some corners lie between its top and bottom faces \
                        (steps, pockets and bosses are not recognised yet)";

const NOT_CLOSED: &str = "the mesh is not closed, or its top and bottom faces differ";

/// Looks for a plate in `mesh`, possibly with round bosses on its top face. `Err` carries the
/// reason in the user's words.
pub(crate) fn recognize_plate(mesh: &Mesh) -> Result<Recognition, String> {
    let why = match recognize_prism(mesh) {
        Ok(found) => return Ok(found),
        Err(why) => why,
    };
    // Something may stand on the top face: the plate is then what lies below it, its top face
    // closed over the bosses' footprints.
    let Some((plate, found)) = strip_bosses(mesh)? else { return Err(why) };
    let mut recognised = recognize_prism(&plate)?;
    let (offset, top) = match &recognised {
        Recognition::Plate { plate, offset } => (*offset, offset[2] + plate.z),
        Recognition::Profile { profile, offset, .. } => (*offset, offset[2] + profile.z),
        Recognition::Unrecognized { .. } => return Err(why),
    };
    let (mut bosses, mut pads) = (Vec::new(), Vec::new());
    for b in &found {
        let [x, y] = [snap_mm(b.foot.middle()[0] - offset[0]), snap_mm(b.foot.middle()[1] - offset[1])];
        let h = snap_mm(b.top - top);
        match b.foot {
            Footprint::Round { r, .. } => bosses.push(BossSpec { x, y, d: snap_mm(2.0 * r), h }),
            Footprint::Rect { lo, hi } => pads.push(PadSpec { x, y, w: snap_mm(hi[0] - lo[0]), l: snap_mm(hi[1] - lo[1]), h }),
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
        Recognition::Unrecognized { .. } => unreachable!("handled above"),
    };
    let error = (volume - mesh.volume()) / mesh.volume();
    if error.abs() > MAX_VOLUME_ERROR {
        return Err(format!("the rebuilt plate would differ from the mesh by {:.1}% in volume", error.abs() * 100.0));
    }
    Ok(recognised)
}
