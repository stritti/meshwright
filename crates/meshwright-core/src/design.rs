//! The SolidCraft design document an importer writes.
//!
//! SolidCraft's document format is internal and has no published schema. What is modelled here
//! is the subset meshwright writes, taken from designs SolidCraft itself saved as JSON (see
//! `docs/design-format.md`). SolidCraft upgrades older format versions when it opens a file, so
//! the importer pins one version, [`FORMAT`], and is tested against each SolidCraft release.
//!
//! Fields SolidCraft derives on its own (`param_names`, `face_names`, `edge_refs`) are left out.
//! `edge_names` are written: without them SolidCraft re-finds fillet edges only by their pick
//! points, which can snap to the wrong edge once a parameter moves the geometry.

use serde::Serialize;

use crate::mesh::Mesh;

/// The document format version meshwright writes (SolidCraft 0.45 saves `solidcraft/63`).
pub const FORMAT: &str = "solidcraft/63";

/// A complete SolidCraft design.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Design {
    format: &'static str,
    /// The design's name; empty names it after the imported file.
    pub name: String,
    units: &'static str,
    /// User and model parameters, in the order the parameter panel lists them.
    pub params: Vec<Param>,
    /// The timeline.
    pub features: Vec<Feature>,
    next_id: u32,
    reserved_body_names: Vec<String>,
    next_component_color_index: u32,
}

impl Design {
    /// The design as the JSON SolidCraft opens.
    ///
    /// # Panics
    ///
    /// Never: every field serialises (no maps with non-string keys, no failing `Serialize`).
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("a design always serialises")
    }

    /// Writes the design as JSON to `writer` as it is serialised, without building it in memory
    /// first. The plugin writes large mesh designs this way.
    pub fn write_json<W: std::io::Write>(&self, writer: W) -> std::io::Result<()> {
        serde_json::to_writer(writer, self).map_err(std::io::Error::other)
    }

    /// The design as indented JSON, for golden files and reading by eye.
    ///
    /// # Panics
    ///
    /// Never, as [`Design::to_json`].
    #[must_use]
    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).expect("a design always serialises")
    }
}

/// A named parameter.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Param {
    /// The name expressions refer to, such as `base_x`.
    pub name: String,
    /// Its expression, such as `"60 mm"` or `"base_x / 2"`.
    pub expr: String,
    unit: &'static str,
    /// `true` for a parameter that belongs to a feature (a sketch dimension), `false` for one the
    /// user sees in the parameter panel.
    pub model: bool,
}

impl Param {
    /// A user parameter in millimetres.
    #[must_use]
    pub fn user(name: impl Into<String>, millimetres: f64) -> Self {
        Self { name: name.into(), expr: format!("{} mm", fmt_number(millimetres)), unit: "mm", model: false }
    }

    /// A model parameter (a sketch dimension) driven by `expr`.
    #[must_use]
    pub fn model(name: impl Into<String>, expr: impl Into<String>) -> Self {
        Self { name: name.into(), expr: expr.into(), unit: "mm", model: true }
    }

    /// A model parameter for an angle dimension, driven by `expr` (in degrees).
    #[must_use]
    pub fn model_angle(name: impl Into<String>, expr: impl Into<String>) -> Self {
        Self { name: name.into(), expr: expr.into(), unit: "deg", model: true }
    }

    /// A user parameter without a unit, such as a pattern's count (SolidCraft refuses a length
    /// as a count).
    #[must_use]
    pub fn count(name: impl Into<String>, count: usize) -> Self {
        Self { name: name.into(), expr: count.to_string(), unit: "", model: false }
    }

    /// A user parameter in degrees.
    #[must_use]
    pub fn angle(name: impl Into<String>, degrees: f64) -> Self {
        Self { name: name.into(), expr: format!("{} deg", fmt_number(degrees)), unit: "deg", model: false }
    }
}

/// One timeline entry.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Feature {
    /// Unique within the design; later features refer to earlier ones by it.
    pub id: u32,
    /// The name shown in the timeline.
    pub name: String,
    suppressed: bool,
    /// Bodies this feature creates.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub body_names: Vec<String>,
    /// What the feature is.
    #[serde(flatten)]
    pub kind: FeatureKind,
}

