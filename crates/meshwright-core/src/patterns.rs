//! Hole patterns: rows, grids and bolt circles among a plate's through holes.
//!
//! Follows mesh2cad's rules and names (`docs/limits.md` of mesh2cad 0.32): holes of one
//! diameter form a row when there are three or more on a straight line at one pitch, an
//! axis-aligned grid of at least two by two, or a bolt circle of three or more equally spaced on
//! one circle. A set that is a grid stays a grid even though its corners also lie on a circle.
//! Two holes are not a pattern, and a set with one hole missing or out of place stays single
//! holes. Written independently from that description; no mesh2cad code was used.
//!
//! In SolidCraft a pattern is one hole feature and a pattern feature that repeats it, so a
//! change of `hole_row_1_n` or `hole_circle_1_r` rebuilds every hole of the pattern.

#![allow(clippy::many_single_char_names)] // n, d, r, x, y as in mesh2cad's parameter names

use std::f64::consts::PI;

use crate::recognize::HoleSpec;

/// How far a hole may sit from where a pattern puts it, in mm. Recognised positions are exact to
/// a micrometre; this allows a few of those.
pub const TOLERANCE: f64 = 0.005;

/// A plate's holes, grouped.
#[derive(Debug, Clone, PartialEq)]
pub enum HoleGroup {
    /// A hole in no pattern.
    Single(HoleSpec),
    /// `n` holes from `(x, y)` at `pitch` along the direction `angle` (degrees, from +X).
    Row {
        /// Count.
        n: usize,
        /// Centre to centre.
        pitch: f64,
        /// Diameter.
        d: f64,
        /// First hole's centre X.
        x: f64,
        /// First hole's centre Y.
        y: f64,
        /// Direction from the first hole, degrees from +X, in (-90, 90].
        angle: f64,
    },
    /// `nx` by `ny` holes from the corner `(x, y)` along +X and +Y.
    Grid {
        /// Count along X.
        nx: usize,
        /// Count along Y.
        ny: usize,
        /// Spacing along X.
        pitch_x: f64,
        /// Spacing along Y.
        pitch_y: f64,
        /// Diameter.
        d: f64,
        /// Corner hole's centre X (the lowest X and Y).
        x: f64,
        /// Corner hole's centre Y.
        y: f64,
    },
    /// `n` holes equally spaced on a circle of radius `r` about `(cx, cy)`.
    Circle {
        /// Count.
        n: usize,
        /// Diameter.
        d: f64,
        /// Pitch circle radius, centre to hole centre.
        r: f64,
        /// Pitch circle centre X.
        cx: f64,
        /// Pitch circle centre Y.
        cy: f64,
        /// Angle of the first hole, degrees from +X, in (-180/n, 180/n].
        angle: f64,
    },
}

/// Groups `holes` into patterns, keeping the order of their first holes. Holes in no pattern
/// come out as [`HoleGroup::Single`], in their order.
#[must_use]
pub fn group_holes(holes: &[HoleSpec]) -> Vec<HoleGroup> {
    // Holes of one diameter, by their index in `holes`.
    let mut buckets: Vec<Vec<usize>> = Vec::new();
    for (i, hole) in holes.iter().enumerate() {
        // As mesh2cad does, counterbored and countersunk holes stay single holes.
        if hole.head.is_some() || hole.blind.is_some() {
            buckets.push(vec![i]);
            continue;
        }
        match buckets
            .iter_mut()
            .find(|b| holes[b[0]].head.is_none() && holes[b[0]].blind.is_none() && (holes[b[0]].d - hole.d).abs() <= TOLERANCE)
        {
            Some(bucket) => bucket.push(i),
            None => buckets.push(vec![i]),
        }
    }
    // Each group with the index of its first hole, so that the groups, and the single holes
    // among them, keep the order of `holes`.
    let mut groups: Vec<(usize, HoleGroup)> = Vec::new();
    for bucket in buckets {
        let centres: Vec<[f64; 2]> = bucket.iter().map(|&i| [holes[i].x, holes[i].y]).collect();
        let d = holes[bucket[0]].d;
        if let Some(group) = grid(&centres, d).or_else(|| row(&centres, d)).or_else(|| circle(&centres, d)) {
            groups.push((bucket[0], group));
        } else {
            groups.extend(bucket.into_iter().map(|i| (i, HoleGroup::Single(holes[i].clone()))));
        }
    }
    groups.sort_by_key(|(first, _)| *first);
    groups.into_iter().map(|(_, group)| group).collect()
}

