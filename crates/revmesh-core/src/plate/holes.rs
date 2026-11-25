//! The holes, slots and cut-outs through a plate: round holes, stepped holes (counterbores,
//! countersinks), blind holes, pockets, straight slots and polygonal cut-outs.

use super::geometry::{distance, fit_circle, sagitta, segment_distance, signed_area};
use super::walls::on_wall;
use super::{ANGLE_HUNDREDTHS, ANGLE_WHOLE_TOL, CHORD_FACET_SPAN, FULL_TURN_TOL, MAX_HOLE_STEP, MAX_STEP_RATIO, MIN_HOLE_SEGMENTS, P2};
use crate::mesh::{Vec3, snap_mm};
use crate::recognize::{HoleHead, HoleSpec, SlotSpec};
use std::f64::consts::PI;

/// A hole's rounded parameters, its circle as fitted, and the index of its loop.
pub(super) type FittedHole = (HoleSpec, (P2, f64), usize);

/// A slot as fitted to its loop: the middle, the long axis, half the distance between the end
/// centres and the ends' radius.
#[derive(Debug, Clone, Copy)]
pub(super) struct SlotGeometry {
    pub(super) centre: P2,
    pub(super) axis: P2,
    pub(super) half: f64,
    pub(super) r: f64,
}

impl SlotGeometry {
    /// The centres of its two round ends.
    pub(super) fn ends(&self) -> [P2; 2] {
        let at = |s: f64| [self.centre[0] + s * self.axis[0], self.centre[1] + s * self.axis[1]];
        [at(self.half), at(-self.half)]
    }

    /// How far `p` lies from the segment between the end centres.
    pub(super) fn reach(&self, p: P2) -> f64 {
        let [a, b] = self.ends();
        segment_distance(p, a, b)
    }

    /// Its round ends as walls: each end's outer half circle, bounded by reaching no further
    /// than `r·√2` from the end's tip. That disk holds every point and every chord of the outer
    /// half and leaves out the inner half, which lies inside the slot.
    pub(super) fn halves(&self) -> Vec<(P2, f64, P2, f64)> {
        self.ends()
            .into_iter()
            .zip([1.0, -1.0])
            .map(|(end, s)| {
                let tip = [end[0] + s * self.r * self.axis[0], end[1] + s * self.r * self.axis[1]];
                (end, self.r, tip, self.r * std::f64::consts::SQRT_2)
            })
            .collect()
    }

    pub(super) fn matches(&self, other: &Self, tol: f64) -> bool {
        let parallel = (self.axis[0] * other.axis[1] - self.axis[1] * other.axis[0]).abs() * self.half <= tol;
        distance(self.centre, other.centre) <= tol && (self.half - other.half).abs() <= tol && (self.r - other.r).abs() <= tol && parallel
    }
}

/// A cut-out's loop that is not round: a straight slot, or the reason it is neither.
pub(super) type FittedSlot = (SlotSpec, SlotGeometry, usize);

/// A face's cut-outs: round holes, slots, and polygons (their corners and loop index).
pub(super) type Cutouts = (Vec<FittedHole>, Vec<FittedSlot>, Vec<(Vec<P2>, usize)>);

/// A hole whose mouth is wider than its bore, at the top or the bottom face, in coordinates
/// relative to the plate's lowest corner.
#[derive(Debug, Clone)]
pub(super) struct Stepped {
    pub(super) centre: P2,
    pub(super) bore_r: f64,
    pub(super) head_r: f64,
    /// The bore's and the mouth's loops as meshed where they meet a face. Their walls follow
    /// these chords, not the circles: a corner an exporter adds within a wall facet lies on a
    /// chord, up to its sagitta inside the circle.
    pub(super) bore: Vec<P2>,
    pub(super) mouth: Vec<P2>,
    /// The face the mouth opens on, and which way is into the plate from it (-1 down from the
    /// top, +1 up from the bottom).
    pub(super) face_z: f64,
    pub(super) inward: f64,
    /// How deep the mouth goes: to a counterbore's floor, or to where a countersink's cone meets
    /// the bore.
    pub(super) depth: f64,
    pub(super) counterbore: bool,
    /// A round of the rim (its radius `head_r - bore_r`, as deep), not a countersink's cone, and
    /// how far off it its facets lie at most.
    pub(super) rounded: bool,
    pub(super) round_slack: f64,
}

