//! # meshwright-plugin
//!
//! The SolidCraft side of meshwright: a WebAssembly component in the `importer-plugin` world of
//! `craft:plugin@0.2.0` (vendored in `wit/plugin.wit`, from SolidCraft 0.45.0). It reads the file
//! SolidCraft hands it, lets [`meshwright_core`] do the work, and writes the design back.
//!
//! Build it for `wasm32-unknown-unknown` (SolidCraft links no WASI) and wrap the module as a
//! component; `cargo xtask dist` does both and packages the plugin.
//!
//! On any other target the crate is empty: the bindings export symbols a native linker refuses,
//! and everything worth testing natively lives in `meshwright-core`. Lint it with
//! `cargo clippy -p meshwright-plugin --target wasm32-unknown-unknown`.

#![cfg(target_arch = "wasm32")]
#![allow(unsafe_code, missing_docs, unreachable_pub, clippy::pedantic)] // generated bindings

wit_bindgen::generate!({ path: "wit", world: "importer-plugin" });

use craft::plugin::types::{FieldFailure, PluginError};
use craft::plugin::{control, design, source};
use exports::craft::plugin::importer::Guest;
use meshwright_core::{ImportOptions, import};

/// Largest read or write the host accepts in one call.
const CHUNK: usize = 1 << 20;

struct Importer;

impl Guest for Importer {
    fn import_file(_entry: String, file: source::FileInfo, options: String) -> Result<String, PluginError> {
        let options = ImportOptions::from_json(&options).map_err(|e| invalid("options", e))?;
        let bytes = read_source(&file)?;
        control::progress(0.5, "Recognising features");
        let result = import(&bytes, &file.name, &options).map_err(|e| invalid("file", e))?;
        for chunk in result.design.to_json().as_bytes().chunks(CHUNK) {
            design::write(chunk)?;
        }
        control::progress(1.0, "Done");
        Ok(result.report.to_json())
    }
}

/// The whole file being imported (path `""`), read in chunks.
fn read_source(file: &source::FileInfo) -> Result<Vec<u8>, PluginError> {
    let mut bytes = Vec::with_capacity(usize::try_from(file.size).unwrap_or(0));
    while (bytes.len() as u64) < file.size {
        if control::cancelled() {
            return Err(PluginError::Cancelled);
        }
        let chunk = source::read("", bytes.len() as u64, CHUNK as u32)?;
        if chunk.is_empty() {
            break;
        }
        bytes.extend_from_slice(&chunk);
        control::progress(0.4 * bytes.len() as f32 / file.size.max(1) as f32, "Reading STL");
    }
    Ok(bytes)
}

fn invalid(field: &str, reason: impl ToString) -> PluginError {
    PluginError::InvalidInput(FieldFailure { field: field.into(), reason: reason.to_string() })
}

export!(Importer);
