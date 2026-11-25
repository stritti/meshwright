use crate::design::fmt_number;
use crate::i18n::Locale;

/// A rectangular plate from the origin, extruded up, with vertical through holes.
#[derive(Debug, Clone, PartialEq)]
pub struct PlateSpec {
    /// Size along X.
    pub x: f64,
    /// Size along Y.
    pub y: f64,
    /// Thickness along Z.
    pub z: f64,
    /// Through holes.
    pub holes: Vec<HoleSpec>,
    /// Straight through slots.
    pub slots: Vec<SlotSpec>,
    /// Round bosses standing on the top face.
    pub bosses: Vec<BossSpec>,
    /// Rectangular blocks standing on the top face.
    pub pads: Vec<PadSpec>,
    /// Rectangular blind pockets in the top or the bottom face.
    pub pockets: Vec<PocketSpec>,
    /// Openings of straight sides through the plate, each its corners counter-clockwise.
    pub cutouts: Vec<Vec<[f64; 2]>>,
    /// The radius of the four vertical corner edges, if they are rounded.
    pub outline_fillet_r: Option<f64>,
    /// The radius of all twelve edges (the four vertical corner edges and the eight on the top
    /// and bottom faces), if the box is rounded all over; mesh2cad's `edge_fillet_r`. Never set
    /// together with `outline_fillet_r`.
    pub edge_fillet_r: Option<f64>,
}

/// A rectangular block standing on the top face, its sides along X and Y: `w` along X, `l`
/// along Y and `h` high, about its middle `(x, y)`.
#[derive(Debug, Clone, PartialEq)]
pub struct PadSpec {
    /// Middle X.
    pub x: f64,
    /// Middle Y.
    pub y: f64,
    /// Size along X.
    pub w: f64,
    /// Size along Y.
    pub l: f64,
    /// Height above the top face.
    pub h: f64,
}

/// A rectangular blind pocket with a flat floor, its sides along X and Y: `w` along X, `l`
/// along Y and `depth` deep from the face it opens on, about its middle `(x, y)`.
#[derive(Debug, Clone, PartialEq)]
pub struct PocketSpec {
    /// Middle X.
    pub x: f64,
    /// Middle Y.
    pub y: f64,
    /// Size along X.
    pub w: f64,
    /// Size along Y.
    pub l: f64,
    /// Depth from the face.
    pub depth: f64,
    /// Whether it opens on the bottom face (else on the top).
    pub bottom: bool,
}

/// A blind pocket whose mouth is any polygon of straight sides (not just an axis-aligned
/// rectangle), cut from the top or the bottom face down to a flat floor.
#[derive(Debug, Clone, PartialEq)]
pub struct OutlinePocketSpec {
    /// The mouth's corners, counter-clockwise from the lowest, the leftmost of those.
    pub corners: Vec<[f64; 2]>,
    /// Depth from the face.
    pub depth: f64,
    /// Whether it opens on the bottom face (else on the top).
    pub bottom: bool,
}

impl OutlinePocketSpec {
    /// Whether `p` lies inside the mouth's polygon, or within `tol` of its edge.
    #[must_use]
    #[allow(clippy::many_single_char_names)] // p, n and the edge vectors, as in the geometry
    pub fn contains(&self, p: [f64; 2], tol: f64) -> bool {
        let ring = &self.corners;
        let n = ring.len();
        let on_edge = ring.iter().zip(ring.iter().cycle().skip(1)).any(|(a, b)| {
            let (u, v) = ([b[0] - a[0], b[1] - a[1]], [p[0] - a[0], p[1] - a[1]]);
            let len2 = u[0] * u[0] + u[1] * u[1];
            let t = if len2 <= 0.0 { 0.0 } else { ((u[0] * v[0] + u[1] * v[1]) / len2).clamp(0.0, 1.0) };
            ((v[0] - t * u[0]).powi(2) + (v[1] - t * u[1]).powi(2)).sqrt() <= tol
        });
        if on_edge {
            return true;
        }
        // Even-odd ray casting along +X.
        let mut inside = false;
        for i in 0..n {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            if (a[1] > p[1]) != (b[1] > p[1]) && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0] {
                inside = !inside;
            }
        }
        inside
    }
}

/// A round boss standing on the top face: a cylinder `d` wide and `h` high about `(x, y)`.
#[derive(Debug, Clone, PartialEq)]
pub struct BossSpec {
    /// Centre X.
    pub x: f64,
    /// Centre Y.
    pub y: f64,
    /// Diameter.
    pub d: f64,
    /// Height above the top face.
    pub h: f64,
    /// The radius of the round where the boss meets the top face, if it has one.
    pub fillet: Option<f64>,
    /// The taper of a drafted boss (a cone narrowing upwards), degrees from the axis: `d` is
    /// then its diameter at the top face.
    pub draft_deg: Option<f64>,
    /// A chamfer of the boss's top edge: `w` in from the side, `h` down from the top.
    pub chamfer: Option<BossChamfer>,
}

