//! Reading mesh files of the formats the plugin accepts: STL, OBJ and PLY.
//!
//! [`Format`] tells them apart by the file's name, or by sniffing the first bytes.
//! [`MeshReader`] reads any of them in chunks, as the plugin receives the file, so the file
//! never has to be held in memory next to the mesh made from it; [`parse`] reads a file that is
//! already in memory. Errors are [`MeshError`], localised like [`crate::stl::StlError`]'s.

use std::fmt;

use crate::i18n::Locale;
use crate::mesh::{Mesh, Triangle, Vec3};
use crate::obj::{ObjReader, ObjReason};
use crate::ply::{PlyReader, PlyReason};
use crate::stl::{StlError, StlReader};

/// A mesh file format the plugin accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// STL, binary or ASCII.
    Stl,
    /// Wavefront OBJ.
    Obj,
    /// PLY, ASCII or binary little-endian.
    Ply,
}

impl Format {
    /// The format of a file named `name`, by its extension.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name.rsplit('.').next().map(str::to_ascii_lowercase).as_deref() {
            Some("stl") => Some(Self::Stl),
            Some("obj") => Some(Self::Obj),
            Some("ply") => Some(Self::Ply),
            _ => None,
        }
    }

    /// The format of a file whose first bytes are `head`, when the name tells nothing.
    #[must_use]
    pub fn sniff(head: &[u8]) -> Option<Self> {
        if head.starts_with(b"ply\n") || head.starts_with(b"ply\r\n") {
            return Some(Self::Ply);
        }
        if crate::stl::looks_ascii_pub(head) {
            return Some(Self::Stl);
        }
        // OBJ is text whose lines start with a keyword; a binary STL header is mostly zeros.
        let text = head.iter().take_while(|b| b.is_ascii()).count();
        if text > 0 && head[..text].iter().all(|b| b.is_ascii_graphic() || b" \t\r\n".contains(b)) {
            let first = String::from_utf8_lossy(&head[..text]);
            let keyword = first.split_whitespace().next().unwrap_or_default();
            if matches!(keyword, "v" | "f" | "#" | "o" | "g" | "s" | "vt" | "vn") {
                return Some(Self::Obj);
            }
        }
        None
    }
}

/// Why a mesh file could not be read, in any accepted format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeshError {
    /// Any format: the file exceeds the caller's input budget.
    SizeLimit {
        /// Size of the file in bytes.
        size: u64,
        /// Maximum supported size in bytes.
        max_bytes: u64,
    },
    /// Any format: a corner coordinate is not a finite number (`nan` or `inf`).
    NonFinite {
        /// 1-based number of the triangle.
        triangle: usize,
    },
    /// Any format: a corner coordinate is finite but too large for a single-precision float.
    OutOfRange {
        /// 1-based number of the triangle.
        triangle: usize,
    },
    /// Any format: the triangle buffer could not be allocated.
    Allocation,
    /// Any format: the file holds no triangles.
    Empty,
    /// An STL file; see [`StlError`].
    Stl(StlError),
    /// An OBJ line that broke the grammar.
    Obj {
        /// 1-based line number.
        line: usize,
        /// What was wrong.
        reason: ObjReason,
    },
    /// A PLY header line that broke the grammar.
    PlyHeader {
        /// 1-based line number.
        line: usize,
        /// What was wrong.
        reason: PlyReason,
    },
    /// A PLY body record that broke the grammar.
    PlyBody {
        /// 1-based record number within the body.
        record: usize,
        /// What was wrong.
        reason: PlyReason,
    },
}

impl MeshError {
    /// The error in `locale`'s words.
    #[must_use]
    pub fn message(&self, locale: Locale) -> String {
        match self {
            Self::SizeLimit { size, max_bytes } => locale.format("mesh.size_limit", &[("size", size), ("max_bytes", max_bytes)]),
            Self::NonFinite { triangle } => locale.format("mesh.non_finite", &[("triangle", triangle)]),
            Self::OutOfRange { triangle } => locale.format("mesh.out_of_range", &[("triangle", triangle)]),
            Self::Allocation => locale.text("mesh.allocation").to_owned(),
            Self::Empty => locale.text("mesh.empty").to_owned(),
            Self::Stl(e) => e.message(locale),
            Self::Obj { line, reason } => locale.format("obj.syntax", &[("line", line), ("reason", &reason.message(locale))]),
            Self::PlyHeader { line, reason } => locale.format("ply.header", &[("line", line), ("reason", &reason.message(locale))]),
            Self::PlyBody { record, reason } => locale.format("ply.body", &[("record", record), ("reason", &reason.message(locale))]),
        }
    }
}

