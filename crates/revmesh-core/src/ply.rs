//! Reading PLY files (Polygon File Format), ASCII and binary little-endian.
//!
//! Only what a mesh needs: a `vertex` element with `x`, `y` and `z` properties (extra scalar
//! properties such as colours are read and ignored) and a `face` element whose `vertex_indices`
//! list holds the corners (0-based indices; faces with more than three corners are triangulated
//! as a fan; any other list on the face element, e.g. texture coordinates, is read and ignored,
//! as is any other property of the face). `comment` and `obj_info` header lines are skipped, and
//! so are the records of every other element (`edge`, `material`, …); the formats other than
//! `ascii 1.0` and `binary_little_endian 1.0` are refused.
//!
//! [`PlyReader`] reads the file in chunks, as the plugin receives it, so the file never has to be
//! held in memory next to the mesh made from it. [`write_ascii`] and [`write_binary`] write a mesh
//! as PLY (used by the tests and the fixture generator).

use std::fmt;

use crate::format::{MeshError, extend_reserved, fmt_num, push_checked, push_reserved, welded_vertices};
use crate::i18n::Locale;
use crate::mesh::{Mesh, Triangle, Vec3};

/// A line longer than this is refused: a face with millions of corners is not a part.
const LINE_MAX: usize = 4 << 20;

/// Why a PLY file broke the grammar, in the header or in a body record. Localised through the
/// `ply.reason.*` catalog keys; [`fmt::Display`] gives the English words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlyReason {
    /// A header keyword or element that is not part of the grammar this reader accepts.
    UnexpectedKeyword(String),
    /// A `format` line other than `ascii 1.0` or `binary_little_endian 1.0`.
    UnsupportedFormat(String),
    /// The header has no `element vertex`.
    MissingVertex,
    /// The `vertex` element has no scalar `x`, `y` and `z` properties.
    NoVertexCoords,
    /// The header has no `element face`.
    MissingFace,
    /// The `face` element has no list property.
    NoFaceList,
    /// A number that does not parse (a header count or a body value).
    BadNumber,
    /// The file ends before every announced record is read.
    Truncated,
    /// A face corner index outside the vertices read so far.
    IndexOutOfRange(usize),
    /// A face with fewer than three corners.
    FaceTooSmall(usize),
    /// One line longer than 4 MiB.
    LineTooLong,
}

impl PlyReason {
    /// The reason in `locale`'s words.
    #[must_use]
    pub fn message(&self, locale: Locale) -> String {
        match self {
            Self::UnexpectedKeyword(keyword) => locale.format("ply.reason.unexpected_keyword", &[("keyword", keyword)]),
            Self::UnsupportedFormat(format) => locale.format("ply.reason.unsupported_format", &[("format", format)]),
            Self::MissingVertex => locale.text("ply.reason.missing_vertex").to_owned(),
            Self::NoVertexCoords => locale.text("ply.reason.no_vertex_coords").to_owned(),
            Self::MissingFace => locale.text("ply.reason.missing_face").to_owned(),
            Self::NoFaceList => locale.text("ply.reason.no_face_list").to_owned(),
            Self::BadNumber => locale.text("ply.reason.bad_number").to_owned(),
            Self::Truncated => locale.text("ply.reason.truncated").to_owned(),
            Self::IndexOutOfRange(n) => locale.format("ply.reason.index_out_of_range", &[("n", n)]),
            Self::FaceTooSmall(n) => locale.format("ply.reason.face_too_small", &[("n", n)]),
            Self::LineTooLong => locale.text("ply.reason.line_too_long").to_owned(),
        }
    }
}

impl fmt::Display for PlyReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedKeyword(keyword) => write!(f, "unexpected {keyword}"),
            Self::UnsupportedFormat(format) => write!(f, "unsupported PLY format {format}"),
            Self::MissingVertex => f.write_str("the header has no vertex element"),
            Self::NoVertexCoords => f.write_str("the vertex element has no x, y and z coordinates"),
            Self::MissingFace => f.write_str("the header has no face element"),
            Self::NoFaceList => f.write_str("the face element has no vertex index list"),
            Self::BadNumber => f.write_str("bad number"),
            Self::Truncated => f.write_str("the file ends before all records are read"),
            Self::IndexOutOfRange(n) => write!(f, "vertex index {n} is out of range"),
            Self::FaceTooSmall(n) => write!(f, "a face has fewer than three corners, this one has {n}"),
            Self::LineTooLong => f.write_str("a line is too long"),
        }
    }
}

/// A PLY property type: its name in the header, its size and how to read it little-endian.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlyType {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    F32,
    F64,
}