/// A chamfer of a boss's top edge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BossChamfer {
    /// How far in from the side it reaches on the top.
    pub w: f64,
    /// How far down from the top it reaches on the side.
    pub h: f64,
}

impl BossSpec {
    /// The boss's volume, in mm³, with the material its foot's round adds: the ring a quarter
    /// circle's outside sweeps round the cylinder (Pappus: the square of side `f` beside the
    /// wall, less the quarter disk, about the axis).
    #[must_use]
    pub fn volume(&self) -> f64 {
        use std::f64::consts::PI;
        let r = self.d / 2.0;
        let ring = self.fillet.map_or(0.0, |f| 2.0 * PI * (f * f * (r + f / 2.0) - PI * f * f / 4.0 * (r + f) + f.powi(3) / 3.0));
        if let Some(draft) = self.draft_deg {
            // A frustum from the full radius at the face to the top's.
            let top = r - self.h * draft.to_radians().tan();
            return PI * self.h / 3.0 * (r * r + r * top + top * top) + ring;
        }
        // A chamfer takes off the triangle `w` x `h` swept about the axis at its centroid.
        let chamfer = self.chamfer.map_or(0.0, |c| 2.0 * PI * (r - c.w / 3.0) * c.w * c.h / 2.0);
        PI * r * r * self.h + ring - chamfer
    }
}

/// A straight obround (stadium) slot through the plate along Z: two round ends `w` wide whose
/// centres are `len` apart, about the middle `(x, y)`, its long axis `angle` degrees from X
/// (0 up to but not including 180).
#[derive(Debug, Clone, PartialEq)]
pub struct SlotSpec {
    /// Middle X.
    pub x: f64,
    /// Middle Y.
    pub y: f64,
    /// Between the end centres.
    pub len: f64,
    /// Width, the ends' diameter.
    pub w: f64,
    /// The long axis from X, degrees.
    pub angle: f64,
}

impl SlotSpec {
    /// The slot's area seen from above, in mm².
    #[must_use]
    pub fn area(&self) -> f64 {
        self.len * self.w + std::f64::consts::PI * self.w * self.w / 4.0
    }
}

/// A vertical hole, through the plate or blind.
#[derive(Debug, Clone, PartialEq)]
pub struct HoleSpec {
    /// Centre X.
    pub x: f64,
    /// Centre Y.
    pub y: f64,
    /// Diameter (of the bore, below a counterbore or countersink).
    pub d: f64,
    /// A counterbore or countersink at one end, if it has one.
    pub head: Option<HoleHead>,
    /// Where a blind hole stops; `None` for a hole through the plate.
    pub blind: Option<Blind>,
}

/// A blind hole's end: a flat floor `depth` below the face it is drilled from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Blind {
    /// Depth from the face.
    pub depth: f64,
    /// Whether it is drilled from the bottom face (else from the top).
    pub bottom: bool,
}

/// The wider mouth of a hole, at the top or the bottom face.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HoleHead {
    /// A flat-bottomed recess: `d` wide and `depth` deep from the face it opens on.
    Counterbore {
        /// Diameter.
        d: f64,
        /// Depth from the face.
        depth: f64,
        /// Whether it opens on the bottom face (else on the top).
        bottom: bool,
    },
    /// A cone: `d` wide at the face it opens on, narrowing at `angle` (degrees, the cone's full
    /// angle) down to the bore.
    Countersink {
        /// Diameter at the face.
        d: f64,
        /// The cone's full angle, degrees.
        angle: f64,
        /// Whether it opens on the bottom face (else on the top).
        bottom: bool,
    },
    /// A round of radius `r` where the bore meets the face: the mouth is `2 r` wider.
    Round {
        /// The round's radius.
        r: f64,
        /// Whether it is on the bottom face's rim (else on the top's).
        bottom: bool,
    },
}

impl HoleHead {
    /// Whether it opens on the bottom face (else on the top's).
    #[must_use]
    pub fn bottom(&self) -> bool {
        match self {
            Self::Counterbore { bottom, .. } | Self::Countersink { bottom, .. } | Self::Round { bottom, .. } => *bottom,
        }
    }
}