/// Whether every one of `centres` lies within the tolerance of one of `predicted`, one each.
fn matches(centres: &[[f64; 2]], predicted: &[[f64; 2]]) -> bool {
    let mut used = vec![false; predicted.len()];
    centres.len() == predicted.len()
        && centres.iter().all(|c| {
            let hit =
                predicted.iter().enumerate().find(|(k, p)| !used[*k] && (p[0] - c[0]).hypot(p[1] - c[1]) <= TOLERANCE).map(|(k, _)| k);
            hit.inspect(|&k| used[k] = true).is_some()
        })
}

/// The holes `group` stands for, from its own (rounded) numbers.
fn predicted(group: &HoleGroup) -> Vec<[f64; 2]> {
    match *group {
        HoleGroup::Single(ref h) => vec![[h.x, h.y]],
        HoleGroup::Row { n, pitch, x, y, angle, .. } => {
            let (sin, cos) = angle.to_radians().sin_cos();
            (0..n).map(|i| [x + i as f64 * pitch * cos, y + i as f64 * pitch * sin]).collect()
        }
        HoleGroup::Grid { nx, ny, pitch_x, pitch_y, x, y, .. } => {
            (0..ny).flat_map(|j| (0..nx).map(move |i| [x + i as f64 * pitch_x, y + j as f64 * pitch_y])).collect()
        }
        HoleGroup::Circle { n, r, cx, cy, angle, .. } => {
            (0..n).map(|k| (angle + 360.0 * k as f64 / n as f64).to_radians()).map(|a| [cx + r * a.cos(), cy + r * a.sin()]).collect()
        }
    }
}

/// `rounded` if the holes it stands for are still `centres` (rounding a pitch or an angle adds
/// up along a long row), else `exact` if that is, else no pattern: a set whose gaps are each
/// near enough can still drift from the pattern as a whole.
fn keep_rounded(centres: &[[f64; 2]], rounded: HoleGroup, exact: HoleGroup) -> Option<HoleGroup> {
    [rounded, exact].into_iter().find(|group| matches(centres, &predicted(group)))
}

/// `degrees` rounded to the coarsest of 1°, 0.5°, 0.1°, 0.01° and 0.001° that moves a point at
/// `reach` mm from the pivot by no more than the tolerance: hole centres are exact to a
/// micrometre, so an angle measured between them carries noise in its third decimal.
fn snap_angle(degrees: f64, reach: f64) -> f64 {
    for step in [1.0, 0.5, 0.1, 0.01, 0.001] {
        let rounded = (degrees / step).round() * step;
        if ((rounded - degrees).to_radians() * reach).abs() <= TOLERANCE {
            return if rounded == 0.0 { 0.0 } else { rounded };
        }
    }
    degrees
}

fn snap(value: f64) -> f64 {
    let snapped = (value * 1000.0).round() / 1000.0;
    if snapped == 0.0 { 0.0 } else { snapped }
}

/// The distinct values among `values` (within the tolerance), sorted.
fn levels(values: impl Iterator<Item = f64>) -> Vec<f64> {
    let mut sorted: Vec<f64> = values.collect();
    sorted.sort_by(f64::total_cmp);
    let mut out: Vec<f64> = Vec::new();
    for v in sorted {
        if out.last().is_none_or(|last| v - last > TOLERANCE) {
            out.push(v);
        }
    }
    out
}

/// The common step of sorted `values`, if they are equally spaced.
fn even_step(values: &[f64]) -> Option<f64> {
    let step = (values[values.len() - 1] - values[0]) / (values.len() - 1) as f64;
    values.iter().enumerate().all(|(i, v)| (values[0] + step * i as f64 - v).abs() <= TOLERANCE).then_some(step)
}

