use std::f64::consts::PI;

use crate::plate::P2;
use crate::plate::geometry::{fit_circle, ransac_circle};

#[test]
fn fits_a_circle_exactly() {
    let ring: Vec<P2> = (0..12).map(|i| f64::from(i) * PI / 6.0).map(|a| [3.0 + 2.0 * a.cos(), -1.0 + 2.0 * a.sin()]).collect();
    let ([x, y], r) = fit_circle(&ring).expect("a circle");
    assert!((x - 3.0).abs() < 1e-12 && (y + 1.0).abs() < 1e-12 && (r - 2.0).abs() < 1e-12);
}

/// A fixed-seed RANSAC fit finds the circle through a rim's corners and drops the one an
/// exporter pushed off it, with the same answer on every run (roadmap 6.2).
#[test]
fn ransac_fits_a_circle_drops_outliers_and_is_deterministic() {
    let mut points: Vec<P2> = (0..16).map(|i| f64::from(i) * PI / 8.0).map(|a| [3.0 + 2.0 * a.cos(), -1.0 + 2.0 * a.sin()]).collect();
    points.push([5.5, -1.0]);
    let (c1, r1, in1) = ransac_circle(&points, 1e-3).expect("a circle");
    let (c2, r2, in2) = ransac_circle(&points, 1e-3).expect("a circle");
    assert_eq!(in1, in2);
    assert!((c1[0] - c2[0]).abs() < 1e-12 && (c1[1] - c2[1]).abs() < 1e-12 && (r1 - r2).abs() < 1e-12);
    assert_eq!(in1.len(), 16);
    assert!(!in1.contains(&16));
    assert!((c1[0] - 3.0).abs() < 1e-9 && (c1[1] + 1.0).abs() < 1e-9 && (r1 - 2.0).abs() < 1e-9);
}

/// Collinear points have no circle through them, so the RANSAC fit gives up.
#[test]
fn ransac_finds_no_circle_in_collinear_points() {
    let points: Vec<P2> = (0..6).map(|i| [f64::from(i), 2.0 * f64::from(i)]).collect();
    assert!(ransac_circle(&points, 1e-3).is_none());
}