impl HoleSpec {
    /// The material the hole removes from a plate `z` thick, in mm³.
    #[must_use]
    pub fn volume(&self, z: f64) -> f64 {
        let area = |d: f64| std::f64::consts::PI * d * d / 4.0;
        let bore = area(self.d) * self.blind.map_or(z, |b| b.depth);
        match self.head {
            None => bore,
            Some(HoleHead::Counterbore { d, depth, .. }) => bore + (area(d) - area(self.d)) * depth,
            // The round takes off the corner between the bore and the face: the square of side
            // `r` beside the bore less the quarter disk, swept about the axis (Pappus).
            Some(HoleHead::Round { r, .. }) => {
                use std::f64::consts::PI;
                let b = self.d / 2.0;
                bore + 2.0 * PI * (r * r * (b + r / 2.0) - PI * r * r / 4.0 * (b + r) + r.powi(3) / 3.0)
            }
            Some(HoleHead::Countersink { d, angle, .. }) => {
                // The frustum from the face down to where the cone meets the bore, less the bore
                // it overlaps.
                let (big, small) = (d / 2.0, self.d / 2.0);
                let h = (big - small) / (angle.to_radians() / 2.0).tan();
                bore + std::f64::consts::PI * h / 3.0 * (big * big + big * small + small * small) - area(self.d) * h
            }
        }
    }
}

impl PlateSpec {
    /// The same plate as an outline: `extrude_len` thick with its four corners as the outline,
    /// the way mesh2cad's extrusion module writes a rectangle extruded along X or Y (its box
    /// vocabulary is a Z extrusion's).
    #[must_use]
    pub fn into_profile(self) -> ProfileSpec {
        ProfileSpec {
            corners: vec![[0.0, 0.0], [self.x, 0.0], [self.x, self.y], [0.0, self.y]],
            z: self.z,
            holes: self.holes,
            slots: self.slots,
            bosses: self.bosses,
            pads: self.pads,
            pockets: self.pockets,
            outline_pockets: Vec::new(),
            cutouts: self.cutouts,
            outline_fillet_r: self.outline_fillet_r,
            // A box rounded all over is a Z plate (mesh2cad's box vocabulary is a Z
            // extrusion's); recognize() tries Z first and never converts it to a profile.
        }
    }

    /// One line describing the plate, for the loss report.
    #[must_use]
    pub fn describe(&self) -> String {
        self.describe_in(Locale::En)
    }

    /// [`Self::describe`] in `locale`'s words.
    #[must_use]
    pub fn describe_in(&self, locale: Locale) -> String {
        describe(
            locale,
            [self.x, self.y, self.z],
            None,
            [self.holes.len(), self.slots.len(), self.bosses.len(), self.cutouts.len(), self.pads.len(), self.pockets.len()],
            self.outline_fillet_r,
            self.edge_fillet_r,
        )
    }

    /// The volume of the plate this spec describes, in mm³: what SolidCraft measures once it has
    /// rebuilt the design.
    #[must_use]
    pub fn volume(&self) -> f64 {
        let base = if let Some(r) = self.edge_fillet_r {
            self.x * self.y * self.z - crate::plate::edge_fillet_removed(self.x, self.y, self.z, r)
        } else {
            (self.x * self.y - self.outline_fillet_r.map_or(0.0, crate::plate::corner_loss)) * self.z
        };
        let holes: f64 = self.holes.iter().map(|h| h.volume(self.z)).sum();
        let slots: f64 = self.slots.iter().map(SlotSpec::area).sum();
        let bosses: f64 = self.bosses.iter().map(BossSpec::volume).sum();
        let cutouts: f64 = self.cutouts.iter().map(|c| polygon_area(c)).sum();
        base - (slots + cutouts) * self.z - holes + bosses + blocks(&self.pads, &self.pockets)
    }
}

/// A plate whose outline is any polygon of straight sides (counter-clockwise, in the first
/// quadrant), extruded up from the XY plane, with vertical through holes.
#[derive(Debug, Clone, PartialEq)]
pub struct ProfileSpec {
    /// The outline's corners as if they were sharp, counter-clockwise.
    pub corners: Vec<[f64; 2]>,
    /// Thickness along Z.
    pub z: f64,
    /// Through holes.
    pub holes: Vec<HoleSpec>,
    /// Straight through slots.
    pub slots: Vec<SlotSpec>,
    /// Round bosses standing on the top face.
    pub bosses: Vec<BossSpec>,
    /// Rectangular blocks standing on the top face.
    pub pads: Vec<PadSpec>,
    /// Rectangular blind pockets in the top or the bottom face.
    pub pockets: Vec<PocketSpec>,
    /// Blind pockets of any outline of straight sides in the top or the bottom face.
    pub outline_pockets: Vec<OutlinePocketSpec>,
    /// Openings of straight sides through the plate, each its corners counter-clockwise.
    pub cutouts: Vec<Vec<[f64; 2]>>,
    /// The radius of every vertical corner edge, inside and outside corners alike, if they are
    /// rounded.
    pub outline_fillet_r: Option<f64>,
}