fn grid(centres: &[[f64; 2]], d: f64) -> Option<HoleGroup> {
    let xs = levels(centres.iter().map(|c| c[0]));
    let ys = levels(centres.iter().map(|c| c[1]));
    if xs.len() < 2 || ys.len() < 2 || xs.len() * ys.len() != centres.len() {
        return None;
    }
    let (pitch_x, pitch_y) = (even_step(&xs)?, even_step(&ys)?);
    let mut seen = vec![false; centres.len()];
    for c in centres {
        let i = xs.iter().position(|x| (x - c[0]).abs() <= TOLERANCE)?;
        let j = ys.iter().position(|y| (y - c[1]).abs() <= TOLERANCE)?;
        if std::mem::replace(&mut seen[j * xs.len() + i], true) {
            return None;
        }
    }
    let (nx, ny, x, y) = (xs.len(), ys.len(), xs[0], ys[0]);
    keep_rounded(
        centres,
        HoleGroup::Grid { nx, ny, pitch_x: snap(pitch_x), pitch_y: snap(pitch_y), d, x, y },
        HoleGroup::Grid { nx, ny, pitch_x, pitch_y, d, x, y },
    )
}

fn row(centres: &[[f64; 2]], d: f64) -> Option<HoleGroup> {
    let n = centres.len();
    if n < 3 {
        return None;
    }
    // The two holes farthest apart are the row's ends.
    let (mut a, mut b, mut far) = (0, 0, 0.0);
    for i in 0..n {
        for j in i + 1..n {
            let dist = (centres[i][0] - centres[j][0]).hypot(centres[i][1] - centres[j][1]);
            if dist > far {
                (a, b, far) = (i, j, dist);
            }
        }
    }
    if far <= TOLERANCE {
        return None;
    }
    let mut dir = [(centres[b][0] - centres[a][0]) / far, (centres[b][1] - centres[a][1]) / far];
    // Run from left to right (or upwards along a vertical row), so the angle is in (-90, 90].
    if dir[0] < -1e-12 || (dir[0].abs() <= 1e-12 && dir[1] < 0.0) {
        dir = [-dir[0], -dir[1]];
    }
    let start = if (centres[b][0] - centres[a][0]) * dir[0] + (centres[b][1] - centres[a][1]) * dir[1] > 0.0 { a } else { b };
    let origin = centres[start];
    let mut along = Vec::with_capacity(n);
    for c in centres {
        let (dx, dy) = (c[0] - origin[0], c[1] - origin[1]);
        if (dx * dir[1] - dy * dir[0]).abs() > TOLERANCE {
            return None;
        }
        along.push(dx * dir[0] + dy * dir[1]);
    }
    along.sort_by(f64::total_cmp);
    let pitch = even_step(&along)?;
    let angle = dir[1].atan2(dir[0]).to_degrees();
    let (x, y) = (origin[0], origin[1]);
    keep_rounded(
        centres,
        HoleGroup::Row { n, pitch: snap(pitch), d, x, y, angle: snap_angle(angle, far) },
        HoleGroup::Row { n, pitch, d, x, y, angle },
    )
}