/// What a blind hole, a pocket, a boss or a pad stands on, seen from above: a circle, or a
/// rectangle along X and Y.
#[derive(Debug, Clone)]
pub(super) enum Footprint {
    Round {
        centre: P2,
        r: f64,
    },
    Rect {
        lo: P2,
        hi: P2,
    },
    /// Any polygon of straight sides (a pocket's mouth), its corners counter-clockwise.
    Polygon {
        ring: Vec<P2>,
    },
}

impl Footprint {
    /// Whether `p` lies on or inside it.
    pub(super) fn contains(&self, p: P2, tol: f64) -> bool {
        match self {
            Self::Round { centre, r } => distance(p, *centre) <= *r + tol,
            Self::Rect { lo, hi } => p[0] >= lo[0] - tol && p[0] <= hi[0] + tol && p[1] >= lo[1] - tol && p[1] <= hi[1] + tol,
            Self::Polygon { ring } => inside_polygon(p, ring, tol),
        }
    }

    /// Whether `p` lies on the wall standing on it, `ring` its loop as meshed (a round wall
    /// follows that loop's chords).
    pub(super) fn on_wall(&self, p: P2, ring: &[P2], tol: f64) -> bool {
        match self {
            Self::Round { centre, r } => on_wall(p, *centre, *r, ring, tol),
            Self::Rect { lo, hi } => {
                self.contains(p, tol)
                    && ((p[0] - lo[0]).abs() <= tol
                        || (p[0] - hi[0]).abs() <= tol
                        || (p[1] - lo[1]).abs() <= tol
                        || (p[1] - hi[1]).abs() <= tol)
            }
            Self::Polygon { ring } => ring.iter().zip(ring.iter().cycle().skip(1)).any(|(a, b)| point_segment_distance(p, *a, *b) <= tol),
        }
    }

    /// Whether `p` lies within `inward` inside the wall, or on it: the middle of a wall facet's
    /// edge, which bows in by the chord's sagitta on a round wall.
    pub(super) fn near_wall(&self, p: P2, inward: f64, tol: f64) -> bool {
        match self {
            Self::Round { centre, r } => {
                let d = distance(p, *centre);
                d <= *r + tol && d >= *r - inward - tol
            }
            Self::Rect { .. } | Self::Polygon { .. } => self.on_wall(p, &[], tol),
        }
    }

    /// How far a round wall's facets come inside the circle, `ring` its loop as meshed; a
    /// rectangle's and a polygon's walls are flat.
    pub(super) fn sagitta(&self, ring: &[P2]) -> f64 {
        match self {
            Self::Round { r, .. } => sagitta(ring, *r),
            Self::Rect { .. } | Self::Polygon { .. } => 0.0,
        }
    }

    /// Its area: the loop's as meshed for a circle (the facets' polygon), the rectangle's, the
    /// polygon's.
    pub(super) fn area(&self, ring: &[P2]) -> f64 {
        match self {
            Self::Round { .. } => signed_area(ring).abs(),
            Self::Rect { lo, hi } => (hi[0] - lo[0]) * (hi[1] - lo[1]),
            Self::Polygon { ring } => signed_area(ring).abs(),
        }
    }

    pub(super) fn middle(&self) -> P2 {
        match self {
            Self::Round { centre, .. } => *centre,
            Self::Rect { lo, hi } => [f64::midpoint(lo[0], hi[0]), f64::midpoint(lo[1], hi[1])],
            Self::Polygon { ring } => {
                let n = ring.len() as f64;
                [ring.iter().map(|p| p[0]).sum::<f64>() / n, ring.iter().map(|p| p[1]).sum::<f64>() / n]
            }
        }
    }
}

/// Whether `p` lies inside the counter-clockwise polygon `ring`, or within `tol` of its edge.
fn inside_polygon(p: P2, ring: &[P2], tol: f64) -> bool {
    let n = ring.len();
    let on_edge = ring.iter().zip(ring.iter().cycle().skip(1)).any(|(a, b)| point_segment_distance(p, *a, *b) <= tol);
    if on_edge {
        return true;
    }
    // Even-odd ray casting along +X.
    let mut inside = false;
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        if (a[1] > p[1]) != (b[1] > p[1]) && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0] {
            inside = !inside;
        }
    }
    inside
}