/// The kinds of feature meshwright writes.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FeatureKind {
    /// A mesh body, as SolidCraft's own STL import makes it.
    MeshImport {
        /// The file it came from.
        file: String,
        /// The meshes, one body each.
        meshes: Vec<MeshData>,
    },
    /// A sketch on an origin plane.
    Sketch {
        /// The plane.
        plane: Plane,
        /// Its geometry and constraints.
        sketch: Sketch,
    },
    /// An extrusion of every profile of a sketch.
    Extrude {
        /// The sketch feature's id.
        sketch: u32,
        /// Which profiles.
        profiles: Profiles,
        /// How far.
        extent: Extent,
        /// What it does to existing bodies.
        operation: Operation,
    },
    /// A hole drilled from a face.
    Hole {
        /// The entry point as numbers, a pick SolidCraft uses to find the face.
        position: [f64; 3],
        /// The entry point as expressions, which follow parameters.
        at: [String; 3],
        /// The drilling direction.
        direction: [f64; 3],
        /// The diameter expression.
        diameter: String,
        /// How deep a blind hole goes from the face (an expression); through all without it.
        #[serde(skip_serializing_if = "Option::is_none")]
        depth: Option<String>,
        /// The hole's kind.
        hole: HoleKind,
    },
    /// Copies of earlier features (holes), in a row, a grid or round an axis.
    Pattern {
        /// The ids of the features it repeats.
        features: Vec<u32>,
        /// Where the copies go.
        pattern: PatternKind,
    },
    /// A move of whole bodies, by a translation whose three parts are expressions.
    Move {
        /// The bodies it moves.
        bodies: Vec<String>,
        /// The translation along X, Y and Z.
        translate: [String; 3],
    },
    /// A constant-radius fillet.
    Fillet {
        /// Topological names of the edges, which keep the fillet on the right edges when
        /// parameters change. `F<id>:side:<line>` is the side face an extrusion `<id>` made from
        /// sketch line `<line>`; an edge is two faces joined by `|`.
        edge_names: Vec<String>,
        /// A point on each edge, in the same order.
        edges: Vec<[f64; 3]>,
        /// The radius expression.
        radius: String,
    },
}

/// A mesh body, serialised in SolidCraft's layout: `positions` (flat `x, y, z` per vertex) and
/// `triangles` (three vertex indices each), one vertex per triangle corner as STL has them.
///
/// The mesh is kept as it is and laid out only while the design is written, so a large mesh is
/// never held twice. Coordinates are written as single-precision numbers, the precision STL
/// stores them in, which keeps the design small (`0.1` rather than `0.10000000149011612`).
#[derive(Debug, Clone, PartialEq)]
pub struct MeshData {
    /// The body's name.
    pub name: String,
    /// The mesh.
    pub mesh: Mesh,
}

impl MeshData {
    /// The body `name` made of `mesh`.
    #[must_use]
    pub fn new(name: impl Into<String>, mesh: Mesh) -> Self {
        Self { name: name.into(), mesh }
    }
}

impl Serialize for MeshData {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::{SerializeSeq, SerializeStruct};

        struct Positions<'a>(&'a Mesh);
        impl Serialize for Positions<'_> {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                let mut seq = serializer.serialize_seq(Some(self.0.triangles.len() * 9))?;
                for value in self.0.triangles.iter().flatten().flatten() {
                    seq.serialize_element(&(*value as f32))?;
                }
                seq.end()
            }
        }
        struct Indices(usize);
        impl Serialize for Indices {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                let mut seq = serializer.serialize_seq(Some(self.0))?;
                for index in 0..self.0 {
                    seq.serialize_element(&index)?;
                }
                seq.end()
            }
        }

        let mut out = serializer.serialize_struct("MeshData", 3)?;
        out.serialize_field("name", &self.name)?;
        out.serialize_field("positions", &Positions(&self.mesh))?;
        out.serialize_field("triangles", &Indices(self.mesh.triangles.len() * 3))?;
        out.end()
    }
}

/// A sketch plane.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Plane {
    /// One of the origin planes: `XY`, `XZ` or `YZ`.
    Origin {
        /// The plane's name.
        name: &'static str,
    },
}

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

/// Which profiles of a sketch an extrusion uses.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Profiles {
    /// Every profile.
    All,
}

/// How far an extrusion goes.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Extent {
    /// The distance expression.
    pub distance: String,
    convention: &'static str,
    direction: &'static str,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    through_all: bool,
}

impl Extent {
    /// `distance` along the sketch normal.
    #[must_use]
    pub fn positive(distance: impl Into<String>) -> Self {
        Self { distance: distance.into(), convention: "total_distance", direction: "positive", through_all: false }
    }

    /// Through everything, both ways from the sketch plane.
    #[must_use]
    pub fn through_all() -> Self {
        Self { distance: "0".into(), convention: "total_distance", direction: "symmetric", through_all: true }
    }
}

/// What a feature does to existing bodies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    /// Makes a new body.
    NewBody,
    /// Cuts the profile out of existing bodies.
    Cut,
}

/// Where a pattern puts its copies. Counts, spacings and the origin are expressions.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PatternKind {
    /// `count1` copies `spacing1` apart along `dir1`, and as many rows of them `spacing2` apart
    /// along `dir2`; the counts include the original.
    Rectangular {
        /// First direction.
        dir1: [f64; 3],
        /// Copies along it, the original included.
        count1: String,
        /// Their spacing.
        spacing1: String,
        /// Second direction, for a grid.
        #[serde(skip_serializing_if = "Option::is_none")]
        dir2: Option<[f64; 3]>,
        /// Copies along it.
        #[serde(skip_serializing_if = "Option::is_none")]
        count2: Option<String>,
        /// Their spacing.
        #[serde(skip_serializing_if = "Option::is_none")]
        spacing2: Option<String>,
    },
    /// `count` copies spread over `angle` about the axis through `origin`.
    Circular {
        /// A point on the axis.
        origin: [String; 3],
        /// The axis direction.
        axis: [f64; 3],
        /// Copies, the original included.
        count: String,
        /// The angle they spread over (`"360 deg"`: all the way round).
        angle: String,
    },
}