fn circle(centres: &[[f64; 2]], d: f64) -> Option<HoleGroup> {
    let n = centres.len();
    if n < 3 {
        return None;
    }
    let centre = [centres.iter().map(|c| c[0]).sum::<f64>() / n as f64, centres.iter().map(|c| c[1]).sum::<f64>() / n as f64];
    let radii: Vec<f64> = centres.iter().map(|c| (c[0] - centre[0]).hypot(c[1] - centre[1])).collect();
    let r = radii.iter().sum::<f64>() / n as f64;
    if r <= TOLERANCE || radii.iter().any(|q| (q - r).abs() > TOLERANCE) {
        return None;
    }
    let mut angles: Vec<f64> = centres.iter().map(|c| (c[1] - centre[1]).atan2(c[0] - centre[0])).collect();
    angles.sort_by(f64::total_cmp);
    let step = 2.0 * PI / n as f64;
    // Equal spacing: every gap, the wrap-around one included, is one step (as an arc length).
    let gaps = (0..n).map(|i| if i + 1 < n { angles[i + 1] - angles[i] } else { angles[0] + 2.0 * PI - angles[i] });
    if gaps.into_iter().any(|g| ((g - step) * r).abs() > TOLERANCE) {
        return None;
    }
    // The first hole is the one nearest +X, so the angle is in (-180/n, 180/n].
    let first = angles.iter().copied().min_by(|p, q| p.abs().total_cmp(&q.abs())).unwrap_or(0.0);
    let half = 180.0 / n as f64;
    let mut angle = first.to_degrees();
    if angle <= -half + 1e-9 {
        angle += 2.0 * half;
    }
    keep_rounded(
        centres,
        HoleGroup::Circle { n, d, r: snap(r), cx: snap(centre[0]), cy: snap(centre[1]), angle: snap_angle(angle, r) },
        HoleGroup::Circle { n, d, r, cx: centre[0], cy: centre[1], angle },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn holes(centres: &[(f64, f64)], d: f64) -> Vec<HoleSpec> {
        centres.iter().map(|&(x, y)| HoleSpec { x, y, d, head: None, blind: None }).collect()
    }

    fn on_circle(n: usize, r: f64, centre: (f64, f64), start_deg: f64, d: f64) -> Vec<HoleSpec> {
        (0..n)
            .map(|i| (start_deg + 360.0 * i as f64 / n as f64).to_radians())
            .map(|a| HoleSpec { x: snap(centre.0 + r * a.cos()), y: snap(centre.1 + r * a.sin()), d, head: None, blind: None })
            .collect()
    }

    #[test]
    fn finds_a_grid() {
        let found = group_holes(&holes(&[(10.0, 10.0), (30.0, 10.0), (50.0, 10.0), (10.0, 25.0), (30.0, 25.0), (50.0, 25.0)], 5.0));
        assert_eq!(found, [HoleGroup::Grid { nx: 3, ny: 2, pitch_x: 20.0, pitch_y: 15.0, d: 5.0, x: 10.0, y: 10.0 }]);
    }

    #[test]
    fn a_square_is_a_grid_not_a_circle() {
        let found = group_holes(&holes(&[(0.0, 0.0), (10.0, 0.0), (0.0, 10.0), (10.0, 10.0)], 3.0));
        assert!(matches!(found[..], [HoleGroup::Grid { nx: 2, ny: 2, .. }]), "{found:?}");
    }

    #[test]
    fn finds_rows_in_any_direction() {
        let found = group_holes(&holes(&[(40.0, 10.0), (10.0, 10.0), (25.0, 10.0)], 4.0));
        assert_eq!(found, [HoleGroup::Row { n: 3, pitch: 15.0, d: 4.0, x: 10.0, y: 10.0, angle: 0.0 }]);
        let found = group_holes(&holes(&[(30.0, 30.0), (10.0, 10.0), (20.0, 20.0), (40.0, 40.0)], 4.0));
        assert_eq!(found, [HoleGroup::Row { n: 4, pitch: 14.142, d: 4.0, x: 10.0, y: 10.0, angle: 45.0 }]);
        let found = group_holes(&holes(&[(5.0, 30.0), (5.0, 10.0), (5.0, 20.0)], 4.0));
        assert!(matches!(found[..], [HoleGroup::Row { x: 5.0, y: 10.0, angle: 90.0, .. }]), "{found:?}");
        let found = group_holes(&holes(&[(0.0, 20.0), (10.0, 10.0), (20.0, 0.0)], 4.0));
        assert!(matches!(found[..], [HoleGroup::Row { x: 0.0, y: 20.0, angle: -45.0, .. }]), "{found:?}");
    }

    #[test]
    fn finds_bolt_circles() {
        let found = group_holes(&on_circle(6, 15.0, (60.0, 40.0), 15.0, 4.0));
        assert_eq!(found, [HoleGroup::Circle { n: 6, d: 4.0, r: 15.0, cx: 60.0, cy: 40.0, angle: 15.0 }]);
        // The first hole is the one nearest +X: a start of 100 degrees on 3 holes is -20.
        let found = group_holes(&on_circle(3, 10.0, (0.0, 0.0), 100.0, 3.0));
        assert!(matches!(found[..], [HoleGroup::Circle { n: 3, angle, .. }] if (angle + 20.0).abs() < 1e-9), "{found:?}");
    }

    #[test]
    fn keeps_singles_where_there_is_no_pattern() {
        // Two holes are no pattern.
        assert!(group_holes(&holes(&[(0.0, 0.0), (10.0, 0.0)], 3.0)).iter().all(|g| matches!(g, HoleGroup::Single(_))));
        // Uneven pitch.
        assert!(group_holes(&holes(&[(0.0, 0.0), (10.0, 0.0), (25.0, 0.0)], 3.0)).iter().all(|g| matches!(g, HoleGroup::Single(_))));
        // A bolt circle with one hole missing.
        let mut circle = on_circle(6, 15.0, (0.0, 0.0), 0.0, 4.0);
        circle.remove(2);
        assert!(group_holes(&circle).iter().all(|g| matches!(g, HoleGroup::Single(_))));
        // A grid with one hole missing.
        assert!(
            group_holes(&holes(&[(0.0, 0.0), (10.0, 0.0), (0.0, 10.0), (10.0, 10.0), (20.0, 0.0)], 3.0))
                .iter()
                .all(|g| matches!(g, HoleGroup::Single(_)))
        );
    }

    #[test]
    fn single_holes_keep_their_order() {
        let all = vec![
            HoleSpec { x: 0.0, y: 0.0, d: 3.0, head: None, blind: None },
            HoleSpec { x: 10.0, y: 5.0, d: 4.0, head: None, blind: None },
            HoleSpec { x: 30.0, y: 0.0, d: 3.0, head: None, blind: None },
        ];
        let found = group_holes(&all);
        assert_eq!(found, all.into_iter().map(HoleGroup::Single).collect::<Vec<_>>());
    }

    /// Rounding the pitch of a long row to a micrometre would move its last holes by more than
    /// the tolerance; the exact pitch is kept then.
    #[test]
    fn long_rows_keep_an_exact_pitch() {
        let centres: Vec<(f64, f64)> = (0..60).map(|i| f64::from(i) * 10.000_4).map(|v| (snap(v), snap(v))).collect();
        let found = group_holes(&holes(&centres, 2.0));
        let [ref group @ HoleGroup::Row { n: 60, .. }] = found[..] else { panic!("{found:?}") };
        let points: Vec<[f64; 2]> = centres.iter().map(|&(x, y)| [x, y]).collect();
        assert!(matches(&points, &predicted(group)), "{group:?}");
    }

    /// Every gap of this circle is within the tolerance of 30°, but the holes drift up to 28 µm
    /// from a regular bolt circle, so it is no pattern.
    #[test]
    fn a_drifting_circle_is_no_pattern() {
        let n = 12;
        let circle: Vec<HoleSpec> = (0..n)
            .map(|k: usize| 30.0 * k as f64 + 0.0135 * k.min(n - k) as f64)
            .map(f64::to_radians)
            .map(|a| HoleSpec { x: snap(20.0 * a.cos()), y: snap(20.0 * a.sin()), d: 3.0, head: None, blind: None })
            .collect();
        let found = group_holes(&circle);
        assert!(found.iter().all(|g| matches!(g, HoleGroup::Single(_))), "{found:?}");
    }

    #[test]
    fn counterbored_holes_stay_single() {
        let mut row = holes(&[(10.0, 10.0), (25.0, 10.0), (40.0, 10.0)], 4.0);
        row[1].head = Some(crate::recognize::HoleHead::Counterbore { d: 8.0, depth: 2.0, bottom: false });
        assert!(group_holes(&row).iter().all(|g| matches!(g, HoleGroup::Single(_))));
    }

    #[test]
    fn blind_holes_stay_single() {
        let mut row = holes(&[(10.0, 10.0), (25.0, 10.0), (40.0, 10.0)], 4.0);
        for hole in &mut row {
            hole.blind = Some(crate::recognize::Blind { depth: 3.0, bottom: false });
        }
        assert!(group_holes(&row).iter().all(|g| matches!(g, HoleGroup::Single(_))));
    }

    #[test]
    fn groups_by_diameter() {
        let mut all = holes(&[(10.0, 10.0), (110.0, 10.0), (10.0, 70.0), (110.0, 70.0)], 5.0);
        all.extend(on_circle(6, 15.0, (60.0, 40.0), 0.0, 4.0));
        all.push(HoleSpec { x: 60.0, y: 40.0, d: 10.0, head: None, blind: None });
        let found = group_holes(&all);
        assert!(matches!(found[0], HoleGroup::Grid { nx: 2, ny: 2, .. }), "{found:?}");
        assert!(matches!(found[1], HoleGroup::Circle { n: 6, .. }), "{found:?}");
        assert_eq!(found[2], HoleGroup::Single(HoleSpec { x: 60.0, y: 40.0, d: 10.0, head: None, blind: None }));
    }
}