impl PlyType {
    fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "char" | "int8" => Self::I8,
            "uchar" | "uint8" => Self::U8,
            "short" | "int16" => Self::I16,
            "ushort" | "uint16" => Self::U16,
            "int" | "int32" => Self::I32,
            "uint" | "uint32" => Self::U32,
            "float" | "float32" => Self::F32,
            "double" | "float64" => Self::F64,
            _ => return None,
        })
    }

    fn size(self) -> usize {
        match self {
            Self::I8 | Self::U8 => 1,
            Self::I16 | Self::U16 => 2,
            Self::I32 | Self::U32 | Self::F32 => 4,
            Self::F64 => 8,
        }
    }

    /// Reads `bytes` (exactly [`PlyType::size`] of them) as a coordinate.
    fn read(self, bytes: &[u8]) -> f64 {
        match self {
            Self::I8 => f64::from(i8::from_le_bytes([bytes[0]])),
            Self::U8 => f64::from(bytes[0]),
            Self::I16 => f64::from(i16::from_le_bytes(bytes[..2].try_into().unwrap_or([0, 0]))),
            Self::U16 => f64::from(u16::from_le_bytes(bytes[..2].try_into().unwrap_or([0, 0]))),
            Self::I32 => f64::from(i32::from_le_bytes(bytes[..4].try_into().unwrap_or([0, 0, 0, 0]))),
            Self::U32 => f64::from(u32::from_le_bytes(bytes[..4].try_into().unwrap_or([0, 0, 0, 0]))),
            Self::F32 => f64::from(f32::from_le_bytes(bytes[..4].try_into().unwrap_or([0, 0, 0, 0]))),
            Self::F64 => f64::from_le_bytes(bytes[..8].try_into().unwrap_or([0; 8])),
        }
    }

    /// Reads `bytes` as an integer (a list count or a corner index); `None` for the float types,
    /// which cannot hold a count.
    fn read_int(self, bytes: &[u8]) -> Option<i64> {
        Some(match self {
            Self::I8 => i64::from(i8::from_le_bytes([bytes[0]])),
            Self::U8 => i64::from(bytes[0]),
            Self::I16 => i64::from(i16::from_le_bytes(bytes[..2].try_into().ok()?)),
            Self::U16 => i64::from(u16::from_le_bytes(bytes[..2].try_into().ok()?)),
            Self::I32 => i64::from(i32::from_le_bytes(bytes[..4].try_into().ok()?)),
            Self::U32 => i64::from(u32::from_le_bytes(bytes[..4].try_into().ok()?)),
            Self::F32 | Self::F64 => return None,
        })
    }
}

/// One property of an element, in the order the header declares.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Prop {
    Scalar { ty: PlyType, name: String },
    List { count: PlyType, item: PlyType, name: String },
}

/// An element the header declared. `vertex` and `face` are read into the mesh; every other
/// element's records are consumed and discarded.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Element {
    name: String,
    kind: ElementKind,
    count: u64,
    props: Vec<Prop>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ElementKind {
    Vertex,
    Face,
    Skip,
}

/// What comes after `end_header`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BodyFormat {
    Ascii,
    Binary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Header,
    BodyAscii,
    BodyBinary,
}

/// Reads a PLY file in chunks.
///
/// Create it with the file's first bytes and its size, then [`push`](Self::push) every byte of the
/// file in order, from the start, and [`finish`](Self::finish).
#[derive(Debug)]
pub struct PlyReader {
    phase: Phase,
    /// The file's total size, to refuse a record that could never fit in it.
    size: u64,
    /// The bytes after the last consumed line or record.
    pending: Vec<u8>,
    /// 1-based header line being read.
    line: usize,
    /// Body records read so far.
    record: usize,
    /// The elements declared so far (header phase) and fixed after `end_header`.
    elements: Vec<Element>,
    /// The index into `elements` whose properties the next `property` line declares.
    current: Option<usize>,
    format: Option<BodyFormat>,
    /// The element whose records the body is in, and how many of them are left.
    body_pos: usize,
    body_left: u64,
    vertices: Vec<Vec3>,
    triangles: Vec<Triangle>,
}

impl PlyReader {
    /// A reader for a file of `size` bytes that starts with `head`; the head must carry the
    /// `ply` magic, or the first line is refused.
    pub fn new(head: &[u8], size: u64) -> Result<Self, MeshError> {
        let magic = head.starts_with(b"ply\n") || head.starts_with(b"ply\r\n") || head == b"ply";
        if !magic {
            let word = String::from_utf8_lossy(head).split_whitespace().next().unwrap_or_default().to_owned();
            return Err(MeshError::PlyHeader { line: 1, reason: PlyReason::UnexpectedKeyword(word) });
        }
        Ok(Self {
            phase: Phase::Header,
            size,
            pending: Vec::new(),
            line: 0,
            record: 0,
            elements: Vec::new(),
            current: None,
            format: None,
            body_pos: 0,
            body_left: 0,
            vertices: Vec::new(),
            triangles: Vec::new(),
        })
    }

    /// Reads the next bytes of the file.
    pub fn push(&mut self, chunk: &[u8]) -> Result<(), MeshError> {
        extend_reserved(&mut self.pending, chunk)?;
        let mut buf = std::mem::take(&mut self.pending);
        let mut consumed = 0usize;
        let result = self.consume(&buf, &mut consumed);
        buf.drain(..consumed);
        self.pending = buf;
        result
    }

    /// Consumes complete lines or records from the front of `buf`, advancing `consumed`.
    fn consume(&mut self, buf: &[u8], consumed: &mut usize) -> Result<(), MeshError> {
        loop {
            match self.phase {
                Phase::Header => {
                    let Some(pos) = buf[*consumed..].iter().position(|b| *b == b'\n') else {
                        if buf.len() - *consumed > LINE_MAX {
                            return Err(MeshError::PlyHeader { line: self.line + 1, reason: PlyReason::LineTooLong });
                        }
                        break;
                    };
                    let text = String::from_utf8_lossy(&buf[*consumed..*consumed + pos]).into_owned();
                    *consumed += pos + 1;
                    self.line += 1;
                    if self.header_line(&text)? {
                        self.start_body()?;
                    }
                }
                Phase::BodyAscii => {
                    let Some(pos) = buf[*consumed..].iter().position(|b| *b == b'\n') else {
                        if buf.len() - *consumed > LINE_MAX {
                            return Err(MeshError::PlyBody { record: self.record + 1, reason: PlyReason::LineTooLong });
                        }
                        break;
                    };
                    let text = String::from_utf8_lossy(&buf[*consumed..*consumed + pos]).into_owned();
                    *consumed += pos + 1;
                    self.ascii_record(&text)?;
                }
                Phase::BodyBinary => {
                    let rest = &buf[*consumed..];
                    if self.body_pos >= self.elements.len() {
                        *consumed = buf.len();
                        break;
                    }
                    let needed = binary_needed(&self.elements[self.body_pos].props, rest, self.size, self.record + 1)?;
                    let Some(used) = needed else { break };
                    let record = &rest[..used];
                    *consumed += used;
                    self.binary_record(record)?;
                }
            }
        }
        Ok(())
    }

