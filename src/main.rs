use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fmt::Write;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use chrono::{DateTime, Local, NaiveDateTime};
use clap::builder::PossibleValue;
use clap::{Parser, ValueEnum};
use exif::In;
use glob::*;
use handlebars_misc_helpers::{env_helpers, path_helpers, regex_helpers, string_helpers};
use log::*;
use log4rs::append::console::{ConsoleAppender, Target};
use serde_json::value::*;
use sha1::{Digest, Sha1};

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
enum Mode {
	Move,
	Copy,
	SymLink,
	HardLink,
	Info,
}

impl Default for Mode {
	fn default() -> Self { Self::Move }
}

impl std::fmt::Display for Mode {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.to_possible_value().expect("no values are skipped").get_name().fmt(f)
	}
}

impl ValueEnum for Mode {
	fn value_variants<'a>() -> &'a [Self] { &[Self::Move, Self::Copy, Self::SymLink, Self::HardLink, Self::Info] }

	fn to_possible_value(&self) -> Option<PossibleValue> {
		Some(match self {
			Self::Move => PossibleValue::new("mv"),
			Self::Copy => PossibleValue::new("cp"),
			Self::SymLink => PossibleValue::new("symlink"),
			Self::HardLink => PossibleValue::new("ln"),
			Self::Info => PossibleValue::new("info"),
		})
	}
}

#[derive(Parser, Debug)]
#[command(version, about = "Bulk rename large collections of images using Exif and OS data in the destination names")]
struct Args {
	#[arg(help = "A list of glob patterns, each identifying a set of files to inspect and rename")]
	sources: Vec<String>,