/// The distance from `p` to the segment `a`–`b`.
fn point_segment_distance(p: P2, a: P2, b: P2) -> f64 {
    let (u, v) = ([b[0] - a[0], b[1] - a[1]], [p[0] - a[0], p[1] - a[1]]);
    let len2 = u[0] * u[0] + u[1] * u[1];
    let t = if len2 <= 0.0 { 0.0 } else { ((u[0] * v[0] + u[1] * v[1]) / len2).clamp(0.0, 1.0) };
    ((v[0] - t * u[0]).powi(2) + (v[1] - t * u[1]).powi(2)).sqrt()
}

/// A loop that is a rectangle along X and Y (once the corners an exporter added along its sides
/// are left out): its lowest and highest corner.
pub(super) fn axis_rectangle(ring: &[P2], tol: f64) -> Option<(P2, P2)> {
    let corners = drop_collinear(ring, tol);
    if corners.len() != 4 {
        return None;
    }
    let along_axes = (0..4).all(|i| {
        let (a, b) = (corners[i], corners[(i + 1) % 4]);
        (a[0] - b[0]).abs() <= tol || (a[1] - b[1]).abs() <= tol
    });
    let lo = [corners.iter().map(|c| c[0]).fold(f64::INFINITY, f64::min), corners.iter().map(|c| c[1]).fold(f64::INFINITY, f64::min)];
    let hi =
        [corners.iter().map(|c| c[0]).fold(f64::NEG_INFINITY, f64::max), corners.iter().map(|c| c[1]).fold(f64::NEG_INFINITY, f64::max)];
    (along_axes && hi[0] - lo[0] > tol && hi[1] - lo[1] > tol).then_some((lo, hi))
}

/// A blind hole or pocket: a bore or a rectangle from the top or the bottom face down to a flat
/// floor, in coordinates relative to the plate's lowest corner.
#[derive(Debug, Clone)]
pub(super) struct BlindHole {
    pub(super) foot: Footprint,
    /// Its loop as meshed where it meets the face.
    pub(super) ring: Vec<P2>,
    /// The face it is drilled from, and which way is into the plate from it (as for [`Stepped`]).
    pub(super) face_z: f64,
    pub(super) inward: f64,
    /// How deep the floor lies below the face.
    pub(super) depth: f64,
}

impl BlindHole {
    pub(super) fn reaches(&self, p: P2, tol: f64) -> bool {
        self.foot.contains(p, tol)
    }

    pub(super) fn sagitta(&self) -> f64 {
        self.foot.sagitta(&self.ring)
    }

    /// Whether `p` lies on the hole's wall or its floor.
    pub(super) fn holds(&self, p: Vec3, tol: f64) -> bool {
        let q = [p[0], p[1]];
        let depth = (p[2] - self.face_z) * self.inward;
        if depth < -tol || depth > self.depth + tol {
            return false;
        }
        self.foot.on_wall(q, &self.ring, tol) || ((depth - self.depth).abs() <= tol && self.reaches(q, tol))
    }

    /// Whether the facet `cap` (facing up or down) is part of the floor.
    pub(super) fn holds_cap(&self, cap: &[Vec3; 3], tol: f64) -> bool {
        cap.iter().all(|p| ((p[2] - self.face_z) * self.inward - self.depth).abs() <= tol && self.reaches([p[0], p[1]], tol))
    }
}

impl Stepped {
    pub(super) fn reaches(&self, p: P2, tol: f64) -> bool {
        distance(p, self.centre) <= self.head_r + tol
    }

    pub(super) fn on_wall(&self, p: P2, radius: f64, wall: &[P2], tol: f64) -> bool {
        on_wall(p, self.centre, radius, wall, tol)
    }

    /// How far the longer facets of the bore's and the mouth's loops come inside their circles.
    pub(super) fn sagitta(&self) -> f64 {
        sagitta(&self.bore, self.bore_r).max(sagitta(&self.mouth, self.head_r))
    }