    /// The mesh, once every byte has been pushed.
    pub fn finish(mut self) -> Result<Mesh, MeshError> {
        if !self.pending.is_empty() {
            match self.phase {
                Phase::Header => {
                    self.line += 1;
                    let text = String::from_utf8_lossy(&self.pending).into_owned();
                    if self.header_line(&text)? {
                        self.start_body()?;
                    }
                }
                Phase::BodyAscii => {
                    let text = String::from_utf8_lossy(&self.pending).into_owned();
                    self.ascii_record(&text)?;
                }
                Phase::BodyBinary => {}
            }
            self.pending.clear();
        }
        match self.phase {
            Phase::Header => {
                return Err(MeshError::PlyHeader { line: self.line.max(1), reason: PlyReason::Truncated });
            }
            Phase::BodyAscii | Phase::BodyBinary => {
                if self.body_pos < self.elements.len() {
                    return Err(MeshError::PlyBody { record: self.record + 1, reason: PlyReason::Truncated });
                }
            }
        }
        if self.triangles.is_empty() {
            return Err(MeshError::Empty);
        }
        Ok(Mesh::new(self.triangles))
    }

    /// One header line; `Ok(true)` when it was `end_header`.
    fn header_line(&mut self, text: &str) -> Result<bool, MeshError> {
        let line = self.line;
        let err = |reason| MeshError::PlyHeader { line, reason };
        let mut words = text.split_whitespace();
        let Some(keyword) = words.next() else { return Ok(false) };
        match keyword {
            "ply" => {
                if self.line != 1 || words.next().is_some() {
                    return Err(err(PlyReason::UnexpectedKeyword("ply".to_owned())));
                }
            }
            "format" => {
                let rest: Vec<&str> = words.collect();
                if self.format.is_some() || rest.len() != 2 {
                    return Err(err(PlyReason::UnexpectedKeyword("format".to_owned())));
                }
                self.format = Some(match rest[0] {
                    "ascii" if rest[1] == "1.0" => BodyFormat::Ascii,
                    "binary_little_endian" if rest[1] == "1.0" => BodyFormat::Binary,
                    other => return Err(err(PlyReason::UnsupportedFormat(other.to_owned()))),
                });
            }
            "comment" | "obj_info" => {}
            "element" => {
                let name = words.next().unwrap_or_default();
                let count: u64 = words.next().and_then(|w| w.parse().ok()).ok_or(err(PlyReason::BadNumber))?;
                if words.next().is_some() {
                    return Err(err(PlyReason::UnexpectedKeyword("element".to_owned())));
                }
                let kind = match name {
                    "vertex" => ElementKind::Vertex,
                    "face" => ElementKind::Face,
                    _ => ElementKind::Skip,
                };
                if self.elements.iter().any(|e| e.name == name) {
                    return Err(err(PlyReason::UnexpectedKeyword(name.to_owned())));
                }
                self.elements.push(Element { name: name.to_owned(), kind, count, props: Vec::new() });
                self.current = Some(self.elements.len() - 1);
            }
            "property" => {
                let Some(idx) = self.current else { return Err(err(PlyReason::UnexpectedKeyword("property".to_owned()))) };
                let args: Vec<&str> = words.collect();
                let prop = if args.first() == Some(&"list") {
                    if args.len() != 4 {
                        return Err(err(PlyReason::UnexpectedKeyword("property".to_owned())));
                    }
                    let count = PlyType::from_name(args[1]).ok_or_else(|| err(PlyReason::UnexpectedKeyword(args[1].to_owned())))?;
                    let item = PlyType::from_name(args[2]).ok_or_else(|| err(PlyReason::UnexpectedKeyword(args[2].to_owned())))?;
                    Prop::List { count, item, name: args[3].to_owned() }
                } else {
                    if args.len() != 2 {
                        return Err(err(PlyReason::UnexpectedKeyword("property".to_owned())));
                    }
                    let ty = PlyType::from_name(args[0]).ok_or_else(|| err(PlyReason::UnexpectedKeyword(args[0].to_owned())))?;
                    Prop::Scalar { ty, name: args[1].to_owned() }
                };
                self.elements[idx].props.push(prop);
            }
            "end_header" => {
                if words.next().is_some() {
                    return Err(err(PlyReason::UnexpectedKeyword("end_header".to_owned())));
                }
                return Ok(true);
            }
            other => return Err(err(PlyReason::UnexpectedKeyword(other.to_owned()))),
        }
        Ok(false)
    }

    /// Validates the header and switches to its body format.
    fn start_body(&mut self) -> Result<(), MeshError> {
        let line = self.line;
        let err = |reason| MeshError::PlyHeader { line, reason };
        let format = self.format.ok_or_else(|| err(PlyReason::UnsupportedFormat("none".to_owned())))?;
        let vertex = self.elements.iter().find(|e| e.kind == ElementKind::Vertex).ok_or(err(PlyReason::MissingVertex))?;
        let coord = |name: &str| vertex.props.iter().any(|p| matches!(p, Prop::Scalar { name: n, .. } if n == name));
        if !vertex.props.iter().all(|p| matches!(p, Prop::Scalar { .. })) || !coord("x") || !coord("y") || !coord("z") {
            return Err(err(PlyReason::NoVertexCoords));
        }
        let face = self.elements.iter().find(|e| e.kind == ElementKind::Face).ok_or(err(PlyReason::MissingFace))?;
        if !face.props.iter().any(|p| matches!(p, Prop::List { .. })) {
            return Err(err(PlyReason::NoFaceList));
        }
        self.phase = match format {
            BodyFormat::Ascii => Phase::BodyAscii,
            BodyFormat::Binary => Phase::BodyBinary,
        };
        self.body_pos = 0;
        self.body_left = self.elements[0].count;
        self.skip_done();
        Ok(())
    }

