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

## Examples

Find all Sony RAW files in mounted memory cards and copy them into a new, flat folder named after the current datetime,
while assigning them unique names produced by appending the EXIF timestamp with a sequence number:

```bash
exif-namer -m cp "/media/**/*.ARW" -d "{{SysDateTimeNow}}/{{ExifDateTimeOriginal}}_{{SysIdx}}{{SysDotExt}}" -v --no-sha1
```

Obtain all the available metadata for a specified Sony RAW file and send them to stdout, one line for each property.
The metadata listed here can be used to determine a destination name when copying/moving in bulk:

```bash
exif-namer -m info /media/nico/D9F7-3979/DCIM/100MSDCF/DSC04696.ARW
```
{{ExifArtist}} "Nico_Orru"
{{ExifBitsPerSample}} "8"
{{ExifBrightnessValue}} "21134_2560"
{{ExifColorSpace}} "1"
{{ExifComponentsConfiguration}} "YCbCr_"
{{ExifCompositeImage}} "0"
{{ExifCompressedBitsPerPixel}} "9_1"
{{ExifCompression}} "7"
{{ExifContrast}} "0"
{{ExifCopyright}} ""
{{ExifCustomRendered}} "0"
{{ExifDateTime}} "20240727_163855"
{{ExifDateTimeDigitized}} "20240727_163855"
{{ExifDateTimeOriginal}} "20240727_163855"
{{ExifDigitalZoomRatio}} "16_16"
{{ExifExifVersion}} "2_32"
{{ExifExposureBiasValue}} "0_10"
{{ExifExposureMode}} "1"
{{ExifExposureProgram}} "1"
{{ExifExposureTime}} "1_3200"
{{ExifFNumber}} "25_10"
{{ExifFileSource}} "digital_still_camera"
{{ExifFlash}} "16"
{{ExifFlashpixVersion}} "1_0"
{{ExifFocalLength}} "590_10"
{{ExifFocalLengthIn35mmFilm}} "59"
{{ExifImageDescription}} "_"
{{ExifImageLength}} "4000"
{{ExifImageWidth}} "6000"
{{ExifInteroperabilityIndex}} "R98"
{{ExifInteroperabilityVersion}} "1_00"
{{ExifJPEGInterchangeFormat}} "565248"
{{ExifJPEGInterchangeFormatLength}} "2644046"
{{ExifLensModel}} "SAMYANG_AF_35-150mm_F2-2_8"
{{ExifLensSpecification}} "350_10"
{{ExifLightSource}} "255"
{{ExifMake}} "SONY"
{{ExifMakerNote}} "0x730000200700010000000000000002200400010000000000 ... 00000000000000000000000000000000000000000000000000" (76634 chars total)
{{ExifMaxApertureValue}} "582_256"
{{ExifMeteringMode}} "5"
{{ExifModel}} "ILCE-9M3"
{{ExifOffsetTime}} "+00_00"
{{ExifOffsetTimeDigitized}} "+00_00"
{{ExifOffsetTimeOriginal}} "+00_00"
{{ExifOrientation}} "1"
{{ExifPhotographicSensitivity}} "250"
{{ExifPhotometricInterpretation}} "6"
{{ExifPixelXDimension}} "6000"
{{ExifPixelYDimension}} "4000"
{{ExifPlanarConfiguration}} "1"
{{ExifRecommendedExposureIndex}} "250"
{{ExifReferenceBlackWhite}} "0_1"
{{ExifResolutionUnit}} "2"
{{ExifSamplesPerPixel}} "3"
{{ExifSaturation}} "0"
{{ExifSceneCaptureType}} "0"
{{ExifSceneType}} "directly_photographed_image"
{{ExifSensitivityType}} "2"
{{ExifSharpness}} "0"
{{ExifSoftware}} "ILCE-9M3_v1_00"
{{ExifSubSecTime}} "902"
{{ExifSubSecTimeDigitized}} "902"
{{ExifSubSecTimeOriginal}} "902"
{{ExifTagTiff254}} "1"
{{ExifTagTiff330}} "139002"
{{ExifTagTiff50341}} "0x5072696e74494d0030333030000003000200010000000300 ... 008b00000010270000cb03000010270000e51b000010270000" (214 chars total)
{{ExifTagTiff50740}} "240"
{{ExifTagTiff700}} "60"
{{ExifTnArtist}} "Nico_Orru"
{{ExifTnCompression}} "6"
{{ExifTnCopyright}} ""
{{ExifTnDateTime}} "20240727_163855"
{{ExifTnImageDescription}} "_"
{{ExifTnJPEGInterchangeFormat}} "44042"
{{ExifTnJPEGInterchangeFormatLength}} "10028"
{{ExifTnMake}} "SONY"
{{ExifTnModel}} "ILCE-9M3"
{{ExifTnOrientation}} "1"
{{ExifTnResolutionUnit}} "2"
{{ExifTnSoftware}} "ILCE-9M3_v1_00"
{{ExifTnTagTiff254}} "1"
{{ExifTnXResolution}} "72_1"
{{ExifTnYCbCrPositioning}} "2"
{{ExifTnYResolution}} "72_1"
{{ExifUserComment}} "0x000000000000000000000000000000000000000000000000 ... 00000000000000000000000000000000000000000000000000" (130 chars total)
{{ExifWhiteBalance}} "1"
{{ExifXResolution}} "350_1"
{{ExifYCbCrCoefficients}} "299_1000"
{{ExifYCbCrPositioning}} "2"
{{ExifYCbCrSubSampling}} "2"
{{ExifYResolution}} "350_1"
{{SysCwd}} "/home/nico/Temporary"
{{SysDateTimeAccessed}} "20240804_150412"
{{SysDateTimeCreated}} "20240727_163855"
{{SysDateTimeModified}} "20240727_163855"
{{SysDateTimeNow}} "20240804_163040"
{{SysDotExt}} ".ARW"
{{SysExt}} "ARW"
{{SysFullName}} "DSC04696.ARW"
{{SysIdx}} "000000"
{{SysName}} "DSC04696"
{{SysPath}} "/media/nico/D9F7-3979/DCIM/100MSDCF"
{{SysPathAncestor0}} "/media/nico/D9F7-3979/DCIM/100MSDCF/DSC04696.ARW"
{{SysPathAncestor1}} "/media/nico/D9F7-3979/DCIM/100MSDCF"
{{SysPathAncestor2}} "/media/nico/D9F7-3979/DCIM"
{{SysPathAncestor3}} "/media/nico/D9F7-3979"
{{SysPathAncestor4}} "/media/nico"
{{SysPathAncestor5}} "/media"
{{SysPathAncestor6}} "/"
{{SysPathElem0}} "/"
{{SysPathElem1}} "media"
{{SysPathElem2}} "nico"
{{SysPathElem3}} "D9F7-3979"
{{SysPathElem4}} "DCIM"
{{SysPathElem5}} "100MSDCF"
{{SysPathElem6}} "DSC04696.ARW"
{{SysPathHead0}} "/"
{{SysPathHead1}} "/media"
{{SysPathHead2}} "/media/nico"
{{SysPathHead3}} "/media/nico/D9F7-3979"
{{SysPathHead4}} "/media/nico/D9F7-3979/DCIM"
{{SysPathHead5}} "/media/nico/D9F7-3979/DCIM/100MSDCF"
{{SysPathHead6}} "/media/nico/D9F7-3979/DCIM/100MSDCF/DSC04696.ARW"
{{SysPathTail0}} "/media/nico/D9F7-3979/DCIM/100MSDCF"
{{SysPathTail1}} "media/nico/D9F7-3979/DCIM/100MSDCF"
{{SysPathTail2}} "nico/D9F7-3979/DCIM/100MSDCF"
{{SysPathTail3}} "D9F7-3979/DCIM/100MSDCF"
{{SysPathTail4}} "DCIM/100MSDCF"
{{SysPathTail5}} "100MSDCF"
{{SysSha1}} "acb807cc2da240e36bb4ea64e9b184b06a7e1d17"
{{SysSize}} "29470720"
{{SysUuid}} "4c9d68e6-75e3-4cf9-a3de-9b92c43e3a30"
```
