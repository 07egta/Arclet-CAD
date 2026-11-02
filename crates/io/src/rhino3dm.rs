//! Rhino `.3dm` header and top-level chunk listing, written from the public description of the
//! file layout: a 32-byte ASCII header (`3D Geometry File Format ` followed by an 8-byte decimal
//! version field), then records of a 4-byte little-endian type code and, for ordinary records, a
//! 4-byte little-endian length and that many body bytes.
//!
//! Staged: this module reads the header, the version and the chunk type codes only. It decodes no
//! geometry and does not inflate compressed sections. [`read_geometry`] validates the file and then
//! returns [`Error::Unsupported`]. Rhino files that the workspace imports as geometry go through
//! cadmpeg instead (see `native_cad`).
//!
//! Every read is bounds-checked, every length is checked against the bytes that remain, and the
//! walk stops at [`MAX_CHUNKS`] records, so hostile input yields an [`Error`] and never a panic.

use std::fmt;

/// The 24-byte signature at the start of every `.3dm` file.
pub const SIGNATURE: &[u8] = b"3D Geometry File Format ";
/// Length of the header: the signature and the 8-byte version field.
pub const HEADER_LEN: usize = 32;
/// Largest `.3dm` file this module reads (the same cap as native designs).
pub const MAX_3DM_BYTES: usize = 256 << 20;
/// Most top-level records one listing reports before it refuses the file.
pub const MAX_CHUNKS: usize = 65_536;

/// A record whose type code has this bit set carries no length field and no body.
const SHORT_RECORD_FLAG: u32 = 0x8000_0000;

/// Why a `.3dm` file could not be listed or read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The file does not start with the `3D Geometry File Format ` signature.
    #[error("not a Rhino 3DM file (no `3D Geometry File Format` header)")]
    NotRhino,
    /// The file ends inside the header or inside a record header.
    #[error("3DM file is truncated at byte {at}")]
    Truncated { at: usize },
    /// A field is malformed, or a record claims more bytes than the file holds.
    #[error("3DM file is invalid: {0}")]
    Invalid(String),
    /// The file is larger than the size limit.
    #[error("3DM file is {len} bytes, over the {limit}-byte limit")]
    TooLarge { len: usize, limit: usize },
    /// The file is well formed as far as this module reads, but the content is not decoded yet.
    #[error("{0}")]
    Unsupported(String),
}

/// One top-level record of a `.3dm` file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chunk {
    /// Byte offset of the record's type code.
    pub offset: usize,
    /// The record's type code as stored.
    pub typecode: u32,
    /// Length of the body in bytes; zero for a short record.
    pub body_len: usize,
}

/// The header and top-level records of a `.3dm` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listing {
    /// The version from the header's version field.
    pub version: u32,
    /// The top-level records, in file order.
    pub chunks: Vec<Chunk>,
}

impl Listing {
    /// The type codes in file order, for display.
    pub fn typecodes(&self) -> Vec<u32> {
        self.chunks.iter().map(|c| c.typecode).collect()
    }
}

impl fmt::Display for Listing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Rhino 3DM version {}, {} top-level records", self.version, self.chunks.len())?;
        for c in &self.chunks {
            writeln!(f, "  {:#010x} at byte {} ({} body bytes)", c.typecode, c.offset, c.body_len)?;
        }
        Ok(())
    }
}

/// Does the data start with the `.3dm` signature? A short or empty buffer answers `false`.
pub fn is_3dm(bytes: &[u8]) -> bool {
    bytes.starts_with(SIGNATURE)
}

/// Read the version from the header of a `.3dm` file, without walking the records.
pub fn read_version(bytes: &[u8]) -> Result<u32, Error> {
    check_header(bytes)
}

/// List the header version and the top-level record type codes of a `.3dm` file.
pub fn list_chunks(bytes: &[u8]) -> Result<Listing, Error> {
    list_chunks_limited(bytes, MAX_3DM_BYTES)
}

/// Validate a `.3dm` file and then report that its geometry is not decoded yet.
///
/// Always returns `Err`: [`Error::Unsupported`] for a well-formed file, or the structural error
/// from [`list_chunks`] for a malformed one. Callers that import geometry should use the cadmpeg
/// path instead.
pub fn read_geometry(bytes: &[u8]) -> Result<Listing, Error> {
    let listing = list_chunks(bytes)?;
    Err(Error::Unsupported(format!(
        "Rhino 3DM version {} geometry is not decoded yet: {} top-level records were listed, no bodies were read",
        listing.version,
        listing.chunks.len()
    )))
}

