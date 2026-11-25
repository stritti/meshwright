use serde::Serialize;

use crate::mesh::snap_dim;

/// A sketch: points, curves between them, and constraints on both.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Sketch {
    /// Points; index 0 is the fixed origin.
    pub points: Vec<SketchPoint>,
    /// Curves, referring to points by index.
    pub curves: Vec<SketchCurve>,
    /// Constraints, referring to points and curves by index.
    pub constraints: Vec<Constraint>,
    /// Geometry linked from outside the sketch, such as its X axis.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<Link>,
    counters: Counters,
}

/// Geometry a sketch takes from elsewhere: `j<n>`, a projection of one of the sketch's own axes.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Link {
    /// `j<n>`.
    pub id: String,
    kind: &'static str,
    source: LinkSource,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
struct LinkSource {
    #[serde(rename = "type")]
    kind: &'static str,
    name: &'static str,
}

/// A sketch point.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SketchPoint {
    /// `origin`, or `p<n>`.
    pub id: String,
    /// Position in sketch coordinates.
    pub pos: [f64; 2],
    /// Whether the solver may move it.
    pub fixed: bool,
    /// The link (`j<n>`) that places it, for a point of linked geometry.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    /// The expressions that place a fixed point, which follow the parameters they name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exprs: Option<PointExprs>,
}

/// Where a point placed by parameters lies: an expression for each coordinate.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PointExprs {
    /// X.
    pub x: String,
    /// Y.
    pub y: String,
}

impl SketchPoint {
    fn new(id: impl Into<String>, pos: [f64; 2], fixed: bool) -> Self {
        Self { id: id.into(), pos, fixed, link: None, exprs: None }
    }
}

/// A sketch curve.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SketchCurve {
    /// A line between two points.
    Line {
        /// `l<n>`.
        id: String,
        /// Start point index.
        a: usize,
        /// End point index.
        b: usize,
        /// Construction geometry makes no profile.
        construction: bool,
        /// The link (`j<n>`) it comes from, for linked geometry.
        #[serde(skip_serializing_if = "Option::is_none")]
        link: Option<String>,
    },
    /// An arc about centre `c`, counter-clockwise from point `a` to point `b`.
    Arc {
        /// `a<n>`.
        id: String,
        /// Centre point index.
        c: usize,
        /// Start point index.
        a: usize,
        /// End point index.
        b: usize,
        /// Construction geometry makes no profile.
        construction: bool,
    },
}