    /// One ASCII body line: a vertex or a face record.
    fn ascii_record(&mut self, text: &str) -> Result<(), MeshError> {
        if self.body_pos >= self.elements.len() {
            return Ok(());
        }
        self.record += 1;
        let record = self.record;
        let err = |reason| MeshError::PlyBody { record, reason };
        let kind = self.elements[self.body_pos].kind;
        let props = self.elements[self.body_pos].props.clone();
        let mut words = text.split_whitespace();
        match kind {
            ElementKind::Vertex => {
                let mut corner = [0.0f64; 3];
                for prop in &props {
                    let Prop::Scalar { name, .. } = prop else { return Err(err(PlyReason::NoVertexCoords)) };
                    let Some(word) = words.next() else { return Err(err(PlyReason::BadNumber)) };
                    let value: f64 = word.parse().map_err(|_| err(PlyReason::BadNumber))?;
                    match name.as_str() {
                        "x" => corner[0] = value,
                        "y" => corner[1] = value,
                        "z" => corner[2] = value,
                        _ => {}
                    }
                }
                if words.next().is_some() {
                    return Err(err(PlyReason::BadNumber));
                }
                push_reserved(&mut self.vertices, corner)?;
            }
            ElementKind::Face => {
                let indices = self.ascii_face(&props, &mut words, record)?;
                self.face_from_indices(&indices, record)?;
            }
            // Another element's record: consumed, not read.
            ElementKind::Skip => {}
        }
        self.advance();
        Ok(())
    }

    /// The face indices of one ASCII face record: the `vertex_indices` list (or the first list
    /// when none is named that); every other list is consumed without reading it as indices.
    fn ascii_face(&mut self, props: &[Prop], words: &mut std::str::SplitWhitespace<'_>, record: usize) -> Result<Vec<usize>, MeshError> {
        let err = |reason| MeshError::PlyBody { record, reason };
        let list_pos = list_position(props);
        let mut indices: Vec<usize> = Vec::new();
        for (i, prop) in props.iter().enumerate() {
            match prop {
                Prop::Scalar { .. } => {
                    if words.next().is_none() {
                        return Err(err(PlyReason::BadNumber));
                    }
                }
                Prop::List { .. } => {
                    let Some(word) = words.next() else { return Err(err(PlyReason::BadNumber)) };
                    let count: i64 = word.parse().map_err(|_| err(PlyReason::BadNumber))?;
                    let Ok(count) = usize::try_from(count) else { return Err(err(PlyReason::BadNumber)) };
                    if Some(i) == list_pos {
                        for _ in 0..count {
                            let Some(word) = words.next() else { return Err(err(PlyReason::BadNumber)) };
                            let n: i64 = word.parse().map_err(|_| err(PlyReason::BadNumber))?;
                            let Some(idx) = checked_index(n, self.vertices.len()) else {
                                return Err(err(PlyReason::IndexOutOfRange(usize::try_from(n.max(0)).unwrap_or(0))));
                            };
                            push_reserved(&mut indices, idx)?;
                        }
                    } else {
                        for _ in 0..count {
                            if words.next().is_none() {
                                return Err(err(PlyReason::BadNumber));
                            }
                        }
                    }
                }
            }
        }
        Ok(indices)
    }

    /// One binary body record of `bytes` (exactly its size, see [`binary_needed`]).
    fn binary_record(&mut self, bytes: &[u8]) -> Result<(), MeshError> {
        self.record += 1;
        let record = self.record;
        let err = |reason| MeshError::PlyBody { record, reason };
        let kind = self.elements[self.body_pos].kind;
        let props = self.elements[self.body_pos].props.clone();
        let mut off = 0usize;
        match kind {
            ElementKind::Vertex => {
                let mut corner = [0.0f64; 3];
                for prop in &props {
                    let Prop::Scalar { ty, name } = prop else { return Err(err(PlyReason::NoVertexCoords)) };
                    let value = ty.read(&bytes[off..off + ty.size()]);
                    match name.as_str() {
                        "x" => corner[0] = value,
                        "y" => corner[1] = value,
                        "z" => corner[2] = value,
                        _ => {}
                    }
                    off += ty.size();
                }
                push_reserved(&mut self.vertices, corner)?;
            }
            ElementKind::Face => {
                let indices = self.binary_face(&props, bytes, record, &mut off)?;
                self.face_from_indices(&indices, record)?;
            }
            // Another element's record: consumed, not read.
            ElementKind::Skip => {}
        }
        self.advance();
        Ok(())
    }

