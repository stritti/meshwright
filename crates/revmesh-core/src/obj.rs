//! Reading OBJ files (Wavefront OBJ, the text format every CAD tool exports).
//!
//! Only what a mesh needs: `v x y z` vertex lines (extra values after the coordinates, e.g.
//! colours, are ignored) and `f` face lines. Face indices may carry
//! texture/normal parts (`f 1/1/1 2/2/2 3/3/3`), may be negative (counting back from the last
//! vertex), and faces with more than three corners are triangulated as a fan. Every other line
//! (`#`, `o`, `g`, `s`, `vt`, `vn`, `usemtl`, `mtllib`, …) is skipped.
//!
//! [`ObjReader`] reads the file in chunks, as the plugin receives it, so the file never has to be
//! held in memory next to the mesh made from it. [`write()`] writes a mesh as OBJ (used by the
//! tests and the fixture generator).

use std::fmt;

use crate::format::{MeshError, extend_reserved, fmt_num, push_checked, push_reserved, welded_vertices};
use crate::i18n::Locale;
use crate::mesh::{Mesh, Triangle, Vec3};

/// A line longer than this is refused: a face with millions of corners is not a part.
const LINE_MAX: usize = 4 << 20;

/// Why an OBJ line broke the grammar. Localised through the `obj.syntax.reason.*` catalog keys;
/// [`fmt::Display`] gives the English words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjReason {
    /// A `v` line without at least three coordinates.
    VertexNotThree,
    /// A coordinate that is not a finite number.
    BadCoordinate,
    /// A `f` entry that is not a vertex index.
    BadFace,
    /// A vertex index outside the vertices read so far.
    IndexOutOfRange(usize),
    /// A face with fewer than three corners.
    FaceTooSmall(usize),
}

impl ObjReason {
    /// The reason in `locale`'s words.
    #[must_use]
    pub fn message(&self, locale: Locale) -> String {
        match self {
            Self::VertexNotThree => locale.text("obj.syntax.reason.vertex_not_three").to_owned(),
            Self::BadCoordinate => locale.text("obj.syntax.reason.bad_coordinate").to_owned(),
            Self::BadFace => locale.text("obj.syntax.reason.bad_face").to_owned(),
            Self::IndexOutOfRange(n) => locale.format("obj.syntax.reason.index_out_of_range", &[("n", n)]),
            Self::FaceTooSmall(n) => locale.format("obj.syntax.reason.face_too_small", &[("n", n)]),
        }
    }
}

impl fmt::Display for ObjReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VertexNotThree => f.write_str("a vertex line has three coordinates"),
            Self::BadCoordinate => f.write_str("bad coordinate"),
            Self::BadFace => f.write_str("a face lists vertex indices"),
            Self::IndexOutOfRange(n) => write!(f, "vertex index {n} is out of range"),
            Self::FaceTooSmall(n) => write!(f, "a face has fewer than three corners, this one has {n}"),
        }
    }
}

/// Reads an OBJ file in chunks.
///
/// Create it with the file's first bytes and its size, then [`push`](Self::push) every byte of the
/// file in order, from the start, and [`finish`](Self::finish).
#[derive(Debug)]
pub struct ObjReader {
    /// The unfinished line from the last push.
    pending: Vec<u8>,
    line: usize,
    vertices: Vec<Vec3>,
    triangles: Vec<Triangle>,
}

impl ObjReader {
    /// A reader for a file of `size` bytes that starts with `head`. OBJ has no magic, so the
    /// head only bounds the first push.
    #[must_use]
    pub fn new(head: &[u8], size: u64) -> Self {
        let _ = (head, size);
        Self { pending: Vec::new(), line: 0, vertices: Vec::new(), triangles: Vec::new() }
    }

    /// Reads the next bytes of the file.
    pub fn push(&mut self, chunk: &[u8]) -> Result<(), MeshError> {
        let Some(last_newline) = chunk.iter().rposition(|b| *b == b'\n') else {
            extend_reserved(&mut self.pending, chunk)?;
            return self.check_pending();
        };
        let (complete, rest) = chunk.split_at(last_newline);
        let mut text = std::mem::take(&mut self.pending);
        extend_reserved(&mut text, complete)?;
        for l in text.split(|b| *b == b'\n') {
            self.line += 1;
            obj_line(&String::from_utf8_lossy(l), self.line, &mut self.vertices, &mut self.triangles)?;
        }
        self.pending.clear();
        extend_reserved(&mut self.pending, &rest[1..])?;
        self.check_pending()
    }

