//! Comparing a rebuilt part with the mesh it was recognised from.
//!
//! mesh2cad checks every script it writes by rebuilding the part and measuring how far the
//! rebuilt solid's surface lies from the input mesh's (the Hausdorff distance of the two
//! triangulations, plus the volume error), refusing the script when the deviation is over
//! `max(4 * tol, 0.1)` mm or the volume error over 1 %, with `tol` 0.1 % of the bounding box's
//! diagonal. revmesh does the same in `cargo xtask check`: the importer's design is evaluated
//! and exported again by SolidCraft (`solidcraft-cli eval`, `solidcraft-cli run --out`), and
//! this module compares the exported mesh with the source STL. Sampling is deterministic (a
//! fixed low-discrepancy sequence), as the plugin sandbox has no randomness.

#![allow(clippy::many_single_char_names)] // points a, b, c, p, as in the geometry
use std::collections::HashMap;

use crate::mesh::{Mesh, Vec3, cross, dot, length, sub};

/// Surface points sampled on each mesh, as mesh2cad does.
pub const SAMPLES: usize = 4000;

/// Largest volume error of a rebuild, in percent, as mesh2cad's gate allows.
pub const MAX_VOLUME_ERROR_PCT: f64 = 1.0;

/// The fit tolerance, as a share of the source mesh's bounding box diagonal (mesh2cad's default).
pub const TOL_SHARE: f64 = 1e-3;

/// Smallest deviation limit in mm (mesh2cad's `max(4 * tol, 0.1)`).
pub const MIN_DEVIATION_LIMIT: f64 = 0.1;

/// The surface deviation between two meshes: the largest and the mean distance from sampled
/// points on each mesh to the other mesh, in mm.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Deviation {
    /// The Hausdorff distance: the worst point on either mesh.
    pub max: f64,
    /// The mean distance of the sampled points.
    pub mean: f64,
    /// The worst distance from `a`'s surface to `b`.
    pub a_to_b: f64,
    /// The worst distance from `b`'s surface to `a`.
    pub b_to_a: f64,
}

/// The deviation of a rebuilt mesh from the source mesh it was recognised from, sampled both ways.
#[must_use]
pub fn hausdorff(a: &Mesh, b: &Mesh, samples: usize) -> Deviation {
    let grid_b = TriGrid::new(b);
    let grid_a = TriGrid::new(a);
    let da = sample_distances(a, b, &grid_b, samples);
    let db = sample_distances(b, a, &grid_a, samples);
    let worst = |d: &[f64]| d.iter().copied().fold(0.0, f64::max);
    let mean = |d: &[f64]| d.iter().sum::<f64>() / d.len().max(1) as f64;
    Deviation { max: worst(&da).max(worst(&db)), mean: f64::midpoint(mean(&da), mean(&db)), a_to_b: worst(&da), b_to_a: worst(&db) }
}

/// The distances from `samples` points sampled on `mesh`'s surface to `other`'s surface (`grid`
/// is `other`'s index, so the distances are measured against `other`'s triangles).
fn sample_distances(mesh: &Mesh, other: &Mesh, grid: &TriGrid, samples: usize) -> Vec<f64> {
    sample_points(mesh, samples).iter().map(|p| grid.distance(other, *p)).collect()
}

/// `samples` points on the mesh's surface, spread by area, at fixed low-discrepancy positions
/// inside each triangle (deterministic: no randomness).
fn sample_points(mesh: &Mesh, samples: usize) -> Vec<Vec3> {
    let areas: Vec<f64> = mesh.triangles.iter().map(triangle_area).collect();
    let total: f64 = areas.iter().sum();
    if total <= 0.0 || samples == 0 {
        return Vec::new();
    }
    let mut cumulative = Vec::with_capacity(areas.len());
    let mut acc = 0.0;
    for a in &areas {
        acc += *a;
        cumulative.push(acc);
    }
    let mut out = Vec::with_capacity(samples);
    for k in 0..samples {
        let target = (k as f64 + 0.5) / samples as f64 * total;
        let i = cumulative.partition_point(|&c| c < target).min(areas.len() - 1);
        let [a, b, c] = mesh.triangles[i];
        // Low-discrepancy fractions, fixed: the same samples every run.
        let r1 = frac((k as f64 + 0.5) * 0.754_877_666_246_692_6);
        let r2 = frac((k as f64 + 0.5) * 0.569_840_290_998_053_4);
        let (s, t) = (1.0 - r1.sqrt(), r2 * r1.sqrt());
        out.push([
            a[0] + s * (b[0] - a[0]) + t * (c[0] - a[0]),
            a[1] + s * (b[1] - a[1]) + t * (c[1] - a[1]),
            a[2] + s * (b[2] - a[2]) + t * (c[2] - a[2]),
        ]);
    }
    out
}