/// A sketch constraint.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Constraint {
    /// Line `l` is horizontal.
    Horizontal {
        /// `k<n>`.
        id: String,
        /// Curve index.
        l: usize,
    },
    /// Line `l` is vertical.
    Vertical {
        /// `k<n>`.
        id: String,
        /// Curve index.
        l: usize,
    },
    /// Points `p` and `q` coincide.
    Coincident {
        /// `k<n>`.
        id: String,
        /// Point index.
        p: usize,
        /// Point index.
        q: usize,
    },
    /// Line `l` has a length driven by the model parameter `param`.
    Length {
        /// `k<n>`.
        id: String,
        /// Curve index.
        l: usize,
        /// The length now.
        value: f64,
        /// The model parameter that drives it.
        param: String,
        /// Where the dimension text sits, relative to the line.
        #[serde(skip_serializing_if = "Option::is_none")]
        text: Option<[f64; 2]>,
    },
    /// Curves `a` and `b` are tangent where they meet.
    Tangent {
        /// `k<n>`.
        id: String,
        /// Curve index.
        a: usize,
        /// Curve index.
        b: usize,
    },
    /// Curves `a` and `b` are equal (two arcs: one radius).
    Equal {
        /// `k<n>`.
        id: String,
        /// Curve index.
        a: usize,
        /// Curve index.
        b: usize,
    },
    /// Point `p` is the midpoint of line `l`.
    Midpoint {
        /// `k<n>`.
        id: String,
        /// Point index.
        p: usize,
        /// Curve index.
        l: usize,
    },
    /// Arc or circle `c` has a diameter driven by `param`.
    Diameter {
        /// `k<n>`.
        id: String,
        /// Curve index.
        c: usize,
        /// The diameter now.
        value: f64,
        /// The model parameter that drives it.
        param: String,
    },
    /// Point `q` lies `value` along X from point `p`, driven by `param`.
    DistanceX {
        /// `k<n>`.
        id: String,
        /// Point index.
        p: usize,
        /// Point index.
        q: usize,
        /// The distance now.
        value: f64,
        /// The model parameter that drives it.
        param: String,
    },
    /// Point `q` lies `value` along Y from point `p`, driven by `param`.
    DistanceY {
        /// `k<n>`.
        id: String,
        /// Point index.
        p: usize,
        /// Point index.
        q: usize,
        /// The distance now.
        value: f64,
        /// The model parameter that drives it.
        param: String,
    },
    /// Line `b` is turned `value` (radians) from line `a`, driven by `param` (in degrees).
    Angle {
        /// `k<n>`.
        id: String,
        /// Curve index.
        a: usize,
        /// Curve index.
        b: usize,
        /// The angle now, in radians.
        value: f64,
        /// The model parameter that drives it.
        param: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
struct Counters {
    #[serde(skip_serializing_if = "is_zero")]
    a: usize,
    #[serde(skip_serializing_if = "is_zero")]
    j: usize,
    k: usize,
    l: usize,
    p: usize,
}

#[allow(clippy::trivially_copy_pass_by_ref)] // serde passes the field by reference
fn is_zero(n: &usize) -> bool {
    *n == 0
}

/// A rectangle from the origin to `(width, height)` on the XY plane, its sides driven by the
/// model parameters `d1` (along X) and `d2` (along Y). Lines are `l1` (bottom), `l2` (right),
/// `l3` (top) and `l4` (left).
#[must_use]
pub fn rectangle_sketch(width: f64, height: f64) -> Sketch {
    let point = SketchPoint::new;
    let line = |n: usize, a: usize, b: usize| SketchCurve::Line { id: format!("l{n}"), a, b, construction: false, link: None };
    Sketch {
        points: vec![
            point("origin", [0.0, 0.0], true),
            point("p1", [0.0, 0.0], false),
            point("p2", [width, 0.0], false),
            point("p3", [width, height], false),
            point("p4", [0.0, height], false),
        ],
        curves: vec![line(1, 1, 2), line(2, 2, 3), line(3, 3, 4), line(4, 4, 1)],
        constraints: vec![
            Constraint::Horizontal { id: "k1".into(), l: 0 },
            Constraint::Horizontal { id: "k2".into(), l: 2 },
            Constraint::Vertical { id: "k3".into(), l: 1 },
            Constraint::Vertical { id: "k4".into(), l: 3 },
            Constraint::Coincident { id: "k5".into(), p: 0, q: 1 },
            Constraint::Length { id: "k6".into(), l: 0, value: width, param: "d1".into(), text: Some([0.0, -5.0]) },
            Constraint::Length { id: "k7".into(), l: 1, value: height, param: "d2".into(), text: Some([0.0, -5.0]) },
        ],
        links: vec![],
        counters: Counters { a: 0, j: 0, k: 7, l: 4, p: 4 },
    }
}

/// A closed polygon through `corners` (counter-clockwise) on the XY plane. Line `l<k>` runs from
/// corner `k - 1` to corner `k` (wrapping round), point `p<k>` is corner `k - 1`. Lines along X or
/// Y are constrained horizontal or vertical; the corners are otherwise free, as drawn.
#[must_use]
pub fn polygon_sketch(corners: &[[f64; 2]]) -> Sketch {
    polygon_sketch_sized(corners, &[])
}

/// Like [`polygon_sketch`], but with a `Length` constraint per side driven by the model parameter
/// `lens[i]`, where `Some`, so the outline re-dimensions in SolidCraft side by side like the
/// rectangle's do. See [`dimensioned_sides`] for which sides may drive.
#[must_use]
pub fn polygon_sketch_dimensioned(corners: &[[f64; 2]], lens: &[Option<String>]) -> Sketch {
    polygon_sketch_sized(corners, lens)
}

/// Which sides of a closed polygon may get a driving `Length` constraint. Sides are grouped by
/// line direction; within a group of two or more parallel sides, loop closure makes one length
/// dependent on the others (for axis-aligned sides, a sum in their direction; for parallel
/// slanted sides, as in a parallelogram, an equality), and driving a dependent one too would
/// leave no solution when one parameter changes. So the longest side of every such group does
/// not drive.
#[allow(clippy::many_single_char_names)] // points a, b, directions d, u, as in the geometry
#[must_use]
pub fn dimensioned_sides(corners: &[[f64; 2]]) -> Vec<bool> {
    let n = corners.len();
    let side = |i: usize| (corners[i], corners[(i + 1) % n]);
    let len = |i: usize| {
        let (a, b) = side(i);
        (a[0] - b[0]).hypot(a[1] - b[1])
    };
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for i in 0..n {
        let (a, b) = side(i);
        let d = [b[0] - a[0], b[1] - a[1]];
        let group = groups.iter_mut().find(|g| {
            let (c, e) = side(g[0]);
            let u = [e[0] - c[0], e[1] - c[1]];
            (d[0] * u[1] - d[1] * u[0]).abs() <= 1e-9 * len(i) * len(g[0])
        });
        match group {
            Some(g) => g.push(i),
            None => groups.push(vec![i]),
        }
    }
    let mut dim = vec![true; n];
    for g in &groups {
        if let Some(&longest) = g.iter().filter(|_| g.len() >= 2).max_by(|&&a, &&b| len(a).total_cmp(&len(b))) {
            dim[longest] = false;
        }
    }
    dim
}

fn polygon_sketch_sized(corners: &[[f64; 2]], lens: &[Option<String>]) -> Sketch {
    let n = corners.len();
    let mut points = vec![SketchPoint::new("origin", [0.0, 0.0], true)];
    points.extend(corners.iter().enumerate().map(|(i, c)| SketchPoint::new(format!("p{}", i + 1), *c, false)));
    let curves = (0..n)
        .map(|i| SketchCurve::Line { id: format!("l{}", i + 1), a: i + 1, b: (i + 1) % n + 1, construction: false, link: None })
        .collect();
    let mut constraints = Vec::new();
    for i in 0..n {
        let (a, b) = (corners[i], corners[(i + 1) % n]);
        if (a[1] - b[1]).abs() < 1e-9 {
            constraints.push(Constraint::Horizontal { id: format!("k{}", constraints.len() + 1), l: i });
        } else if (a[0] - b[0]).abs() < 1e-9 {
            constraints.push(Constraint::Vertical { id: format!("k{}", constraints.len() + 1), l: i });
        }
        if let Some(param) = lens.get(i).and_then(|l| l.as_ref()) {
            constraints.push(Constraint::Length {
                id: format!("k{}", constraints.len() + 1),
                l: i,
                value: snap_dim((a[0] - b[0]).hypot(a[1] - b[1])),
                param: param.clone(),
                text: None,
            });
        }
    }
    let counters = Counters { a: 0, j: 0, k: constraints.len(), l: n, p: n };
    Sketch { points, curves, constraints, links: vec![], counters }
}

/// A sketch on the XY plane of closed polygons (each `corners` counter-clockwise or not), drawn as
/// [`polygon_sketch`] draws its outline: free corners, sides along X or Y held horizontal or
/// vertical.
#[must_use]
pub fn polygons_sketch(polygons: &[Vec<[f64; 2]>]) -> Sketch {
    polygons_sketch_sized(polygons, &[])
}

/// Like [`polygons_sketch`], but with a `Length` constraint per side driven by the model parameter
/// `lens[polygon][side]`, where `Some`, so every side re-dimensions in SolidCraft. See
/// [`dimensioned_sides`] for which sides may drive.
#[must_use]
pub fn polygons_sketch_dimensioned(polygons: &[Vec<[f64; 2]>], lens: &[Vec<Option<String>>]) -> Sketch {
    polygons_sketch_sized(polygons, lens)
}

fn polygons_sketch_sized(polygons: &[Vec<[f64; 2]>], lens: &[Vec<Option<String>>]) -> Sketch {
    let mut sketch = Sketch {
        points: vec![SketchPoint::new("origin", [0.0, 0.0], true)],
        curves: vec![],
        constraints: vec![],
        links: vec![],
        counters: Counters { a: 0, j: 0, k: 0, l: 0, p: 0 },
    };
    add_polygons(&mut sketch, polygons, lens);
    sketch
}

fn add_polygons(sketch: &mut Sketch, polygons: &[Vec<[f64; 2]>], lens: &[Vec<Option<String>>]) {
    for (pi, corners) in polygons.iter().enumerate() {
        let n = corners.len();
        let first = sketch.points.len();
        let base_point = sketch.counters.p;
        let base_line = sketch.counters.l;
        for (i, c) in corners.iter().enumerate() {
            sketch.points.push(SketchPoint::new(format!("p{}", base_point + i + 1), *c, false));
        }
        for i in 0..n {
            let line = sketch.curves.len();
            sketch.curves.push(SketchCurve::Line {
                id: format!("l{}", base_line + i + 1),
                a: first + i,
                b: first + (i + 1) % n,
                construction: false,
                link: None,
            });
            let (a, b) = (corners[i], corners[(i + 1) % n]);
            if (a[1] - b[1]).abs() < 1e-9 {
                sketch.constraints.push(Constraint::Horizontal { id: format!("k{}", sketch.counters.k + 1), l: line });
                sketch.counters.k += 1;
            } else if (a[0] - b[0]).abs() < 1e-9 {
                sketch.constraints.push(Constraint::Vertical { id: format!("k{}", sketch.counters.k + 1), l: line });
                sketch.counters.k += 1;
            }
            if let Some(param) = lens.get(pi).and_then(|l| l.get(i)).and_then(|o| o.as_ref()) {
                sketch.constraints.push(Constraint::Length {
                    id: format!("k{}", sketch.counters.k + 1),
                    l: line,
                    value: snap_dim((a[0] - b[0]).hypot(a[1] - b[1])),
                    param: param.clone(),
                    text: None,
                });
                sketch.counters.k += 1;
            }
        }
        sketch.counters.p += n;
        sketch.counters.l += n;
    }
}

/// A straight side counts as tangent to an arc side at their shared corner when its direction
/// is perpendicular to the arc's radius there, within the recogniser's gentle-turn limit
/// (sin 30°); see [`polygon_sketch_arcs`].
const TANGENT_SIN: f64 = 0.5;

/// Whether the straight side from `line_from` through `corner` is tangent to the arc about
/// `centre` at `corner`: the side's direction is perpendicular to the arc's radius there,
/// within the recogniser's gentle-turn limit.
#[must_use]
pub fn line_arc_tangent(line_from: [f64; 2], corner: [f64; 2], centre: [f64; 2]) -> bool {
    let l = (corner[0] - line_from[0]).hypot(corner[1] - line_from[1]);
    let r = (corner[0] - centre[0]).hypot(corner[1] - centre[1]);
    let u = [(corner[0] - line_from[0]) / l, (corner[1] - line_from[1]) / l];
    let n = [(corner[0] - centre[0]) / r, (corner[1] - centre[1]) / r];
    (u[0] * n[0] + u[1] * n[1]).abs() <= TANGENT_SIN
}

/// A closed polygon through `corners` (counter-clockwise) where side `i` is the arc about
/// `arcs[i].0` with radius `arcs[i].1` when `Some`, else a straight line — like
/// [`polygon_sketch_dimensioned`], with a `Length` constraint on each driving line side
/// (`lens[i]`) and a `Diameter` constraint on each driving arc side (`arc_params[i]`, the model
/// parameter behind the side's `arc_<i>_d` user parameter). An arc side's centre is a free
/// sketch point: the solver places it equidistant from the side's two corners at the driving
/// diameter, so the centre follows the corners when a parameter changes — like a slot's arc
/// centres, which are free points too — and the sketch stays solvable. A straight side whose
/// shared corner with an arc side turns gently is held tangent to the arc (as a slot's sides
/// are), which keeps a driving diameter change solvable to first order even for a
/// semicircular side. Arc side pairs in `arc_equals` share one radius (`Equal`, as a slot's
/// ends do) and only the first of a pair drives. `arcs` and `arc_params` are parallel to
/// `corners`.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn polygon_sketch_arcs(
    corners: &[[f64; 2]],
    arcs: &[Option<([f64; 2], f64)>],
    lens: &[Option<String>],
    arc_params: &[Option<String>],
    arc_equals: &[(usize, usize)],
) -> Sketch {
    let n = corners.len();
    let mut points = vec![SketchPoint::new("origin", [0.0, 0.0], true)];
    points.extend(corners.iter().enumerate().map(|(i, c)| SketchPoint::new(format!("p{}", i + 1), *c, false)));
    let mut centre_of = vec![None; n];
    for (i, arc) in arcs.iter().enumerate() {
        if let Some((centre, _)) = arc {
            let idx = points.len();
            centre_of[i] = Some(idx);
            points.push(SketchPoint::new(format!("p{idx}"), *centre, false));
        }
    }
    let mut curves: Vec<SketchCurve> = Vec::new();
    let mut constraints = Vec::new();
    let mut n_lines = 0;
    let mut n_arcs = 0;
    let mut side_curve = vec![0_usize; n];
    for i in 0..n {
        let (a, b) = (corners[i], corners[(i + 1) % n]);
        if let (Some((_, r)), Some(ci)) = (arcs[i], centre_of[i]) {
            n_arcs += 1;
            let curve = curves.len();
            side_curve[i] = curve;
            curves.push(SketchCurve::Arc { id: format!("a{n_arcs}"), c: ci, a: i + 1, b: (i + 1) % n + 1, construction: false });
            if let Some(param) = arc_params.get(i).and_then(Clone::clone) {
                constraints.push(Constraint::Diameter {
                    id: format!("k{}", constraints.len() + 1),
                    c: curve,
                    value: snap_dim(2.0 * r),
                    param,
                });
            }
        } else {
            n_lines += 1;
            let line = curves.len();
            side_curve[i] = line;
            curves.push(SketchCurve::Line { id: format!("l{n_lines}"), a: i + 1, b: (i + 1) % n + 1, construction: false, link: None });
            if (a[1] - b[1]).abs() < 1e-9 {
                constraints.push(Constraint::Horizontal { id: format!("k{}", constraints.len() + 1), l: line });
            } else if (a[0] - b[0]).abs() < 1e-9 {
                constraints.push(Constraint::Vertical { id: format!("k{}", constraints.len() + 1), l: line });
            }
            if let Some(param) = lens.get(i).and_then(|l| l.as_ref()) {
                constraints.push(Constraint::Length {
                    id: format!("k{}", constraints.len() + 1),
                    l: line,
                    value: snap_dim((a[0] - b[0]).hypot(a[1] - b[1])),
                    param: param.clone(),
                    text: None,
                });
            }
        }
    }
    // A straight side meeting an arc side at a gently turning corner (within the recogniser's
    // gentle-turn limit) is tangent to the arc: hold it with a `Tangent` constraint, like a
    // slot's sides. Without it, growing a semicircular side's diameter would need its centre to
    // slide off the chord — second order, which no solver does from the snapped position.
    for i in 0..n {
        let Some((centre, _)) = arcs[i] else { continue };
        let prev = (i + n - 1) % n;
        if arcs[prev].is_none() && line_arc_tangent(corners[prev], corners[i], centre) {
            constraints.push(Constraint::Tangent { id: format!("k{}", constraints.len() + 1), a: side_curve[prev], b: side_curve[i] });
        }
        let next = (i + 1) % n;
        if arcs[next].is_none() && line_arc_tangent(corners[(i + 2) % n], corners[next], centre) {
            constraints.push(Constraint::Tangent { id: format!("k{}", constraints.len() + 1), a: side_curve[i], b: side_curve[next] });
        }
    }
    for &(p, q) in arc_equals {
        constraints.push(Constraint::Equal { id: format!("k{}", constraints.len() + 1), a: side_curve[p], b: side_curve[q] });
    }
    let counters = Counters { a: n_arcs, j: 0, k: constraints.len(), l: n_lines, p: points.len() - 1 };
    Sketch { points, curves, constraints, links: vec![], counters }
}

