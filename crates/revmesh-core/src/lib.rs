//! # revmesh-core
//!
//! The host-independent part of revmesh: everything that turns the bytes of an STL file into a
//! SolidCraft design, without knowing anything about WebAssembly or the plugin interface.
//!
//! The pipeline is:
//!
//! 1. [`stl::parse`] reads binary or ASCII STL into a [`mesh::Mesh`].
//! 2. [`recognize::recognize`] looks for features (plates, holes, fillets, …) in the mesh.
//! 3. [`import::import`] turns what was recognised into a [`design::Design`], a SolidCraft
//!    document with a parametric feature history, and a [`report::LossReport`] of what could not
//!    be carried over.
//!
//! The crate is kept free of host calls so that every step can be tested with `cargo test` on the
//! developer's machine. The `revmesh-plugin` crate is a thin wrapper that feeds it the file and
//! writes the design back to SolidCraft.
//!
//! Parts of the recognition are ported from [mesh2cad](https://github.com/emirmrkaljevic/mesh2cad)
//! (MIT licensed); see `NOTICE` at the repository root.

pub mod design;
pub mod import;
pub mod mesh;
pub mod patterns;
pub mod plate;
pub mod recognize;
pub mod report;
pub mod stl;
pub mod synth;

pub use import::{ImportOptions, ImportResult, import, import_mesh};