	#[arg(
		short,
		long,
		default_value = "{{SysPath}}/{{SysName}}_{{SysIdx}}{{SysDotExt}}",
		help = "Destination string template. Uses Handlebars syntax",
		long_help = "Properties are populated by inspecting the source file. \
			Use -m info for details of properties available for each source file"
	)]
	destination: String,

	#[arg(short, long, default_value_t=Mode::Move)]
	mode: Mode,

	#[arg(
		short,
		long,
		default_value = "%Y%m%d_%H%M%S",
		help = "Format string for datetime type properties. Uses chrono and POSIX date syntax"
	)]
	timestamp_format: String,

	#[arg(short, long, default_value_t = false, help = "Log more debugging information.")]
	verbose: bool,

	#[arg(short = 'n', long, default_value_t = false, help = "Do not apply any changes to the filesystem")]
	dry_run: bool,

	#[arg(short, long, default_value_t = false, help = "Force overwrite if destination file exists")]
	force: bool,

	#[arg(long, default_value_t = false, help = "When moving files, delete the source folder if empty")]
	delete_empty_dirs: bool,

	#[arg(
		long,
		default_value_t = false,
		help = "Convert symlink targets to absolute path even if a relative path is available"
	)]
	force_absolute_symlinks: bool,

	#[arg(long, default_value_t = 100, help = "Truncate long values in -m info. Set to 0 for infinite length")]
	max_display_len: usize,

	#[arg(long, default_value_t = 0, help = "Index counter start")]
	idx_start: usize,

	#[arg(long, default_value_t = 6, help = "Width of zero-padding for index counter")]
	idx_width: usize,

	#[arg(
		long,
		default_value = "[^\\w\\+\\-]+",
		help = "Regex pattern which identifies invalid characters or sequences in properties"
	)]
	invalid_characters: String,

	#[arg(long, default_value = "_", help = "Replacement for invalid characters or sequences in properties")]
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

	fn from_opt_path<P: AsRef<Path>>(from: Option<P>) -> Self {
		match from {
			Some(dir) => PropertyValue::Path(PathBuf::from(dir.as_ref())),
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
	now: DateTime<Local>,
	cwd: PathBuf,
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
			ExifAttrFormatter::new(args.timestamp_format.clone(), &args.invalid_characters, args.replacement.clone())?;
		let mut handlebars = handlebars::Handlebars::new();
		handlebars.set_dev_mode(true);
		handlebars.set_prevent_indent(true);
		handlebars.register_escape_fn(handlebars::no_escape);
		string_helpers::register(&mut handlebars);
		regex_helpers::register(&mut handlebars);
		path_helpers::register(&mut handlebars);
		regex_helpers::register(&mut handlebars);
		env_helpers::register(&mut handlebars);
		handlebars
			.register_template_string(DESTINATION_TEMPLATE_ID, &args.destination)
			.map_err(|e| regex::Error::Syntax(format!("Handlebar syntax error in {}: {}", args.destination, e)))?;
		let now = Local::now();
		let cwd = std::env::current_dir().expect("Unable to determine current directory");
		Ok(App { args, now, cwd, attr_formatter, handlebars })
	}

	fn find_matches(&self, pattern: &str) -> Result<Vec<PathBuf>, PatternError> {
		let mut out = Vec::new();
		for iter in glob::glob(pattern)? {
			match iter {
				Ok(path) =>
					if path.is_file() {
						out.push(path)
					},
				Err(e) => error!("Invalid glob pattern {}: {}", pattern, e),
			}
		}
		return Ok(out);
	}

	fn extract_properties<F>(&self, src: &PathBuf, mut add_property: F)
	where F: FnMut(&str, &PropertyValue) {
		// global properties
		add_property(
			// extension without the leading dot
			with_sys_prefix!("DateTimeNow"),
			&PropertyValue::Timestamp(self.now.naive_local()),
		);
		add_property(
			// extension without the leading dot
			with_sys_prefix!("Cwd"),
			&PropertyValue::from_opt_path(Some(&self.cwd)),
		);
		// Path properties
		add_property(
			// extension without the leading dot
			with_sys_prefix!("Ext"),
			&PropertyValue::from_opt_path(src.extension()),
		);
		add_property(
			// extension with the leading dot
			with_sys_prefix!("DotExt"),
			&PropertyValue::from_opt_path(src.extension().map(|ext| {
				let mut d = OsStr::new(".").to_os_string();
				d.push(ext);
				d
			})),
		);
		add_property(
			// name without extension
			with_sys_prefix!("Name"),
			&PropertyValue::from_opt_path(src.file_stem()),
		);
		add_property(
			// name with extension
			with_sys_prefix!("FullName"),
			&PropertyValue::from_opt_path(src.file_name()),
		);
		let parent = src.parent();
		add_property(with_sys_prefix!("Path"), &PropertyValue::from_opt_path(parent));
		let mut path_head = PathBuf::new();
		let components = src.components().collect::<Vec<_>>();
		let n_components = components.len();
		for (i, component) in components.iter().enumerate() {
			add_property(
				&format!(concat!(sys_prefix!(), "PathElem{}"), i),
				&PropertyValue::from_opt_path(Some(component)),
			);
			path_head.push(component);
			add_property(
				&format!(concat!(sys_prefix!(), "PathAncestor{}"), n_components - i - 1),
				&PropertyValue::from_opt_path(Some(path_head.as_path())),
			);
			add_property(
				&format!(concat!(sys_prefix!(), "PathHead{}"), i),
				&PropertyValue::from_opt_path(Some(path_head.as_path())),
			);
		}
		if let Some(up) = parent {
			let mut path_tail = up.components();
			for i in 0..(n_components - 1) {
				add_property(
					&format!(concat!(sys_prefix!(), "PathTail{}"), i),
					&PropertyValue::from_opt_path(Some(&path_tail)),
				);
				path_tail.next();
			}
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
		let mut idx_counter: usize = self.args.idx_start;
		// iterate through all globs
		for glob in &self.args.sources {
			debug!("Matching pattern '{}'", glob);
			let paths = self.find_matches(glob).expect("Error extracting source files");

			self.apply_matches(&paths, &mut idx_counter);

			if self.args.mode == Mode::Move && self.args.delete_empty_dirs {
				self.cleanup_empty_dirs(&paths);
			}
		}
	}

	fn contains_files<P: AsRef<Path>>(&self, dir: P) -> io::Result<bool> {
		for maybe_child in fs::read_dir(dir)? {
			let child = maybe_child?;
			if child.file_type()?.is_dir() && (child.file_name() == "." || child.file_name() == "..") {
				continue;
			}
			return Ok(true);
		}
		Ok(false)
	}

	fn delete_empty_dir<P: AsRef<Path>>(&self, path_ref: P) -> bool {
		let candidate_path = path_ref.as_ref();
		if !self.args.dry_run {
			debug!("Attempting to delete directory {:?}", &candidate_path);
			if let Ok(contains_files) = self.contains_files(candidate_path) {
				if !contains_files {
					if let Err(e) = fs::remove_dir(candidate_path) {
						error!("Unable to delete directory {:?}: {}", candidate_path, e);
					} else {
						return true;
					}
				}
			}
		}
		return false;
	}

	fn cleanup_empty_dirs(&self, paths: &Vec<PathBuf>) {
		let mut candidate_paths = BTreeSet::new();

		for src_path in paths.iter() {
			if let Some(parent) = src_path.parent() {
				for ancestor in parent.ancestors() {
					if !ancestor.as_os_str().is_empty() && !candidate_paths.contains(ancestor) {
						candidate_paths.insert(PathBuf::from(ancestor));
					}
				}
			}
		}

		for candidate_path in candidate_paths.iter().rev() {
			let deleted = self.delete_empty_dir(candidate_path);
			if self.args.verbose {
				println!("{} {:?}", if deleted { "rmdir" } else { "#rmdir" }, candidate_path);
			}
		}
	}

	fn apply_matches(&self, paths: &Vec<PathBuf>, idx_counter: &mut usize) {
		// for each file matching the current glob
		for src_path in paths.iter() {
			// extract properties as a String -> Value map
			let mut data = serde_json::value::Map::new();
			self.extract_properties(src_path, |key, value| {
				let value_as_string = self.attr_formatter.as_string(value);
				data.insert(key.to_owned(), Value::String(value_as_string));
			});
			data.insert(
				with_sys_prefix!("Idx").to_string(),
				Value::String(format!("{:01$}", idx_counter, self.args.idx_width)),
			);
			*idx_counter += 1;

			match self.handlebars.render(DESTINATION_TEMPLATE_ID, &data) {
				Ok(dest) => {
					let dest_path = PathBuf::from(dest);
					self.apply_mode(self.args.mode, src_path, &dest_path, &data);
				}
				Err(e) => error!("Invalid pattern or data {}: {}", &self.args.destination, e),
			}
		}
	}

	fn apply_mode(&self, mode: Mode, src: &PathBuf, dest: &PathBuf, data: &Map<String, Value>) {
		if self.args.verbose {
			println!("{} {:?} {:?}", mode, src, dest);
		}

		if self.args.mode != Mode::Info {
			if same_file::is_same_file(src, dest).unwrap_or(false) {
				warn!("Source and destination file are the same, skipping");
				return;
			}

			if dest.exists() || dest.is_symlink() {
				if self.args.force {
					if let Err(e) = fs::remove_file(dest) {
						error!("Destination exists, and --force specified, but could not remove: {}", e);
						return;
					}
				} else {
					warn!("Destination file exists, skipping. Use --force to overwrite");
					return;
				}
			}

			if self.args.dry_run {
				debug!("Dry run mode, will not make any filesystem change");
				return;
			}

			if let Some(parent) = dest.parent() {
				if !parent.exists() {
					if let Err(e) = fs::create_dir_all(parent) {
						error!("Could not create containing directory {:?}: {}", parent, e);
						return;
					}
				}
			}
		}

		#[allow(deprecated)]
		match self.args.mode {
			Mode::Move =>
				if let Err(e) = fs::rename(src, dest) {
					error!("Could not rename {:?}: {}", src, e);
				},
			Mode::Copy =>
				if let Err(e) = fs::copy(src, dest) {
					error!("Could not copy {:?}: {}", src, e);
				},
			Mode::SymLink => {
				// if src is absolute, we use the absolute path no matter what
				let target = if src.is_absolute() {
					src.to_path_buf()
				} else {
					// if src is a relative path, we need the absolute path to either use it,
					// or determine a relative path from the link name
					let src_absolute = std::path::absolute(src).unwrap_or_else(|_| self.cwd.join(src));
					if self.args.force_absolute_symlinks {
						src_absolute
					} else if let Some(src_relative) = pathdiff::diff_paths(
						&src_absolute,
						std::path::absolute(dest).unwrap_or_else(|_| self.cwd.join(dest)).parent().unwrap(),
					) {
						if self.args.verbose {
							println!("# -> {:?}", src_relative);
						}
						src_relative
					} else {
						src_absolute
					}
				};

				if let Err(e) = fs::soft_link(target, dest) {
					// this is deprecated, but we are sure we are linking files rather than
					// directories, so there is no need to call the os-dependent version
					error!("Could not symlink {:?}: {}", src, e);
				}
			}
			Mode::HardLink =>
				if let Err(e) = fs::hard_link(src, dest) {
					error!("Could not hard link {:?}: {}", src, e);
				},
			// if "-m info" is enabled, display the data contained in the properties table
			Mode::Info =>
				for (key, value) in data {
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
				},
		}
	}
}

fn main() {
	use log4rs::config::*;

	let log_appender = Appender::builder()
		.build("stdout".to_string(), Box::new(ConsoleAppender::builder().target(Target::Stderr).build()));
	let log_root = Root::builder().appender("stdout".to_string()).build(LevelFilter::Info);
	let log_config = Config::builder().appender(log_appender).build(log_root);

	init_config(log_config.expect("Invalid log configuration")).expect("Unable to initialize log4rs");

	let app = App::new(Args::parse()).expect("Invalid arguments");
	app.run();
}
