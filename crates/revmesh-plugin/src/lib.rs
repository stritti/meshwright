//! # revmesh-plugin
//!
//! The SolidCraft side of revmesh: a WebAssembly component in the `importer-plugin` world of
//! `craft:plugin@0.2.0` (vendored in `wit/plugin.wit`, from SolidCraft 0.49.1). It reads the file
//! SolidCraft hands it, lets [`revmesh_core`] do the work, and writes the design back.
//!
//! Memory is the tight budget (128 MiB by default), so nothing is held twice: the STL is parsed
//! chunk by chunk as it is read, the mesh moves into the design, and the design is serialised
//! straight into 1 MiB writes to the host.
//!
//! Build it for `wasm32-unknown-unknown` (SolidCraft links no WASI) and wrap the module as a
//! component; `cargo xtask dist` does both and packages the plugin.
//!
//! In auto mode the recognised design is also checked in the app's own sandbox before it is
//! written: the manifest requests the `evaluate@0.2` capability, the serialised design is staged
//! and `document.inspect` rebuilds it there, and a timeline feature that fails falls back to the
//! mesh body with the reason in the loss report (roadmap 7.1; the policy itself lives in
//! `revmesh_core::import::apply_self_check`).
//!
//! Every import also checks whether a newer release than this build is out and notes it in the
//! loss report. The sandbox has no network, so the newest release cannot be asked for at import
//! time: `cargo xtask dist` bakes it into the component as `REVMESH_LATEST_VERSION` (see
//! [`latest_release`]). A release published after this component was packaged is therefore not
//! visible to the plugin — the fresh check runs where the network exists, in `cargo xtask
//! plugin-test` and `cargo xtask check`; detecting it inside SolidCraft would need a host
//! interface in `craft:plugin` that does not exist in 0.2.0.
//!
//! On any other target the crate is empty: the bindings export symbols a native linker refuses,
//! and everything worth testing natively lives in `revmesh-core`. Lint it with
//! `cargo clippy -p revmesh-plugin --target wasm32-unknown-unknown`.

#![cfg(target_arch = "wasm32")]
#![allow(unsafe_code, missing_docs, unreachable_pub, clippy::pedantic)] // generated bindings

wit_bindgen::generate!({ path: "wit", world: "importer-plugin" });

use craft::plugin::types::{FieldFailure, PluginError};
use craft::plugin::{control, design, evaluate, log, source};
use exports::craft::plugin::importer::Guest;
use revmesh_core::import::{self, Mode, SelfCheck};
use revmesh_core::mesh::Mesh;
use revmesh_core::stl::StlReader;
use revmesh_core::{ImportOptions, Locale, import_mesh, version};

/// Largest read or write the host accepts in one call.
const CHUNK: usize = 1 << 20;

// Binary STL expands from 50 to 72 bytes per triangle. Mesh mode also needs the
// read/write buffers, while auto recognition builds face and wall indexes on top of the mesh
// (measured: about 250 bytes per triangle in all, so 16 MiB of STL peaks at roughly 90 MiB —
// inside the 128 MiB the manifest asks for; 32 MiB would not fit without index-based face
// storage). Rotated X/Y recognition attempts would hold a second mesh copy on top and can
// exceed the budget on densely banded meshes, so `recognize` skips them above
// `MAX_ROTATED_TRIANGLES` and only tries Z then. Keep a lower input ceiling for mesh mode.
const AUTO_MAX_BYTES: u64 = 16 << 20;
const MESH_MAX_BYTES: u64 = 32 << 20;

struct Importer;

