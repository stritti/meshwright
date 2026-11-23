//! Feature recognition: finding the parametric part a mesh was made from.
//!
//! revmesh follows mesh2cad's approach and parameter names, one family of parts at a time.
//! Recognised so far: plates, an outline of straight sides extruded along Z, optionally with one
//! radius on every vertical corner, with round through holes; see [`crate::plate`]. A
//! rectangular outline becomes a [`PlateSpec`], driven by its sizes; any other a [`ProfileSpec`].
//! Slots, counterbores and pockets follow; whatever is not recognised stays a mesh.

mod spec;
#[cfg(test)]
mod tests;
mod to_design;

pub use spec::{Blind, BossSpec, HoleHead, HoleSpec, PadSpec, PlateSpec, PocketSpec, ProfileSpec, SlotSpec};

use crate::mesh::{Mesh, Vec3};

/// What recognition found.
#[derive(Debug, Clone, PartialEq)]
pub enum Recognition {
    /// A rectangular plate, possibly with holes and rounded corners.
    Plate {
        /// The plate, its corner at the origin.
        plate: PlateSpec,
        /// Where the plate's corner was in the file; the design puts it at the origin.
        offset: Vec3,
    },
    /// A plate with any outline of straight sides, possibly with holes and rounded corners.
    Profile {
        /// The plate, its bounding box's corner at the origin.
        profile: ProfileSpec,
        /// Where that corner was in the file; the design puts it at the origin.
        offset: Vec3,
    },
    /// Nothing the recognisers know; the mesh is kept as it is.
    Unrecognized {
        /// Why, in the user's words.
        reason: String,
    },
}

/// Looks for a part the recognisers know in `mesh`.
#[must_use]
pub fn recognize(mesh: &Mesh) -> Recognition {
    crate::plate::recognize_plate(mesh).unwrap_or_else(|reason| Recognition::Unrecognized { reason })
}