    /// Whether `p` lies on the hole's surface: its bore, its counterbore's wall or floor, or its
    /// countersink's cone.
    pub(super) fn holds(&self, p: Vec3, tol: f64) -> bool {
        let q = [p[0], p[1]];
        let r = distance(q, self.centre);
        let depth = (p[2] - self.face_z) * self.inward;
        // The bore starts where the mouth ends.
        if depth >= self.depth - tol && self.on_wall(q, self.bore_r, &self.bore, tol) {
            return true;
        }
        if depth < -tol || depth > self.depth + tol {
            return false;
        }
        if self.counterbore {
            self.on_wall(q, self.head_r, &self.mouth, tol)
                || ((depth - self.depth).abs() <= tol
                    && r >= self.bore_r - sagitta(&self.bore, self.bore_r) - tol
                    && r <= self.head_r + tol)
        } else if self.rounded {
            // On the quarter circle about (mouth radius, the round's depth), or off it by no more
            // than its facets lie.
            let f = self.head_r - self.bore_r;
            let off = ((r - self.head_r).hypot(depth - f) - f).abs();
            // (Facets round the axis bow inward: a point may lie inside the bore by its loop's
            // sagitta.)
            r >= self.bore_r - sagitta(&self.bore, self.bore_r) - tol && r <= self.head_r + tol && off <= self.round_slack + tol
        } else {
            // The cone's facets run between the mouth's and the bore's loops and come inside the
            // cone as their chords come inside the circles; a corner an exporter adds within a
            // facet lies there too. Allow chords two facets long, as for the walls' edges.
            let cone = self.head_r - depth * (self.head_r - self.bore_r) / self.depth;
            let inside =
                CHORD_FACET_SPAN * (sagitta(&self.mouth, self.head_r) / self.head_r).max(sagitta(&self.bore, self.bore_r) / self.bore_r);
            r <= cone + tol && r >= cone * (1.0 - inside) - tol
        }
    }

    /// Whether the facet `cap` (facing up or down) is part of the mouth: a piece of the
    /// counterbore's annular floor, or a facet of the countersink's cone. Its corners lying on
    /// the hole is not enough: a disk across the bore has them all on the bore's circle.
    pub(super) fn holds_cap(&self, cap: &[Vec3; 3], tol: f64) -> bool {
        let depth = |p: &Vec3| (p[2] - self.face_z) * self.inward;
        let centroid = [(cap[0][0] + cap[1][0] + cap[2][0]) / 3.0, (cap[0][1] + cap[1][1] + cap[2][1]) / 3.0];
        let r = distance(centroid, self.centre);
        if self.rounded {
            // Its corners on the round as the mesh's corners are; the middles of its edges and its
            // centroid within a facet's reach of it (50° along the round at most, the loops'
            // facets round the axis) and in the round's depth range.
            let f = self.head_r - self.bore_r;
            let round_the_axis = (sagitta(&self.mouth, self.head_r) / self.head_r).max(sagitta(&self.bore, self.bore_r) / self.bore_r);
            let reach = f * (1.0 - (super::MAX_HOLE_STEP / 2.0).cos()) + self.head_r * round_the_axis;
            // (Never inside the bore: a facet across it is no part of the round.)
            let bore_in = self.bore_r - sagitta(&self.bore, self.bore_r) - tol;
            let near = |p: &Vec3| {
                let r = distance([p[0], p[1]], self.centre);
                let d = depth(p);
                d >= -tol
                    && d <= f + tol
                    && r >= bore_in
                    && r <= self.head_r + tol
                    && ((r - self.head_r).hypot(d - f) - f).abs() <= reach + tol
            };
            let middle = |a: &Vec3, b: &Vec3| [f64::midpoint(a[0], b[0]), f64::midpoint(a[1], b[1]), f64::midpoint(a[2], b[2])];
            let centroid3 = [centroid[0], centroid[1], (cap[0][2] + cap[1][2] + cap[2][2]) / 3.0];
            let inner = [middle(&cap[0], &cap[1]), middle(&cap[1], &cap[2]), middle(&cap[2], &cap[0]), centroid3];
            // (Its facets near the face may be flatter than the tolerance: they lie on the round all
            // the same, and a disk across the bore does not.)
            return cap.iter().all(|p| self.holds(*p, tol)) && inner.iter().all(near);
        }
        if self.counterbore {
            // Outside the bore's meshed loop, whose chords come within their sagitta of the
            // centre: a facet of the floor lies there, a disk across the bore does not.
            cap.iter().all(|p| (depth(p) - self.depth).abs() <= tol)
                && r >= self.bore_r - sagitta(&self.bore, self.bore_r) - tol
                && r <= self.head_r + tol
        } else {
            let (low, high) = cap.iter().map(depth).fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), d| (lo.min(d), hi.max(d)));
            // Its corners, the middles of its edges and its centroid on the cone: a facet joining
            // corners far apart round the axis cuts across the hole.
            let middle = |a: &Vec3, b: &Vec3| [f64::midpoint(a[0], b[0]), f64::midpoint(a[1], b[1]), f64::midpoint(a[2], b[2])];
            let centroid = [centroid[0], centroid[1], (cap[0][2] + cap[1][2] + cap[2][2]) / 3.0];
            let inner = [middle(&cap[0], &cap[1]), middle(&cap[1], &cap[2]), middle(&cap[2], &cap[0]), centroid];
            cap.iter().chain(&inner).all(|p| self.holds(*p, tol) && depth(p) <= self.depth + tol) && high - low > tol
        }
    }

    pub(super) fn head(&self) -> HoleHead {
        let bottom = self.inward > 0.0;
        if self.rounded {
            return HoleHead::Round { r: snap_mm(self.head_r - self.bore_r), bottom };
        }
        let d = snap_mm(2.0 * self.head_r);
        if self.counterbore {
            HoleHead::Counterbore { d, depth: snap_mm(self.depth), bottom }
        } else {
            let angle = 2.0 * ((self.head_r - self.bore_r) / self.depth).atan().to_degrees();
            // A whole degree when that moves the cone's edge by no more than a micrometre, else
            // a hundredth of one.
            let whole = angle.round();
            let angle = if ((whole - angle).to_radians() * self.depth).abs() <= ANGLE_WHOLE_TOL {
                whole
            } else {
                (angle * ANGLE_HUNDREDTHS).round() / ANGLE_HUNDREDTHS
            };
            HoleHead::Countersink { d, angle, bottom }
        }
    }
}

