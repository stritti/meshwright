//! Reading STL files, binary and ASCII.
//!
//! Binary STL is an 80-byte header, a little-endian `u32` triangle count and 50 bytes per
//! triangle (normal, three corners, attribute word). ASCII STL starts with `solid`; some binary
//! files start with `solid` too, so the decision is made on whether the size matches the count.
//!
//! [`StlReader`] reads a file in chunks, as the plugin receives it, so the file never has to be
//! held in memory next to the mesh made from it. [`parse`] reads a file that is in memory.

use std::fmt;

use crate::mesh::{Mesh, Triangle, Vec3, cross, length, sub};

/// Why an STL file could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StlError {
    /// The file is shorter than a binary header and not ASCII STL either.
    TooShort,
    /// A binary file whose size does not match its triangle count.
    Truncated {
        /// Triangles the header announces.
        expected: u64,
        /// Bytes the file actually has.
        actual: u64,
    },
    /// An ASCII file that does not follow the grammar, with the line it broke on.
    Syntax {
        /// 1-based line number.
        line: usize,
        /// What was wrong.
        reason: String,
    },
    /// A corner coordinate is not a finite number (`nan` or `inf`).
    NonFinite {
        /// 1-based number of the triangle.
        triangle: usize,
    },
    /// A corner coordinate is finite but too large for a single-precision float, the precision
    /// meshes are written in (an ASCII file can hold `1e39`; binary files cannot).
    OutOfRange {
        /// 1-based number of the triangle.
        triangle: usize,
    },
    /// The STL exceeds the caller's input budget.
    SizeLimit {
        /// Size of the STL in bytes.
        size: u64,
        /// Maximum supported size in bytes.
        max_bytes: u64,
    },
    /// The STL triangle buffer could not be allocated.
    Allocation,
    /// The file holds no triangles.
    Empty,
}

impl fmt::Display for StlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort => write!(f, "the file is too short to be an STL file"),
            Self::Truncated { expected, actual } => {
                write!(f, "the binary STL header announces {expected} triangles, but the file has only {actual} bytes")
            }
            Self::Syntax { line, reason } => write!(f, "ASCII STL, line {line}: {reason}"),
            Self::NonFinite { triangle } => write!(f, "triangle {triangle} has a corner that is not a finite number (nan or inf)"),
            Self::OutOfRange { triangle } => {
                write!(f, "triangle {triangle} has a coordinate too large for an STL mesh (beyond about 3.4e38)")
            }
            Self::SizeLimit { size, max_bytes } => {
                write!(f, "the STL is {size} bytes; this import mode supports at most {max_bytes} bytes")
            }
            Self::Allocation => write!(f, "there is not enough memory to hold the STL triangles"),
            Self::Empty => write!(f, "the file holds no triangles"),
        }
    }
}

impl std::error::Error for StlError {}

const HEADER: usize = 80;
/// The header and the triangle count.
const PREAMBLE: usize = HEADER + 4;
const RECORD: usize = 50;

/// Reads an STL file that is in memory, binary or ASCII.
pub fn parse(bytes: &[u8]) -> Result<Mesh, StlError> {
    let mut reader = StlReader::new(bytes, bytes.len() as u64)?;
    reader.push(bytes)?;
    reader.finish()
}

/// Reads an STL file in chunks.
///
/// Create it with the file's first bytes (at least [`StlReader::HEAD`] of them, or the whole file
/// if it is shorter) and its size, then [`push`](Self::push) every byte of the file in order,
/// from the start, and [`finish`](Self::finish).
#[derive(Debug)]
pub struct StlReader {
    format: Format,
    /// Bytes of the file pushed so far.
    offset: u64,
    /// The unfinished binary record or ASCII line from the last push.
    pending: Vec<u8>,
    triangles: Vec<Triangle>,
}

#[derive(Debug)]
enum Format {
    /// Binary, with the triangle count its header announces.
    Binary {
        triangles: u64,
    },
    Ascii {
        line: usize,
        corners: Vec<Vec3>,
        block: AsciiBlock,
    },
}

#[derive(Debug, PartialEq, Eq)]
enum AsciiBlock {
    Outside,
    Solid,
    Facet,
    Loop,
    EndFacet,
}

impl StlReader {
    /// How many bytes from the start of the file [`StlReader::new`] needs to tell the format.
    pub const HEAD: usize = PREAMBLE;

