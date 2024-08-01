use chrono::DateTime;
use chrono::NaiveDateTime;
use clap::Parser;
use exif::In;
use glob::*;
use handlebars_misc_helpers::{env_helpers, path_helpers, regex_helpers, string_helpers};
use log::*;
use log4rs::append::console::ConsoleAppender;
use serde_json::value::*;
use sha1::{Digest, Sha1};
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
	sources: Vec<String>,

	#[arg(short, long, default_value = "{{SysPath}}/{{SysName}}_{{SysSha1}}.{{SysExt}}")]
	destination: String,

	#[arg(short, long, default_value = "%Y%m%d_%H%M%S")]
	timestamp_format: String,

	#[arg(short, long, default_value_t = false)]
	verbose: bool,

	#[arg(short = 'n', long, default_value_t = false)]
	dry_run: bool,

	#[arg(short, long, default_value_t = false)]
	info: bool,

	#[arg(short, long, default_value_t = 100)]
	max_display_len: usize,

	#[arg(short, long, default_value = "[^\\w\\+\\-]+")]
	sanitize: String,

	#[arg(short, long, default_value = "_")]
	replacement: String,
}

#[derive(Clone, Debug)]
enum PropertyValue {
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

impl PropertyValue {
	fn from_opt_str(from: Option<&str>) -> Self {
		match from {
			Some(word) => PropertyValue::Text(String::from(word)),
			None => PropertyValue::Nothing,
		}
	}

	fn from_opt_str_datetime(from: Option<&str>) -> Self {
		match from {
			Some(word) => match NaiveDateTime::parse_from_str(word, "%Y:%m:%d %H:%M:%S") {
				Ok(dt) => PropertyValue::Timestamp(dt),
				Err(e) => {
					warn!("Unable to parse '{}' as date: {:?}", word, e);
					PropertyValue::Text(String::from(word))
				}
			},
			None => PropertyValue::Nothing,
		}
	}

	fn from_opt_path(from: Option<&Path>) -> Self {
		match from {
			Some(dir) => PropertyValue::Path(PathBuf::from(dir)),
			None => PropertyValue::Nothing,
		}
	}

	fn from_opt_integer<T>(from: Option<&T>) -> Self
	where T: Into<i64> + Copy {
		match from {
			Some(n) => PropertyValue::Integer((*n).into()),
			None => PropertyValue::Nothing,
		}
	}

	fn from_opt_real<T>(from: Option<&T>) -> Self
	where T: Into<f64> + Copy {
		match from {
			Some(v) => PropertyValue::Real((*v).into()),
			None => PropertyValue::Nothing,
		}
	}

	fn from_opt_rational<T, U>(from: Option<&T>) -> Self
	where
		T: Pair<U>,
		U: Into<i64> + Copy, {
		match from {
			Some(r) => {
				let (n, d) = r.as_pair();
				PropertyValue::Fraction(n.into(), d.into())
			}
			None => PropertyValue::Nothing,
		}
	}