fn list_chunks_limited(bytes: &[u8], limit: usize) -> Result<Listing, Error> {
    if bytes.len() > limit {
        return Err(Error::TooLarge { len: bytes.len(), limit });
    }
    let version = check_header(bytes)?;
    let mut chunks = Vec::new();
    let mut at = HEADER_LEN;
    while at < bytes.len() {
        if chunks.len() >= MAX_CHUNKS {
            return Err(Error::Invalid(format!("more than {MAX_CHUNKS} top-level records")));
        }
        let typecode = read_u32(bytes, at)?;
        if typecode & SHORT_RECORD_FLAG != 0 {
            chunks.push(Chunk { offset: at, typecode, body_len: 0 });
            at = offset_add(at, 4)?;
            continue;
        }
        let len_at = offset_add(at, 4)?;
        let body_len = read_u32(bytes, len_at)? as usize;
        let body_start = offset_add(at, 8)?;
        let body_end = body_start.checked_add(body_len).ok_or_else(|| Error::Invalid(format!("record at byte {at} has an oversized length")))?;
        if body_end > bytes.len() {
            return Err(Error::Invalid(format!(
                "record {typecode:#010x} at byte {at} claims {body_len} body bytes but only {} remain",
                bytes.len() - body_start
            )));
        }
        chunks.push(Chunk { offset: at, typecode, body_len });
        at = body_end;
    }
    Ok(Listing { version, chunks })
}

fn check_header(bytes: &[u8]) -> Result<u32, Error> {
    if !is_3dm(bytes) {
        if bytes.len() < SIGNATURE.len() && SIGNATURE.starts_with(bytes) {
            return Err(Error::Truncated { at: bytes.len() });
        }
        return Err(Error::NotRhino);
    }
    let field = bytes.get(SIGNATURE.len()..HEADER_LEN).ok_or(Error::Truncated { at: bytes.len() })?;
    let text = std::str::from_utf8(field).map_err(|_| Error::Invalid("version field is not ASCII".into()))?;
    let digits = text.trim_matches(|c: char| c == ' ' || c == '\0');
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error::Invalid(format!("version field `{}` is not a decimal number", text.escape_debug())));
    }
    digits.parse::<u32>().map_err(|_| Error::Invalid("version number is out of range".into()))
}

fn read_u32(bytes: &[u8], at: usize) -> Result<u32, Error> {
    let end = offset_add(at, 4)?;
    let field = bytes.get(at..end).ok_or(Error::Truncated { at: bytes.len() })?;
    let mut word = [0u8; 4];
    word.copy_from_slice(field);
    Ok(u32::from_le_bytes(word))
}