    /// A reader with an input size budget checked before allocating the triangle buffer.
    pub fn new_with_size_limit(head: &[u8], size: u64, max_bytes: u64) -> Result<Self, StlError> {
        if size > max_bytes {
            return Err(StlError::SizeLimit { size, max_bytes });
        }
        Self::new(head, size)
    }

    /// A reader for a file of `size` bytes that starts with `head`.
    pub fn new(head: &[u8], size: u64) -> Result<Self, StlError> {
        if let Some(count) = binary_count(head) {
            let expected_len = PREAMBLE as u64 + count * RECORD as u64;
            if expected_len == size {
                // The size check bounds the count by the file size, so this allocation is not
                // driven by the header alone.
                let mut triangles = Vec::new();
                let count_usize = usize::try_from(count).map_err(|_| StlError::Allocation)?;
                triangles.try_reserve_exact(count_usize).map_err(|_| StlError::Allocation)?;
                return Ok(Self { format: Format::Binary { triangles: count }, offset: 0, pending: Vec::with_capacity(RECORD), triangles });
            }
            if !looks_ascii(head) {
                return Err(StlError::Truncated { expected: count, actual: size });
            }
        }
        if looks_ascii(head) {
            return Ok(Self {
                format: Format::Ascii { line: 0, corners: Vec::with_capacity(3), block: AsciiBlock::Outside },
                offset: 0,
                pending: Vec::new(),
                triangles: Vec::new(),
            });
        }
        Err(StlError::TooShort)
    }

    /// Reads the next bytes of the file.
    pub fn push(&mut self, mut chunk: &[u8]) -> Result<(), StlError> {
        let start = self.offset;
        self.offset += chunk.len() as u64;
        match &mut self.format {
            Format::Binary { .. } => {
                if start < PREAMBLE as u64 {
                    let skip = (PREAMBLE as u64 - start).min(chunk.len() as u64) as usize;
                    chunk = &chunk[skip..];
                }
                if !self.pending.is_empty() {
                    let missing = (RECORD - self.pending.len()).min(chunk.len());
                    self.pending.extend_from_slice(&chunk[..missing]);
                    chunk = &chunk[missing..];
                    if self.pending.len() == RECORD {
                        let triangle = read_record(&self.pending);
                        push_checked(&mut self.triangles, triangle)?;
                        self.pending.clear();
                    }
                }
                let (records, rest) = chunk.as_chunks::<RECORD>();
                for record in records {
                    push_checked(&mut self.triangles, read_record(record))?;
                }
                self.pending.extend_from_slice(rest);
            }
            Format::Ascii { line, corners, block } => {
                let Some(last_newline) = chunk.iter().rposition(|b| *b == b'\n') else {
                    self.pending.extend_from_slice(chunk);
                    return Ok(());
                };
                // Complete lines: what was pending, and this chunk up to its last newline.
                let (complete, rest) = chunk.split_at(last_newline);
                let mut text = std::mem::take(&mut self.pending);
                text.extend_from_slice(complete);
                for l in text.split(|b| *b == b'\n') {
                    *line += 1;
                    ascii_line(&String::from_utf8_lossy(l), *line, corners, block, &mut self.triangles)?;
                }
                self.pending = rest[1..].to_vec();
            }
        }
        Ok(())
    }

    /// The mesh, once every byte has been pushed.
    pub fn finish(mut self) -> Result<Mesh, StlError> {
        match &mut self.format {
            Format::Binary { triangles: expected } => {
                // Ending early at a record boundary leaves no partial record, so the count decides.
                if !self.pending.is_empty() || self.triangles.len() as u64 != *expected {
                    return Err(StlError::Truncated { expected: *expected, actual: self.offset });
                }
            }
            Format::Ascii { line, corners, block } => {
                if !self.pending.is_empty() {
                    *line += 1;
                    let text = String::from_utf8_lossy(&self.pending);
                    ascii_line(&text, *line, corners, block, &mut self.triangles)?;
                }
                if *block != AsciiBlock::Outside {
                    return Err(StlError::Syntax { line: *line, reason: "the file ends inside a solid or facet".into() });
                }
            }
        }
        if self.triangles.is_empty() {
            return Err(StlError::Empty);
        }
        Ok(Mesh::new(self.triangles))
    }
}

