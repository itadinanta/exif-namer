use std::fs::File;
use std::io::BufReader;

use exif::{Exif, Reader};

const CR2: &str = "tests/data/exif.cr2";
const CR3: &str = "tests/data/exif.cr3";

fn read_container(path: &str) -> Result<Exif, exif::Error> {
	let file = File::open(path).expect("fixture present");
	let mut reader = BufReader::new(file);
	Reader::new().read_from_container(&mut reader)
}

fn display(exif: &Exif, tag: exif::Tag) -> Option<String> {
	exif.fields().find(|f| f.tag == tag).map(|f| f.display_value().to_string())
}

/// CR2 is TIFF-based, so the upstream container sniffing handles it.
#[test]
fn cr2_yields_datetime_original() {
	let exif = read_container(CR2).expect("CR2 parses");
	assert_eq!(display(&exif, exif::Tag::DateTimeOriginal).as_deref(), Some("2024-08-15 01:07:58"));
}

#[test]
fn cr2_yields_camera_make_and_model() {
	let exif = read_container(CR2).expect("CR2 parses");
	let make = display(&exif, exif::Tag::Make).unwrap_or_default();
	let model = display(&exif, exif::Tag::Model).unwrap_or_default();
	assert_eq!(make.trim_matches('"'), "Canon");
	assert_eq!(model.trim_matches('"'), "Canon EOS M");
}

/// Guards the fixture: the naming logic keys off the field set, so a large
/// change in parsed field count is worth noticing.
#[test]
fn cr2_field_count_is_in_expected_range() {
	let exif = read_container(CR2).expect("CR2 parses");
	assert!(exif.fields().count() > 60, "unexpectedly few fields");
}

/// Documents the remaining gap: CR3 is an ISOBMFF container with the EXIF data
/// split across separate `CMT*` boxes, which upstream rejects outright.
/// Invert or replace this test once CR3 support lands.
#[test]
fn cr3_is_not_supported_by_upstream() {
	match read_container(CR3) {
		Ok(_) => panic!("CR3 parsed; upstream support landed, update this test"),
		Err(e) => eprintln!("CR3 error as of kamadak-exif 0.6.2: {e}"),
	}
}
