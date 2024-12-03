# GaPiX

GaPiX: GPX analysis and information

GaPiX is a command-line tool to simplify, analyse and join GPX tracks. The basic
usage is `gapix [OPTIONS] *.gpx`. GaPiX never changes your input files, all changes are
written to new files, and by default GaPiX does not overwrite output files that
already exist, but this can be overridden with the `--force` flag. You can get a
list of all the options and their default values by running `gapix --help`.

As well as `.gpx` files GaPiX can also read `.fit` files which record
[Activities](https://developer.garmin.com/fit/file-types/activity/)
i.e. rides. 

# Joining GPX Files
Sometimes a ride might get split up by your device into multiple tracks due to
power or GPS interruption. Or perhaps you just rode it in several stages in the
first place. If this is the case, GaPiX can join them together into a new file
with a single track. The join is done by simply concatenating the points
together, no attempt is made to interpolate new points. So if there is a large
gap between the tracks the result will also contain that gap.

```shell
gapix --join *.gpx
```

The input files will be sorted by their filenames and then joined into a new
file with one track. The name of the new file will be based on the name of the
first file with "joined.gpx" appended. Joining can be combined with
simplification and analysis.

# Simplification

I initially wrote this tool because the GPX files produced by my Garmin Edge
1040 are huge - about 13MB for a 200km ride. This is far too large for [Audax
UK](https://www.audax.uk/) to validate for a DIY ride (max file size of 1.25Mb).
The files are so large because the Edge 1040 writes a trackpoint every second,
each one has extra information such as heart rate and temperature, and it
records lat-long to a ridiculous number of decimal places, e.g.
"53.0758009292185306549072265625" and elevation likewise to femtometre precision
e.g. "173.8000030517578125".

In reality, the device only measures elevation to 1 decimal place and 6 decimal
places are sufficient to record lat-long to within 11cm of accuracy: see
[Decimal degrees](https://en.wikipedia.org/wiki/Decimal_degrees).

This program shrinks the files down by simplifying the individual trackpoints to
just (lat,lon), elevation and time and by applying the
[Ramer-Douglas-Peucker algorithm](https://en.wikipedia.org/wiki/Ramer%E2%80%93Douglas%E2%80%93Peucker_algorithm)
to eliminate unnecessary trackpoints - that is, those that lie along the line.

Usage as follows:

```shell
gapix --metres=5 *.gpx
```

For each input file "FILE.gpx", a new file "FILE.simplified.gpx" will be written
alongside.

## Size Reduction Estimates
An original file from a Garmin Edge 1040 is 11.5Mb with 31,358 trackpoints and
was 200km long.

|--metres|Output Points|File Size|Quality|
|-|-|-|-|
|1  |4374 (13%) |563Kb|Near-perfect map to the road|
|5  |1484 (4.7%)|192Kb|Very close map to the road, mainly stays within the road lines|
|10 |978 (3.1%) |127Kb|Very Good - good enough for submission to Audax UK|
|20 |636 (2.0%) |83Kb |Ok - within a few metres of the road|
|50 |387 (1.2%) |51Kb |Poor - cuts off a lot of corners|
|100|236 (0.8%) |31Kb |Very poor - significant corner truncation|


# Analysis Spreadsheet
GaPiX was written by, and primarily intended for, use by audaxers (randonneurs).
It can produce a spreadsheet (.xlsx format) which breaks rides down into Stages,
where a Stage is either *Moving* or a *Control* (a stop for food or proof of
prescence). GaPiX has no idea of where the ride organiser placed the controls,
so detection of them is automatic based on you not moving for a while. This is
not always 100% foolproof, as there is no real way of distinguishing between a
Control stop and a long pause for traffic lights or a bathroom break. Detecting
[info
controls](https://www.audax.uk/about-audax/new-to-audax#:~:text=Some%20rides%20also%20have%20%22information%20controls%22%20which%20require%20you%20to%20answer%20a%20simple%20question%20about%20something%20(for%20example%2C%20a%20road%20sign)%20at%20the%20relevant%20location)
is particularly problematic as they are usually just quick "stop-and-go"
experiences, so there is no way of discriminating between them and a stop for
traffic. As such, they will usually not be detected by GaPiX unless you reduce
the `min-control-time` significantly, but this is likely to produce a lot of
false positives.

The defaults for Stage analysis work fine in most cases, but there are several
command line options which allow you tweak the Control detection:

- `--analyse`: Turns on analysis and generates a `.xlsx` spreadsheet.

The three options for Control detection are:

- `--control-speed`: Dropping below this speed is used to *potentially* signal
  the start of a Control.
- `--min-control-time`: How long you must be stopped for this stop to be
  considered a Control.
- `--control-resumption-distance`: How far you must move from your Control stop
  to be considered Moving again. This parameter is designed to deal with you
  pushing your bike around the car park or taking the GPS in the store with you.

This just controls the output:

- `trackpoint-hyperlinks`: When writing the .xlsx, whether to include a
  hyperlink to Google Maps for each trackpoint. This can be handy when
  debugging, but it will slow down opening the spreadsheet a lot if you use
  LibreOffice (I don't have Excel so I don't know about that).

When doing Analysis, GaPiX will attempt to reverse-geocode your ride stages.
This involves looking up a placename from its (lat,lon) coordinate. In order to
do this GaPiX needs a database of places. GaPiX will automatically download this
from [geonames.org](https://www.geonames.org/) and cache it locally. You need to
specify the list of countries on the command line using the `--countries`
option, which takes a comma-separated list of
[2-letter country ISO codes](https://en.wikipedia.org/wiki/List_of_ISO_3166_country_codes)

```shell
gapix --analyse --countries=GB,FR,IE,US *.gpx
```

Note that the ISOCode for the UK is "GB"!

The download is normally only done once, the first time you specify that
country. To force a re-download, use the `force-geonames-download` flag. It's
not necessary to do this often, new settlements aren't created every day.

You can turn off geocoding by specifying an empty list for `--countries`.

Click to see an [example spreadsheet](Horseshoe%20Pass%20200.xlsx).

# Other Options
- `--force`: always re-generate and overwrite output files, even if they already
  exist.


# Logging
GaPiX normally runs quietly, but you can get a lot of detail by enabling
logging to the console using the `RUST_LOG` environment variable. On Linux:

```shell
RUST_LOG=DEBUG gapix [OPTIONS] *.gpx
```

and on Windows: 

```shell
$env:RUST_LOG=DEBUG
gapix.exe [OPTIONS] *.gpx
```

Note that GaPiX processes all the input files in parallel, so the log might be a
bit confusing if you ask it to process many files at once.

# Installation
GaPiX is written in Rust. The EXE is self contained. There is a release on
Github which contains files for Windows and Linux. Or build from source using
[cargo](https://doc.rust-lang.org/cargo):

```shell
git clone https://github.com/PhilipDaniels/gapix
cd gapix/gapix
cargo install --locked --path .
```

If you don't have Rust, you can install it from [rustup](https://rustup.rs/).

# Caveats
* GaPiX has only been tested on my own GPX and FIT files from a Garmin Edge
  1040.
* GPX files always store times in UTC. Conversion into local times has only been
  tested by me in the UK. It *should* work if you cross a timezone boundary or
  transition from Daylight Saving Time during a ride, but I have no way of
  testing that.