impl ProfileSpec {
    /// One line describing the plate, for the loss report.
    #[must_use]
    pub fn describe(&self) -> String {
        self.describe_in(Locale::En)
    }

    /// [`Self::describe`] in `locale`'s words.
    #[must_use]
    pub fn describe_in(&self, locale: Locale) -> String {
        let size = self.size();
        describe(
            locale,
            [size[0], size[1], self.z],
            Some(self.corners.len()),
            [
                self.holes.len(),
                self.slots.len(),
                self.bosses.len(),
                self.cutouts.len(),
                self.pads.len(),
                self.pockets.len() + self.outline_pockets.len(),
            ],
            self.outline_fillet_r,
            None,
        )
    }

    /// The size of the outline's bounding box along X and Y.
    #[must_use]
    pub fn size(&self) -> [f64; 2] {
        let max = |axis: usize| self.corners.iter().map(|c| c[axis]).fold(0.0, f64::max);
        [max(0), max(1)]
    }

    /// The volume of the plate this spec describes, in mm³.
    #[must_use]
    pub fn volume(&self) -> f64 {
        let n = self.corners.len();
        let mut area = (0..n)
            .map(|i| {
                let (a, b) = (self.corners[i], self.corners[(i + 1) % n]);
                a[0] * b[1] - b[0] * a[1]
            })
            .sum::<f64>()
            / 2.0;
        if let Some(r) = self.outline_fillet_r {
            for i in 0..n {
                let (a, c, b) = (self.corners[(i + n - 1) % n], self.corners[i], self.corners[(i + 1) % n]);
                area -= crate::plate::fillet_loss(a, c, b, r);
            }
        }
        let holes: f64 = self.holes.iter().map(|h| h.volume(self.z)).sum();
        let slots: f64 = self.slots.iter().map(SlotSpec::area).sum();
        let bosses: f64 = self.bosses.iter().map(BossSpec::volume).sum();
        let cutouts: f64 = self.cutouts.iter().map(|c| polygon_area(c)).sum();
        let outline_pockets: f64 = self.outline_pockets.iter().map(|p| polygon_area(&p.corners) * p.depth).sum();
        (area - slots - cutouts) * self.z - holes - outline_pockets + bosses + blocks(&self.pads, &self.pockets)
    }
}

/// "A 70 x 40 x 6 mm plate with a 6-sided outline, 2 holes and 3 mm rounded corners".
/// The area a polygon encloses, either way round.
fn polygon_area(corners: &[[f64; 2]]) -> f64 {
    let n = corners.len();
    ((0..n).map(|i| corners[i][0] * corners[(i + 1) % n][1] - corners[(i + 1) % n][0] * corners[i][1]).sum::<f64>() / 2.0).abs()
}

/// The volume pads add and pockets take away, in mm³.
fn blocks(pads: &[PadSpec], pockets: &[PocketSpec]) -> f64 {
    pads.iter().map(|p| p.w * p.l * p.h).sum::<f64>() - pockets.iter().map(|p| p.w * p.l * p.depth).sum::<f64>()
}

/// `counts` are the holes, the slots, the bosses, the cut-outs, the pads and the pockets.
fn describe(
    locale: Locale,
    size: [f64; 3],
    sides: Option<usize>,
    counts: [usize; 6],
    fillet_r: Option<f64>,
    edge_r: Option<f64>,
) -> String {
    let number = |v: f64| locale.decimal(&fmt_number(v));
    let [x, y, z] = size.map(number);
    let mut parts = Vec::new();
    if let Some(n) = sides {
        parts.push(locale.format("summary.outline", &[("n", &n)]));
    }
    for (n, kind) in counts.into_iter().zip(["hole", "slot", "boss", "cutout", "pad", "pocket"]) {
        match n {
            0 => {}
            1 => parts.push(locale.text(&format!("summary.{kind}.one")).to_owned()),
            n => parts.push(locale.format(&format!("summary.{kind}.other"), &[("n", &n)])),
        }
    }
    if let Some(r) = fillet_r {
        parts.push(locale.format("summary.rounded_corners", &[("r", &number(r))]));
    }
    if let Some(r) = edge_r {
        parts.push(locale.format("summary.rounded_edges", &[("r", &number(r))]));
    }
    let plate = locale.format("summary.plate", &[("x", &x), ("y", &y), ("z", &z)]);
    let Some((last, rest)) = parts.split_last() else {
        return plate;
    };
    let parts = if rest.is_empty() { last.clone() } else { locale.format("summary.list", &[("rest", &rest.join(", ")), ("last", last)]) };
    locale.format("summary.with", &[("plate", &plate), ("parts", &parts)])
}
