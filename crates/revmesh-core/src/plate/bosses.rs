//! What stands on the top face: round bosses and rectangular pads, stripped off so the plate
//! below can be recognised.

use super::geometry::{boundary_loops, signed_area};
use super::holes::{axis_rectangle, drop_collinear, round_hole};
use super::{CHORD_FACET_SPAN, P2, tolerance};
use crate::mesh::{Mesh, Vec3, cross, length, sub};

use super::holes::Footprint;
/// A round boss as found, in the file's coordinates: its axis, radius, the height of its top,
/// its footprint loop in the plate's top face as meshed, and that loop without the corners an
/// exporter may have added on its facets (the chords its wall follows).
pub(super) struct FoundBoss {
    pub(super) foot: Footprint,
    pub(super) top: f64,
    /// The radius of the round at its foot, 0 without one, and the cylinder's radius above it
    /// (the footprint's, by the round's radius less where the round's first, nearly flat
    /// facets lie in the top face).
    pub(super) fillet: f64,
    pub(super) wall_r: f64,
    pub(super) footprint: Vec<P2>,
    pub(super) ring: Vec<P2>,
}

pub(super) const NO_BOSS: &str = "something stands on the top face that is neither a straight round boss nor a rectangular pad \
                       (other shapes on the top face are not recognised yet)";

/// The plate under round bosses: the mesh without what stands on its top face, that face closed
/// over each boss's footprint, and the bosses. The top face is a flat face facing up, below the
/// highest; each such face is tried, the largest first (a boss's top may be larger than what
/// is left of the plate's). `Ok(None)`: nothing stands on round footprints in any of them.
pub(super) fn strip_bosses(mesh: &Mesh) -> Result<Option<(Mesh, Vec<FoundBoss>)>, String> {
    let Some(bounds) = mesh.bounds() else { return Ok(None) };
    let size = bounds.size();
    let tol = tolerance(size[0].max(size[1]).max(size[2]));
    let mut levels: Vec<(f64, f64)> = Vec::new();
    for t in &mesh.triangles {
        let (len, nz) = facing(t);
        if let (true, Some(z)) = (nz > 0.5, level(t, tol)) {
            match levels.iter_mut().find(|(at, _)| (at - z).abs() <= tol) {
                Some((_, area)) => *area += len / 2.0,
                None => levels.push((z, len / 2.0)),
            }
        }
    }
    levels.retain(|(z, _)| *z < bounds.max[2] - tol);
    levels.sort_by(|a, b| b.1.total_cmp(&a.1));
    let mut refused = None;
    for (top_z, _) in levels {
        match strip_bosses_at(mesh, top_z, tol) {
            Ok(Some(found)) => return Ok(Some(found)),
            Ok(None) => {}
            Err(why) => refused = refused.or(Some(why)),
        }
    }
    refused.map_or(Ok(None), Err)
}

/// A facet's area (twice it) and how far its normal points up.
pub(super) fn facing(t: &crate::mesh::Triangle) -> (f64, f64) {
    let n = cross(sub(t[1], t[0]), sub(t[2], t[0]));
    let len = length(n);
    (len, if len > 0.0 { n[2] / len } else { 0.0 })
}

/// The height of a facet that lies flat, within `tol`.
pub(super) fn level(t: &crate::mesh::Triangle, tol: f64) -> Option<f64> {
    let (lo, hi) = t.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| (lo.min(p[2]), hi.max(p[2])));
    (hi - lo <= tol).then_some(f64::midpoint(lo, hi))
}