/// `x - floor(x)` for a positive `x`.
fn frac(x: f64) -> f64 {
    x - x.floor()
}

/// Twice a triangle's area.
fn triangle_area(t: &[Vec3; 3]) -> f64 {
    length(cross(sub(t[1], t[0]), sub(t[2], t[0])))
}

/// A uniform grid over a mesh's triangles, for point-to-mesh distances: each cell holds the
/// triangles whose bounding box meets it (triangles spanning too many cells are held apart and
/// always checked).
struct TriGrid {
    lo: Vec3,
    cell: f64,
    dims: [usize; 3],
    cells: HashMap<[i32; 3], Vec<u32>>,
    wide: Vec<u32>,
}

/// Cells across the largest side of the grid.
const GRID_ACROSS: usize = 32;
/// A triangle whose box spans more cells than this goes to the always-checked list.
const MAX_CELLS_PER_TRIANGLE: usize = 64;

impl TriGrid {
    // The cell indices are bounded by GRID_ACROSS (<= 32), so the casts below cannot wrap.
    #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss, clippy::cast_lossless)]
    fn new(mesh: &Mesh) -> Self {
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        for p in mesh.triangles.iter().flatten() {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        let extent = [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]];
        let largest = extent.iter().copied().fold(0.0, f64::max).max(1e-9);
        let cell = largest / GRID_ACROSS as f64;
        let dims = extent.map(|e| usize::try_from((e / cell).ceil() as i64).unwrap_or(GRID_ACROSS).clamp(1, GRID_ACROSS));
        let mut grid = TriGrid { lo, cell, dims, cells: HashMap::new(), wide: Vec::new() };
        for (i, t) in mesh.triangles.iter().enumerate() {
            let (mut tlo, mut thi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
            for p in t {
                for k in 0..3 {
                    tlo[k] = tlo[k].min(p[k]);
                    thi[k] = thi[k].max(p[k]);
                }
            }
            let c = |v: f64, k: usize| (((v - lo[k]) / cell).floor() as i32).clamp(0, dims[k] as i32 - 1);
            let (a, b) = ([c(tlo[0], 0), c(tlo[1], 1), c(tlo[2], 2)], [c(thi[0], 0), c(thi[1], 1), c(thi[2], 2)]);
            let span = (0..3).map(|k| (b[k] - a[k] + 1) as usize).product::<usize>();
            if span > MAX_CELLS_PER_TRIANGLE {
                grid.wide.push(i as u32);
                continue;
            }
            for x in a[0]..=b[0] {
                for y in a[1]..=b[1] {
                    for z in a[2]..=b[2] {
                        grid.cells.entry([x, y, z]).or_default().push(i as u32);
                    }
                }
            }
        }
        grid
    }

    /// The distance from `p` to the mesh's surface: the nearest triangle, found by searching the
    /// grid's cells in rings around `p`'s cell and stopping once the rings left can only be
    /// farther than the best found.
    #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss, clippy::cast_lossless)]
    fn distance(&self, mesh: &Mesh, p: Vec3) -> f64 {
        if mesh.triangles.is_empty() {
            return f64::INFINITY;
        }
        let key = |v: f64, k: usize| (((v - self.lo[k]) / self.cell).floor() as i32).clamp(0, self.dims[k] as i32 - 1);
        let at = [key(p[0], 0), key(p[1], 1), key(p[2], 2)];
        let mut best = f64::INFINITY;
        let max_ring = *self.dims.iter().max().unwrap_or(&1) as i32;
        for ring in 0..=max_ring {
            for x in (at[0] - ring)..=(at[0] + ring) {
                for y in (at[1] - ring)..=(at[1] + ring) {
                    for z in (at[2] - ring)..=(at[2] + ring) {
                        // Only the shell of this ring, not the rings before it.
                        if (x - at[0]).abs().max((y - at[1]).abs()).max((z - at[2]).abs()) != ring {
                            continue;
                        }
                        if let Some(list) = self.cells.get(&[x, y, z]) {
                            for &i in list {
                                best = best.min(point_triangle_distance(p, &mesh.triangles[i as usize]));
                            }
                        }
                    }
                }
            }
            for &i in &self.wide {
                best = best.min(point_triangle_distance(p, &mesh.triangles[i as usize]));
            }
            // Cells at chebyshev distance ring+1 or more are at least `ring * cell` away.
            if best <= ring as f64 * self.cell {
                break;
            }
        }
        best
    }
}

/// The distance from `p` to a triangle, clamped to its edges and corners (Ericson, Real-Time
/// Collision Detection 5.1.5).
fn point_triangle_distance(p: Vec3, t: &[Vec3; 3]) -> f64 {
    let [a, b, c] = *t;
    let ab = sub(b, a);
    let ac = sub(c, a);
    let ap = sub(p, a);
    let (d1, d2) = (dot(ab, ap), dot(ac, ap));
    if d1 <= 0.0 && d2 <= 0.0 {
        return length(ap);
    }
    let bp = sub(p, b);
    let (d3, d4) = (dot(ab, bp), dot(ac, bp));
    if d3 >= 0.0 && d4 <= d3 {
        return length(bp);
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return length(sub(ap, [ab[0] * v, ab[1] * v, ab[2] * v]));
    }
    let cp = sub(p, c);
    let (d5, d6) = (dot(ab, cp), dot(ac, cp));
    if d6 >= 0.0 && d5 <= d6 {
        return length(cp);
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return length(sub(ap, [ac[0] * w, ac[1] * w, ac[2] * w]));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        let bc = sub(c, b);
        return length(sub(bp, [bc[0] * w, bc[1] * w, bc[2] * w]));
    }
    let denom = 1.0 / (va + vb + vc);
    let (v, w) = (vb * denom, vc * denom);
    let d = sub(ap, [ab[0] * v + ac[0] * w, ab[1] * v + ac[1] * w, ab[2] * v + ac[2] * w]);
    length(d)
}

/// What `cargo xtask check` measures for one import: how far the rebuilt part lies from the source
/// mesh, and the gate mesh2cad uses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RebuildCheck {
    /// The volume error of the rebuild against the source mesh, in percent.
    pub volume_error_pct: f64,
    /// The Hausdorff distance of the two surfaces, in mm.
    pub max_deviation: f64,
    /// The deviation limit for this part's size, in mm: `max(4 * tol, 0.1)`.
    pub deviation_limit: f64,
}

