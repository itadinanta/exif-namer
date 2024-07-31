use chrono::NaiveDateTime;
use chrono::{DateTime, Utc};
use clap::Parser;
use glob::*;
use log::*;
use log4rs::append::console::ConsoleAppender;
use std::collections::{BTreeMap, HashMap};
use std::ffi::OsStr;
use std::fmt::Display;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::time::UNIX_EPOCH;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
	#[arg(short, long, default_value = "**/*.ARW")]
	glob: String,

	#[arg(short, long, default_value = "%Y%m%d_%H%M%S")]
	date_time_format: String,

	#[arg(short, long, default_value = "%p/%n.%e")]
	name_pattern: String,
}

struct App {
	args: Args,
}

#[derive(Clone, Debug)]
enum ExifAttr {
	Text(String),
	Path(PathBuf),
	Timestamp(NaiveDateTime),
	Integer(i64),
	Fraction(i64, i64),
	Real(f64),
	Nothing,
}

trait Pair<I> {
	fn as_pair(&self) -> (I, I);
}

impl Pair<u32> for exif::Rational {
	fn as_pair(&self) -> (u32, u32) { (self.num, self.denom) }
}

impl Pair<i32> for exif::SRational {
	fn as_pair(&self) -> (i32, i32) { (self.num, self.denom) }
}

impl ExifAttr {
	fn from_opt_str(from: Option<&str>) -> Self {
		match from {
			Some(word) => ExifAttr::Text(String::from(word)),
			None => ExifAttr::Nothing,
		}
	}

	fn from_opt_str_datetime(from: Option<&str>) -> Self {
		match from {
			Some(word) => match chrono::NaiveDateTime::parse_from_str(word, "%Y:%m:%d %H:%M:%S") {
				Ok(dt) => ExifAttr::Timestamp(dt),
				Err(e) => {
					warn!("Unable to parse '{}' as date: {:?}", word, e);
					ExifAttr::Text(String::from(word))
				}
			},
			None => ExifAttr::Nothing,
		}
	}

	fn from_opt_path(from: Option<&Path>) -> Self {
		match from {
			Some(dir) => ExifAttr::Path(PathBuf::from(dir)),
			None => ExifAttr::Nothing,
		}
	}

	fn from_opt_integer<T>(from: Option<&T>) -> Self
	where T: Into<i64> + Copy {
		match from {
			Some(n) => ExifAttr::Integer((*n).into()),
			None => ExifAttr::Nothing,
		}
	}

	fn from_opt_real<T>(from: Option<&T>) -> Self
	where T: Into<f64> + Copy {
		match from {
			Some(v) => ExifAttr::Real((*v).into()),
			None => ExifAttr::Nothing,
		}
	}

	fn from_opt_rational<T, U>(from: Option<&T>) -> Self
	where
		T: Pair<U>,
		U: Into<i64> + Copy, {
		match from {
			Some(r) => {
				let (n, d) = r.as_pair();
				ExifAttr::Fraction(n.into(), d.into())
			}
			None => ExifAttr::Nothing,
		}
	}

	fn from_opt_filetime(from: Option<std::time::SystemTime>) -> ExifAttr {
		match from
			.and_then(|t| t.duration_since(UNIX_EPOCH).ok())
			.and_then(|d| DateTime::from_timestamp(d.as_secs() as i64, d.subsec_nanos()))
			.map(|dt| dt.naive_utc())
		{
			Some(t) => ExifAttr::Timestamp(t),
			None => ExifAttr::Nothing,
		}
	}
}

impl App {
	pub fn find_matches(&self) -> Result<Vec<PathBuf>, PatternError> {
		let mut out = Vec::new();
		for iter in glob::glob(&self.args.glob)? {
			match iter {
				Ok(path) => out.push(path),
				Err(e) => error!("{:?}", e),
			}
		}
		return Ok(out);
	}