    fn check_pending(&self) -> Result<(), MeshError> {
        if self.pending.len() > LINE_MAX {
            return Err(MeshError::Obj { line: self.line + 1, reason: ObjReason::BadFace });
        }
        Ok(())
    }

    /// The mesh, once every byte has been pushed.
    pub fn finish(mut self) -> Result<Mesh, MeshError> {
        if !self.pending.is_empty() {
            self.line += 1;
            let text = String::from_utf8_lossy(&self.pending);
            obj_line(&text, self.line, &mut self.vertices, &mut self.triangles)?;
        }
        if self.triangles.is_empty() {
            return Err(MeshError::Empty);
        }
        Ok(Mesh::new(self.triangles))
    }
}

/// One OBJ line.
fn obj_line(text: &str, line: usize, vertices: &mut Vec<Vec3>, triangles: &mut Vec<Triangle>) -> Result<(), MeshError> {
    let mut words = text.split_whitespace();
    let Some(keyword) = words.next() else { return Ok(()) };
    match keyword {
        "v" => {
            let coords: Vec<f64> =
                words.map(str::parse).collect::<Result<_, _>>().map_err(|_| MeshError::Obj { line, reason: ObjReason::BadCoordinate })?;
            if coords.len() < 3 || coords[..3].iter().any(|v| !v.is_finite()) {
                return Err(MeshError::Obj { line, reason: ObjReason::VertexNotThree });
            }
            push_reserved(vertices, [coords[0], coords[1], coords[2]])?;
            Ok(())
        }
        "f" => {
            let mut indices: Vec<usize> = Vec::new();
            for word in words {
                let index = word.split('/').next().unwrap_or_default();
                let n: i64 = index.parse().map_err(|_| MeshError::Obj { line, reason: ObjReason::BadFace })?;
                let resolved = if n > 0 { n - 1 } else { i64::try_from(vertices.len()).unwrap_or(i64::MAX).saturating_add(n) };
                let resolved = usize::try_from(resolved).map_err(|_| MeshError::Obj {
                    line,
                    reason: ObjReason::IndexOutOfRange(usize::try_from(n.unsigned_abs()).unwrap_or(0)),
                })?;
                if resolved >= vertices.len() {
                    return Err(MeshError::Obj { line, reason: ObjReason::IndexOutOfRange(resolved + 1) });
                }
                push_reserved(&mut indices, resolved)?;
            }
            if indices.len() < 3 {
                return Err(MeshError::Obj { line, reason: ObjReason::FaceTooSmall(indices.len()) });
            }
            for k in 1..indices.len() - 1 {
                let triangle = [vertices[indices[0]], vertices[indices[k]], vertices[indices[k + 1]]];
                push_checked(triangles, triangle)?;
            }
            Ok(())
        }
        // Every other keyword (comments, groups, materials, texture and normal coords, …).
        _ => Ok(()),
    }
}