    /// The face indices of one binary face record: the `vertex_indices` list (or the first list
    /// when none is named that); every other list's bytes are skipped.
    fn binary_face(&mut self, props: &[Prop], bytes: &[u8], record: usize, off: &mut usize) -> Result<Vec<usize>, MeshError> {
        let err = |reason| MeshError::PlyBody { record, reason };
        let list_pos = list_position(props);
        let mut indices: Vec<usize> = Vec::new();
        for (i, prop) in props.iter().enumerate() {
            match prop {
                Prop::Scalar { ty, .. } => *off += ty.size(),
                Prop::List { count, item, .. } => {
                    let Some(n) = count.read_int(&bytes[*off..*off + count.size()]) else {
                        return Err(err(PlyReason::BadNumber));
                    };
                    let Ok(n) = usize::try_from(n) else { return Err(err(PlyReason::BadNumber)) };
                    *off += count.size();
                    if Some(i) == list_pos {
                        for _ in 0..n {
                            let Some(v) = item.read_int(&bytes[*off..*off + item.size()]) else {
                                return Err(err(PlyReason::BadNumber));
                            };
                            *off += item.size();
                            let Some(idx) = checked_index(v, self.vertices.len()) else {
                                return Err(err(PlyReason::IndexOutOfRange(usize::try_from(v.max(0)).unwrap_or(0))));
                            };
                            push_reserved(&mut indices, idx)?;
                        }
                    } else {
                        // Checked: a crafted count must not wrap the offset (usize is 32 bit on
                        // wasm32), or the record would end early and the slices below panic.
                        let bytes = n.checked_mul(item.size()).and_then(|b| off.checked_add(b)).ok_or(err(PlyReason::BadNumber))?;
                        *off = bytes;
                    }
                }
            }
        }
        Ok(indices)
    }

    /// Fan-triangulates a face's corner indices into checked triangles.
    fn face_from_indices(&mut self, indices: &[usize], record: usize) -> Result<(), MeshError> {
        if indices.len() < 3 {
            return Err(MeshError::PlyBody { record, reason: PlyReason::FaceTooSmall(indices.len()) });
        }
        for k in 1..indices.len() - 1 {
            let triangle = [self.vertices[indices[0]], self.vertices[indices[k]], self.vertices[indices[k + 1]]];
            push_checked(&mut self.triangles, triangle)?;
        }
        Ok(())
    }

    /// Moves to the next body element once the current one's records are read.
    fn advance(&mut self) {
        self.body_left -= 1;
        self.skip_done();
    }

    /// Skips elements whose announced count is zero, so `body_pos == elements.len()` means the
    /// body is fully read.
    fn skip_done(&mut self) {
        while self.body_pos < self.elements.len() && self.body_left == 0 {
            self.body_pos += 1;
            if self.body_pos < self.elements.len() {
                self.body_left = self.elements[self.body_pos].count;
            }
        }
    }
}

/// A corner index from the file as `usize`, refused when negative or past the vertices read so
/// far. The index in a file is 0-based.
fn checked_index(n: i64, vertices: usize) -> Option<usize> {
    let idx = usize::try_from(n).ok()?;
    (idx < vertices).then_some(idx)
}

/// The position of the list property that holds a face's corner indices: the one named
/// `vertex_indices`, else the first list.
fn list_position(props: &[Prop]) -> Option<usize> {
    props
        .iter()
        .position(|p| matches!(p, Prop::List { name, .. } if name == "vertex_indices"))
        .or_else(|| props.iter().position(|p| matches!(p, Prop::List { .. })))
}

/// How many bytes of `buf` the next binary record needs, or `Ok(None)` when `buf` holds no
/// complete record yet. A list count whose bytes cannot even fit in the file — or whose product
/// would wrap `usize` on wasm32 — is a bad number, not a truncated file.
fn binary_needed(props: &[Prop], buf: &[u8], file_size: u64, record: usize) -> Result<Option<usize>, MeshError> {
    let err = || MeshError::PlyBody { record, reason: PlyReason::BadNumber };
    let mut off = 0usize;
    for prop in props {
        match prop {
            Prop::Scalar { ty, .. } => {
                off += ty.size();
                if buf.len() < off {
                    return Ok(None);
                }
            }
            Prop::List { count, item, .. } => {
                if buf.len() < off + count.size() {
                    return Ok(None);
                }
                let n = count.read_int(&buf[off..off + count.size()]).ok_or_else(err)?;
                let Ok(n) = usize::try_from(n) else { return Err(err()) };
                off += count.size();
                // Checked: on wasm32 `usize` is 32 bit, so a crafted count times the item size
                // can wrap and make the record look complete.
                let end = n.checked_mul(item.size()).and_then(|b| off.checked_add(b)).ok_or_else(err)?;
                if u64::try_from(end).unwrap_or(u64::MAX) > file_size {
                    return Err(err());
                }
                off = end;
                if buf.len() < off {
                    return Ok(None);
                }
            }
        }
    }
    Ok(Some(off))
}

/// Writes a mesh as ASCII PLY (shared vertices once, faces as `3 a b c` lines, 0-based).
#[must_use]
pub fn write_ascii(mesh: &Mesh) -> Vec<u8> {
    let (vertices, faces) = welded_vertices(mesh);
    let mut out = format!(
        "ply\nformat ascii 1.0\ncomment revmesh test fixture\nelement vertex {}\nproperty float x\nproperty float y\nproperty float z\nelement face {}\nproperty list uchar int vertex_indices\nend_header\n",
        vertices.len(),
        faces.len()
    );
    for v in &vertices {
        let line = format!("{} {} {}\n", fmt_num(v[0]), fmt_num(v[1]), fmt_num(v[2]));
        out.push_str(&line);
    }
    for f in &faces {
        let line = format!("3 {} {} {}\n", f[0], f[1], f[2]);
        out.push_str(&line);
    }
    out.into_bytes()
}

