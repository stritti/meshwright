//! The plate's walls (outline sides, rounded corners, hole bores), indexed on a grid for the
//! corner checks.

use super::geometry::{distance, sagitta, segment_distance, segments};
use crate::recognize::ArcSide;
use std::collections::HashMap;
use std::f64::consts::PI;

use super::{P2, WALL_CELL_MIN_TOL, WALL_CELLS_ACROSS, WALL_SAMPLE_HALVES};
/// Whether `p` lies on the round wall standing on `wall`, a loop meshed round the circle about
/// `centre` of radius `radius`: on the circle, or on one of the loop's chords.
pub(super) fn on_wall(p: P2, centre: P2, radius: f64, wall: &[P2], tol: f64) -> bool {
    let r = distance(p, centre);
    (r - radius).abs() <= tol
        || (r < radius && r >= radius - sagitta(wall, radius) - tol && segments(wall).any(|(a, b)| segment_distance(p, a, b) <= tol))
}

/// One piece of a plate's walls, seen from above.
pub(super) enum Wall {
    /// A straight wall, or a facet of the top face's boundary as meshed.
    Segment(P2, P2),
    /// A rounded corner: on the circle about `centre` of radius `r`, no farther from the sharp
    /// `corner` than its tangent points (`t`); the rest of the circle lies farther away.
    Arc { centre: P2, r: f64, corner: P2, t: f64 },
    /// An arc side of the outline: on the circle about `centre` of radius `r`, within the span
    /// from `a` to `b` counter-clockwise.
    ArcSide { centre: P2, r: f64, a: P2, b: P2 },
    /// A hole's bore, as fitted (not as its rounded parameters).
    Circle(P2, f64),
}

/// Whether `p` lies within the counter-clockwise span from `a` to `b` about `centre`, with
/// `slack` radians of allowance at the ends.
fn in_arc_span(p: P2, centre: P2, a: P2, b: P2, slack: f64) -> bool {
    let angle = |q: P2| (q[1] - centre[1]).atan2(q[0] - centre[0]);
    let (aa, bb, ap) = (angle(a), angle(b), angle(p));
    let theta = (bb - aa).rem_euclid(2.0 * PI);
    let d = (ap - aa).rem_euclid(2.0 * PI);
    d <= theta + slack || d >= 2.0 * PI - slack
}

/// The walls of a recognised plate, indexed on a grid so that checking a corner against them
/// costs the same however many corners and holes the plate has.
pub(super) struct Walls {
    pub(super) pieces: Vec<Wall>,
    pub(super) cells: HashMap<(i64, i64), Vec<usize>>,
    pub(super) cell: f64,
    pub(super) tol: f64,
}