	fn from_opt_filetime(from: Option<std::time::SystemTime>) -> PropertyValue {
		match from
			.and_then(|t| t.duration_since(UNIX_EPOCH).ok())
			.and_then(|d| DateTime::from_timestamp(d.as_secs() as i64, d.subsec_nanos()))
			.map(|dt| dt.naive_utc())
		{
			Some(t) => PropertyValue::Timestamp(t),
			None => PropertyValue::Nothing,
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

	fn fmt<W>(&self, value: &PropertyValue, f: &mut W) -> std::fmt::Result
	where W: Write {
		match value {
			// write!(f, "{}", strings)
			PropertyValue::Text(ref text) => f.write_str(text),
			PropertyValue::Path(ref path) => f.write_str(path.to_str().unwrap_or("")),
			PropertyValue::Timestamp(ref timestamp) =>
				f.write_str(&timestamp.format(&self.date_time_format).to_string()),
			PropertyValue::Integer(ref value) => write!(f, "{}", value),
			PropertyValue::Fraction(ref num, ref den) => write!(f, "{}_{}", num, den),
			PropertyValue::Real(ref value) => write!(f, "{}", value),
			PropertyValue::Nothing => Ok(()),
		}
	}

	fn sanitize_value(&self, value: &String) -> String {
		self.sanitize_value_pattern.replace_all(value, &self.sanitize_replacement).to_string()
	}

	pub fn sanitize_key(&self, key: &String) -> String { self.sanitize_key_pattern.replace_all(key, "").to_string() }

	pub fn as_string(&self, value: &PropertyValue) -> String {
		let mut value_as_string = String::new();
		if let Err(e) = self.fmt(&value, &mut value_as_string) {
			error!("Cannot convert {:?} to string: {}", value, e)
		}
		match value {
			PropertyValue::Path(_) => value_as_string,
			_ => self.sanitize_value(&value_as_string),
		}
	}
}

struct App<'a> {
	args: Args,
	attr_formatter: ExifAttrFormatter,
	handlebars: handlebars::Handlebars<'a>,
}

macro_rules! sys_prefix {
	() => {
		"Sys"
	};
}

macro_rules! with_sys_prefix {
	($name:expr) => {
		concat!(sys_prefix!(), $name)
	};
}

const DESTINATION_TEMPLATE_ID: &'static str = "destination";

impl<'a> App<'a> {
	fn new(args: Args) -> Result<Self, regex::Error> {
		let attr_formatter =
			ExifAttrFormatter::new(args.timestamp_format.clone(), &args.sanitize, args.replacement.clone())?;
		let mut handlebars = handlebars::Handlebars::new();
		string_helpers::register(&mut handlebars);
		regex_helpers::register(&mut handlebars);
		path_helpers::register(&mut handlebars);
		regex_helpers::register(&mut handlebars);
		env_helpers::register(&mut handlebars);
		handlebars
			.register_template_string(DESTINATION_TEMPLATE_ID, &args.destination)
			.map_err(|e| regex::Error::Syntax(format!("Handlebar syntax error in {}: {}", args.destination, e)))?;

		Ok(App { args, attr_formatter, handlebars })
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

	fn extract_properties<F>(&self, src: &PathBuf, mut add_property: F)
	where F: FnMut(&str, &PropertyValue) {
		// Path properties
		add_property(
			with_sys_prefix!("Ext"),
			&PropertyValue::from_opt_path(src.extension().map(PathBuf::from).as_ref().map(PathBuf::as_path)),
		);
		add_property(
			with_sys_prefix!("Name"),
			&PropertyValue::from_opt_path(src.file_stem().map(PathBuf::from).as_ref().map(PathBuf::as_path)),
		);
		add_property(
			with_sys_prefix!("FullName"),
			&PropertyValue::from_opt_path(src.file_name().map(PathBuf::from).as_ref().map(PathBuf::as_path)),
		);
		add_property(with_sys_prefix!("Path"), &PropertyValue::from_opt_path(src.parent()));
		let mut partial_path = PathBuf::new();
		let components = src.components().collect::<Vec<_>>();
		let n_components = components.len();
		for (i, component) in components.iter().enumerate() {
			add_property(
				&format!(concat!(sys_prefix!(), "PathElem{}"), i),
				&PropertyValue::from_opt_path(Some(PathBuf::from(component.as_os_str()).as_path())),
			);
			partial_path.push(component);
			add_property(
				&format!(concat!(sys_prefix!(), "Path{}"), i),
				&PropertyValue::from_opt_path(Some(partial_path.as_path())),
			);
			add_property(
				&format!(concat!(sys_prefix!(), "RPath{}"), n_components - i - 1),
				&PropertyValue::from_opt_path(Some(partial_path.as_path())),
			);
		}

		// Filesystem metadata properties
		match fs::metadata(src) {
			Ok(metadata) => {
				add_property(
					with_sys_prefix!("DateTimeCreated"),
					&PropertyValue::from_opt_filetime(metadata.created().ok()),
				);
				add_property(
					with_sys_prefix!("DateTimeModified"),
					&PropertyValue::from_opt_filetime(metadata.modified().ok()),
				);
				add_property(
					with_sys_prefix!("DateTimeAccessed"),
					&PropertyValue::from_opt_filetime(metadata.accessed().ok()),
				);
				add_property(with_sys_prefix!("Size"), &PropertyValue::Integer(metadata.len() as i64));
			}
			Err(e) => error!("Unable to read fs metadata for {:?}: {}", src, e),
		}

		// File content - Sha1 properties
		if let Ok(mut file) = fs::File::open(&src) {
			let mut hasher = Sha1::new();
			match io::copy(&mut file, &mut hasher) {
				Ok(_) => {
					add_property(&with_sys_prefix!("Sha1"), &PropertyValue::Text(hex::encode(hasher.finalize())));
				}
				Err(e) => error!("Unable to compute hash for {:?}: {}", &src, e),
			}
		}

		// File content - Exif properties
		let exif_file = fs::File::open(src);
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
							exif::Value::Byte(ref n) => PropertyValue::from_opt_integer(n.first()),
							exif::Value::Ascii(ref text) => {
								let src = text.first().map(|v| std::str::from_utf8(&*v)).and_then(Result::ok);
								match f.tag {
									exif::Tag::DateTime
									| exif::Tag::DateTimeOriginal
									| exif::Tag::DateTimeDigitized => PropertyValue::from_opt_str_datetime(src),
									_ => PropertyValue::from_opt_str(src),
								}
							}
							exif::Value::Short(ref n) => PropertyValue::from_opt_integer(n.first()),
							exif::Value::Long(ref n) => PropertyValue::from_opt_integer(n.first()),
							exif::Value::Rational(ref r) => PropertyValue::from_opt_rational(r.first()),
							exif::Value::SByte(ref n) => PropertyValue::from_opt_integer(n.first()),
							exif::Value::Undefined(_, _) => PropertyValue::Text(f.display_value().to_string()),
							exif::Value::SShort(ref n) => PropertyValue::from_opt_integer(n.first()),
							exif::Value::SLong(ref n) => PropertyValue::from_opt_integer(n.first()),
							exif::Value::SRational(ref r) => PropertyValue::from_opt_rational(r.first()),
							exif::Value::Float(ref v) => PropertyValue::from_opt_real(v.first()),
							exif::Value::Double(ref v) => PropertyValue::from_opt_real(v.first()),
							exif::Value::Unknown(_, _, _) => PropertyValue::Nothing,
						};
						let key = match f.ifd_num {
							In::THUMBNAIL => format!("Tn{}", f.tag),
							_ => f.tag.to_string(),
						};
						add_property(&self.attr_formatter.sanitize_key(&key), &value);
					}
				}
			}
			Err(e) => error!("Unable to read EXIF from {:?}: {}", src, e),
		}
	}

	fn run(&self) {
		// iterate through all globs
		for glob in &self.args.sources {
			info!("Matching pattern '{}'", glob);
			let paths = self.find_matches(glob).expect("Error extracting source files");

			for src_path in paths.iter().filter(|f| f.is_file()) {
				let mut data = serde_json::value::Map::new();
				self.extract_properties(src_path, |key, value| {
					let value_as_string = self.attr_formatter.as_string(value);
					data.insert(key.to_owned(), Value::String(value_as_string));
				});
				if self.args.info {
					for (key, value) in &data {
						let value_as_str = value.as_str().expect("The data table should only contain strings");
						let len = value_as_str.len();
						if self.args.max_display_len > 0 && len > self.args.max_display_len {
							println!(
								"{{{{{}}}}} \"{} ... {}\" ({} chars total)",
								key,
								&value_as_str[..self.args.max_display_len / 2],
								&value_as_str[len - self.args.max_display_len / 2..],
								len
							);
						} else {
							println!("{{{{{}}}}} \"{}\"", key, value_as_str);
						}
					}
				}
				match self.handlebars.render(DESTINATION_TEMPLATE_ID, &data) {
					Ok(dest) => {
						let dest_path = PathBuf::from(dest);
						println!("{:?} {:?}", src_path, dest_path);
					}
					Err(e) => error!("Invalid pattern or data {}: {}", &self.args.destination, e),
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