/// Writes a mesh as binary little-endian PLY (`float` vertices, `uchar`/`int` face lists).
#[must_use]
pub fn write_binary(mesh: &Mesh) -> Vec<u8> {
    let (vertices, faces) = welded_vertices(mesh);
    let mut out = format!(
        "ply\nformat binary_little_endian 1.0\ncomment revmesh test fixture\nelement vertex {}\nproperty float x\nproperty float y\nproperty float z\nelement face {}\nproperty list uchar int vertex_indices\nend_header\n",
        vertices.len(),
        faces.len()
    )
    .into_bytes();
    for v in &vertices {
        for c in v {
            out.extend_from_slice(&(*c as f32).to_le_bytes());
        }
    }
    for f in &faces {
        out.push(3u8);
        for i in f {
            out.extend_from_slice(&i32::try_from(*i).unwrap_or(i32::MAX).to_le_bytes());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tetrahedron of volume 1000/6 mm³.
    fn tetra() -> Mesh {
        Mesh::new(vec![
            [[0.0, 0.0, 0.0], [0.0, 10.0, 0.0], [10.0, 0.0, 0.0]],
            [[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [0.0, 0.0, 10.0]],
            [[0.0, 0.0, 0.0], [0.0, 0.0, 10.0], [0.0, 10.0, 0.0]],
            [[10.0, 0.0, 0.0], [0.0, 10.0, 0.0], [0.0, 0.0, 10.0]],
        ])
    }

    fn read(bytes: &[u8]) -> Result<Mesh, MeshError> {
        read_chunked(bytes, bytes.len().max(1))
    }

    fn read_chunked(bytes: &[u8], chunk: usize) -> Result<Mesh, MeshError> {
        let mut r = PlyReader::new(bytes, bytes.len() as u64)?;
        let mut off = 0;
        while off < bytes.len() {
            let end = (off + chunk).min(bytes.len());
            r.push(&bytes[off..end])?;
            off = end;
        }
        r.finish()
    }

    const HEADER: &str = "ply\nformat ascii 1.0\nelement vertex 4\nproperty float x\nproperty float y\nproperty float z\nelement face 4\nproperty list uchar int vertex_indices\nend_header\n";
    const TETRA_BODY: &str = "0 0 0\n10 0 0\n0 10 0\n0 0 10\n3 0 2 1\n3 0 1 3\n3 0 3 2\n3 1 2 3\n";

    #[test]
    fn reads_an_ascii_tetrahedron() {
        let text = format!("{HEADER}{TETRA_BODY}");
        let mesh = read(text.as_bytes()).expect("reads");
        assert_eq!(mesh.triangles.len(), 4);
        assert!((mesh.volume().abs() - 1000.0 / 6.0).abs() < 1e-9, "{}", mesh.volume());
    }

    #[test]
    fn reads_in_odd_sized_chunks() {
        let text = format!("{HEADER}{TETRA_BODY}");
        assert_eq!(read_chunked(text.as_bytes(), 7).expect("reads").triangles.len(), 4);
    }

    #[test]
    fn ascii_and_binary_writers_round_trip() {
        let mesh = tetra();
        for bytes in [write_ascii(&mesh), write_binary(&mesh)] {
            assert_eq!(read_chunked(&bytes, 7).expect("reads").triangles, mesh.triangles);
            assert_eq!(read(&bytes).expect("reads").triangles, mesh.triangles);
        }
    }

    #[test]
    fn comments_obj_info_and_extra_scalar_properties_are_skipped() {
        let text = "ply\nformat ascii 1.0\ncomment made by a test\nobj_info whatever\nelement vertex 4\nproperty float x\nproperty float y\nproperty float z\nproperty uchar red\nelement face 4\nproperty list uchar int vertex_indices\nproperty uchar facing\nend_header\n0 0 0 255\n10 0 0 255\n0 10 0 255\n0 0 10 255\n3 0 2 1 7\n3 0 1 3 7\n3 0 3 2 7\n3 1 2 3 7\n";
        assert_eq!(read(text.as_bytes()).expect("reads").triangles.len(), 4);
    }

    #[test]
    fn the_head_must_carry_the_ply_magic() {
        let err = PlyReader::new(b"solid x\n", 8).expect_err("an error");
        assert_eq!(err, MeshError::PlyHeader { line: 1, reason: PlyReason::UnexpectedKeyword("solid".to_owned()) });
    }

    #[test]
    fn unsupported_formats_are_refused_in_the_header() {
        let text = "ply\nformat binary_big_endian 1.0\nend_header\n";
        let err = read(text.as_bytes()).expect_err("an error");
        assert_eq!(err, MeshError::PlyHeader { line: 2, reason: PlyReason::UnsupportedFormat("binary_big_endian".to_owned()) });
        assert_eq!(
            err.to_string(),
            "PLY header line 2: unsupported PLY format binary_big_endian; ascii 1.0 and binary_little_endian 1.0 are read"
        );
        assert!(err.message(Locale::De).contains("PLY-Kopf, Zeile 2"), "{}", err.message(Locale::De));
    }

    #[test]
    fn broken_headers_are_refused_with_their_line() {
        let cases: [(&str, usize, PlyReason); 5] = [
            ("ply\nelement vertex 0\nelement vertex 1\nend_header\n", 3, PlyReason::UnexpectedKeyword("vertex".to_owned())),
            ("ply\nformat ascii 1.0\nelement vertex x\nend_header\n", 3, PlyReason::BadNumber),
            ("ply\nformat ascii 1.0\nelement face 0\nproperty list uchar int v\nend_header\n", 5, PlyReason::MissingVertex),
            (
                "ply\nformat ascii 1.0\nelement vertex 0\nproperty float x\nproperty float y\nproperty float z\nend_header\n",
                7,
                PlyReason::MissingFace,
            ),
            (
                "ply\nformat ascii 1.0\nelement vertex 0\nproperty float x\nproperty float y\nelement face 0\nproperty uchar red\nend_header\n",
                8,
                PlyReason::NoVertexCoords,
            ),
        ];
        for (text, line, reason) in cases {
            assert_eq!(read(text.as_bytes()).expect_err("an error"), MeshError::PlyHeader { line, reason: reason.clone() }, "{text}");
        }
        // A face element without a list property.
        let text = "ply\nformat ascii 1.0\nelement vertex 0\nproperty float x\nproperty float y\nproperty float z\nelement face 0\nproperty int v\nend_header\n";
        assert_eq!(read(text.as_bytes()).expect_err("an error"), MeshError::PlyHeader { line: 9, reason: PlyReason::NoFaceList });
    }

    #[test]
    fn a_truncated_body_is_refused() {
        let text = format!("{HEADER}0 0 0\n10 0 0\n");
        let err = read(text.as_bytes()).expect_err("an error");
        assert_eq!(err, MeshError::PlyBody { record: 3, reason: PlyReason::Truncated });
        // A header that never ends.
        let err = read(b"ply\nformat ascii 1.0\n").expect_err("an error");
        assert_eq!(err, MeshError::PlyHeader { line: 2, reason: PlyReason::Truncated });
    }

    #[test]
    fn broken_body_records_are_refused_with_their_record_number() {
        let bad_index = format!("{HEADER}0 0 0\n10 0 0\n0 10 0\n0 0 10\n3 0 1 9\n");
        assert_eq!(
            read(bad_index.as_bytes()).expect_err("an error"),
            MeshError::PlyBody { record: 5, reason: PlyReason::IndexOutOfRange(9) }
        );
        let small_face = format!("{HEADER}0 0 0\n10 0 0\n0 10 0\n0 0 10\n2 0 1\n");
        assert_eq!(
            read(small_face.as_bytes()).expect_err("an error"),
            MeshError::PlyBody { record: 5, reason: PlyReason::FaceTooSmall(2) }
        );
        let bad_number = format!("{HEADER}0 0 x\n");
        assert_eq!(read(bad_number.as_bytes()).expect_err("an error"), MeshError::PlyBody { record: 1, reason: PlyReason::BadNumber });
    }

    #[test]
    fn a_truncated_binary_body_is_refused() {
        let mut bytes = write_binary(&tetra());
        bytes.truncate(bytes.len() - 5);
        let err = read(&bytes).expect_err("an error");
        assert_eq!(err, MeshError::PlyBody { record: 8, reason: PlyReason::Truncated });
    }

    #[test]
    fn other_elements_are_skipped_ascii_and_binary() {
        // MeshLab and scanners write `element edge 0` (or `material`); Blender writes `edge` for
        // loose edges. Their records are consumed and discarded, even non-empty ones.
        let ascii = "ply\nformat ascii 1.0\nelement vertex 4\nproperty float x\nproperty float y\nproperty float z\nelement face 4\nproperty list uchar int vertex_indices\nelement edge 0\nproperty int vertex1\nelement material 2\nproperty uchar red\nend_header\n0 0 0\n10 0 0\n0 10 0\n0 0 10\n3 0 2 1\n3 0 1 3\n3 0 3 2\n3 1 2 3\n255\n0\n";
        assert_eq!(read(ascii.as_bytes()).expect("reads").triangles.len(), 4);

        let mut binary = b"ply\nformat binary_little_endian 1.0\nelement vertex 4\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar int vertex_indices\nelement edge 2\nproperty int vertex1\nproperty int vertex2\nend_header\n".to_vec();
        for v in [[0.0f32, 0.0, 0.0], [10.0, 0.0, 0.0], [0.0, 10.0, 0.0], [0.0, 0.0, 10.0]] {
            for c in v {
                binary.extend_from_slice(&c.to_le_bytes());
            }
        }
        binary.push(3u8);
        for i in [0i32, 2, 1] {
            binary.extend_from_slice(&i.to_le_bytes());
        }
        for e in [[0i32, 1], [1, 2]] {
            for i in e {
                binary.extend_from_slice(&i.to_le_bytes());
            }
        }
        assert_eq!(read(&binary).expect("reads").triangles.len(), 1);
    }

    #[test]
    fn face_lists_other_than_vertex_indices_are_ignored() {
        // Textured meshes carry a second list on the face element, e.g. MeshLab's texcoords.
        let ascii = "ply\nformat ascii 1.0\nelement vertex 4\nproperty float x\nproperty float y\nproperty float z\nelement face 4\nproperty list uchar int vertex_indices\nproperty list uchar float texcoord\nend_header\n0 0 0\n10 0 0\n0 10 0\n0 0 10\n3 0 2 1 3 0.5 0.5 0.5\n3 0 1 3 3 0.1 0.2 0.3\n3 0 3 2 3 0 0 0\n3 1 2 3 3 1 1 1\n";
        assert_eq!(read(ascii.as_bytes()).expect("reads").triangles.len(), 4);

        let mut binary = b"ply\nformat binary_little_endian 1.0\nelement vertex 4\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar int vertex_indices\nproperty list uchar float texcoord\nend_header\n".to_vec();
        for v in [[0.0f32, 0.0, 0.0], [10.0, 0.0, 0.0], [0.0, 10.0, 0.0], [0.0, 0.0, 10.0]] {
            for c in v {
                binary.extend_from_slice(&c.to_le_bytes());
            }
        }
        binary.push(3u8);
        for i in [0i32, 2, 1] {
            binary.extend_from_slice(&i.to_le_bytes());
        }
        binary.push(3u8);
        for t in [0.5f32, 0.5, 0.5] {
            binary.extend_from_slice(&t.to_le_bytes());
        }
        assert_eq!(read(&binary).expect("reads").triangles.len(), 1);
    }

    #[test]
    fn a_crafted_list_count_cannot_wrap_the_record_size() {
        // `property list uint int vertex_indices` with count 0xFFFF_FFFF: the count times the
        // item size must be checked, not wrapped — on wasm32 `usize` is 32 bit, so
        // 0x4000_0000 * 4 wraps to 0, the record looks complete, and the slices past it panic
        // (trap). The count also exceeds the whole file, so it is refused as a bad number on
        // every target.
        let mut binary = b"ply\nformat binary_little_endian 1.0\nelement vertex 1\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uint int vertex_indices\nend_header\n".to_vec();
        for c in [0.0f32, 0.0, 0.0] {
            binary.extend_from_slice(&c.to_le_bytes());
        }
        binary.extend_from_slice(&u32::MAX.to_le_bytes());
        let err = read(&binary).expect_err("an error");
        assert_eq!(err, MeshError::PlyBody { record: 2, reason: PlyReason::BadNumber });
    }

    #[test]
    fn a_file_without_faces_is_empty() {
        let text = "ply\nformat ascii 1.0\nelement vertex 1\nproperty float x\nproperty float y\nproperty float z\nelement face 0\nproperty list uchar int vertex_indices\nend_header\n0 0 0\n";
        assert_eq!(read(text.as_bytes()).expect_err("an error"), MeshError::Empty);
    }

    #[test]
    fn non_finite_and_out_of_range_corners_are_refused_like_stl() {
        let nan = format!("{HEADER}0 0 nan\n10 0 0\n0 10 0\n0 0 10\n3 0 1 2\n");
        assert_eq!(read(nan.as_bytes()).expect_err("an error"), MeshError::NonFinite { triangle: 1 });
        let big = format!("{HEADER}0 0 0\n1e39 0 0\n0 10 0\n0 0 10\n3 0 1 2\n");
        assert_eq!(read(big.as_bytes()).expect_err("an error"), MeshError::OutOfRange { triangle: 1 });
    }

    #[test]
    fn reasons_are_localised() {
        let err = MeshError::PlyBody { record: 7, reason: PlyReason::IndexOutOfRange(9) };
        assert_eq!(err.to_string(), "PLY record 7: vertex index 9 is out of range");
        assert!(err.message(Locale::De).contains("PLY-Datensatz 7"), "{}", err.message(Locale::De));
        assert!(!err.message(Locale::De).contains('{'));
    }

    /// A binary tetrahedron whose vertex coordinates are written as `coords` (one PLY type per
    /// axis) and whose face list as `list` (count type, index type).
    fn binary_tetra(coords: [&str; 3], list: [&str; 2]) -> Vec<u8> {
        let mut bytes = format!(
            "ply\nformat binary_little_endian 1.0\nelement vertex 4\nproperty {} x\nproperty {} y\nproperty {} z\nelement face 4\nproperty list {} {} vertex_indices\nend_header\n",
            coords[0], coords[1], coords[2], list[0], list[1]
        )
        .into_bytes();
        let write = |bytes: &mut Vec<u8>, ty: &str, v: u8| match ty {
            "char" | "uchar" => bytes.push(v),
            "short" | "ushort" => bytes.extend_from_slice(&u16::from(v).to_le_bytes()),
            "int" | "uint" => bytes.extend_from_slice(&u32::from(v).to_le_bytes()),
            "double" => bytes.extend_from_slice(&f64::from(v).to_le_bytes()),
            _ => panic!("no writer for {ty}"),
        };
        for corner in [[0, 0, 0], [10, 0, 0], [0, 10, 0], [0, 0, 10]] {
            for (ty, v) in coords.iter().zip(corner) {
                write(&mut bytes, ty, v);
            }
        }
        for face in [[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]] {
            write(&mut bytes, list[0], 3);
            for i in face {
                write(&mut bytes, list[1], i);
            }
        }
        bytes
    }

    #[test]
    fn reads_every_binary_integer_and_double_type() {
        for (coords, list) in [
            (["double", "short", "char"], ["char", "ushort"]),
            (["ushort", "uint", "uchar"], ["short", "uint"]),
            (["int", "double", "ushort"], ["ushort", "char"]),
        ] {
            let mesh = read(&binary_tetra(coords, list)).unwrap_or_else(|e| panic!("{coords:?} {list:?}: {e}"));
            assert_eq!(mesh.triangles.len(), 4, "{coords:?} {list:?}");
            assert!((mesh.volume().abs() - 1000.0 / 6.0).abs() < 1e-9, "{coords:?} {list:?}: {}", mesh.volume());
        }
    }

    #[test]
    fn the_last_record_needs_no_newline() {
        let text = format!("{HEADER}{}", TETRA_BODY.trim_end());
        assert_eq!(read(text.as_bytes()).expect("reads").triangles.len(), 4);
    }

    #[test]
    fn lines_longer_than_the_limit_are_refused_in_header_and_body() {
        // A header or ASCII body line is held until it ends; past `LINE_MAX` bytes it is refused,
        // so a crafted file cannot grow the pending buffer without bound.
        let mut header = b"ply\nformat ascii 1.0\ncomment ".to_vec();
        header.resize(header.len() + LINE_MAX + 1, b'x');
        let err = read(&header).expect_err("too long");
        assert_eq!(err, MeshError::PlyHeader { line: 3, reason: PlyReason::LineTooLong });

        let mut body = HEADER.as_bytes().to_vec();
        body.resize(body.len() + LINE_MAX + 1, b'0');
        let err = read(&body).expect_err("too long");
        assert_eq!(err, MeshError::PlyBody { record: 1, reason: PlyReason::LineTooLong });
    }
}