impl Walls {
    /// The walls of the outline `corners` (rounded with `radius`), of the holes' `circles` and,
    /// as meshed, of the top face's boundary `loops` (a facet's chord of an arc is no wall of the
    /// true part, but its corners and its subdivisions lie on it).
    /// `arcs` are further bounded round walls: centre, radius, the corner each rounds and how far
    /// from that corner it reaches. `arc_sides` are the outline's curved sides, as fitted; their
    /// endpoint chords are not indexed as straight walls.
    #[allow(clippy::too_many_arguments)] // every wall kind the plate may have, one index
    pub(super) fn new(
        corners: &[P2],
        radius: Option<f64>,
        circles: &[(P2, f64)],
        arcs: &[(P2, f64, P2, f64)],
        arc_sides: &[Option<ArcSide>],
        loops: &[Vec<P2>],
        size: f64,
        tol: f64,
    ) -> Self {
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
            // An arc side's endpoint chord is no straight wall: it cuts across the part's
            // interior, and a wall tapering to it must fail the checks. The side's meshed
            // facets still enter through `loops`.
            if arc_sides.get(i).map_or(true, |a| a.is_none()) {
                let d = unit(c, b);
                // The side's straight part, between the tangent points of its two corners' arcs.
                let (t0, t1) = (rounds[i].0, rounds[(i + 1) % n].0);
                walls.push(Wall::Segment([c[0] + d[0] * t0, c[1] + d[1] * t0], [b[0] - d[0] * t1, b[1] - d[1] * t1]));
            }
            if r > 0.0 {
                walls.push(Wall::Arc { centre: rounds[i].1, r, corner: c, t: rounds[i].0 });
            }
        }
        walls.extend(circles.iter().map(|&(c, r)| Wall::Circle(c, r)));
        walls.extend(arcs.iter().map(|&(centre, r, corner, t)| Wall::Arc { centre, r, corner, t }));
        let n = corners.len();
        walls.extend(
            arc_sides
                .iter()
                .enumerate()
                .filter_map(|(i, a)| a.map(|a| Wall::ArcSide { centre: a.centre, r: a.r, a: corners[i], b: corners[(i + 1) % n] })),
        );
        for ring in loops {
            walls.extend((0..ring.len()).map(|i| Wall::Segment(ring[i], ring[(i + 1) % ring.len()])));
        }
        // About 256 cells across the plate, never finer than a few tolerances.
        let cell = (size / WALL_CELLS_ACROSS).max(WALL_CELL_MIN_TOL * tol);
        // Each wall goes into the cells along it, not into every cell of its bounding box (a long
        // diagonal side or a large circle would fill thousands): points on it half a cell apart,
        // each with the cells around it, cover every cell within a tolerance of it.
        let mut cells: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
        for (k, wall) in walls.iter().enumerate() {
            #[allow(clippy::cast_sign_loss)] // counts of half cells along a wall, never negative
            let samples: Vec<P2> = match *wall {
                Wall::Segment(a, b) => {
                    let steps = (distance(a, b) / (WALL_SAMPLE_HALVES * cell)).ceil().max(1.0) as usize;
                    (0..=steps).map(|s| s as f64 / steps as f64).map(|t| [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])]).collect()
                }
                Wall::Arc { centre, r, .. } | Wall::Circle(centre, r) => {
                    let steps = ((2.0 * PI * r) / (WALL_SAMPLE_HALVES * cell)).ceil().max(8.0) as usize;
                    (0..steps)
                        .map(|s| 2.0 * PI * s as f64 / steps as f64)
                        .map(|a| [centre[0] + r * a.cos(), centre[1] + r * a.sin()])
                        .collect()
                }
                Wall::ArcSide { centre, r, a, b } => {
                    let aa = (a[1] - centre[1]).atan2(a[0] - centre[0]);
                    let theta = ((b[1] - centre[1]).atan2(b[0] - centre[0]) - aa).rem_euclid(2.0 * PI);
                    let steps = ((theta * r) / (WALL_SAMPLE_HALVES * cell)).ceil().max(8.0) as usize;
                    (0..=steps)
                        .map(|s| aa + theta * s as f64 / steps as f64)
                        .map(|ang| [centre[0] + r * ang.cos(), centre[1] + r * ang.sin()])
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
    pub(super) fn holds_chord(&self, a: P2, b: P2) -> bool {
        let p = [f64::midpoint(a[0], b[0]), f64::midpoint(a[1], b[1])];
        let half2 = distance(a, b).powi(2) / 4.0;
        let tol = self.tol;
        // The middle of a chord lies `r - sqrt(r² - (len/2)²)` inside its circle, maybe cells away
        // from it; the walls are looked up at the chord's ends, which lie on them.
        let on_circle = |centre: P2, r: f64| r * r > half2 && (distance(p, centre) - (r * r - half2).sqrt()).abs() <= tol;
        let follows = |k: usize| match self.pieces[k] {
            Wall::Segment(s, e) => segment_distance(p, s, e) <= tol,
            Wall::Arc { centre, r, corner, t } => on_circle(centre, r) && distance(p, corner) <= t + tol,
            Wall::ArcSide { centre, r, a, b } => on_circle(centre, r) && in_arc_span(p, centre, a, b, tol / r),
            Wall::Circle(centre, r) => on_circle(centre, r),
        };
        [a, b, p].iter().any(|q| {
            let key = ((q[0] / self.cell).floor() as i64, (q[1] / self.cell).floor() as i64);
            self.cells.get(&key).is_some_and(|candidates| candidates.iter().any(|&k| follows(k)))
        })
    }

    /// Whether `p` (relative to the plate's corner) lies on one of the walls.
    pub(super) fn holds(&self, p: P2) -> bool {
        let key = ((p[0] / self.cell).floor() as i64, (p[1] / self.cell).floor() as i64);
        let tol = self.tol;
        self.cells.get(&key).is_some_and(|candidates| {
            candidates.iter().any(|&k| match self.pieces[k] {
                Wall::Segment(a, b) => segment_distance(p, a, b) <= tol,
                Wall::Arc { centre, r, corner, t } => (distance(p, centre) - r).abs() <= tol && distance(p, corner) <= t + tol,
                Wall::ArcSide { centre, r, a, b } => (distance(p, centre) - r).abs() <= tol && in_arc_span(p, centre, a, b, tol / r),
                Wall::Circle(centre, r) => (distance(p, centre) - r).abs() <= tol,
            })
        })
    }
}