/// Fits a circle to a hole's corners and checks that they all lie on it. A corner off the circle
/// that lies on the straight line between its neighbours (an exporter split a rim edge there) is
/// left out, and the circle fitted again.
pub(super) fn round_hole(ring: &[P2], tol: f64) -> Result<(HoleSpec, (P2, f64)), String> {
    let first = round_ring(ring, tol);
    let Some((centre, r)) = fit_circle(ring) else { return first };
    if first.is_ok() {
        return first;
    }
    let n = ring.len();
    let kept: Vec<P2> = (0..n)
        .filter(|&i| {
            let p = ring[i];
            (distance(p, centre) - r).abs() <= tol || segment_distance(p, ring[(i + n - 1) % n], ring[(i + 1) % n]) > tol / 4.0
        })
        .map(|i| ring[i])
        .collect();
    if kept.len() == n { first } else { round_ring(&kept, tol) }
}

pub(super) fn round_ring(ring: &[P2], tol: f64) -> Result<(HoleSpec, (P2, f64)), String> {
    if ring.len() < MIN_HOLE_SEGMENTS {
        return Err(format!(
            "a cut-out has only {} corners, so it is taken for a polygon, not a round hole (polygonal cut-outs are not recognised yet)",
            ring.len()
        ));
    }
    let not_round = || "a cut-out is not round and no straight slot (D-shapes and other cut-outs are not recognised yet)".to_owned();
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
    if (around.abs() - 2.0 * PI).abs() > FULL_TURN_TOL || steps.iter().any(|s| *s > MAX_HOLE_STEP || *s > MAX_STEP_RATIO * median) {
        return Err(not_round());
    }
    Ok((HoleSpec { x: snap_mm(cx), y: snap_mm(cy), d: snap_mm(2.0 * r), head: None, blind: None }, ([cx, cy], r)))
}

/// A straight slot's loop: two round ends of one radius joined by two straight sides, every
/// corner on that outline and every facet following it (within its sagitta on the ends), the
/// ends faceted as finely as a round hole. Its spec is rounded; its geometry as fitted, along
/// the rounded angle.
pub(super) fn slot_shape(ring: &[P2], tol: f64) -> Option<(SlotSpec, SlotGeometry)> {
    slot_fit(ring, tol).or_else(|| {
        // An exporter may have split facets: the slot is fitted again without the corners they
        // added (as a round hole's rim is).
        let kept = drop_collinear(ring, tol);
        (kept.len() < ring.len()).then(|| slot_fit(&kept, tol)).flatten()
    })
}

