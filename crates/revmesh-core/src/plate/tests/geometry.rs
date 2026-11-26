use std::f64::consts::PI;

use crate::plate::P2;
use crate::plate::geometry::{fit_circle, fit_torus, ransac_circle};

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

/// The torus about a known axis is a circle in the (radial, axial) profile plane (roadmap 6.3):
/// the fit recovers a quarter round's centre and radius exactly, and gives up on coincident or
/// collinear profile points.
#[test]
fn fits_a_torus_profile_exactly() {
    let f = 1.5;
    let profile: Vec<P2> = [0.0, 0.3, 0.9, 1.2, PI / 2.0].iter().map(|a| [11.0 - f * a.sin(), f - f * a.cos()]).collect();
    let ([hr, d], r) = fit_torus(&profile, 1e-3).expect("a torus");
    assert!((hr - 11.0).abs() < 1e-9 && (d - f).abs() < 1e-9 && (r - f).abs() < 1e-9);
    // Coincident points fit no circle (the algebraic system is singular, and a garbage fit is
    // refused by the on-circle check).
    assert!(fit_torus(&[[0.5, 0.5], [0.5, 0.5], [0.5, 0.5]], 1e-3).is_none());
    // Collinear points neither.
    assert!(fit_torus(&[[0.0, 0.0], [1.0, 1.0], [2.0, 2.0]], 1e-3).is_none());
    // A corner an exporter added within a facet lies inside the torus: the fit ignores it and
    // the result is still exact.
    let mut noisy = profile.clone();
    let mid = [f64::midpoint(profile[1][0], profile[2][0]), f64::midpoint(profile[1][1], profile[2][1])];
    noisy.push(mid);
    let ([hr, d], r) = fit_torus(&noisy, 1e-3).expect("a torus through the on-circle corners");
    assert!((hr - 11.0).abs() < 1e-9 && (d - f).abs() < 1e-9 && (r - f).abs() < 1e-9);
    // Only two distinct interior band locations (a round in three 30° bands): no circle, so
    // the caller falls back to the per-corner formula.
    let two = [[10.5, 0.134], [10.5, 0.134], [10.134, 0.5], [10.134, 0.5]];
    assert!(fit_torus(&two, 1e-3).is_none());
}
