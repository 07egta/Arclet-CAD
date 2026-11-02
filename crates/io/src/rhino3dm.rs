//! Rhino `.3dm` header and top-level chunk listing, written from the public description of the
//! file layout: a 32-byte ASCII header (`3D Geometry File Format ` followed by an 8-byte decimal
//! version field), then records of a 4-byte little-endian type code and a value field:
//! 4 bytes in archive versions below 50, 8 bytes in versions 50 and newer. In a normal
//! record that value is the body length; in a TCODE_SHORT record it is an inline value,
//! and no body follows. This is a structural listing, not a geometry decoder.
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

/// A record whose type code has this bit set carries an inline value instead of body bytes.
/// Its value field is still present (4 bytes before archive 50, 8 bytes thereafter).
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
    /// Raw length or inline value from the chunk header. For short chunks this is a value,
    /// not a byte count (preserved for diagnostics and future decoding).
    pub value: u64,
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
            if c.typecode & SHORT_RECORD_FLAG != 0 {
                writeln!(f, "  {:#010x} at byte {} (inline value {})", c.typecode, c.offset, c.value)?;
            } else {
                writeln!(f, "  {:#010x} at byte {} ({} body bytes)", c.typecode, c.offset, c.body_len)?;
            }
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
    // File-format versions 1..=5 use 32-bit chunk values. Version 50 and newer
    // use 64-bit values even for short records (see McNeel ON_BinaryArchive).
    let wide = version >= 50;
    let value_bytes = if wide { 8 } else { 4 };
    let mut chunks = Vec::new();
    let mut at = HEADER_LEN;
    while at < bytes.len() {
        if chunks.len() >= MAX_CHUNKS {
            return Err(Error::Invalid(format!("more than {MAX_CHUNKS} top-level records")));
        }
        let typecode = read_u32(bytes, at)?;
        let value_at = offset_add(at, 4)?;
        let value = read_chunk_value(bytes, value_at, wide)?;
        let body_start = offset_add(value_at, value_bytes)?;
        if typecode & SHORT_RECORD_FLAG != 0 {
            chunks.push(Chunk { offset: at, typecode, value, body_len: 0 });
            at = body_start;
            continue;
        }
        let body_len = usize::try_from(value).map_err(|_| Error::Invalid(format!("record at byte {at} has an oversized length")))?;
        let body_end = body_start.checked_add(body_len).ok_or_else(|| Error::Invalid(format!("record at byte {at} has an oversized length")))?;
        if body_end > bytes.len() {
            return Err(Error::Invalid(format!(
                "record {typecode:#010x} at byte {at} claims {body_len} body bytes but only {} remain",
                bytes.len() - body_start
            )));
        }
        chunks.push(Chunk { offset: at, typecode, value, body_len });
        at = body_end;
    }
    Ok(Listing { version, chunks })
}