fn push_checked(triangles: &mut Vec<Triangle>, triangle: Triangle) -> Result<(), StlError> {
    if triangle.iter().flatten().any(|v| !v.is_finite()) {
        return Err(StlError::NonFinite { triangle: triangles.len() + 1 });
    }
    if triangle.iter().flatten().any(|v| v.abs() > f64::from(f32::MAX)) {
        return Err(StlError::OutOfRange { triangle: triangles.len() + 1 });
    }
    if triangles.len() == triangles.capacity() {
        triangles.try_reserve(1).map_err(|_| StlError::Allocation)?;
    }
    triangles.push(triangle);
    Ok(())
}

/// Writes `mesh` as binary STL, the facet normals computed from the corners.
#[must_use]
pub fn write_binary(mesh: &Mesh) -> Vec<u8> {
    write_binary_with_header(mesh, b"binary STL written by revmesh")
}

fn write_binary_with_header(mesh: &Mesh, header: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; HEADER];
    out[..header.len().min(HEADER)].copy_from_slice(&header[..header.len().min(HEADER)]);
    out.extend_from_slice(&(mesh.triangles.len() as u32).to_le_bytes());
    for [a, b, c] in &mesh.triangles {
        let normal = cross(sub(*b, *a), sub(*c, *a));
        let len = length(normal);
        let normal = if len > 0.0 { normal.map(|v| v / len) } else { [0.0; 3] };
        for value in normal.iter().chain(a).chain(b).chain(c) {
            out.extend_from_slice(&(*value as f32).to_le_bytes());
        }
        out.extend_from_slice(&[0u8; 2]);
    }
    out
}

fn binary_count(bytes: &[u8]) -> Option<u64> {
    let raw: [u8; 4] = bytes.get(HEADER..PREAMBLE)?.try_into().ok()?;
    Some(u64::from(u32::from_le_bytes(raw)))
}

fn looks_ascii(bytes: &[u8]) -> bool {
    let start = bytes.iter().position(|b| !b.is_ascii_whitespace()).unwrap_or(bytes.len());
    bytes[start..].starts_with(b"solid")
}

/// One 50-byte record: the normal (skipped, it is recomputed from the corners wherever it
/// matters), three corners and the attribute word.
fn read_record(record: &[u8]) -> Triangle {
    let read_f32 = |at: usize| f64::from(f32::from_le_bytes([record[at], record[at + 1], record[at + 2], record[at + 3]]));
    let corner = |k: usize| -> Vec3 {
        let at = 12 + k * 12;
        [read_f32(at), read_f32(at + 4), read_f32(at + 8)]
    };
    [corner(0), corner(1), corner(2)]
}

