# gapix

GaPiX: GPX analysis and information

A small command-line tool to join and simplify GPX tracks.

I wrote this tool because the GPX files produced by my Garmin
Edge 1040 are huge - about 13MB for a 200km ride. This is far
too large for [Audax UK](https://www.audax.uk/) to validate
for a DIY ride (max file size of 1.25Mb). The files are so
large because the Edge 1040 writes a trackpoint every second, each
one has extra information such as heart rate and temperature, and it
records lat-long to a ridiculous number of decimal places,
e.g. "53.0758009292185306549072265625" and elevation likewise
to femtometre precision "173.8000030517578125".

In reality, the device only measures elevation to 1 decimal place and
6 decimal places are sufficient to record lat-long to within 11cm
of accuracy: see https://en.wikipedia.org/wiki/Decimal_degrees

This program shrinks the files down by simplifying the individual
trackpoints to just lat-long, elevation and time and optionally
by applying the [Ramer-Douglas-Peucker algorithm](https://en.wikipedia.org/wiki/Ramer%E2%80%93Douglas%E2%80%93Peucker_algorithm) to
eliminate unnecessary trackpoints - that is, those that lie
along the line.


# How to use

When **gapix** is run it looks for its input files
in the same folder as the exe. This is mainly for convenience -
I have a known folder containing a copy of the exe, I then
drop the GPXs I want to process into that folder and double-click
a batch file setup with the appropriate command line options
to process them. The program produces an output
filename ending in ".simplified.gpx" and never overwrites the
source file. If the output file already exists, nothing happens.

There are two command line options:

* `--metres=NN` - simplify the output file by applying the RDP
  algorithm with an accurancy of NN metres. 10 is a good value
  (see below for some estimates of reduction sizes).
* `--join` - joins all the input files together, producing
  one file with a single track. The name of the first file is
  used to derive the name of the output file.

If you specify both options then the input files will be joined
and then the result file simplified. But typically, I have
two folders setup with separate batch files, one for
joining and one for simplifying. For example, in my
"simplify" folder I have a batch file with the command

`gapix.exe --metres=10`

which gives a very good size reduction while still being an
excellent fit to the road.


# Size Reduction Estimates

The original file is 11.5Mb with 31,358 trackpoints and was 200km long.

It was from a Garmin Edge 1040 which records 1 trackpoint every second. 
including a lot of extension data such as heartrate and temperature.

|--metres|Output Points|File Size|Quality|
|-|-|-|-|
|1  |4374 (13%) |563Kb|Near-perfect map to the road|
|5  |1484 (4.7%)|192Kb|Very close map to the road, mainly stays within the road lines|
|10 |978 (3.1%) |127Kb|Very Good - good enough for submission to Audax UK|
|20 |636 (2.0%) |83Kb |Ok - within a few metres of the road|
|50 |387 (1.2%) |51Kb |Poor - cuts off a lot of corners|
|100|236 (0.8%) |31Kb |Very poor - significant corner truncation|

# Installation

There is a release on Github, one for Windows and one for Linux.
Or build from source using cargo.

# Caveats
* Has only been tested on my own GPX files from a Garmin Edge 1040.

# TODO
- Move model into its own crate.
- Reverse geocode the stopped stages and the first and last point.
  Use a separate crate, maybe publish it.
- Track splitting. Put file-level waypoints on the nearest split track.
- Waypoint processing for warnings etc.
- Use Rayon - CAN'T - Time crate blows up in to_local_offset.
- Change to use Chrono and Chrono-TZ? Probably. First need to be
  able to reverse geocode lat-lon to timezone name.
- XLSX: Create images to represent the stage profiles.
- XLSX: Display is wrong when time goes over 24 hours.

# Design Questions
- I think it's technically wrong to simply merge all tracks and segments?
  They may exist due to GPS interruptions, device restarts etc.
  Fixing this would make things a lot more complicated though.
- Consider using a new type for DGBSStationType (0..=1023) on waypoint. The
  current design validates when reading a document, but does not validate that
  it is set to a valid value at runtime. The newtype pattern would require a lot
  of boilerplate though, and derive_more doesn't really help with a lot of it.
- Other possible newtypes with the same issues: lat/lon on waypoint and bounds,
- and degrees on waypoint.magvar.

# Links
- GPX XSD: https://www.topografix.com/GPX/1/1/gpx.xsd
- Trackpoint extensions XSD: https://www8.garmin.com/xmlschemas/TrackPointExtensionv1.xsd

# Geocoding
We might not need timezones.txt, it gives us the offset to apply. But the main file uses
the timezone name as the key, e.g. (GB "Europe/London").

The record for Bothamsall (tab separated):

2655132	Bothamsall	Bothamsall	Bothamsall	53.25313	-0.98949	P	PPL	GB		ENG	J9	37UC	37UC008	0		45	Europe/London	2018-07-03


geonameid         : integer id of record in geonames database
name              : name of geographical point (utf8) varchar(200)
asciiname         : name of geographical point in plain ascii characters, varchar(200)
alternatenames    : alternatenames, comma separated, ascii names automatically transliterated, convenience attribute from alternatename table, varchar(10000)
latitude          : latitude in decimal degrees (wgs84)
longitude         : longitude in decimal degrees (wgs84)
feature class     : see http://www.geonames.org/export/codes.html, char(1)
feature code      : see http://www.geonames.org/export/codes.html, varchar(10)
country code      : ISO-3166 2-letter country code, 2 characters
cc2               : alternate country codes, comma separated, ISO-3166 2-letter country code, 200 characters
admin1 code       : fipscode (subject to change to iso code), see exceptions below, see file admin1Codes.txt for display names of this code; varchar(20)
admin2 code       : code for the second administrative division, a county in the US, see file admin2Codes.txt; varchar(80) 
admin3 code       : code for third level administrative division, varchar(20)
admin4 code       : code for fourth level administrative division, varchar(20)
population        : bigint (8 byte int) 
elevation         : in meters, integer
dem               : digital elevation model, srtm3 or gtopo30, average elevation of 3''x3'' (ca 90mx90m) or 30''x30'' (ca 900mx900m) area in meters, integer. srtm processed by cgiar/ciat.
timezone          : the iana timezone id (see file timeZone.txt) varchar(40)
modification date : date of last modification in yyyy-MM-dd format

P = feature class = city or village
PPL = feature code = populated place
GB = country code
<missing> = cc2 alternate country code
ENG = admin1 code = alternate country code (England)
J9 = admin2 code. Full key is GB.ENG.J9 = Nottinghamshire
37UC = admin3 code. 
37UC008 = admin4 code.
0 = population (not known I guess)
45 = elevation in meters
Europe/London = timezone


Files we need to parse
admin1codes - for the country. Key = GB.ENG, Value = GB.ENG	England	England	6269131
admin2codes.txt - for the county. Key = GB.ENG.J9, Value = GB.ENG.J9	Nottinghamshire	Nottinghamshire	2641169
countryinfo.txt - so we can get the continent
allCountries.txt - for the detailed places. We only need:
  - the name (priority to UTF-8, then ASCII, then alternate)
  - the lat and lon
  - country code ("GB"), admin1 code ("ENG"), admin3code ("J9" = Nottinghamshire)
  - timezone


Restrict to UK, EUR, NA, SA, ASIA, WORLD
Stats by country: http://www.geonames.org/statistics/

Place Fields decode
===================
NAME                                            => "Bothamsall"
COUNTRYCODE.ADMIN1.ADMIN2 (in admin2Codes.txt)  => "Nottinghamshire"
COUNTRYCODE.ADMIN1 (in admin1CodesASCII.txt)    => "England"
COUNTRYCODE (in countryInfo.txt)                => "United Kingdom" (also continent "Europe")

