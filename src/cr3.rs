//! Experimental Canon CR3 support on top of the public `kamadak-exif` API.
//!
//! CR3 is an ISOBMFF container; the EXIF payload lives in `CMT1`..`CMT4`
//! boxes nested under a Canon-UUID box inside `moov`. Measured on
//! `tests/data/exif.cr3`: each `CMT*` body is a self-contained little-endian
//! TIFF (own `II*\0` header, IFD0 at +8) and every internal offset stays
//! inside its own body, so each body can be handed to `Reader::read_raw`
//! directly and the fields merged afterwards.
//!
//! `read_raw` still parses each box's IFD0 with `Context::Tiff`, which
//! misnames the EXIF-style tags in `CMT2`+ (e.g. `Tag(Tiff, 36867)` instead
//! of `DateTimeOriginal`). The fork this replaces fixed that by parsing each
//! segment with a default context (`index 0 -> Tiff`, the rest -> `Exif`);
//! here the equivalent is a post-hoc rewrite of `Tag`'s context, since the
//! tag is a public tuple struct.

use std::io::{BufRead, Seek, SeekFrom};

use exif::{Context, Error, Field, Reader, Tag};

/// Ext-endian-uuid of the Canon CR3 container extension box.
const CANON_UUID: [u8; 16] = [
	0x85, 0xc0, 0xb6, 0x87, 0x82, 0x0f, 0x11, 0xe0, 0x81, 0x11, 0xf4, 0xce, 0x46, 0x2b, 0x6a, 0x48,
];

/// True if the reader starts with an ISOBMFF `ftyp` box branded `crx `.
/// Leaves the reader at the start of the file either way.
pub fn is_cr3<R: BufRead + Seek>(reader: &mut R) -> bool {
	let mut head = [0u8; 12];
	if reader.read_exact(&mut head).is_err() {
		let _ = reader.rewind();
		return false;
	}
	let _ = reader.rewind();
	head[4..8] == *b"ftyp" && head[8..12] == *b"crx "
}

/// Parses the EXIF fields of a CR3 file into a flat field list, ready to be
/// iterated like `Exif::fields()`.
pub fn read_fields<R: BufRead + Seek>(reader: &mut R) -> Result<Vec<Field>, Error> {
	let end = reader.seek(SeekFrom::End(0))?;
	reader.rewind()?;
	let mut bodies = Vec::new();
	walk_boxes(reader, 0, end, false, &mut bodies)?;

	let mut fields = Vec::new();
	for (index, body) in bodies.into_iter().enumerate() {
		for field in Reader::new().read_raw(body)?.fields() {
			let mut field = field.clone();
			if index > 0 && field.tag.0 == Context::Tiff {
				field.tag = Tag(Context::Exif, field.tag.1);
			}
			fields.push(field);
		}
	}
	if fields.is_empty() {
		return Err(Error::InvalidFormat("no CMT* boxes found in CR3 file"));
	}
	Ok(fields)
}

/// Scans ISOBMFF boxes in `[start, end)`, collecting `CMT*` bodies that sit
/// below the Canon-UUID box (`in_canon_uuid`), recursing only into the
/// containers that can legally hold them (`moov`, the Canon UUID box).
fn walk_boxes<R: BufRead + Seek>(
	reader: &mut R,
	start: u64,
	end: u64,
	in_canon_uuid: bool,
	out: &mut Vec<Vec<u8>>,
) -> Result<(), Error> {
	let mut pos = start;
	while pos + 8 <= end {
		reader.seek(SeekFrom::Start(pos))?;
		let mut head = [0u8; 16];
		reader.read_exact(&mut head[..8])?;
		let (size, body_at) = match u32::from_be_bytes(head[..4].try_into().unwrap()) {
			0 => (end - pos, 8),
			1 => {
				reader.read_exact(&mut head[8..16])?;
				(u64::from_be_bytes(head[8..16].try_into().unwrap()), 16)
			}
			n => (u64::from(n), 8),
		};
		// A box smaller than its own header, or running past the end we were
		// given, means the tail is not a box list: stop scanning here.
		if size < body_at || pos + size > end {
			break;
		}
		let body = pos + body_at;
		let body_end = pos + size;
		match &head[4..8] {
			b"moov" => walk_boxes(reader, body, body_end, false, out)?,
			uuid if uuid == b"uuid" => {
				let mut ext = [0u8; 16];
				if body + 16 <= body_end && reader.read_exact(&mut ext).is_ok() && ext == CANON_UUID {
					walk_boxes(reader, body + 16, body_end, true, out)?;
				}
			}
			cmt if in_canon_uuid && cmt.starts_with(b"CMT") => {
				let mut buf = vec![0u8; (body_end - body) as usize];
				reader.seek(SeekFrom::Start(body))?;
				reader.read_exact(&mut buf)?;
				out.push(buf);
			}
			_ => {}
		}
		pos += size;
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::fs::File;

	fn fixture(name: &str) -> std::io::BufReader<File> {
		let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
			.join("tests/data")
			.join(name);
		std::io::BufReader::new(File::open(path).expect("fixture present"))
	}

	#[test]
	fn only_cr3_is_detected() {
		assert!(is_cr3(&mut fixture("exif.cr3")));
		assert!(!is_cr3(&mut fixture("exif.cr2")));
	}

	#[test]
	fn cr3_yields_datetime_original_in_exif_context() {
		let fields = read_fields(&mut fixture("exif.cr3")).expect("CR3 parses");
		let found = fields
			.iter()
			.find(|f| f.tag == exif::Tag::DateTimeOriginal)
			.expect("DateTimeOriginal present");
		assert_eq!(found.display_value().to_string(), "2024-06-17 23:38:31");
	}

	#[test]
	fn cr3_camera_identity_stays_in_tiff_context() {
		let fields = read_fields(&mut fixture("exif.cr3")).expect("CR3 parses");
		let make = fields.iter().find(|f| f.tag == exif::Tag::Make).expect("Make present");
		assert_eq!(make.tag.0, Context::Tiff);
		assert_eq!(make.display_value().to_string(), "\"Canon\"");
	}

	#[test]
	fn cr3_field_count_matches_fork() {
		// The exif-rs fork parsed this fixture into 100 fields; the public-API
		// route must not silently lose or duplicate any of them.
		let fields = read_fields(&mut fixture("exif.cr3")).expect("CR3 parses");
		assert_eq!(fields.len(), 100);
	}
}