fn check_header(bytes: &[u8]) -> Result<u32, Error> {
    if !is_3dm(bytes) {
        if !bytes.is_empty() && bytes.len() < SIGNATURE.len() && SIGNATURE.starts_with(bytes) {
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

/// Read the 32- or 64-bit chunk value without changing the archive's endianness.
/// This field exists for short records too; the high half of the modern form must
/// be included or the following top-level chunk will be read at the wrong offset.
fn read_chunk_value(bytes: &[u8], at: usize, wide: bool) -> Result<u64, Error> {
    let low = u64::from(read_u32(bytes, at)?);
    if !wide {
        return Ok(low);
    }
    let high = u64::from(read_u32(bytes, offset_add(at, 4)?)?);
    Ok(low | (high << 32))
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
        let mut out = (typecode | SHORT_RECORD_FLAG).to_le_bytes().to_vec();
        out.extend_from_slice(&0u32.to_le_bytes());
        out
    }

    fn modern_record(typecode: u32, body: &[u8]) -> Vec<u8> {
        let mut out = typecode.to_le_bytes().to_vec();
        out.extend_from_slice(&(body.len() as u64).to_le_bytes());
        out.extend_from_slice(body);
        out
    }

    fn modern_short(typecode: u32, value: u64) -> Vec<u8> {
        let mut out = (typecode | SHORT_RECORD_FLAG).to_le_bytes().to_vec();
        out.extend_from_slice(&value.to_le_bytes());
        out
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
        let bytes = file("4", &[record(0x0000_0001, b"abc"), short(0x0000_0010), record(0x0000_0002, &[])]);
        let listing = list_chunks(&bytes)?;
        assert_eq!(listing.version, 4);
        assert_eq!(listing.typecodes(), vec![0x1, 0x8000_0010, 0x2]);
        let lens: Vec<usize> = listing.chunks.iter().map(|c| c.body_len).collect();
        assert_eq!(lens, vec![3, 0, 0]);
        assert_eq!(listing.chunks[0].offset, HEADER_LEN);
        assert_eq!(listing.chunks[1].offset, HEADER_LEN + 8 + 3);
        assert_eq!(listing.chunks[2].offset, HEADER_LEN + 8 + 3 + 8);
        Ok(())
    }

    #[test]
    fn modern_64_bit_chunks_and_short_values_advance_by_full_headers() -> Result<(), Error> {
        let bytes = file("70", &[
            modern_record(0x0000_0001, b"abc"),
            modern_short(0x0000_0010, 0x0102_0304_0506_0708),
            modern_record(0x0000_0002, b"x"),
        ]);
        let listed = list_chunks(&bytes)?;
        assert_eq!(listed.version, 70);
        assert_eq!(listed.typecodes(), vec![0x1, 0x8000_0010, 0x2]);
        assert_eq!(listed.chunks[0].offset, HEADER_LEN);
        assert_eq!(listed.chunks[0].value, 3);
        assert_eq!(listed.chunks[1].offset, HEADER_LEN + 12 + 3);
        assert_eq!(listed.chunks[1].value, 0x0102_0304_0506_0708);
        assert_eq!(listed.chunks[1].body_len, 0);
        assert_eq!(listed.chunks[2].offset, HEADER_LEN + 12 + 3 + 12);
        assert_eq!(listed.chunks[2].body_len, 1);
        assert!(format!("{listed}").contains("inline value"));
        Ok(())
    }

    #[test]
    fn real_rhino_7_box_has_exact_64_bit_chunk_boundaries() -> Result<(), Error> {
        // A separately authored 10 × 20 × 30 mm Rhino 7 fixture is already used
        // by native_cad's geometry tests. The independent structural reader must
        // traverse the SAME real file, not just hand-written miniature chunks.
        let source = include_bytes!("native_cad/fixtures/block.3dm");
        let listed = list_chunks(source)?;
        assert_eq!(listed.version, 70);
        assert_eq!(source.len(), 19_497);
        assert_eq!(listed.chunks.len(), 18);
        assert_eq!(listed.chunks.first().map(|c| (c.offset, c.typecode, c.body_len)), Some((32, 1, 124)));
        assert_eq!(listed.chunks.last().map(|c| (c.offset, c.typecode, c.value)), Some((19_477, 0x7fff, 8)));
        assert_eq!(listed.chunks.last().map(|c| c.offset + 12 + c.body_len), Some(source.len()));
        assert!(matches!(read_geometry(source), Err(Error::Unsupported(_))), "listing alone is not geometry decoding");
        Ok(())
    }

    #[test]
    fn malformed_modern_chunk_lengths_and_partial_headers_are_rejected() {
        let mut huge = header("70");
        huge.extend_from_slice(&1u32.to_le_bytes());
        huge.extend_from_slice(&u64::MAX.to_le_bytes());
        assert!(matches!(list_chunks(&huge), Err(Error::Invalid(_))));

        for count in 0..8 {
            let mut partial = header("70");
            partial.extend_from_slice(&1u32.to_le_bytes());
            partial.extend_from_slice(&[0u8; 8][..count]);
            assert_eq!(list_chunks(&partial), Err(Error::Truncated { at: partial.len() }));
        }
        // A complete short record has a value, not a body, even if its numeric
        // inline value would exceed the remainder of the file.
        let good = file("50", &[modern_short(0x0000_0010, u64::MAX)]);
        let listed = list_chunks(&good).expect("short record has no payload");
        assert_eq!(listed.chunks[0].body_len, 0);
        assert_eq!(listed.chunks[0].value, u64::MAX);
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
        for v in ["       x", "        ", "12ab5678"] {
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
        bytes.extend_from_slice(&2u32.to_le_bytes());
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