	pub fn extract_props(&self, path: &PathBuf) -> BTreeMap<String, ExifAttr> {
		let mut out = BTreeMap::new();
		out.insert("fs.Ext".to_owned(), ExifAttr::from_opt_str(path.extension().map(OsStr::to_str).flatten()));
		out.insert("fs.Name".to_owned(), ExifAttr::from_opt_str(path.file_stem().map(OsStr::to_str).flatten()));
		out.insert("fs.FullName".to_owned(), ExifAttr::from_opt_str(path.file_name().map(OsStr::to_str).flatten()));
		out.insert("fs.Path".to_owned(), ExifAttr::from_opt_path(path.parent()));

		match fs::metadata(path) {
			Ok(metadata) => {
				out.insert("fs.DateTimeCreated".to_owned(), ExifAttr::from_opt_filetime(metadata.created().ok()));
				out.insert("fs.DateTimeModified".to_owned(), ExifAttr::from_opt_filetime(metadata.modified().ok()));
				out.insert("fs.DateTimeAccessed".to_owned(), ExifAttr::from_opt_filetime(metadata.accessed().ok()));
				out.insert("fs.FileSize".to_owned(), ExifAttr::Integer(metadata.len() as i64));
			}
			Err(err) => error!("Unable to read metadata for {:?}: {:?}", path, err),
		}
		let exif_file = fs::File::open(path);
		match exif_file {
			Ok(file) => {
				let mut bufreader = io::BufReader::new(&file);
				let exifreader = exif::Reader::new();
				if let Ok(exif) = exifreader.read_from_container(&mut bufreader) {
					for f in exif.fields() {
						info!(
							"{:30} {:50} {:10} {:.50}",
							f.tag,
							f.tag.description().unwrap_or(""),
							f.ifd_num,
							f.display_value().with_unit(&exif).to_string()
						);
						let value = match f.value {
							exif::Value::Byte(ref n) => ExifAttr::from_opt_integer(n.first()),
							exif::Value::Ascii(ref text) => {
								let src = text.first().map(|v| std::str::from_utf8(&*v)).and_then(Result::ok);
								match f.tag {
									exif::Tag::DateTime
									| exif::Tag::DateTimeOriginal
									| exif::Tag::DateTimeDigitized => ExifAttr::from_opt_str_datetime(src),
									_ => ExifAttr::from_opt_str(src),
								}
							}
							exif::Value::Short(ref n) => ExifAttr::from_opt_integer(n.first()),
							exif::Value::Long(ref n) => ExifAttr::from_opt_integer(n.first()),
							exif::Value::Rational(ref r) => ExifAttr::from_opt_rational(r.first()),
							exif::Value::SByte(ref n) => ExifAttr::from_opt_integer(n.first()),
							exif::Value::Undefined(_, _) => ExifAttr::Text(f.display_value().to_string()),
							exif::Value::SShort(ref n) => ExifAttr::from_opt_integer(n.first()),
							exif::Value::SLong(ref n) => ExifAttr::from_opt_integer(n.first()),
							exif::Value::SRational(ref r) => ExifAttr::from_opt_rational(r.first()),
							exif::Value::Float(ref v) => ExifAttr::from_opt_real(v.first()),
							exif::Value::Double(ref v) => ExifAttr::from_opt_real(v.first()),
							exif::Value::Unknown(_, _, _) => ExifAttr::Nothing,
						};
						out.insert(format!("{}.{}", f.ifd_num, f.tag), value);
					}
				}
			}
			Err(err) => error!("Unable to read EXIF from {:?}: {:?}", path, err),
		}
		out
	}
}

fn main() {
	use log4rs::config::*;

	let log_appender = Appender::builder().build("stdout".to_string(), Box::new(ConsoleAppender::builder().build()));
	let log_root = Root::builder().appender("stdout".to_string()).build(LevelFilter::Info);
	let log_config = Config::builder().appender(log_appender).build(log_root);

	init_config(log_config.expect("Invalid log configuration")).expect("Unable to initialize log4rs");

	let app = App { args: Args::parse() };

	info!("Matching>: {}", &app.args.glob);

	let paths = app.find_matches().expect("Error extracting source files");

	for path in &paths {
		info!("{:?}", &path);
		info!("{:?}", app.extract_props(path));
	}
}