impl fmt::Display for MeshError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message(Locale::En))
    }
}

impl std::error::Error for MeshError {}

impl From<StlError> for MeshError {
    fn from(e: StlError) -> Self {
        Self::Stl(e)
    }
}

/// Pushes onto a vector whose growth may be driven by the file, reserving fallibly first: in the
/// plugin's sandbox a failed allocation aborts the component, so growth goes through
/// `try_reserve` and surfaces as [`MeshError::Allocation`].
pub(crate) fn push_reserved<T>(vec: &mut Vec<T>, value: T) -> Result<(), MeshError> {
    if vec.len() == vec.capacity() {
        vec.try_reserve(1).map_err(|_| MeshError::Allocation)?;
    }
    vec.push(value);
    Ok(())
}

/// Extends a byte buffer fallibly, for the same reason as [`push_reserved`].
pub(crate) fn extend_reserved(vec: &mut Vec<u8>, bytes: &[u8]) -> Result<(), MeshError> {
    if vec.len() + bytes.len() > vec.capacity() {
        vec.try_reserve(bytes.len()).map_err(|_| MeshError::Allocation)?;
    }
    vec.extend_from_slice(bytes);
    Ok(())
}

/// Pushes a triangle, checking its corners like [`crate::stl`] does: finite, and within the
/// single-precision range meshes are written in.
pub(crate) fn push_checked(triangles: &mut Vec<Triangle>, triangle: Triangle) -> Result<(), MeshError> {
    if triangle.iter().flatten().any(|v| !v.is_finite()) {
        return Err(MeshError::NonFinite { triangle: triangles.len() + 1 });
    }
    if triangle.iter().flatten().any(|v| v.abs() > f64::from(f32::MAX)) {
        return Err(MeshError::OutOfRange { triangle: triangles.len() + 1 });
    }
    push_reserved(triangles, triangle)
}

/// A coordinate as text: shortest round-trip form, shared by the OBJ and PLY writers.
pub(crate) fn fmt_num(v: f64) -> String {
    let rounded = (v * 1e9).round() / 1e9;
    if f64::to_bits(rounded) == f64::to_bits(v) { format!("{rounded}") } else { format!("{v}") }
}

/// The mesh's corners as shared vertices plus per-triangle indices into them, the way OBJ and
/// PLY store a mesh (STL repeats the corners in every triangle).
pub(crate) fn welded_vertices(mesh: &Mesh) -> (Vec<Vec3>, Vec<[usize; 3]>) {
    let mut index: std::collections::HashMap<[u64; 3], usize> = std::collections::HashMap::new();
    let mut vertices = Vec::new();
    let mut faces = Vec::with_capacity(mesh.triangles.len());
    for triangle in &mesh.triangles {
        let mut face = [0usize; 3];
        for (k, point) in triangle.iter().enumerate() {
            let key = point.map(f64::to_bits);
            let i = *index.entry(key).or_insert_with(|| {
                vertices.push(*point);
                vertices.len() - 1
            });
            face[k] = i;
        }
        faces.push(face);
    }
    (vertices, faces)
}

/// Reads a mesh file of any accepted format in chunks, as the plugin receives it.
///
/// Create it with the file's first bytes (at least [`MeshReader::HEAD`] of them, or the whole file
/// if it is shorter), its size and its name, then [`push`](Self::push) every byte of the file in
/// order, from the start, and [`finish`](Self::finish).
#[derive(Debug)]
pub enum MeshReader {
    /// An STL file.
    Stl(StlReader),
    /// An OBJ file.
    Obj(ObjReader),
    /// A PLY file.
    Ply(PlyReader),
}

impl MeshReader {
    /// How many bytes from the start of the file [`MeshReader::new`] needs: enough for every
    /// format's magic (STL's is the longest).
    pub const HEAD: usize = StlReader::HEAD;

