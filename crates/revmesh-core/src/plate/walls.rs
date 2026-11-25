//! The plate's walls (outline sides, rounded corners, hole bores), indexed on a grid for the
//! corner checks.

use super::geometry::{distance, sagitta, segment_distance, segments};
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
    /// A hole's bore, as fitted (not as its rounded parameters).
    Circle(P2, f64),
}

/// The walls of a recognised plate, indexed on a grid so that checking a corner against them
/// costs the same however many corners and holes the plate has.
pub(super) struct Walls {
    pub(super) pieces: Vec<Wall>,
    /// Each piece's name for the import report's per-face deviation ("side 3", "hole 1", ...).
    pub(super) labels: Vec<String>,
    pub(super) cells: HashMap<(i64, i64), Vec<usize>>,
    pub(super) cell: f64,
    pub(super) tol: f64,
}

impl Walls {
    /// The walls of the outline `corners` (rounded with `radius`), of the holes' `circles` (each
    /// carrying its hole's index in the design's sorted holes, where the design numbers them
    /// `hole_<n>`) and, as meshed, of the top face's boundary `loops` (a facet's chord of an arc
    /// is no wall of the true part, but its corners and its subdivisions lie on it).
    /// `arcs` are further bounded round walls: the slot each end belongs to (its index in the
    /// design's sorted `slots`, where the design numbers them `slot_<n>`), the end's centre,
    /// radius, the corner it rounds and how far from that corner it reaches.
    /// `cutout_loops` are the ideal polygonal cut-outs and `slot_sides` the slots' straight sides
    /// (the slot's index, one side's start and end), so their walls are indexed too.
    #[allow(clippy::too_many_arguments)] // the wall kinds of one part, each labelled
    pub(super) fn new(
        corners: &[P2],
        radius: Option<f64>,
        circles: &[(usize, P2, f64)],
        arcs: &[(usize, P2, f64, P2, f64)],
        loops: &[Vec<P2>],
        cutout_loops: &[Vec<P2>],
        slot_sides: &[(usize, P2, P2)],
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
        let mut labels = Vec::new();
        for i in 0..n {
            let (c, b) = (corners[i], corners[(i + 1) % n]);
            let d = unit(c, b);
            // The side's straight part, between the tangent points of its two corners' arcs.
            let (t0, t1) = (rounds[i].0, rounds[(i + 1) % n].0);
            walls.push(Wall::Segment([c[0] + d[0] * t0, c[1] + d[1] * t0], [b[0] - d[0] * t1, b[1] - d[1] * t1]));
            labels.push(format!("side {}", i + 1));
            if r > 0.0 {
                walls.push(Wall::Arc { centre: rounds[i].1, r, corner: c, t: rounds[i].0 });
                labels.push(format!("corner {} round", i + 1));
            }
        }
        // Each circle is labelled with its hole's number in the design (`hole_<n>`), carried
        // from the sorted hole specifications.
        for &(own, c, r) in circles {
            walls.push(Wall::Circle(c, r));
            labels.push(format!("hole {}", own + 1));
        }
        // Each arc is labelled with the slot that owns it (the design's `slot_<n>`), its two
        // ends counted within that slot.
        let mut ends: HashMap<usize, usize> = HashMap::new();
        for &(slot, centre, r, corner, t) in arcs {
            walls.push(Wall::Arc { centre, r, corner, t });
            let end = ends.entry(slot).or_insert(0);
            *end += 1;
            labels.push(format!("slot {} end {}", slot + 1, *end));
        }
        for (m, ring) in loops.iter().enumerate() {
            let what = if m == 0 { "outline as meshed" } else { "cut-out" };
            for i in 0..ring.len() {
                walls.push(Wall::Segment(ring[i], ring[(i + 1) % ring.len()]));
                labels.push(if m == 0 { format!("outline as meshed, facet {}", i + 1) } else { format!("{what} {}, side {}", m, i + 1) });
            }
        }
        // The ideal straight cut-out walls, labelled by the cut-out's number and side.
        for (m, ring) in cutout_loops.iter().enumerate() {
            for i in 0..ring.len() {
                walls.push(Wall::Segment(ring[i], ring[(i + 1) % ring.len()]));
                labels.push(format!("cut-out {}, side {}", m + 1, i + 1));
            }
        }
        // Each slot's two straight sides, labelled with the slot's number in the design
        // (`slot_<n>`), the sides counted within it.
        let mut sides: HashMap<usize, usize> = HashMap::new();
        for &(slot, a, b) in slot_sides {
            walls.push(Wall::Segment(a, b));
            let s = sides.entry(slot).or_insert(0);
            *s += 1;
            labels.push(format!("slot {} side {}", slot + 1, *s));
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
        Self { pieces: walls, labels, cells, cell, tol }
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
            Wall::Circle(centre, r) => on_circle(centre, r),
        };
        [a, b, p].iter().any(|q| {
            let key = ((q[0] / self.cell).floor() as i64, (q[1] / self.cell).floor() as i64);
            self.cells.get(&key).is_some_and(|candidates| candidates.iter().any(|&k| follows(k)))
        })
    }

    /// How far `p` lies from the nearest wall piece, with that piece's index — for the import
    /// report's per-face deviation (a facet chord of a round wall bows inward by its sagitta).
    /// The walls are looked up in the cells around `p` and around each of `near` (a chord's
    /// middle through its ends, which lie on the walls, as `holds_chord` does); a sample no
    /// neighbouring cell knows is measured against every wall, so none is silently dropped.
    pub(super) fn deviation(&self, p: P2, near: &[P2]) -> Option<(f64, usize)> {
        let mut best: Option<(f64, usize)> = None;
        let lookup = |q: P2, best: &mut Option<(f64, usize)>| {
            let key = ((q[0] / self.cell).floor() as i64, (q[1] / self.cell).floor() as i64);
            for dx in [-1, 0, 1] {
                for dy in [-1, 0, 1] {
                    if let Some(candidates) = self.cells.get(&(key.0 + dx, key.1 + dy)) {
                        for &k in candidates {
                            self.measure(p, k, best);
                        }
                    }
                }
            }
        };
        lookup(p, &mut best);
        for &q in near {
            lookup(q, &mut best);
        }
        if best.is_none() {
            for k in 0..self.pieces.len() {
                self.measure(p, k, &mut best);
            }
        }
        best
    }

    /// The distance from `p` to wall piece `k`, kept when it is the nearest so far.
    fn measure(&self, p: P2, k: usize, best: &mut Option<(f64, usize)>) {
        let d = match self.pieces[k] {
            Wall::Segment(a, b) => segment_distance(p, a, b),
            Wall::Arc { centre, r, corner, t } => {
                if distance(p, corner) <= t + self.tol {
                    (distance(p, centre) - r).abs()
                } else {
                    f64::INFINITY
                }
            }
            Wall::Circle(centre, r) => (distance(p, centre) - r).abs(),
        };
        if d < best.map_or(f64::INFINITY, |(b, _)| b) {
            *best = Some((d, k));
        }
    }

    /// Whether `p` (relative to the plate's corner) lies on one of the walls.
    pub(super) fn holds(&self, p: P2) -> bool {
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