fn offset_add(at: usize, n: usize) -> Result<usize, Error> {
    at.checked_add(n).ok_or_else(|| Error::Invalid("offset overflow".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(version: &str) -> Vec<u8> {
        let mut out = SIGNATURE.to_vec();
        out.extend_from_slice(format!("{version:>8}").as_bytes());
        out
    }

    fn record(typecode: u32, body: &[u8]) -> Vec<u8> {
        let mut out = typecode.to_le_bytes().to_vec();
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.extend_from_slice(body);
        out
    }

    fn short(typecode: u32) -> Vec<u8> {
        (typecode | SHORT_RECORD_FLAG).to_le_bytes().to_vec()
    }

    fn file(version: &str, records: &[Vec<u8>]) -> Vec<u8> {
        let mut out = header(version);
        for r in records {
            out.extend_from_slice(r);
        }
        out
    }

    #[test]
    fn header_only_file_reports_version_and_no_records() -> Result<(), Error> {
        let bytes = header("       7");
        assert!(is_3dm(&bytes));
        assert_eq!(read_version(&bytes)?, 7);
        let listing = list_chunks(&bytes)?;
        assert_eq!(listing.version, 7);
        assert!(listing.chunks.is_empty());
        Ok(())
    }

    #[test]
    fn lists_ordinary_and_short_records_in_order() -> Result<(), Error> {
        let bytes = file("50", &[record(0x0000_0001, b"abc"), short(0x0000_0010), record(0x0000_0002, &[])]);
        let listing = list_chunks(&bytes)?;
        assert_eq!(listing.version, 50);
        assert_eq!(listing.typecodes(), vec![0x1, 0x8000_0010, 0x2]);
        let lens: Vec<usize> = listing.chunks.iter().map(|c| c.body_len).collect();
        assert_eq!(lens, vec![3, 0, 0]);
        assert_eq!(listing.chunks[0].offset, HEADER_LEN);
        assert_eq!(listing.chunks[1].offset, HEADER_LEN + 8 + 3);
        Ok(())
    }

    #[test]
    fn geometry_is_reported_unsupported_not_decoded() {
        let bytes = file("8", &[record(0x0000_0001, b"geometry-bytes")]);
        let result = read_geometry(&bytes);
        assert!(matches!(&result, Err(Error::Unsupported(msg)) if msg.contains("not decoded yet")), "{result:?}");
    }

    #[test]
    fn non_rhino_input_is_refused() {
        assert_eq!(list_chunks(b"hello world, not a model"), Err(Error::NotRhino));
        assert_eq!(list_chunks(b""), Err(Error::NotRhino));
        assert!(!is_3dm(b""));
    }

    #[test]
    fn truncated_header_is_refused() {
        let full = header("8");
        for n in 0..HEADER_LEN {
            let r = list_chunks(&full[..n]);
            assert!(r.is_err(), "prefix of {n} bytes must not list");
        }
        assert_eq!(list_chunks(&full[..SIGNATURE.len() + 3]), Err(Error::Truncated { at: SIGNATURE.len() + 3 }));
    }

    #[test]
    fn bad_version_field_is_invalid() {
        for v in ["       x", "        ", "99999999999"] {
            let bytes = header(v);
            assert!(matches!(list_chunks(&bytes), Err(Error::Invalid(_))), "version field {v:?}");
        }
    }

    #[test]
    fn record_header_cut_short_is_truncated() {
        let mut bytes = file("8", &[]);
        bytes.extend_from_slice(&[1, 0, 0]);
        assert_eq!(list_chunks(&bytes), Err(Error::Truncated { at: bytes.len() }));
        let mut bytes = file("8", &[]);
        bytes.extend_from_slice(&[1, 0, 0, 0, 9, 0]);
        assert_eq!(list_chunks(&bytes), Err(Error::Truncated { at: bytes.len() }));
    }

    #[test]
    fn body_longer_than_the_file_is_invalid() {
        let mut bytes = file("8", &[]);
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&5u32.to_le_bytes());
        bytes.extend_from_slice(b"abcd");
        assert!(matches!(list_chunks(&bytes), Err(Error::Invalid(_))));
    }

    #[test]
    fn huge_length_is_invalid_not_a_crash() {
        let mut bytes = file("8", &[]);
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(list_chunks(&bytes), Err(Error::Invalid(_))));
        assert!(matches!(read_geometry(&bytes), Err(Error::Invalid(_))));
    }

    #[test]
    fn length_one_past_the_end_is_invalid() {
        let mut bytes = file("8", &[]);
        bytes.extend_from_slice(&2u32.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.push(0xAA);
        assert!(matches!(list_chunks(&bytes), Err(Error::Invalid(_))));
    }

    #[test]
    fn size_limit_refuses_before_walking() {
        let bytes = file("8", &[record(1, b"x")]);
        assert_eq!(list_chunks_limited(&bytes, 16), Err(Error::TooLarge { len: bytes.len(), limit: 16 }));
        assert!(list_chunks_limited(&bytes, bytes.len()).is_ok());
    }

    #[test]
    fn too_many_records_are_refused() -> Result<(), Error> {
        let records: Vec<Vec<u8>> = (0..=MAX_CHUNKS).map(|_| short(0x10)).collect();
        let bytes = file("8", &records);
        assert!(matches!(list_chunks(&bytes), Err(Error::Invalid(_))));
        let ok = file("8", &records[..MAX_CHUNKS]);
        assert_eq!(list_chunks(&ok)?.chunks.len(), MAX_CHUNKS);
        Ok(())
    }

    #[test]
    fn every_truncation_of_a_valid_file_is_handled_without_panic() {
        let bytes = file("8", &[record(1, b"hello"), short(0x20), record(3, b"0123456789")]);
        assert!(list_chunks(&bytes).is_ok());
        for n in 0..bytes.len() {
            let _ = list_chunks(&bytes[..n]);
            let _ = read_geometry(&bytes[..n]);
        }
    }
}