fn ascii_line(
    line: &str,
    line_no: usize,
    corners: &mut Vec<Vec3>,
    block: &mut AsciiBlock,
    triangles: &mut Vec<Triangle>,
) -> Result<(), StlError> {
    let mut words = line.split_whitespace();
    match words.next() {
        Some("solid") if *block == AsciiBlock::Outside => *block = AsciiBlock::Solid,
        Some("endsolid") if *block == AsciiBlock::Solid => *block = AsciiBlock::Outside,
        Some("facet") if *block == AsciiBlock::Solid => *block = AsciiBlock::Facet,
        Some("outer") if *block == AsciiBlock::Facet && words.next() == Some("loop") => *block = AsciiBlock::Loop,
        Some("endfacet") if *block == AsciiBlock::EndFacet => *block = AsciiBlock::Solid,
        Some("vertex") if *block == AsciiBlock::Loop => {
            if corners.len() == 3 {
                return Err(StlError::Syntax { line: line_no, reason: "a facet has three vertices".into() });
            }
            let bad_length = || StlError::Syntax { line: line_no, reason: "a vertex has three coordinates".into() };
            let mut point = [0.0; 3];
            for value in &mut point {
                *value = words
                    .next()
                    .ok_or_else(bad_length)?
                    .parse()
                    .map_err(|e| StlError::Syntax { line: line_no, reason: format!("bad coordinate: {e}") })?;
            }
            if words.next().is_some() {
                return Err(bad_length());
            }
            corners.push(point);
        }
        Some("endloop") if *block == AsciiBlock::Loop => {
            let [a, b, c] = corners[..] else {
                return Err(StlError::Syntax {
                    line: line_no,
                    reason: format!("a facet has three vertices, this one has {}", corners.len()),
                });
            };
            push_checked(triangles, [a, b, c])?;
            corners.clear();
            *block = AsciiBlock::EndFacet;
        }
        Some(keyword @ ("solid" | "endsolid" | "facet" | "outer" | "endfacet" | "vertex" | "endloop")) => {
            return Err(StlError::Syntax { line: line_no, reason: format!("unexpected {keyword}") });
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::tests::tetrahedron;

    /// Writes `mesh` as binary STL, with `header` as the first bytes of the header.
    pub(crate) fn to_binary(mesh: &Mesh, header: &[u8]) -> Vec<u8> {
        write_binary_with_header(mesh, header)
    }

    /// Reads `bytes` through a reader, `chunk` bytes at a time.
    fn parse_in_chunks(bytes: &[u8], chunk: usize) -> Result<Mesh, StlError> {
        let mut reader = StlReader::new(&bytes[..bytes.len().min(StlReader::HEAD)], bytes.len() as u64)?;
        for part in bytes.chunks(chunk) {
            reader.push(part)?;
        }
        reader.finish()
    }

    const TETRA: &str = include_str!("../tests/data/tetra.stl");

    #[test]
    fn file_size_budget_applies_to_binary_and_ascii_stl() {
        let binary = to_binary(&tetrahedron(10.0), b"binary");
        for bytes in [binary.as_slice(), TETRA.as_bytes()] {
            let size = bytes.len() as u64;
            assert!(StlReader::new_with_size_limit(bytes, size, size).is_ok());
            assert!(matches!(StlReader::new_with_size_limit(bytes, size, size - 1), Err(StlError::SizeLimit { .. })));
        }
    }

    #[test]
    fn reads_binary() {
        let mesh = tetrahedron(10.0);
        assert_eq!(parse(&to_binary(&mesh, b"binary")).expect("parses"), mesh);
    }

    #[test]
    fn reads_binary_whose_header_starts_with_solid() {
        let mesh = tetrahedron(5.0);
        assert_eq!(parse(&to_binary(&mesh, b"solid but binary")).expect("parses"), mesh);
    }

    #[test]
    fn reads_ascii() {
        let mesh = parse(TETRA.as_bytes()).expect("parses");
        assert_eq!(mesh.triangles.len(), 4);
        assert!((mesh.volume() - 1000.0 / 6.0).abs() < 1e-9);
    }

    #[test]
    fn chunked_reads_match_whole_reads() {
        let binary = to_binary(&tetrahedron(7.0), b"binary");
        let crlf = TETRA.replace('\n', "\r\n");
        for bytes in [binary.as_slice(), TETRA.as_bytes(), crlf.as_bytes(), TETRA.trim_end().as_bytes()] {
            let whole = parse(bytes).expect("parses");
            for chunk in [1, 3, 49, 50, 51, 84, 85, 1000] {
                assert_eq!(parse_in_chunks(bytes, chunk).expect("parses in chunks"), whole, "chunks of {chunk}");
            }
        }
    }

    #[test]
    fn rejects_truncated_binary() {
        let mut bytes = to_binary(&tetrahedron(1.0), b"x");
        bytes.truncate(bytes.len() - 10);
        assert!(matches!(parse(&bytes), Err(StlError::Truncated { expected: 4, .. })));
    }

    #[test]
    fn rejects_bad_ascii_vertex() {
        let text = "solid t\nfacet normal 0 0 1\nouter loop\nvertex 0 0\nendloop\nendfacet\nendsolid t\n";
        assert!(matches!(parse(text.as_bytes()), Err(StlError::Syntax { line: 4, .. })));
    }

    #[test]
    fn written_binary_reads_back() {
        let mesh = tetrahedron(10.0);
        let bytes = write_binary(&mesh);
        assert_eq!(bytes.len(), HEADER + 4 + 4 * RECORD);
        assert_eq!(parse(&bytes).expect("parses"), mesh);
    }

    #[test]
    fn rejects_ascii_truncated_after_a_complete_loop_or_facet() {
        for marker in ["endloop", "endfacet"] {
            let end = TETRA.find(marker).expect("fixture contains marker") + marker.len();
            assert!(matches!(parse_in_chunks(&TETRA.as_bytes()[..end], 7), Err(StlError::Syntax { .. })));
        }
    }

    #[test]
    fn rejects_unfinished_last_facet() {
        let text = "solid t\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 1 0 0\n";
        assert!(matches!(parse(text.as_bytes()), Err(StlError::Syntax { line: 5, .. })));
    }

    #[test]
    fn rejects_non_finite_corners() {
        for value in ["nan", "inf", "-inf", "NaN"] {
            let text = TETRA.replacen("vertex 0 10 0", &format!("vertex 0 {value} 0"), 1);
            assert_eq!(parse(text.as_bytes()), Err(StlError::NonFinite { triangle: 1 }), "{value}");
        }
        let mut mesh = tetrahedron(1.0);
        mesh.triangles[2][1][0] = f64::NAN;
        assert_eq!(parse(&to_binary(&mesh, b"x")), Err(StlError::NonFinite { triangle: 3 }));
    }

    #[test]
    fn rejects_binary_that_ends_early_at_a_record_boundary() {
        let bytes = to_binary(&tetrahedron(1.0), b"x");
        // The reader is told the full size, but the source stops after two of four records.
        let mut reader = StlReader::new(&bytes, bytes.len() as u64).expect("binary");
        reader.push(&bytes[..PREAMBLE + 2 * RECORD]).expect("pushes");
        assert!(matches!(reader.finish(), Err(StlError::Truncated { expected: 4, .. })));
    }

    #[test]
    fn rejects_coordinates_beyond_single_precision() {
        let text = TETRA.replacen("vertex 0 10 0", "vertex 0 1e39 0", 1);
        assert_eq!(parse(text.as_bytes()), Err(StlError::OutOfRange { triangle: 1 }));
        let text = TETRA.replacen("vertex 0 10 0", "vertex 0 3e38 0", 1);
        assert!(parse(text.as_bytes()).is_ok(), "within f32 range");
    }

    #[test]
    fn rejects_files_without_triangles() {
        assert_eq!(parse(b"solid empty\nendsolid empty\n"), Err(StlError::Empty));
        assert_eq!(parse(&to_binary(&Mesh::default(), b"x")), Err(StlError::Empty));
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(parse(b"hello"), Err(StlError::TooShort));
    }

    #[test]
    fn errors_say_what_is_wrong() {
        let cases = [
            (StlError::TooShort, "too short"),
            (StlError::Truncated { expected: 4, actual: 90 }, "announces 4 triangles"),
            (StlError::Syntax { line: 4, reason: "bad".into() }, "line 4"),
            (StlError::NonFinite { triangle: 2 }, "triangle 2"),
            (StlError::OutOfRange { triangle: 3 }, "triangle 3"),
            (StlError::SizeLimit { size: 9, max_bytes: 8 }, "at most 8 bytes"),
            (StlError::Allocation, "not enough memory"),
            (StlError::Empty, "no triangles"),
        ];
        for (error, fragment) in cases {
            assert!(error.to_string().contains(fragment), "{error}");
        }
    }

    #[test]
    fn rejects_malformed_ascii_loops() {
        let wrap = |body: &str| format!("solid t\nfacet normal 0 0 1\nouter loop\n{body}\nendloop\nendfacet\nendsolid t\n");
        // A fourth vertex in one loop.
        let text = wrap("vertex 0 0 0\nvertex 1 0 0\nvertex 0 1 0\nvertex 1 1 0");
        assert!(matches!(parse(text.as_bytes()), Err(StlError::Syntax { reason, .. }) if reason.contains("three vertices")));
        // A coordinate that is not a number.
        let text = wrap("vertex 0 x 0\nvertex 1 0 0\nvertex 0 1 0");
        assert!(matches!(parse(text.as_bytes()), Err(StlError::Syntax { reason, .. }) if reason.contains("bad coordinate")));
        // A fourth coordinate.
        let text = wrap("vertex 0 0 0 0\nvertex 1 0 0\nvertex 0 1 0");
        assert!(matches!(parse(text.as_bytes()), Err(StlError::Syntax { reason, .. }) if reason.contains("three coordinates")));
        // A loop closed with two vertices.
        let text = wrap("vertex 0 0 0\nvertex 1 0 0");
        assert!(matches!(parse(text.as_bytes()), Err(StlError::Syntax { reason, .. }) if reason.contains("this one has 2")));
        // A keyword where it does not belong.
        let text = "solid t\nfacet normal 0 0 1\nouter loop\nfacet normal 0 0 1\nendloop\nendfacet\nendsolid t\n";
        assert!(matches!(parse(text.as_bytes()), Err(StlError::Syntax { reason, .. }) if reason.contains("unexpected facet")));
    }
}
