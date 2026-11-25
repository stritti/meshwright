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

/// The per-face surface deviation report for a recognised part (roadmap 5.4): a plate's or a
/// profile's walls measured in the Z frame (another axis's profile is rotated into it), a
/// combined part's per extrusion ("extrude z side 1", "extrude z cut 1 side 2", "extrude z hole
/// 1"), a stack's per band ("band 2 side 1"). Empty for an unrecognised mesh.
pub(crate) fn deviations(mesh: &Mesh, found: &Recognition) -> Vec<(String, f64)> {
    match found {
        Recognition::Plate { .. } => crate::plate::face_deviations(mesh),
        Recognition::Profile { axis, .. } => crate::plate::face_deviations(&axis.rotate_mesh(mesh)),
        Recognition::Combined { spec, .. } => {
            // The combined spec's frame: the bounding box's corner at the origin in world
            // coordinates, then rotated per axis (along Y the profile's second coordinate is
            // −Z). An axis with no outline of its own (one that only a cut or a hole runs
            // along) gets the box's side as the outline, so the cut's and hole's walls are found.
            let bounds = mesh.bounds().expect("recognised meshes are not empty");
            let in_frame =
                |axis: Axis| Mesh::new(mesh.triangles.iter().map(|t| t.map(|p| axis.rotate(crate::mesh::sub(p, bounds.min)))).collect());
            let box_side = |axis: Axis| {
                let [u, v] = axis.profile_axes();
                let lo = if axis.profile_sign(1) < 0.0 { -spec.size[v] } else { 0.0 };
                vec![[0.0, lo], [spec.size[u], lo], [spec.size[u], lo + spec.size[v]], [0.0, lo + spec.size[v]]]
            };
            let mut axes: Vec<Axis> = spec.extrusions.iter().map(|e| e.axis).collect();
            for extra in spec.cuts.iter().map(|c| c.axis).chain(spec.holes.iter().map(|h| h.axis)) {
                if !axes.contains(&extra) {
                    axes.push(extra);
                }
            }
            let mut out = Vec::new();
            for axis in axes {
                let extrusion = spec.extrusions.iter().find(|e| e.axis == axis);
                let outline: Vec<[f64; 2]> = extrusion.map_or_else(|| box_side(axis), |e| e.outline.clone());
                let cutouts: Vec<CutoutSpec> = extrusion
                    .map_or_else(Vec::new, |e| e.cutouts.iter().map(|c| CutoutSpec { corners: c.clone(), arcs: Vec::new() }).collect());
                let cuts: Vec<Vec<[f64; 2]>> = spec.cuts.iter().filter(|c| c.axis == axis).map(|c| c.outline.clone()).collect();
                let circles: Vec<(usize, [f64; 2], f64)> =
                    spec.holes.iter().enumerate().filter(|(_, h)| h.axis == axis).map(|(i, h)| (i, h.centre, h.d / 2.0)).collect();
                let prefix = format!("extrude {}", axis.letter().to_ascii_lowercase());
                out.extend(crate::plate::walls_deviations(
                    &in_frame(axis),
                    &outline,
                    &cutouts,
                    &cuts,
                    &circles,
                    (0.0, spec.size[axis.index()]),
                    &prefix,
                ));
            }
            out
        }
        Recognition::Stack { spec, axis, .. } => {
            // The stack spec's frame: the mesh rotated into the axis's frame, rebased to that
            // frame's lowest corner (as `recognize_stack` does).
            let rotated = axis.rotate_mesh(mesh);
            let min = rotated.bounds().expect("recognised meshes are not empty").min;
            let in_frame = Mesh::new(rotated.triangles.iter().map(|t| t.map(|p| crate::mesh::sub(p, min))).collect());
            let mut out = Vec::new();
            for (k, band) in spec.bands.iter().enumerate() {
                let cutouts: Vec<CutoutSpec> = band.cutouts.iter().map(|c| CutoutSpec { corners: c.clone(), arcs: Vec::new() }).collect();
                let prefix = format!("band {}", k + 1);
                out.extend(crate::plate::walls_deviations(&in_frame, &band.outline, &cutouts, &[], &[], (band.from, band.to), &prefix));
            }
            out
        }
        Recognition::Unrecognized { .. } => Vec::new(),
    }
}

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