impl Guest for Importer {
    fn import_file(_entry: String, file: source::FileInfo, options: String) -> Result<String, PluginError> {
        let locale = built_locale();
        let update = version::update_notice(env!("CARGO_PKG_VERSION"), latest_release(), locale);
        if let Some(notice) = &update {
            log::log(log::Level::Warn, &notice.message);
        }
        let mut options = ImportOptions::from_json(&options).map_err(|e| invalid("options", e))?;
        options.locale = locale;
        let max_bytes = match options.mode {
            Mode::Auto => AUTO_MAX_BYTES,
            Mode::Mesh => MESH_MAX_BYTES,
        };
        let mut mesh = read_mesh(&file, max_bytes, locale)?;
        // An inch file is scaled to millimetres in place, so the mesh is held only once.
        import::convert_units(&mut mesh, options.units).map_err(|e| invalid("file", e.message(locale)))?;
        control::progress(0.5, locale.text("progress.recognising"));
        // The recognised design is checked in the app's sandbox before it is written (roadmap
        // 7.1); when the app cannot rebuild it the import falls back to the mesh body, which
        // import_mesh rebuilds from this same borrow — no second mesh is held.
        let mut result = import_mesh(&mesh, &file.name, &options);
        if options.mode == Mode::Auto && !import::design_has_mesh_body(&result.design) {
            control::progress(0.55, locale.text("progress.checking"));
            let mut staged = Vec::new();
            let check = result.design.write_json(&mut staged).ok().map_or(SelfCheck::Unavailable, |()| self_check(&staged));
            if let SelfCheck::Failed(reason) = &check {
                log::log(log::Level::Warn, &format!("the recognised design failed the app's rebuild check: {reason}"));
            }
            result = import::apply_self_check(result, &mesh, &file.name, &options, update, &check);
        } else if let Some(notice) = update {
            result.report.push(notice);
        }
        control::progress(0.6, locale.text("progress.writing"));
        let mut out = HostWriter { buffer: Vec::with_capacity(CHUNK), error: None };
        let written = result.design.write_json(&mut out).and_then(|()| out.flush_buffer());
        if let Some(error) = out.error {
            return Err(error);
        }
        written.map_err(|e| PluginError::Trap(format!("writing the design: {e}")))?;
        control::progress(1.0, locale.text("progress.done"));
        Ok(result.report.to_json())
    }
}

/// Reads and parses the file being imported (path `""`) chunk by chunk.
fn read_mesh(file: &source::FileInfo, max_bytes: u64, locale: Locale) -> Result<Mesh, PluginError> {
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
            None => {
                reader.insert(StlReader::new_with_size_limit(&chunk, file.size, max_bytes).map_err(|e| invalid("file", e.message(locale)))?)
            }
        };
        stl.push(&chunk).map_err(|e| invalid("file", e.message(locale)))?;
        offset += chunk.len() as u64;
        control::progress(0.4 * offset as f32 / file.size.max(1) as f32, locale.text("progress.reading"));
    }
    let reader = match reader {
        Some(reader) => reader,
        None => StlReader::new(&[], 0).map_err(|e| invalid("file", e.message(locale)))?,
    };
    reader.finish().map_err(|e| invalid("file", e.message(locale)))
}

/// Stages the serialised design in the app's private sandbox (capability `evaluate@0.2`) in 1 MiB
/// pieces, runs `document.inspect` on it — which opens the design and evaluates its history — and
/// discards the session. What the reply says becomes the [`SelfCheck`]. A call error (a design the
/// app cannot open, a reply over 1 MiB) is an error for that call only, so the design is then
/// reported as unchecked rather than failed.
fn self_check(design_json: &[u8]) -> SelfCheck {
    let mut staged = true;
    for chunk in design_json.chunks(CHUNK) {
        if let Err(error) = evaluate::stage(chunk) {
            log::log(log::Level::Warn, &format!("the sandbox self-check is unavailable: {error:?}"));
            staged = false;
            break;
        }
    }
    if !staged {
        evaluate::reset();
        return SelfCheck::Unavailable;
    }
    let outcome = evaluate::run("document.inspect", "{}");
    evaluate::reset();
    match outcome {
        Ok(reply) => SelfCheck::from_inspect_reply(&reply),
        Err(error) => {
            log::log(log::Level::Warn, &format!("the sandbox self-check could not run: {error:?}"));
            SelfCheck::Unavailable
        }
    }
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

/// The language this component was built for: `REVMESH_LOCALE` at compile time (`cargo xtask
/// dist` sets it per package), English when unset.
fn built_locale() -> Locale {
    option_env!("REVMESH_LOCALE").and_then(Locale::from_code).unwrap_or_default()
}

/// The newest release, baked into the component at package time: `cargo xtask dist` builds with
/// `REVMESH_LATEST_VERSION` set to the latest GitHub release (the sandbox itself has no
/// network, so the import-time check in [`Importer::import_file`] compares against this). Built
/// without it, the plugin's own version stands in and the check finds nothing newer.
fn latest_release() -> &'static str {
    option_env!("REVMESH_LATEST_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))
}

fn invalid(field: &str, reason: impl ToString) -> PluginError {
    PluginError::InvalidInput(FieldFailure { field: field.into(), reason: reason.to_string() })
}

export!(Importer);