/// Whether the facet `t` lies on the filleted round boss `b` standing on the top face at
/// `top_z`: every corner and every edge's middle on its cylinder (above the round), its round (a
/// quarter circle's outside swept about the axis) or its flat top, the middles within their own
/// chord's sagitta.
fn filleted_boss_holds(b: &FoundBoss, t: &crate::mesh::Triangle, top_z: f64, tol: f64) -> bool {
    let Footprint::Round { centre, r: foot } = b.foot else { return false };
    let (f, r) = (b.fillet, b.wall_r);
    // An edge round the axis on the wall or the round spans no more than two of the footprint's
    // facets: a longer one cuts through the wall (the top's may cross it).
    let longest = super::geometry::segments(&b.ring).map(|(p, q)| super::geometry::distance(p, q)).fold(0.0, f64::max);
    let off = |p: Vec3, slack: f64| {
        let rho = super::geometry::distance([p[0], p[1]], centre);
        let z = p[2];
        let wall = if z >= top_z + f - slack && z <= b.top + slack { (rho - r).abs() } else { f64::INFINITY };
        let round =
            if rho >= r - slack && z <= top_z + f + slack { ((rho - (r + f)).hypot(z - (top_z + f)) - f).abs() } else { f64::INFINITY };
        let lid = if rho <= r + slack { (z - b.top).abs() } else { f64::INFINITY };
        wall.min(round).min(lid) <= slack
    };
    // A corner an exporter adds within a wall facet lies on one of the wall's chords round the
    // axis (the footprint's loop, drawn at the cylinder's radius), not on the circle.
    let wall_loop = cylinder_loop(b, centre, foot, r);
    let on_wall_chord =
        |p: &Vec3| p[2] >= top_z + f - tol && p[2] <= b.top + tol && super::walls::on_wall([p[0], p[1]], centre, r, &wall_loop, tol);
    let corners = t.iter().all(|p| off(*p, tol) || on_wall_chord(p));
    // An edge along the round (both ends no higher than its top) bows like a chord of its
    // quarter circle, and spans no more of it than a round hole's facet may of its circle (a
    // quarter in one step is a chamfer); any other edge bows like a chord of the cylinder round
    // the axis (a straight edge up the wall not at all).
    let middles = (0..3).all(|i| {
        let (p, q) = (t[i], t[(i + 1) % 3]);
        // No edge runs from the round past its top: there the wall would be a cone.
        let (low, high) = (p[2].min(q[2]), p[2].max(q[2]));
        if low < top_z + f - tol && high > top_z + f + tol {
            return false;
        }
        let across = (p[0] - q[0]).hypot(p[1] - q[1]);
        let on_lid = (p[2] - b.top).abs() <= tol && (q[2] - b.top).abs() <= tol;
        if across > 2.0 * longest + tol && !on_lid {
            return false;
        }
        let on_round = p[2].max(q[2]) <= top_z + f + tol;
        let (radial, rise) =
            (super::geometry::distance([p[0], p[1]], centre) - super::geometry::distance([q[0], q[1]], centre), p[2] - q[2]);
        let along = radial.hypot(rise);
        // How far the middle of a chord `c` long lies inside a circle of radius `radius`.
        let sagitta = |c: f64, radius: f64| radius - (radius * radius - (c * c / 4.0).min(radius * radius)).sqrt();
        let bow = if on_round {
            if along > 2.0 * f * (super::MAX_HOLE_STEP / 2.0).sin() + tol {
                return false;
            }
            sagitta(along, f) + sagitta(across, r)
        } else {
            sagitta(across, r)
        };
        let middle = [f64::midpoint(p[0], q[0]), f64::midpoint(p[1], q[1]), f64::midpoint(p[2], q[2])];
        off(middle, tol + bow) || on_wall_chord(&middle)
    });
    corners && middles
}

/// The footprint's loop (as meshed, without the corners an exporter added along it) drawn at the
/// radius `r` of the cylinder above a round of foot radius `foot`: the chords the cylinder's
/// facets run along.
fn cylinder_loop(b: &FoundBoss, centre: P2, foot: f64, r: f64) -> Vec<P2> {
    b.ring.iter().map(|q| [centre[0] + (q[0] - centre[0]) * r / foot, centre[1] + (q[1] - centre[1]) * r / foot]).collect()
}