/// What drives a slot's sketch: model parameters for the distance between the end centres,
/// the width and the long axis's angle from X, and expressions for where the middle lies.
#[derive(Debug, Clone, PartialEq)]
pub struct SlotDimensions {
    /// Between the end centres.
    pub len: String,
    /// The width (the ends' diameter).
    pub w: String,
    /// The middle's X, an expression (a point placed by it, not a dimension).
    pub x: String,
    /// The middle's Y, as `x`.
    pub y: String,
    /// The long axis from X (in degrees); `None` holds a slot along X or Y horizontal or
    /// vertical instead.
    pub angle: Option<String>,
}

/// A straight slot on the XY plane, as SolidCraft's centre-point slot tool draws it: lines `l1`
/// and `l2` tangent to arcs `a1` (about the end at `+len / 2` along `angle`, in degrees) and
/// `a2`, and the construction line `l3` between the arc centres. Its midpoint `p5` coincides
/// with `p6`, a point placed by the expressions `dims.x` and `dims.y` (SolidCraft's X and Y
/// distances are unsigned, so a dimension could not follow a position across the origin). With
/// an angle, it is measured from the sketch's X axis, linked in as `l4`; without, the slot is
/// held horizontal or vertical.
#[must_use]
pub fn slot_sketch(centre: [f64; 2], len: f64, w: f64, angle: f64, dims: &SlotDimensions) -> Sketch {
    let (sin, cos) = angle.to_radians().sin_cos();
    let (u, v, r) = ([cos, sin], [-sin, cos], w / 2.0);
    let at = |s: f64, t: f64| [centre[0] + s * u[0] + t * v[0], centre[1] + s * u[1] + t * v[1]];
    let half = len / 2.0;
    let mut points = vec![
        SketchPoint::new("origin", [0.0, 0.0], true),
        SketchPoint::new("p1", at(-half, -r), false),
        SketchPoint::new("p2", at(half, -r), false),
        SketchPoint::new("p3", at(half, r), false),
        SketchPoint::new("p4", at(-half, r), false),
        SketchPoint::new("a1.center", at(half, 0.0), false),
        SketchPoint::new("a2.center", at(-half, 0.0), false),
        SketchPoint::new("p5", centre, false),
        SketchPoint { exprs: Some(PointExprs { x: dims.x.clone(), y: dims.y.clone() }), ..SketchPoint::new("p6", centre, true) },
    ];
    let line = |n: usize, a: usize, b: usize, construction: bool| SketchCurve::Line { id: format!("l{n}"), a, b, construction, link: None };
    let mut curves = vec![
        line(1, 1, 2, false),
        SketchCurve::Arc { id: "a1".into(), c: 5, a: 2, b: 3, construction: false },
        line(2, 3, 4, false),
        SketchCurve::Arc { id: "a2".into(), c: 6, a: 4, b: 1, construction: false },
        line(3, 6, 5, true),
    ];
    let mut constraints: Vec<Constraint> = Vec::new();
    let mut add = |make: &dyn Fn(String) -> Constraint| {
        let id = format!("k{}", constraints.len() + 1);
        constraints.push(make(id));
    };
    add(&|id| Constraint::Tangent { id, a: 0, b: 1 });
    add(&|id| Constraint::Tangent { id, a: 2, b: 1 });
    add(&|id| Constraint::Tangent { id, a: 0, b: 3 });
    add(&|id| Constraint::Tangent { id, a: 2, b: 3 });
    add(&|id| Constraint::Equal { id, a: 3, b: 1 });
    add(&|id| Constraint::Midpoint { id, p: 7, l: 4 });
    add(&|id| Constraint::Length { id, l: 0, value: len, param: dims.len.clone(), text: None });
    add(&|id| Constraint::Diameter { id, c: 1, value: w, param: dims.w.clone() });
    add(&|id| Constraint::Coincident { id, p: 7, q: 8 });
    let mut links = vec![];
    match &dims.angle {
        None if sin.abs() < 0.5 => add(&|id| Constraint::Horizontal { id, l: 4 }),
        None => add(&|id| Constraint::Vertical { id, l: 4 }),
        Some(param) => {
            let axis = || Some("j1".to_owned());
            points.push(SketchPoint { link: axis(), ..SketchPoint::new("p7", [-100.0, 0.0], true) });
            points.push(SketchPoint { link: axis(), ..SketchPoint::new("p8", [100.0, 0.0], true) });
            curves.push(SketchCurve::Line { id: "l4".into(), a: 9, b: 10, construction: true, link: axis() });
            links.push(Link { id: "j1".into(), kind: "project", source: LinkSource { kind: "sketch_axis", name: "x" } });
            add(&|id| Constraint::Angle { id, a: 5, b: 4, value: angle.to_radians(), param: param.clone() });
        }
    }
    let counters = Counters { a: 2, j: links.len(), k: constraints.len(), l: curves.len() - 2, p: points.len() - 3 };
    Sketch { points, curves, constraints, links, counters }
}