/// Writes a mesh as OBJ (1-based `f` lines, no texture or normal parts, shared vertices once).
#[must_use]
pub fn write(mesh: &Mesh) -> Vec<u8> {
    let (vertices, faces) = welded_vertices(mesh);
    let mut out = String::from("# revmesh test fixture\n");
    for v in &vertices {
        let line = format!("v {} {} {}\n", fmt_num(v[0]), fmt_num(v[1]), fmt_num(v[2]));
        out.push_str(&line);
    }
    for f in &faces {
        let line = format!("f {} {} {}\n", f[0] + 1, f[1] + 1, f[2] + 1);
        out.push_str(&line);
    }
    out.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOX: &str = "v 0 0 0\nv 10 0 0\nv 10 10 0\nv 0 10 0\nv 0 0 5\nv 10 0 5\nv 10 10 5\nv 0 10 5\nf 1 2 3\nf 1 3 4\nf 5 6 7\nf 5 7 8\nf 1 2 6\nf 1 6 5\nf 2 3 7\nf 2 7 6\nf 3 4 8\nf 3 8 7\nf 4 1 5\nf 4 5 8\n";

    fn box_mesh() -> Mesh {
        let mut r = ObjReader::new(BOX.as_bytes(), BOX.len() as u64);
        r.push(BOX.as_bytes()).expect("push");
        r.finish().expect("finish")
    }

    #[test]
    fn reads_a_box() {
        let mesh = box_mesh();
        assert_eq!(mesh.triangles.len(), 12);
        assert!((mesh.volume() - 500.0).abs() < 1e-9, "{}", mesh.volume());
    }

    #[test]
    fn reads_in_odd_sized_chunks() {
        let mut r = ObjReader::new(BOX.as_bytes(), BOX.len() as u64);
        let mut off = 0;
        while off < BOX.len() {
            let end = (off + 7).min(BOX.len());
            r.push(&BOX.as_bytes()[off..end]).expect("push");
            off = end;
        }
        assert_eq!(r.finish().expect("finish").triangles.len(), 12);
    }

    #[test]
    fn round_trips_through_write() {
        let mesh = box_mesh();
        let bytes = write(&mesh);
        let mut r = ObjReader::new(&bytes, bytes.len() as u64);
        r.push(&bytes).expect("push");
        assert_eq!(r.finish().expect("finish").triangles, mesh.triangles);
    }

    #[test]
    fn faces_may_carry_slashes_negative_indices_and_fans() {
        let text = "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1/1/1 2//2 3/3 4/4/4\nf -4 -3 -2\n";
        let mut r = ObjReader::new(text.as_bytes(), text.len() as u64);
        r.push(text.as_bytes()).expect("push");
        // The quad fans into two triangles, plus the negative-index triangle.
        assert_eq!(r.finish().expect("finish").triangles.len(), 3);
    }

    #[test]
    fn bad_lines_are_refused_with_their_line_number() {
        let text = "v 0 0 0\nv 1 0 0\nv 1 1 0\nf 1 2 9\n";
        let mut r = ObjReader::new(text.as_bytes(), text.len() as u64);
        let err = r.push(text.as_bytes()).expect_err("an error");
        assert_eq!(err, MeshError::Obj { line: 4, reason: ObjReason::IndexOutOfRange(9) });
        assert_eq!(err.to_string(), "OBJ line 4: vertex index 9 is out of range");
        assert!(err.message(Locale::De).contains("OBJ-Zeile 4"), "{}", err.message(Locale::De));
        let text = "v 0 0\nf 1 2 3\n";
        let mut r = ObjReader::new(text.as_bytes(), text.len() as u64);
        let err = r.push(text.as_bytes()).expect_err("an error");
        assert_eq!(err, MeshError::Obj { line: 1, reason: ObjReason::VertexNotThree });
    }

    #[test]
    fn vertex_lines_may_carry_colours() {
        // MeshLab, scanners and slicers write `v x y z r g b`; the first three numbers are the
        // corner, the rest is ignored.
        let text = "v 0 0 0 1 0 0\nv 10 0 0 0 1 0\nv 10 10 0 0 0 1\nf 1 2 3\n";
        let mut r = ObjReader::new(text.as_bytes(), text.len() as u64);
        r.push(text.as_bytes()).expect("push");
        let mesh = r.finish().expect("finish");
        assert_eq!(mesh.triangles, vec![[[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [10.0, 10.0, 0.0]]]);
    }

    #[test]
    fn a_file_without_faces_is_empty() {
        let text = "v 0 0 0\nv 1 0 0\n";
        let mut r = ObjReader::new(text.as_bytes(), text.len() as u64);
        r.push(text.as_bytes()).expect("push");
        assert_eq!(r.finish().expect_err("an error"), MeshError::Empty);
    }

    #[test]
    fn the_last_line_needs_no_newline() {
        let text = "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3";
        let mut r = ObjReader::new(text.as_bytes(), text.len() as u64);
        r.push(text.as_bytes()).expect("push");
        assert_eq!(r.finish().expect("finish").triangles.len(), 1);
    }

    #[test]
    fn a_line_longer_than_the_limit_is_refused() {
        // A line without a newline is held until it ends; past `LINE_MAX` bytes it is refused, so
        // a crafted file cannot grow the pending buffer without bound.
        let mut r = ObjReader::new(b"v", (LINE_MAX + 2) as u64);
        r.push(b"v 0 0 0\n").expect("push");
        let err = r.push(&vec![b'#'; LINE_MAX + 1]).expect_err("too long");
        assert!(matches!(err, MeshError::Obj { line: 2, .. }), "{err:?}");
    }
}