impl RebuildCheck {
    /// Whether the rebuild is within mesh2cad's gate: deviation and volume error.
    #[must_use]
    pub fn ok(&self) -> bool {
        self.max_deviation <= self.deviation_limit && self.volume_error_pct <= MAX_VOLUME_ERROR_PCT
    }
}

/// Compares a rebuilt mesh with the source mesh, as mesh2cad scores a rebuilt script: the volume
/// error in percent and the surface deviation in mm, with the limit for this part's size.
#[must_use]
pub fn check_rebuild(source: &Mesh, rebuilt: &Mesh) -> RebuildCheck {
    let source_volume = source.volume().abs().max(1e-9);
    let volume_error_pct = (rebuilt.volume() - source.volume()).abs() / source_volume * 100.0;
    let diagonal = source.bounds().map_or(1.0, |b| length(sub(b.max, b.min)));
    let tol = TOL_SHARE * diagonal;
    let deviation_limit = (4.0 * tol).max(MIN_DEVIATION_LIMIT);
    let deviation = hausdorff(source, rebuilt, SAMPLES);
    RebuildCheck { volume_error_pct, max_deviation: deviation.max, deviation_limit }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recognize::{HoleSpec, PlateSpec};
    use crate::synth;

    fn block(x: f64, y: f64, z: f64, at: [f64; 3]) -> Mesh {
        let p = |i: f64, j: f64, k: f64| [at[0] + i * x, at[1] + j * y, at[2] + k * z];
        let quad = |a, b, c, d| [[a, b, c], [a, c, d]];
        let faces = [
            quad(p(0., 0., 0.), p(0., 1., 0.), p(1., 1., 0.), p(1., 0., 0.)),
            quad(p(0., 0., 1.), p(1., 0., 1.), p(1., 1., 1.), p(0., 1., 1.)),
            quad(p(0., 0., 0.), p(1., 0., 0.), p(1., 0., 1.), p(0., 0., 1.)),
            quad(p(0., 1., 0.), p(0., 1., 1.), p(1., 1., 1.), p(1., 1., 0.)),
            quad(p(0., 0., 0.), p(0., 0., 1.), p(0., 1., 1.), p(0., 1., 0.)),
            quad(p(1., 0., 0.), p(1., 1., 0.), p(1., 1., 1.), p(1., 0., 1.)),
        ];
        Mesh::new(faces.into_iter().flatten().collect())
    }

    #[test]
    fn identical_meshes_have_no_deviation() {
        let m = block(30.0, 20.0, 5.0, [0.0; 3]);
        let d = hausdorff(&m, &m, 500);
        assert!(d.max < 1e-9, "{}", d.max);
        assert!(d.mean < 1e-9);
    }

    #[test]
    fn a_moved_mesh_deviates_by_the_move() {
        let a = block(30.0, 20.0, 5.0, [0.0; 3]);
        let b = block(30.0, 20.0, 5.0, [3.0, 0.0, 4.0]);
        let d = hausdorff(&a, &b, 500);
        // The corners are 5 mm away, the sampled surface points just under that.
        assert!(d.max <= 5.0 + 1e-9 && d.max > 4.5, "{}", d.max);
        assert!(d.a_to_b <= 5.0 + 1e-9 && d.a_to_b > 4.5 && d.b_to_a <= 5.0 + 1e-9 && d.b_to_a > 4.5);
    }

    /// A hole faceted with 16 corners deviates from one with 48 by about the sagitta difference.
    #[test]
    fn faceted_holes_deviate_by_their_sagitta() {
        let spec = PlateSpec {
            x: 60.0,
            y: 40.0,
            z: 6.0,
            holes: vec![HoleSpec { x: 30.0, y: 20.0, d: 10.0, head: None, blind: None }],
            cutouts: vec![],
            pads: vec![],
            pockets: vec![],
            bosses: vec![],
            side_bosses: Vec::new(),
            slots: vec![],
            outline_fillet_rs: Vec::new(),
            edge_fillet_r: None,
        };
        let coarse = synth::plate(&spec, 16, [0.0; 3]);
        let fine = synth::plate(&spec, 48, [0.0; 3]);
        let d = hausdorff(&coarse, &fine, 2000);
        let r = 5.0;
        let expected = r * (1.0 - (std::f64::consts::PI / 16.0).cos()) - r * (1.0 - (std::f64::consts::PI / 48.0).cos());
        assert!((d.max - expected).abs() < 0.01, "{} vs {expected}", d.max);
    }

    #[test]
    fn the_gate_passes_a_good_rebuild_and_refuses_a_bad_one() {
        let source = block(60.0, 40.0, 6.0, [0.0; 3]);
        let same = block(60.0, 40.0, 6.0, [0.0; 3]);
        let check = check_rebuild(&source, &same);
        assert!(check.ok(), "{check:?}");
        assert!(check.max_deviation < 1e-9);
        assert!(check.volume_error_pct < 1e-9);
        // The limit follows the part's size: max(4 * 0.1% * diagonal, 0.1).
        let diagonal = (60.0_f64.powi(2) + 40.0_f64.powi(2) + 6.0_f64.powi(2)).sqrt();
        assert!((check.deviation_limit - (4.0 * 1e-3 * diagonal).max(0.1)).abs() < 1e-12);
        let bigger = block(66.0, 40.0, 6.0, [0.0; 3]);
        let bad = check_rebuild(&source, &bigger);
        assert!(!bad.ok(), "{bad:?}");
        // The x = 60 face lies 6 mm from the bigger box's x = 66 face.
        assert!((bad.max_deviation - 6.0).abs() < 0.5, "{}", bad.max_deviation);
        assert!((bad.volume_error_pct - 10.0).abs() < 1e-9);
    }
}
