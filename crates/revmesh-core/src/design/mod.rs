//! The SolidCraft design document an importer writes.
//!
//! SolidCraft's document format is internal and has no published schema. What is modelled here
//! is the subset revmesh writes, taken from designs SolidCraft itself saved as JSON (see
//! `docs/design-format.md`). SolidCraft upgrades older format versions when it opens a file, so
//! the importer pins one version, [`FORMAT`], and is tested against each SolidCraft release.
//!
//! Fields SolidCraft derives on its own (`param_names`, `face_names`, `edge_refs`) are left out.
//! `edge_names` are written: without them SolidCraft re-finds fillet edges only by their pick
//! points, which can snap to the wrong edge once a parameter moves the geometry.

mod sketch;
#[cfg(test)]
mod tests;

pub use sketch::{
    Constraint, Link, PointExprs, PolygonAngle, Sketch, SketchCurve, SketchPoint, SlotDimensions, dimensioned_angle, dimensioned_sides,
    polygon_sketch, polygon_sketch_dimensioned, polygons_sketch, polygons_sketch_dimensioned, rectangle_sketch, slot_sketch,
};

use serde::Serialize;

use crate::mesh::Mesh;

/// The document format version revmesh writes: `solidcraft/63` (SolidCraft 0.49.0 upgrades older
/// formats when it opens a file, so this is still read).
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

    /// A user parameter in millimetres driven by `expr`, which follows the parameters it names.
    #[must_use]
    pub fn user_expr(name: impl Into<String>, expr: impl Into<String>) -> Self {
        Self { name: name.into(), expr: expr.into(), unit: "mm", model: false }
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

/// The kinds of feature revmesh writes.
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
    /// A cylinder primitive: `base` is the centre of its bottom face (three expressions), and it
    /// stands `height` along `axis`.
    Cylinder {
        /// The bottom face's centre.
        base: [String; 3],
        /// The direction from the bottom to the top.
        axis: [f64; 3],
        /// The radius expression.
        radius: String,
        /// The height expression.
        height: String,
        /// What it does to existing bodies.
        operation: Operation,
    },
    /// A cone primitive (a frustum): `base` is the centre of its bottom circle of `radius`, its
    /// top circle of `top_radius` lies `height` along `axis`.
    Cone {
        /// The bottom circle's centre.
        base: [String; 3],
        /// The direction from the bottom to the top.
        axis: [f64; 3],
        /// The bottom radius expression.
        radius: String,
        /// The top radius expression.
        top_radius: String,
        /// The height expression.
        height: String,
        /// What it does to existing bodies.
        operation: Operation,
    },
    /// A chamfer of edges: `distance` along the first face (the one facing up most), and
    /// `distance2` along the second when the two differ.
    Chamfer {
        /// Topological names of the edges, as for a fillet.
        edge_names: Vec<String>,
        /// One point on each edge.
        edges: Vec<[f64; 3]>,
        /// The distance along the first face (an expression).
        distance: String,
        /// The distance along the second face, if it differs.
        #[serde(skip_serializing_if = "Option::is_none")]
        distance2: Option<String>,
    },
    /// A box primitive from its lowest `corner` (three expressions), `length` along X, `width`
    /// along Y and `height` along Z.
    Box {
        /// The lowest corner.
        corner: [String; 3],
        /// Along X.
        length: String,
        /// Along Y.
        width: String,
        /// Along Z.
        height: String,
        /// What it does to existing bodies.
        operation: Operation,
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

/// Which profiles of a sketch an extrusion uses.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Profiles {
    /// Every profile.
    All,
    /// The profiles whose outer loops are made of these curves (by id), the loops inside them
    /// their holes.
    Curves {
        /// Each profile's outer loop.
        loops: Vec<Vec<String>>,
    },
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
    /// Where it starts along the sketch normal, if not on the sketch plane.
    #[serde(skip_serializing_if = "Option::is_none")]
    start_offset: Option<String>,
}

impl Extent {
    /// `distance` along the sketch normal.
    #[must_use]
    pub fn positive(distance: impl Into<String>) -> Self {
        Self { distance: distance.into(), convention: "total_distance", direction: "positive", through_all: false, start_offset: None }
    }

    /// The same extent, starting `offset` along the sketch normal from the sketch plane.
    #[must_use]
    pub fn starting_at(mut self, offset: impl Into<String>) -> Self {
        self.start_offset = Some(offset.into());
        self
    }

    /// Through everything, both ways from the sketch plane.
    #[must_use]
    pub fn through_all() -> Self {
        Self { distance: "0".into(), convention: "total_distance", direction: "symmetric", through_all: true, start_offset: None }
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
    /// Adds to an existing body.
    Join,
    /// Keeps only what the profile's extrusion and the existing bodies have in common.
    Intersect,
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
    /// The next `d<n>` sequence number for a model parameter, so
    /// [`Self::next_model_param_name`] need not rescan the parameters per name.
    model_param_seq: usize,
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

    /// A name for the next model parameter, `d<n>`, that no parameter has yet. The sequence
    /// advances monotonically, so allocating a name does not rescan all parameters.
    #[must_use]
    pub fn next_model_param_name(&mut self) -> String {
        loop {
            self.model_param_seq += 1;
            let name = format!("d{}", self.model_param_seq);
            if self.params.iter().all(|p| p.name != name) {
                return name;
            }
        }
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

/// A number as SolidCraft writes it in an expression: no trailing `.0`.
#[must_use]
pub fn fmt_number(value: f64) -> String {
    let text = format!("{value}");
    text.strip_suffix(".0").map_or(text.clone(), str::to_owned)
}