/// The most corners a polygonal cut-out may have, and the least it must turn at each: a loop of
/// more and gentler corners is a curve, faceted.
pub(super) const MAX_CUTOUT_CORNERS: usize = 24;

pub(super) const MIN_CUTOUT_TURN: f64 = 15.0 * PI / 180.0;

/// A cut-out of straight sides: its corners (in the loop's order), without the corners an
/// exporter may have added along a side, every one a real turn.
pub(super) fn polygon_cutout(ring: &[P2], tol: f64) -> Option<Vec<P2>> {
    let corners = drop_collinear(ring, tol);
    let n = corners.len();
    if !(3..=MAX_CUTOUT_CORNERS).contains(&n) {
        return None;
    }
    let turns = (0..n).all(|i| {
        let (a, b, c) = (corners[(i + n - 1) % n], corners[i], corners[(i + 1) % n]);
        let (u, v) = ([b[0] - a[0], b[1] - a[1]], [c[0] - b[0], c[1] - b[1]]);
        (u[0] * v[1] - u[1] * v[0]).atan2(u[0] * v[0] + u[1] * v[1]).abs() >= MIN_CUTOUT_TURN
    });
    turns.then_some(corners)
}

/// The loop without corners on the straight line between their neighbours (within `tol / 4`),
/// as an exporter that splits facets adds them.
pub(super) fn drop_collinear(ring: &[P2], tol: f64) -> Vec<P2> {
    let mut kept: Vec<P2> = Vec::with_capacity(ring.len());
    for (i, p) in ring.iter().enumerate() {
        let prev = kept.last().copied().unwrap_or(ring[ring.len() - 1]);
        if segment_distance(*p, prev, ring[(i + 1) % ring.len()]) > tol / 4.0 {
            kept.push(*p);
        }
    }
    kept
}

