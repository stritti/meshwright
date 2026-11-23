use std::f64::consts::PI;

use crate::plate::P2;
use crate::plate::geometry::fit_circle;

#[test]
fn fits_a_circle_exactly() {
    let ring: Vec<P2> = (0..12).map(|i| f64::from(i) * PI / 6.0).map(|a| [3.0 + 2.0 * a.cos(), -1.0 + 2.0 * a.sin()]).collect();
    let ([x, y], r) = fit_circle(&ring).expect("a circle");
    assert!((x - 3.0).abs() < 1e-12 && (y + 1.0).abs() < 1e-12 && (r - 2.0).abs() < 1e-12);
}
