use chrono::DateTime;
use chrono::NaiveDateTime;
use clap::Parser;
use exif::In;
use glob::*;
use log::*;
use log4rs::append::console::ConsoleAppender;
use serde_json::value::*;
use sha1::{Digest, Sha1};
use std::collections::BTreeMap;
use std::fmt::Write;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::time::UNIX_EPOCH;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
	#[arg()]
	globs: Vec<String>,

	#[arg(short, long, default_value = "%Y%m%d_%H%M%S")]
	date_time_format: String,

	#[arg(short, long, default_value = "{{SysPath}}/{{SysName}}_{{SysSha1}}.{{SysExt}}")]
	out: String,

	#[arg(short, long, default_value_t = false)]
	verbose: bool,

	#[arg(short='n', long, default_value_t = false)]
	dry_run: bool,

	#[arg(short, long, default_value = "[^\\w\\+\\-]+")]
	sanitize: String,

	#[arg(short, long, default_value = "_")]
	replacement: String,
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
			Some(word) => match NaiveDateTime::parse_from_str(word, "%Y:%m:%d %H:%M:%S") {
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

struct ExifAttrFormatter {
	date_time_format: String,
	sanitize_key_pattern: regex::Regex,
	sanitize_value_pattern: regex::Regex,
	sanitize_replacement: String,
}

impl ExifAttrFormatter {
	fn new(
		date_time_format: String,
		sanitize_pattern: &str,
		sanitize_replacement: String,
	) -> Result<Self, regex::Error> {
		Ok(ExifAttrFormatter {
			date_time_format,
			sanitize_key_pattern: regex::Regex::new("\\W+")?,
			sanitize_value_pattern: regex::Regex::new(sanitize_pattern)?,
			sanitize_replacement,
		})
	}

	fn fmt<W>(&self, value: &ExifAttr, f: &mut W) -> std::fmt::Result
	where W: Write {
		match value {
			// write!(f, "{}", strings)
			ExifAttr::Text(ref text) => f.write_str(text),
			ExifAttr::Path(ref path) => f.write_str(path.to_str().unwrap_or("")),
			ExifAttr::Timestamp(ref timestamp) => f.write_str(&timestamp.format(&self.date_time_format).to_string()),
			ExifAttr::Integer(ref value) => write!(f, "{}", value),
			ExifAttr::Fraction(ref num, ref den) => write!(f, "{}_{}", num, den),
			ExifAttr::Real(ref value) => write!(f, "{}", value),
			ExifAttr::Nothing => Ok(()),
		}
	}

	fn sanitize_value(&self, value: &String) -> String {
		self.sanitize_value_pattern.replace_all(value, &self.sanitize_replacement).to_string()
	}

	pub fn sanitize_key(&self, key: &String) -> String { self.sanitize_key_pattern.replace_all(key, "").to_string() }

	pub fn as_string(&self, value: &ExifAttr) -> String {
		let mut value_as_string = String::new();
		if let Err(e) = self.fmt(&value, &mut value_as_string) {
			error!("Cannot convert {:?} to string: {}", value, e)
		}
		match value {
			ExifAttr::Path(_) => value_as_string,
			_ => self.sanitize_value(&value_as_string),
		}
	}
}

struct App {
	args: Args,
	fmt: ExifAttrFormatter,
}

macro_rules! sys_prefix {
	() => {
		"Sys"
	};
}

macro_rules! with_sys_prefix {
	($name:expr) => {
		concat!(sys_prefix!(), $name).to_owned()
	};
}

impl App {
	fn new(args: Args) -> Result<Self, regex::Error> {
		let fmt = ExifAttrFormatter::new(args.date_time_format.clone(), &args.sanitize, args.replacement.clone())?;

		Ok(App { args, fmt })
	}

	fn find_matches(&self, pattern: &str) -> Result<Vec<PathBuf>, PatternError> {
		let mut out = Vec::new();
		for iter in glob::glob(pattern)? {
			match iter {
				Ok(path) => out.push(path),
				Err(e) => error!("Invalid glob pattern {}: {}", pattern, e),
			}
		}
		return Ok(out);
	}

	fn extract_props(&self, path: &PathBuf) -> BTreeMap<String, ExifAttr> {
		let mut out = BTreeMap::new();
		out.insert(
			with_sys_prefix!("Ext"),
			ExifAttr::from_opt_path(path.extension().map(PathBuf::from).as_ref().map(PathBuf::as_path)),
		);
		out.insert(
			with_sys_prefix!("Name"),
			ExifAttr::from_opt_path(path.file_stem().map(PathBuf::from).as_ref().map(PathBuf::as_path)),
		);
		out.insert(
			with_sys_prefix!("FullName"),
			ExifAttr::from_opt_path(path.file_name().map(PathBuf::from).as_ref().map(PathBuf::as_path)),
		);
		out.insert(with_sys_prefix!("Path"), ExifAttr::from_opt_path(path.parent()));
		let mut r_path = PathBuf::new();
		let components = path.components().collect::<Vec<_>>();
		let n_components = components.len();
		for (i, component) in components.iter().enumerate() {
			out.insert(
				format!(concat!(sys_prefix!(), "PathElem{}"), i),
				ExifAttr::from_opt_path(Some(PathBuf::from(component.as_os_str()).as_path())),
			);
			r_path.push(component);
			out.insert(format!(concat!(sys_prefix!(), "Path{}"), i), ExifAttr::from_opt_path(Some(r_path.as_path())));
			out.insert(
				format!(concat!(sys_prefix!(), "RPath{}"), n_components - i - 1),
				ExifAttr::from_opt_path(Some(r_path.as_path())),
			);
		}
		match fs::metadata(path) {
			Ok(metadata) => {
				out.insert(with_sys_prefix!("DateTimeCreated"), ExifAttr::from_opt_filetime(metadata.created().ok()));
				out.insert(with_sys_prefix!("DateTimeModified"), ExifAttr::from_opt_filetime(metadata.modified().ok()));
				out.insert(with_sys_prefix!("DateTimeAccessed"), ExifAttr::from_opt_filetime(metadata.accessed().ok()));
				out.insert(with_sys_prefix!("Size"), ExifAttr::Integer(metadata.len() as i64));
			}
			Err(err) => error!("Unable to read metadata for {:?}: {:?}", path, err),
		}

		if let Ok(mut file) = fs::File::open(&path) {
			let mut hasher = Sha1::new();
			match io::copy(&mut file, &mut hasher) {
				Ok(_) => {
					out.insert(with_sys_prefix!("Sha1"), ExifAttr::Text(hex::encode(hasher.finalize())));
				}
				Err(e) => error!("Unable to compute hash for {:?}: {:?}", &path, e),
			}
		}

		let exif_file = fs::File::open(path);
		match exif_file {
			Ok(file) => {
				let mut buf_reader = io::BufReader::new(&file);
				let exif_reader = exif::Reader::new();
				if let Ok(exif) = exif_reader.read_from_container(&mut buf_reader) {
					for f in exif.fields() {
						debug!(
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
						let key = match f.ifd_num {
							In::THUMBNAIL => format!("Tn{}", f.tag),
							_ => f.tag.to_string(),
						};
						out.insert(self.fmt.sanitize_key(&key), value);
					}
				}
			}
			Err(err) => error!("Unable to read EXIF from {:?}: {:?}", path, err),
		}
		out
	}

	fn run(&self) {
		for glob in &self.args.globs {
			info!("Matching pattern '{}'", glob);
			let paths = self.find_matches(glob).expect("Error extracting source files");

			let reg = handlebars::Handlebars::new();

			for src_path in &paths {
				let props = self.extract_props(src_path);
				let mut data = serde_json::value::Map::new();
				for (key, value) in &props {
					let value_as_string = self.fmt.as_string(&value);
					if self.args.verbose {
						info!("{:50} '{}'", key, value_as_string);
					}
					data.insert(key.to_owned(), Value::String(value_as_string));
				}
				match reg.render_template(&self.args.out, &data) {
					Ok(dest) => {
						let dest_path = PathBuf::from(dest);
						println!("{:?} {:?}", src_path, dest_path);
					}
					Err(e) => error!("Invalid pattern or data {}: {}", &self.args.out, e),
				}
			}
		}
	}
}

fn main() {
	use log4rs::config::*;

	let log_appender = Appender::builder().build("stdout".to_string(), Box::new(ConsoleAppender::builder().build()));
	let log_root = Root::builder().appender("stdout".to_string()).build(LevelFilter::Info);
	let log_config = Config::builder().appender(log_appender).build(log_root);

	init_config(log_config.expect("Invalid log configuration")).expect("Unable to initialize log4rs");

	let app = App::new(Args::parse()).expect("Invalid arguments");
	app.run();
}