#[allow(clippy::many_single_char_names)] // the textbook symbols of the area moments
pub(super) fn slot_fit(ring: &[P2], tol: f64) -> Option<(SlotSpec, SlotGeometry)> {
    let n = ring.len();
    if n < MIN_HOLE_SEGMENTS {
        return None;
    }
    // The centroid and the axis of least inertia, from the loop's area moments (in coordinates
    // about its first corner, so that far from the origin they keep their precision).
    let o = ring[0];
    let (mut a, mut cx, mut cy, mut xx, mut yy, mut xy) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    for i in 0..n {
        let (p, q) = (ring[i], ring[(i + 1) % n]);
        let (x0, y0, x1, y1) = (p[0] - o[0], p[1] - o[1], q[0] - o[0], q[1] - o[1]);
        let cross = x0 * y1 - x1 * y0;
        a += cross / 2.0;
        cx += (x0 + x1) * cross / 6.0;
        cy += (y0 + y1) * cross / 6.0;
        xx += (x0 * x0 + x0 * x1 + x1 * x1) * cross / 12.0;
        yy += (y0 * y0 + y0 * y1 + y1 * y1) * cross / 12.0;
        xy += (x0 * y1 + 2.0 * x0 * y0 + 2.0 * x1 * y1 + x1 * y0) * cross / 24.0;
    }
    if a.abs() <= tol * tol {
        return None;
    }
    let (cx, cy) = (cx / a, cy / a);
    let (sxx, syy, sxy) = (xx / a - cx * cx, yy / a - cy * cy, xy / a - cx * cy);
    let centre = [o[0] + cx, o[1] + cy];
    let mut fitted = (0.5 * (2.0 * sxy).atan2(sxx - syy)).to_degrees().rem_euclid(180.0);
    // The moments are only an estimate where the ends are faceted unevenly: the longest facet,
    // a straight side unless the slot is very short, gives the axis exactly.
    let (p, q) = (0..n).map(|i| (ring[i], ring[(i + 1) % n])).max_by(|a, b| distance(a.0, a.1).total_cmp(&distance(b.0, b.1)))?;
    let side = (q[1] - p[1]).atan2(q[0] - p[0]).to_degrees().rem_euclid(180.0);
    if ((side - fitted + 90.0).rem_euclid(180.0) - 90.0).abs() < 5.0 {
        fitted = side;
    }
    // Its angle as a parameter: exact along X or Y, else in whole degrees or hundredths, as long
    // as the ends move by no more than a micrometre.
    let reach = ring.iter().map(|p| distance(*p, centre)).fold(0.0, f64::max);
    let close = |angle: f64| ((angle - fitted + 90.0).rem_euclid(180.0) - 90.0).abs().to_radians() * reach <= 1e-3;
    let angle = [0.0, 90.0, fitted.round(), (fitted * 100.0).round() / 100.0, fitted]
        .into_iter()
        .find(|a| close(*a))
        .map_or(fitted, |a| a.rem_euclid(180.0));
    let (sin, cos) = angle.to_radians().sin_cos();
    // The straight sides lie the radius either side of the axis: halfway between them is the
    // middle, across the slot (the centroid is not, where the ends are faceted unevenly).
    let offset = |p: &P2| (p[1] - centre[1]) * cos - (p[0] - centre[0]) * sin;
    let (lo, hi) = ring.iter().map(offset).fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), t| (lo.min(t), hi.max(t)));
    let mid = f64::midpoint(lo, hi);
    let centre = [centre[0] - mid * sin, centre[1] + mid * cos];
    let along = |p: &P2| (p[0] - centre[0]) * cos + (p[1] - centre[1]) * sin;
    let across = |p: &P2| (p[1] - centre[1]) * cos - (p[0] - centre[0]) * sin;
    let r = (hi - lo) / 2.0;
    // Each end's centre is where its corners say, `along - sqrt(r² - across²)` (the tip itself
    // need not be a corner: an end of an odd number of facets has none there); the sides'
    // corners lie short of it.
    let end_centre = |side: f64| {
        ring.iter()
            .filter(|p| along(p) * side > 0.0)
            .map(|p| along(p) * side - (r * r - across(p).powi(2)).max(0.0).sqrt())
            .fold(f64::NEG_INFINITY, f64::max)
    };
    let (ahead, behind) = (end_centre(1.0), end_centre(-1.0));
    let half = f64::midpoint(ahead, behind);
    if !half.is_finite() || half <= 2.0 * tol || r <= tol {
        return None;
    }
    let shift = (ahead - behind) / 2.0;
    let centre = [centre[0] + shift * cos, centre[1] + shift * sin];
    let along = |p: &P2| (p[0] - centre[0]) * cos + (p[1] - centre[1]) * sin;
    let g = SlotGeometry { centre, axis: [cos, sin], half, r };
    if ring.iter().any(|p| (g.reach(*p) - r).abs() > tol) {
        return None;
    }
    // Each facet is a piece of a straight side (both corners on one side's line, between the
    // ends) or of one end (both corners past that end's centre, a step no coarser than a round
    // hole's, its middle within its own sagitta of the circle). A facet from a side across to
    // an end cuts the corner.
    let [end_a, end_b] = g.ends();
    for i in 0..n {
        let (p, q) = (ring[i], ring[(i + 1) % n]);
        let on_side = |s: f64| [p, q].iter().all(|c| (across(c) - s * r).abs() <= tol && along(c).abs() <= half + tol);
        if on_side(1.0) || on_side(-1.0) {
            continue;
        }
        let (end, _) =
            [(end_a, 1.0), (end_b, -1.0)].into_iter().find(|(_, s)| along(&p) * s >= half - tol && along(&q) * s >= half - tol)?;
        let turn = (q[1] - end[1]).atan2(q[0] - end[0]) - (p[1] - end[1]).atan2(p[0] - end[0]);
        let len = distance(p, q);
        let bow = r - (r * r - (len * len / 4.0).min(r * r)).sqrt();
        let middle = [f64::midpoint(p[0], q[0]), f64::midpoint(p[1], q[1])];
        if ((turn + PI).rem_euclid(2.0 * PI) - PI).abs() > MAX_HOLE_STEP || (distance(middle, end) - (r - bow)).abs() > tol {
            return None;
        }
    }
    let spec = SlotSpec { x: snap_mm(centre[0]), y: snap_mm(centre[1]), len: snap_mm(2.0 * half), w: snap_mm(2.0 * r), angle };
    Some((spec, g))
}
