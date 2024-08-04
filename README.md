# exif-namer

Utility to bulk copy/link/rename image and other media file sets

```
Usage: exif-namer [OPTIONS] [SOURCES]...

Arguments:
  [SOURCES]...  A list of glob patterns, each identifying a set of files to inspect and rename

Options:
  -d, --destination <DESTINATION>
          Destination string template. Uses Handlebars syntax [default: {{SysPath}}/{{SysName}}_{{SysIdx}}{{SysDotExt}}]
  -m, --mode <MODE>
          [default: mv] [possible values: mv, cp, symlink, ln, info]
  -t, --timestamp-format <TIMESTAMP_FORMAT>
          Format string for datetime type properties. Uses chrono and POSIX date syntax [default: %Y%m%d_%H%M%S]
  -v, --verbose
          Log more debugging information.
  -n, --dry-run
          Do not apply any changes to the filesystem
  -f, --force
          Force overwrite if destination file exists
      --no-strict
          Disable Handlebars strict mode
      --no-sha1
          Disable (slow) sha1 hash calculation
      --no-exif
          Disable exif parsing
      --delete-empty-dirs
          When moving files, delete the source folder if empty
      --force-absolute-symlinks
          Convert symlink targets to absolute path even if a relative path is available
      --max-display-len <MAX_DISPLAY_LEN>
          Truncate long values in -m info. Set to 0 for infinite length [default: 100]
      --idx-start <IDX_START>
          Index counter start [default: 0]
      --idx-width <IDX_WIDTH>
          Width of zero-padding for index counter [default: 6]
      --invalid-characters <INVALID_CHARACTERS>
          Regex pattern which identifies invalid characters or sequences in properties [default: [^\w\+\-]+]
      --replacement <REPLACEMENT>
          Replacement for invalid characters or sequences in properties [default: _]
  -h, --help
          Print help (see more with '--help')
  -V, --version
          Print version
```