    /// A reader with an input size budget checked before anything is allocated.
    pub fn new_with_size_limit(head: &[u8], size: u64, max_bytes: u64, name: &str) -> Result<Self, MeshError> {
        if size > max_bytes {
            return Err(MeshError::SizeLimit { size, max_bytes });
        }
        Self::new(head, size, name)
    }

    /// A reader for a file of `size` bytes that starts with `head` and is named `name`.
    pub fn new(head: &[u8], size: u64, name: &str) -> Result<Self, MeshError> {
        let format = Format::from_name(name).or_else(|| Format::sniff(head)).unwrap_or(Format::Stl);
        Ok(match format {
            Format::Stl => Self::Stl(StlReader::new(head, size)?),
            Format::Obj => Self::Obj(ObjReader::new(head, size)),
            Format::Ply => Self::Ply(PlyReader::new(head, size)?),
        })
    }

    /// Reads the next bytes of the file.
    pub fn push(&mut self, chunk: &[u8]) -> Result<(), MeshError> {
        match self {
            Self::Stl(r) => r.push(chunk).map_err(MeshError::Stl),
            Self::Obj(r) => r.push(chunk),
            Self::Ply(r) => r.push(chunk),
        }
    }

    /// The mesh, once every byte has been pushed.
    pub fn finish(self) -> Result<Mesh, MeshError> {
        match self {
            Self::Stl(r) => r.finish().map_err(MeshError::Stl),
            Self::Obj(r) => r.finish(),
            Self::Ply(r) => r.finish(),
        }
    }
}

/// Reads a mesh file that is in memory, of the format its `name` tells (or its first bytes).
pub fn parse(bytes: &[u8], name: &str) -> Result<Mesh, MeshError> {
    let mut reader = MeshReader::new(bytes, bytes.len() as u64, name)?;
    reader.push(bytes)?;
    reader.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_is_told_by_name_or_sniffed() {
        assert_eq!(Format::from_name("part.STL"), Some(Format::Stl));
        assert_eq!(Format::from_name("part.obj"), Some(Format::Obj));
        assert_eq!(Format::from_name("part.PLY"), Some(Format::Ply));
        assert_eq!(Format::from_name("part.txt"), None);
        assert_eq!(Format::sniff(b"ply\nformat ascii 1.0\n"), Some(Format::Ply));
        assert_eq!(Format::sniff(b"solid x\nendsolid x\n"), Some(Format::Stl));
        assert_eq!(Format::sniff(b"v 0 0 0\nf 1 2 3\n"), Some(Format::Obj));
        assert_eq!(Format::sniff(b"# comment\nv 0 0 0\n"), Some(Format::Obj));
    }

    #[test]
    fn size_limit_is_checked_before_the_format() {
        let err = MeshReader::new_with_size_limit(b"ply\n", 100, 8, "x.ply").expect_err("over the limit");
        assert_eq!(err, MeshError::SizeLimit { size: 100, max_bytes: 8 });
        assert_eq!(err.message(Locale::De), "die Datei ist 100 Bytes groß; dieser Import unterstützt höchstens 8 Bytes");
    }

    #[test]
    fn a_file_at_or_under_the_size_limit_is_read() {
        let text = b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";
        let size = text.len() as u64;
        for max_bytes in [size, size + 1] {
            let mut reader = MeshReader::new_with_size_limit(text, size, max_bytes, "x.obj").expect("within the limit");
            reader.push(text).expect("push");
            assert_eq!(reader.finish().expect("finish").triangles.len(), 1);
        }
        assert!(matches!(MeshReader::new_with_size_limit(text, size, size - 1, "x.obj"), Err(MeshError::SizeLimit { .. })));
    }

    #[test]
    fn errors_are_localised() {
        let err = MeshError::Empty;
        assert_eq!(err.to_string(), "the file holds no triangles");
        let german = err.message(Locale::De);
        assert_ne!(german, err.to_string());
        assert!(!german.contains('{'));
        let err = MeshError::NonFinite { triangle: 2 };
        assert!(err.message(Locale::De).contains("Dreieck 2"));
    }
}
