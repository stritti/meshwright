//! Reading STL files, binary and ASCII.
//!
//! Binary STL is an 80-byte header, a little-endian `u32` triangle count and 50 bytes per
//! triangle (normal, three corners, attribute word). ASCII STL starts with `solid`; some binary
//! files start with `solid` too, so the decision is made on whether the size matches the count.

use std::fmt;

use crate::mesh::{Mesh, Triangle, Vec3};

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
}

impl fmt::Display for StlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort => write!(f, "the file is too short to be an STL file"),
            Self::Truncated { expected, actual } => {
                write!(f, "the binary STL header announces {expected} triangles, but the file has only {actual} bytes")
            }
            Self::Syntax { line, reason } => write!(f, "ASCII STL, line {line}: {reason}"),
        }
    }
}

impl std::error::Error for StlError {}

const HEADER: usize = 80;
const RECORD: usize = 50;

/// Reads an STL file, binary or ASCII.
pub fn parse(bytes: &[u8]) -> Result<Mesh, StlError> {
    if let Some(count) = binary_count(bytes) {
        let expected_len = HEADER as u64 + 4 + count * RECORD as u64;
        if expected_len == bytes.len() as u64 {
            return Ok(parse_binary(bytes, count));
        }
        if !looks_ascii(bytes) {
            return Err(StlError::Truncated { expected: count, actual: bytes.len() as u64 });
        }
    }
    if looks_ascii(bytes) {
        return parse_ascii(&String::from_utf8_lossy(bytes));
    }
    Err(StlError::TooShort)
}

fn binary_count(bytes: &[u8]) -> Option<u64> {
    let raw: [u8; 4] = bytes.get(HEADER..HEADER + 4)?.try_into().ok()?;
    Some(u64::from(u32::from_le_bytes(raw)))
}

fn looks_ascii(bytes: &[u8]) -> bool {
    let start = bytes.iter().position(|b| !b.is_ascii_whitespace()).unwrap_or(bytes.len());
    bytes[start..].starts_with(b"solid")
}

fn parse_binary(bytes: &[u8], count: u64) -> Mesh {
    let read_f32 = |at: usize| f64::from(f32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]));
    let triangles = (0..count as usize)
        .map(|i| {
            // Skip the 12-byte normal: it is recomputed from the corners wherever it matters.
            let base = HEADER + 4 + i * RECORD + 12;
            let corner = |k: usize| -> Vec3 {
                let at = base + k * 12;
                [read_f32(at), read_f32(at + 4), read_f32(at + 8)]
            };
            [corner(0), corner(1), corner(2)]
        })
        .collect();
    Mesh::new(triangles)
}

fn parse_ascii(text: &str) -> Result<Mesh, StlError> {
    let mut triangles: Vec<Triangle> = Vec::new();
    let mut corners: Vec<Vec3> = Vec::with_capacity(3);
    for (index, line) in text.lines().enumerate() {
        let line_no = index + 1;
        let mut words = line.split_whitespace();
        match words.next() {
            Some("vertex") => {
                let coords: Vec<f64> = words
                    .map(str::parse)
                    .collect::<Result<_, _>>()
                    .map_err(|e| StlError::Syntax { line: line_no, reason: format!("bad coordinate: {e}") })?;
                let [x, y, z] = coords[..] else {
                    return Err(StlError::Syntax { line: line_no, reason: "a vertex has three coordinates".into() });
                };
                corners.push([x, y, z]);
            }
            Some("endloop") => {
                let [a, b, c] = corners[..] else {
                    return Err(StlError::Syntax {
                        line: line_no,
                        reason: format!("a facet has three vertices, this one has {}", corners.len()),
                    });
                };
                triangles.push([a, b, c]);
                corners.clear();
            }
            _ => {}
        }
    }
    Ok(Mesh::new(triangles))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::tests::tetrahedron;

    /// Writes `mesh` as binary STL, with `header` as the first bytes of the header.
    pub(crate) fn to_binary(mesh: &Mesh, header: &[u8]) -> Vec<u8> {
        let mut out = vec![0u8; HEADER];
        out[..header.len()].copy_from_slice(header);
        out.extend_from_slice(&(mesh.triangles.len() as u32).to_le_bytes());
        for tri in &mesh.triangles {
            out.extend_from_slice(&[0u8; 12]);
            for corner in tri {
                for value in corner {
                    out.extend_from_slice(&(*value as f32).to_le_bytes());
                }
            }
            out.extend_from_slice(&[0u8; 2]);
        }
        out
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
        let text = include_str!("../tests/data/tetra.stl");
        let mesh = parse(text.as_bytes()).expect("parses");
        assert_eq!(mesh.triangles.len(), 4);
        assert!((mesh.volume() - 1000.0 / 6.0).abs() < 1e-9);
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
    fn rejects_garbage() {
        assert_eq!(parse(b"hello"), Err(StlError::TooShort));
    }
}