/// Whether the facets `top` (facing up, seen from above) of the boss `b` cover its top: one
/// loop round them, every corner of it on the top's rim. A filleted boss's rim is its cylinder's
/// (the footprint's loop drawn at that radius); any other's its footprint.
fn top_covers(b: &FoundBoss, top: &[[P2; 3]], tol: f64) -> bool {
    match b.foot {
        Footprint::Round { centre, r: foot } if b.fillet > 0.0 => {
            let r = b.wall_r;
            let rim = cylinder_loop(b, centre, foot, r);
            let Ok(loops) = boundary_loops(top, tol) else { return false };
            loops.len() == 1 && loops[0].iter().all(|q| super::walls::on_wall(*q, centre, r, &rim, tol))
        }
        _ => {
            let area: f64 = top.iter().map(|t| signed_area(t)).sum();
            (area - b.foot.area(&b.ring)).abs() <= 1e-3 * area.max(tol)
        }
    }
}

/// [`strip_bosses`] with the top face at `top_z`: a boss's footprint is a round loop in that
/// face, its wall stands straight on that loop (within the facets' sagitta), and its top is one
/// flat face.
pub(super) fn strip_bosses_at(mesh: &Mesh, top_z: f64, tol: f64) -> Result<Option<(Mesh, Vec<FoundBoss>)>, String> {
    let flat = |p: &Vec3| [p[0], p[1]];
    // The face's own height: the one most of its area lies at (a round's first, nearly flat
    // facets may lie within the tolerance of it, a little higher).
    let mut heights: Vec<(f64, f64)> = Vec::new();
    for t in &mesh.triangles {
        let (len, nz) = facing(t);
        if nz > 0.5 && level(t, tol).is_some_and(|z| (z - top_z).abs() <= tol) {
            for p in t {
                match heights.iter_mut().find(|(z, _)| (z - p[2]).abs() <= tol * 1e-3) {
                    Some((_, area)) => *area += len,
                    None => heights.push((p[2], len)),
                }
            }
        }
    }
    let top_z = heights.iter().max_by(|a, b| a.1.total_cmp(&b.1)).map_or(top_z, |(z, _)| *z);
    let top: Vec<[P2; 3]> = mesh
        .triangles
        .iter()
        .filter(|t| facing(t).1 > 0.5 && level(t, tol).is_some_and(|z| (z - top_z).abs() <= tol))
        .map(|t| t.map(|p| flat(&p)))
        .collect();
    let Ok(loops) = boundary_loops(&top, tol) else { return Ok(None) };
    let above = |p: &Vec3| p[2] > top_z + tol;
    let upper: Vec<&crate::mesh::Triangle> = mesh.triangles.iter().filter(|t| t.iter().any(above)).collect();
    let mut bosses = Vec::new();
    for ring in loops.iter().filter(|l| signed_area(l) < 0.0) {
        let foot = match round_hole(ring, tol) {
            Ok((_, (centre, r))) => Footprint::Round { centre, r },
            Err(_) => match axis_rectangle(ring, tol) {
                Some((lo, hi)) => Footprint::Rect { lo, hi },
                None => continue,
            },
        };
        let on = |p: &Vec3| foot.contains(flat(p), tol);
        if let Some(top) = upper.iter().flat_map(|t| t.iter()).filter(|p| on(p)).map(|p| p[2]).reduce(f64::max) {
            // Its walls follow the loop's chords as meshed, before any exporter split them.
            // A round foot: the top's rim lies closer to the axis than the footprint, by the
            // round's radius.
            let (fillet, wall_r) = match foot {
                Footprint::Round { centre, r } => {
                    let rim = upper
                        .iter()
                        .flat_map(|t| t.iter())
                        .filter(|p| on(p) && (p[2] - top).abs() <= tol)
                        .map(|p| super::geometry::distance(flat(p), centre))
                        .fold(0.0, f64::max);
                    if r - rim > tol {
                        // The round's radius from its own corners: one `a` out from the cylinder
                        // and `h` up lies on the quarter circle of radius `a + h + sqrt(2ah)`.
                        let samples: Vec<f64> = upper
                            .iter()
                            .flat_map(|t| t.iter())
                            .filter(|p| on(p) && p[2] > top_z + tol && p[2] < top - tol)
                            .map(|p| (super::geometry::distance(flat(p), centre) - rim, p[2] - top_z))
                            .filter(|(a, _)| *a > tol)
                            .map(|(a, h)| a + h + (2.0 * a * h).sqrt())
                            .collect();
                        let f = if samples.is_empty() { r - rim } else { samples.iter().sum::<f64>() / samples.len() as f64 };
                        (f, rim)
                    } else {
                        (0.0, r)
                    }
                }
                Footprint::Rect { .. } => (0.0, 0.0),
            };
            bosses.push(FoundBoss { foot, top, fillet, wall_r, footprint: ring.clone(), ring: drop_collinear(ring, tol) });
        }
    }
    if bosses.is_empty() {
        return Ok(None);
    }
    // Everything above the top face belongs to one boss: a facet of its wall, every corner and
    // every edge's middle on the wall (an edge two facets long at most), or of its flat top.
    let mut tops: Vec<Vec<[P2; 3]>> = vec![Vec::new(); bosses.len()];
    for t in &upper {
        let k = bosses.iter().position(|b| t.iter().all(|p| b.foot.contains(flat(p), tol))).ok_or(NO_BOSS)?;
        let b = &bosses[k];
        let (_, nz) = facing(t);
        if b.fillet > 0.0 {
            if !filleted_boss_holds(b, t, top_z, tol) {
                return Err(NO_BOSS.into());
            }
            if t.iter().all(|p| (p[2] - b.top).abs() <= tol) {
                tops[k].push(t.map(|p| flat(&p)));
            }
            continue;
        }
        if nz > 0.5 {
            if t.iter().any(|p| (p[2] - b.top).abs() > tol) {
                return Err(NO_BOSS.into());
            }
            tops[k].push(t.map(|p| flat(&p)));
        } else if nz < -0.5 {
            return Err(NO_BOSS.into());
        } else {
            let between = |p: &Vec3| p[2] >= top_z - tol && p[2] <= b.top + tol;
            let corner_on = |p: &Vec3| b.foot.on_wall(flat(p), &b.ring, tol) && between(p);
            let middle = |i: usize| {
                let (p, q) = (t[i], t[(i + 1) % 3]);
                [f64::midpoint(p[0], q[0]), f64::midpoint(p[1], q[1]), f64::midpoint(p[2], q[2])]
            };
            let middle_on = |p: Vec3| between(&p) && b.foot.near_wall(flat(&p), CHORD_FACET_SPAN * b.foot.sagitta(&b.ring), tol);
            if !t.iter().all(corner_on) || !(0..3).all(|i| middle_on(middle(i))) {
                return Err(NO_BOSS.into());
            }
        }
    }
    // Each top covers its footprint or, for a filleted boss, its cylinder (the two may be meshed
    // apart, with their own corners).
    for (b, top) in bosses.iter().zip(&tops) {
        let covers = top_covers(b, top, tol);
        if b.top - top_z <= tol + b.fillet || !covers {
            return Err(NO_BOSS.into());
        }
    }
    // What lies within the tolerance of the face lies on it.
    let mut triangles: Vec<crate::mesh::Triangle> = mesh
        .triangles
        .iter()
        .filter(|t| t.iter().all(|p| p[2] <= top_z + tol))
        .map(|t| t.map(|p| if (p[2] - top_z).abs() <= tol { [p[0], p[1], top_z] } else { p }))
        .collect();
    // Each footprint is closed by a fan from its centre over its corners as meshed, so that the
    // top face's loop is joined edge for edge.
    for b in &bosses {
        let middle = b.foot.middle();
        let (c, n) = ([middle[0], middle[1], top_z], b.footprint.len());
        for i in 0..n {
            // The footprint runs clockwise (a hole in the top face): its fan, seen from above,
            // runs the other way.
            let (p, q) = (b.footprint[(i + 1) % n], b.footprint[i]);
            triangles.push([c, [p[0], p[1], top_z], [q[0], q[1], top_z]]);
        }
    }
    Ok(Some((Mesh::new(triangles), bosses)))
}