/// A hole's kind.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HoleKind {
    /// A plain through hole.
    Simple,
    /// A through hole with a flat-bottomed recess at its mouth.
    Counterbore {
        /// The recess's diameter (an expression).
        cb_diameter: String,
        /// Its depth from the face (an expression).
        cb_depth: String,
    },
    /// A through hole with a cone at its mouth.
    Countersink {
        /// The cone's diameter at the face (an expression).
        cs_diameter: String,
        /// The cone's full angle (an expression in degrees).
        cs_angle: String,
    },
}

/// Builds a [`Design`], handing out feature ids and body names in order.
#[derive(Debug, Default)]
pub struct DesignBuilder {
    name: String,
    params: Vec<Param>,
    features: Vec<Feature>,
    bodies: Vec<String>,
}

impl DesignBuilder {
    /// An empty design called `name` (empty: named after the imported file).
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), ..Self::default() }
    }

    /// Adds a parameter.
    pub fn param(&mut self, param: Param) -> &mut Self {
        self.params.push(param);
        self
    }

    /// A name for the next model parameter, `d<n>`, that no parameter has yet.
    #[must_use]
    pub fn next_model_param_name(&self) -> String {
        (1..=self.params.len() + 1).map(|n| format!("d{n}")).find(|name| self.params.iter().all(|p| &p.name != name)).unwrap_or_default()
    }

    /// Appends a feature and returns its id.
    pub fn feature(&mut self, name: impl Into<String>, body_names: Vec<String>, kind: FeatureKind) -> u32 {
        let id = self.features.last().map_or(1, |f| f.id + 1);
        self.bodies.extend(body_names.iter().cloned());
        self.features.push(Feature { id, name: name.into(), suppressed: false, body_names, kind });
        id
    }

    /// The finished design.
    #[must_use]
    pub fn build(self) -> Design {
        let next_id = self.features.last().map_or(1, |f| f.id + 1);
        Design {
            format: FORMAT,
            name: self.name,
            units: "mm",
            params: self.params,
            features: self.features,
            next_id,
            reserved_body_names: self.bodies,
            next_component_color_index: 0,
        }
    }
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
    let n = corners.len();
    let mut points = vec![SketchPoint::new("origin", [0.0, 0.0], true)];
    points.extend(corners.iter().enumerate().map(|(i, c)| SketchPoint::new(format!("p{}", i + 1), *c, false)));
    let curves = (0..n)
        .map(|i| SketchCurve::Line { id: format!("l{}", i + 1), a: i + 1, b: (i + 1) % n + 1, construction: false, link: None })
        .collect();
    let mut constraints = Vec::new();
    for i in 0..n {
        let (a, b) = (corners[i], corners[(i + 1) % n]);
        let id = format!("k{}", constraints.len() + 1);
        if (a[1] - b[1]).abs() < 1e-9 {
            constraints.push(Constraint::Horizontal { id, l: i });
        } else if (a[0] - b[0]).abs() < 1e-9 {
            constraints.push(Constraint::Vertical { id, l: i });
        }
    }
    let counters = Counters { a: 0, j: 0, k: constraints.len(), l: n, p: n };
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

/// A number as SolidCraft writes it in an expression: no trailing `.0`.
#[must_use]
pub fn fmt_number(value: f64) -> String {
    let text = format!("{value}");
    text.strip_suffix(".0").map_or(text.clone(), str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_drop_trailing_zero() {
        assert_eq!(fmt_number(60.0), "60");
        assert_eq!(fmt_number(2.5), "2.5");
    }

    #[test]
    fn builder_numbers_features_and_reserves_bodies() {
        let mut b = DesignBuilder::new("t");
        let first =
            b.feature("Sketch1", vec![], FeatureKind::Sketch { plane: Plane::Origin { name: "XY" }, sketch: rectangle_sketch(1.0, 1.0) });
        let second = b.feature(
            "Extrude1",
            vec!["Body1".into()],
            FeatureKind::Extrude { sketch: first, profiles: Profiles::All, extent: Extent::positive("1"), operation: Operation::NewBody },
        );
        let design = b.build();
        assert_eq!((first, second, design.next_id), (1, 2, 3));
        assert_eq!(design.reserved_body_names, ["Body1"]);
        let json: serde_json::Value = serde_json::from_str(&design.to_json()).expect("valid JSON");
        assert_eq!(json["format"], FORMAT);
        assert_eq!(json["features"][1]["type"], "extrude");
        assert_eq!(json["features"][1]["profiles"]["type"], "all");
        assert!(json["features"][0].get("body_names").is_none());
    }
}
