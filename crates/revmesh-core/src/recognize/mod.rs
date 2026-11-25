//! Feature recognition: finding the parametric part a mesh was made from.
//!
//! revmesh follows mesh2cad's approach and parameter names, one family of parts at a time.
//! Recognised so far: prisms along a principal axis — an outline of straight sides, optionally
//! with one radius on every corner parallel to the axis, with round holes — and boxes with all
//! twelve edges rounded with one radius (`edge_fillet_r`); see [`crate::plate`].
//! A rectangular outline extruded along Z becomes a [`PlateSpec`], driven by its sizes; any
//! other outline, and any outline extruded along X or Y, a [`ProfileSpec`]. Slots, counterbores
//! and pockets follow; whatever is not recognised stays a mesh.
//!
//! Like mesh2cad, which tries a prism along X, Y and Z, [`recognize`] tries each axis: the mesh
//! is rotated so the axis becomes Z (see [`Axis`]), the Z pipeline runs unchanged, and the result
//! is rotated back. Z is tried first, so an axis-aligned box keeps the `base_*` plate mesh2cad's
//! box vocabulary writes for it.
//!
//! When no prism fits, a part may still be the intersection of outlines extruded along two or
//! three axes (a [`CombinedSpec`], see `crate::combined`), as mesh2cad's boolean of extrusions
//! falls back to.

mod axis;
mod combined;
mod spec;
mod stack;
#[cfg(test)]
pub(crate) mod tests;
mod to_design;

pub use axis::Axis;
pub use combined::{CombinedHole, CombinedSpec, Cut, Extrusion};
pub use spec::{
    ArcSide, Blind, BossChamfer, BossSpec, CutoutSpec, HoleHead, HoleSpec, OutlinePocketSpec, PadSpec, PlateSpec, PocketSpec, ProfileSpec,
    SlotSpec,
};
pub use stack::{Band, StackSpec};

use crate::mesh::{Mesh, Vec3};

/// What recognition found.
#[derive(Debug, Clone, PartialEq)]
pub enum Recognition {
    /// A rectangular plate extruded along Z, possibly with holes and rounded corners. (A
    /// rectangle extruded along X or Y is a [`Recognition::Profile`], as mesh2cad writes it.)
    Plate {
        /// The plate, its corner at the origin.
        plate: PlateSpec,
        /// Where the plate's corner was in the file; the design puts it at the origin.
        offset: Vec3,
    },
    /// An outline of straight sides extruded along `axis`, possibly with holes and rounded
    /// corners.
    Profile {
        /// The profile, its bounding box's corner at the origin, in the axis's profile
        /// coordinates (the sketch coordinates on the axis's plane).
        profile: ProfileSpec,
        /// Where that corner was in the file; the design puts it at the origin.
        offset: Vec3,
        /// The axis the outline is extruded along.
        axis: Axis,
    },
    /// The intersection of outlines extruded along two or three axes.
    Combined {
        /// The part, its bounding box's corner at the origin.
        spec: CombinedSpec,
        /// Where that corner was in the file; the design puts it at the origin.
        offset: Vec3,
    },
    /// Outlines stacked along `axis`, each extruded over its own span, joined.
    Stack {
        /// The stack, in its axis's frame, its bounding box's corner at the origin there.
        spec: StackSpec,
        /// Where that corner is in the file (the design puts it at the origin).
        offset: Vec3,
        /// The axis the outlines are stacked along.
        axis: Axis,
    },
    /// Nothing the recognisers know; the mesh is kept as it is.
    Unrecognized {
        /// Why, in the user's words.
        reason: String,
    },
}

/// Looks for a part the recognisers know in `mesh`: a prism along Z, then along X, then along Y
/// (each but the first on the mesh rotated, see [`Axis`]), then an intersection of extrusions.
#[must_use]
pub fn recognize(mesh: &Mesh) -> Recognition {
    let mut reason = String::new();
    for axis in Axis::ALL {
        let rotated;
        let source = if axis == Axis::Z {
            mesh
        } else {
            rotated = axis.rotate_mesh(mesh);
            &rotated
        };
        match crate::plate::recognize_plate(source, axis == Axis::Z) {
            Ok(Recognition::Plate { plate, offset }) => {
                let offset = axis.unrotate(offset).map(crate::mesh::snap_mm);
                if axis == Axis::Z {
                    return Recognition::Plate { plate, offset };
                }
                if plate.edge_fillet_r.is_some() {
                    // A box rounded all over is written as a Z plate (mesh2cad's box vocabulary
                    // is a Z extrusion's); Z was tried first and would have found it.
                    if reason.is_empty() {
                        reason = "a box with all edges rounded along another axis".into();
                    }
                    continue;
                }
                // Along X or Y a rectangle is written as an outline (`extrude_len` and its
                // corners), the way mesh2cad's extrusion module writes it.
                return Recognition::Profile { profile: plate.into_profile(), offset, axis };
            }
            Ok(Recognition::Profile { profile, offset, .. }) => {
                return Recognition::Profile { profile, offset: axis.unrotate(offset).map(crate::mesh::snap_mm), axis };
            }
            Ok(Recognition::Unrecognized { .. } | Recognition::Combined { .. } | Recognition::Stack { .. }) => {
                unreachable!("recognize_plate finds prisms or errs")
            }
            // The first axis's reason is the one reported: Z reads most naturally, and it is the
            // reason every mesh got before X and Y were tried.
            Err(why) => {
                if reason.is_empty() {
                    reason = why;
                }
            }
        }
    }
    // The prisms' reason stays the one reported: a part that is no intersection of extrusions
    // either is most often a prism with something the recognisers do not know yet.
    if let Ok((spec, offset)) = crate::combined::recognize_combined(mesh) {
        return Recognition::Combined { spec, offset };
    }
    // As mesh2cad, a union of partial extrusions when the intersection misses. A part whose
    // faces all lie square to the axes is a stack along each of them: the fewest bands win, the
    // first of Z, X and Y on a tie.
    let mut best: Option<Recognition> = None;
    for axis in Axis::ALL {
        let rotated;
        let source = if axis == Axis::Z {
            mesh
        } else {
            rotated = axis.rotate_mesh(mesh);
            &rotated
        };
        if let Ok((spec, offset)) = crate::stack::recognize_stack(source) {
            let fewer = match &best {
                Some(Recognition::Stack { spec: b, .. }) => spec.bands.len() < b.bands.len(),
                _ => true,
            };
            if fewer {
                best = Some(Recognition::Stack { spec, offset: axis.unrotate(offset).map(crate::mesh::snap_mm), axis });
            }
        }
    }
    best.unwrap_or(Recognition::Unrecognized { reason })
}
