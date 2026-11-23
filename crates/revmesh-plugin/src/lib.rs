//! # revmesh-plugin
//!
//! The SolidCraft side of revmesh: a WebAssembly component in the `importer-plugin` world of
//! `craft:plugin@0.2.0` (vendored in `wit/plugin.wit`, from SolidCraft 0.47.0). It reads the file
//! SolidCraft hands it, lets [`revmesh_core`] do the work, and writes the design back.
//!
//! Memory is the tight budget (128 MiB by default), so nothing is held twice: the STL is parsed
//! chunk by chunk as it is read, the mesh moves into the design, and the design is serialised
//! straight into 1 MiB writes to the host.
//!
//! Build it for `wasm32-unknown-unknown` (SolidCraft links no WASI) and wrap the module as a
//! component; `cargo xtask dist` does both and packages the plugin.
//!
//! On any other target the crate is empty: the bindings export symbols a native linker refuses,
//! and everything worth testing natively lives in `revmesh-core`. Lint it with
//! `cargo clippy -p revmesh-plugin --target wasm32-unknown-unknown`.

#![cfg(target_arch = "wasm32")]
#![allow(unsafe_code, missing_docs, unreachable_pub, clippy::pedantic)] // generated bindings

wit_bindgen::generate!({ path: "wit", world: "importer-plugin" });

use craft::plugin::types::{FieldFailure, PluginError};
use craft::plugin::{control, design, source};
use exports::craft::plugin::importer::Guest;
use revmesh_core::import::Mode;
use revmesh_core::mesh::Mesh;
use revmesh_core::stl::StlReader;
use revmesh_core::{ImportOptions, import_mesh};

/// Largest read or write the host accepts in one call.
const CHUNK: usize = 1 << 20;

// Binary STL expands from 50 to 72 bytes per triangle. Mesh mode also needs the
// read/write buffers, while auto recognition builds face and wall indexes and can
// temporarily hold a second mesh. Keep a lower input ceiling for that mode.
const AUTO_MAX_BYTES: u64 = 8 << 20;
const MESH_MAX_BYTES: u64 = 32 << 20;

struct Importer;

impl Guest for Importer {
    fn import_file(_entry: String, file: source::FileInfo, options: String) -> Result<String, PluginError> {
        let options = ImportOptions::from_json(&options).map_err(|e| invalid("options", e))?;
        let max_bytes = match options.mode {
            Mode::Auto => AUTO_MAX_BYTES,
            Mode::Mesh => MESH_MAX_BYTES,
        };
        let mesh = read_mesh(&file, max_bytes)?;
        control::progress(0.5, "Recognising features");
        let result = import_mesh(mesh, &file.name, &options);
        control::progress(0.6, "Writing the design");
        let mut out = HostWriter { buffer: Vec::with_capacity(CHUNK), error: None };
        let written = result.design.write_json(&mut out).and_then(|()| out.flush_buffer());
        if let Some(error) = out.error {
            return Err(error);
        }
        written.map_err(|e| PluginError::Trap(format!("writing the design: {e}")))?;
        control::progress(1.0, "Done");
        Ok(result.report.to_json())
    }
}

/// Reads and parses the file being imported (path `""`) chunk by chunk.
fn read_mesh(file: &source::FileInfo, max_bytes: u64) -> Result<Mesh, PluginError> {
    let mut offset = 0u64;
    let mut reader: Option<StlReader> = None;
    while offset < file.size {
        if control::cancelled() {
            return Err(PluginError::Cancelled);
        }
        let chunk = source::read("", offset, CHUNK as u32)?;
        if chunk.is_empty() {
            break;
        }
        let stl = match &mut reader {
            Some(stl) => stl,
            // The first chunk holds the header (1 MiB is far more than `StlReader::HEAD`).
            None => reader.insert(StlReader::new_with_size_limit(&chunk, file.size, max_bytes).map_err(|e| invalid("file", e))?),
        };
        stl.push(&chunk).map_err(|e| invalid("file", e))?;
        offset += chunk.len() as u64;
        control::progress(0.4 * offset as f32 / file.size.max(1) as f32, "Reading STL");
    }
    let reader = match reader {
        Some(reader) => reader,
        None => StlReader::new(&[], 0).map_err(|e| invalid("file", e))?,
    };
    reader.finish().map_err(|e| invalid("file", e))
}

/// Buffers the serialised design and hands it to `design::write` 1 MiB at a time. A host error
/// is kept in `error`, as `std::io::Error` cannot carry it.
struct HostWriter {
    buffer: Vec<u8>,
    error: Option<PluginError>,
}

impl HostWriter {
    fn flush_buffer(&mut self) -> std::io::Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }
        match design::write(&self.buffer) {
            Ok(()) => {
                self.buffer.clear();
                Ok(())
            }
            Err(error) => {
                self.error = Some(error);
                Err(std::io::Error::other("SolidCraft refused the design"))
            }
        }
    }
}

impl std::io::Write for HostWriter {
    fn write(&mut self, mut bytes: &[u8]) -> std::io::Result<usize> {
        let written = bytes.len();
        while !bytes.is_empty() {
            let room = CHUNK - self.buffer.len();
            let (now, later) = bytes.split_at(room.min(bytes.len()));
            self.buffer.extend_from_slice(now);
            bytes = later;
            if self.buffer.len() == CHUNK {
                self.flush_buffer()?;
            }
        }
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.flush_buffer()
    }
}

fn invalid(field: &str, reason: impl ToString) -> PluginError {
    PluginError::InvalidInput(FieldFailure { field: field.into(), reason: reason.to_string() })
}

export!(Importer);
