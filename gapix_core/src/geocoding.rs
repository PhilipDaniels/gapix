use std::{
    collections::HashMap,
    fs::{self, File},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    sync::{LazyLock, OnceLock},
};

use chrono_tz::Tz;
use geo::{point, GeodesicDistance};
use log::{debug, info, warn};
use logging_timer::{stime, time};
use rstar::{PointDistance, RTree, RTreeObject, AABB};
use tzf_rs::DefaultFinder;

use crate::byte_counter::ByteCounter;

static OPTIONS: OnceLock<GeocodingOptions> = OnceLock::new();

/// A map of isocode -> Country.
static COUNTRIES: LazyLock<HashMap<String, Country>> = LazyLock::new(|| load_countries());

/// Given key, return the first-level country subdivision.
/// e.g. for "GB.ENG" return "England".
/// In the US this would be a state.
static ADMIN_1_CODES: LazyLock<HashMap<String, String>> = LazyLock::new(|| load_admin_1_codes());

/// Given key, return the second-level country subdivision.
/// e.g. for "GB.ENG.J9" return "Nottinghamshire".
static ADMIN_2_CODES: LazyLock<HashMap<String, String>> = LazyLock::new(|| load_admin_2_codes());

/// The lowest level of places - towns, mountains, parks etc.
/// This is where reverse-geocoding begins.
static PLACES2: LazyLock<RTree<Place>> = LazyLock::new(|| load_places());

static TIMEZONES: LazyLock<tzf_rs::DefaultFinder> = LazyLock::new(|| DefaultFinder::new());

/// Initialises the geocoding system. This involves downloading, filtering and
/// loading various files from geonames.org. This is done on background threads
/// so that hopefully the structures will be available as soon as they are
/// needed, which is when the first stage analysis is done.
#[time]
pub fn initialise_geocoding(options: &GeocodingOptions) {
    if let Some(p) = &options.download_folder {
        fs::create_dir_all(p).unwrap();
    }

    // Clone `options` and stuff it somewhere that other threads can use it.
    // This runs on the main thread and is very quick, so we can assume in the
    // loading threads that the value is set (i.e. we can just unwrap).
    //
    // This is to work around the fact that OnceLock::get() does not block, see
    // the unstabilized API: `wait()` which we would prefer to use instead.
    // Since that doesn't exist, we need to replace our static OnceLocks with
    // LazyLocks which *do* block while they are being initialized. And to do
    // that we need to find some way to pass the options into the LazyLock's
    // `new` function, which is a closure which doesn't normally accept
    // parameters. And this is how we do that!
    OPTIONS
        .set(options.clone())
        .expect("Setting global GeocodingOptions should work");
    let options = OPTIONS.get();
    assert!(options.is_some());

    // Spawn some background threads to "get" these statics right away. This
    // will cause the load process to actually run as soon as possible,
    // hopefully before the data is actually needed by the main thread.
    std::thread::spawn(|| {
        LazyLock::force(&PLACES2);
    });

    std::thread::spawn(|| {
        LazyLock::force(&ADMIN_2_CODES);
    });

    std::thread::spawn(|| {
        LazyLock::force(&TIMEZONES);
    });

    // TODO: Either use or remove the other statics.
}

/// Given a (lat, lon) finds the nearest place and returns a description of it.
#[time]
pub fn reverse_geocode_latlon(point: RTreePoint) -> Option<String> {
    // Lookup this (lat,lon) in the R*Tree to find the Place.
    //let tree = PLACES2.get().unwrap();
    let place = PLACES2.nearest_neighbor(&point);
    if place.is_none() {
        return None;
    }

    let place = place.unwrap();

    // Now use the attributes of the Place to find the country name, county etc.
    let key = format!("{}.{}.{}", place.iso_code, place.admin1, place.admin2);
    match get_admin2_code(&key) {
        Some(code) => Some(format!("{}, {}", place.name, code)),
        None => Some(place.name.clone()),
    }
}

/// Given a 2-letter ISOCode, return the country.
/// n.b. The code for the UK is "GB".
pub fn get_country(iso_code: &str) -> Option<&Country> {
    COUNTRIES.get(iso_code)
}

/// Given key, return the first-level country subdivision.
/// e.g. for "GB.ENG" return "England".
/// In the US this would be a state.
pub fn get_admin1_code(key: &str) -> Option<&String> {
    ADMIN_1_CODES.get(key)
}

/// Given key, return the second-level country subdivision.
/// e.g. for "GB.ENG.J9" return "Nottinghamshire".
pub fn get_admin2_code(key: &str) -> Option<&String> {
    ADMIN_2_CODES.get(key)
}

/// Given a point, finds the nearest place in the database.
pub fn get_nearest_place(point: RTreePoint) -> Option<&'static Place> {
    PLACES2.nearest_neighbor(&point)
}

/// Given a point, finds the timezone.
pub fn get_timezone(point: RTreePoint) -> Option<Tz> {
    let tzname = TIMEZONES.get_tz_name(point[1], point[0]);
    // I suppose parse() might fail, we are passing timezone names from tzf-rs
    // into chrono-tz. They SHOULD be the same though.
    let tz: Tz = tzname.parse().ok()?;
    Some(tz)
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Continent {
    // Code = AF
    Africa,
    // Code = AS
    Asia,
    // Code = EU
    Europe,
    // Code = NA
    NorthAmerica,
    // Code = OC
    Oceania,
    // Code = SA
    SouthAmerica,
    // Code = AN
    Antarctica,
}

impl Continent {
    pub fn as_str(&self) -> &'static str {
        match self {
            Continent::Africa => "AF",
            Continent::Asia => "AS",
            Continent::Europe => "EU",
            Continent::NorthAmerica => "NA",
            Continent::Oceania => "OC",
            Continent::SouthAmerica => "SA",
            Continent::Antarctica => "AN",
        }
    }
}

impl TryFrom<&str> for Continent {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "AF" => Ok(Continent::Africa),
            "AS" => Ok(Continent::Asia),
            "EU" => Ok(Continent::Europe),
            "NA" => Ok(Continent::NorthAmerica),
            "OC" => Ok(Continent::Oceania),
            "SA" => Ok(Continent::SouthAmerica),
            "AN" => Ok(Continent::Antarctica),
            _ => Err(format!("Invalid continent code {value}")),
        }
    }
}

/// Represents a country as read from the file `countryInfo.txt`.
#[derive(Debug, Clone)]
pub struct Country {
    pub iso_code: String,
    pub name: String,
    pub continent: Continent,
}

impl PartialEq for Country {
    fn eq(&self, other: &Self) -> bool {
        self.iso_code == other.iso_code
    }
}

impl Eq for Country {}

impl std::hash::Hash for Country {
    fn hash<H: std::hash::Hasher>(&self, hasher: &mut H) {
        self.iso_code.hash(hasher);
    }
}

/// Represents a place as read from the file 'allCountries.txt'.
#[derive(Debug, Clone)]
pub struct Place {
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub iso_code: String,
    pub admin1: String,
    pub admin2: String
}

pub(crate) type RTreePoint = [f64; 2];

impl Place {
    fn as_rtree_point(&self) -> RTreePoint {
        [self.lat, self.lon]
    }
}

impl RTreeObject for Place {
    type Envelope = AABB<RTreePoint>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_point(self.as_rtree_point())
    }
}

fn rtree_point_to_geo_point(point: &RTreePoint) -> geo::Point {
    point! { x: point[1], y: point[0] }
}

impl PointDistance for Place {
    fn distance_2(
        &self,
        point: &<Self::Envelope as rstar::Envelope>::Point,
    ) -> <<Self::Envelope as rstar::Envelope>::Point as rstar::Point>::Scalar {
        let p1 = rtree_point_to_geo_point(point);
        let p2 = rtree_point_to_geo_point(&self.as_rtree_point());
        p1.geodesic_distance(&p2)
    }
}

#[stime]
fn load_countries() -> HashMap<String, Country> {
    let options = OPTIONS.get().unwrap();

    if options.disable_geocoding() {
        return HashMap::new();
    }

    let filename = download_file(options, "countryInfo.txt");

    let src_file = File::open(&filename).unwrap();
    let rdr = BufReader::new(src_file);
    let mut countries = HashMap::new();

    for line in rdr.lines() {
        let line = line.unwrap();
        if line.starts_with('#') {
            continue;
        }

        let fields: Vec<_> = line.split('\t').collect();
        let iso_code = fields[0];
        let name = fields[4];
        let continent_code = fields[8];
        if iso_code.is_empty() || name.is_empty() || continent_code.is_empty() {
            warn!("countryInfo.txt: Skipping line due to one or more empty fields. iso_code={iso_code}, name={name}, continent_code={continent_code}");
        } else {
            let continent = Continent::try_from(continent_code).unwrap();
            let country = Country {
                iso_code: iso_code.into(),
                name: name.into(),
                continent,
            };
            countries.insert(iso_code.to_string(), country);
        }
    }

    info!("Loaded {} countries from countryInfo.txt", countries.len());
    countries
}

#[stime]
fn load_admin_1_codes() -> HashMap<String, String> {
    let options = OPTIONS.get().unwrap();

    if options.disable_geocoding() {
        return HashMap::new();
    }

    let filename = download_file(options, "admin1CodesASCII.txt");
    let src_file = File::open(&filename).unwrap();
    let rdr = BufReader::new(src_file);
    let mut codes = HashMap::new();

    for line in rdr.lines() {
        let line = line.unwrap();
        let fields: Vec<_> = line.split('\t').collect();
        let key = fields[0];
        let isocode = &key[0..2];
        if !options.include_country(isocode) {
            continue;
        }

        let name = fields[1];
        if key.is_empty() || name.is_empty() {
            warn!("admin1CodesASCII.txt: Skipping line due to one or more empty fields. key={key}, name={name}");
        } else {
            codes.insert(key.to_string(), name.to_string());
        }
    }

    info!(
        "Loaded {} codes from admin1CodesASCII.txt (first-level country subdivisions)",
        codes.len()
    );
    codes
}

#[stime]
fn load_admin_2_codes() -> HashMap<String, String> {
    let options = OPTIONS.get().unwrap();

    if options.disable_geocoding() {
        return HashMap::new();
    }

    let filename = download_file(options, "admin2Codes.txt");
    let src_file = File::open(&filename).unwrap();
    let rdr = BufReader::new(src_file);
    let mut codes = HashMap::new();

    for line in rdr.lines() {
        let line = line.unwrap();
        let fields: Vec<_> = line.split('\t').collect();
        let key = fields[0];
        let isocode = &key[0..2];
        if !options.include_country(isocode) {
            continue;
        }

        let name = fields[1];
        if key.is_empty() || name.is_empty() {
            println!("admin2Codes.txt: Skipping line due to one or more empty fields. key={key}, name={name}");
        } else {
            codes.insert(key.to_string(), name.to_string());
        }
    }

    info!(
        "Loaded {} codes from admin2Codes.txt (second-level country subdivisions)",
        codes.len()
    );
    codes
}

#[stime]
fn load_places() -> RTree<Place> {
    let options = OPTIONS.get().unwrap();

    if options.disable_geocoding() {
        return RTree::new();
    }

    let mut places = Vec::with_capacity(2048);

    for iso_code in &options.countries {
        load_place(&mut places, options, iso_code);
    }

    let tree = RTree::bulk_load(places);

    info!(
        "Loaded {} places from {} country files into the RTree",
        tree.size(),
        options.countries.len()
    );

    tree
}

#[stime]
fn load_place(places: &mut Vec<Place>, options: &GeocodingOptions, iso_code: &str) {
    let src_filename = format!("{}.zip", iso_code);
    let filename = download_file(&options, &src_filename);
    let src_file = File::open(&filename).unwrap();

    let rdr = BufReader::new(src_file);
    let mut zip = zip::ZipArchive::new(rdr).expect("ZIP can be opened");

    let expected_filename = format!("{}.txt", iso_code);
    let mut place_count = 0;
    for i in 0..zip.len() {
        let zip_entry = zip.by_index(i).unwrap();
        if zip_entry.name() != expected_filename {
            continue;
        }

        debug!("Reading {} from {:?}", zip_entry.name(), &filename);
        let rdr = BufReader::new(zip_entry);
        for line in rdr.lines() {
            let line = line.unwrap();
            let fields: Vec<_> = line.split('\t').collect();
            let fc = fields[6];

            // feature_class
            // A = country, state, region
            // H = stream, lake
            // L = parks, area
            // R = road, railrod
            // P = city, village
            // S = spot, building, farm
            // T = mountain, hill, rock
            // U = undersea
            // V = forest, heath
            if !(fc == "P") {
                continue;
            }

            let mut name = fields[1].to_string();
            if name.is_empty() {
                name = fields[2].to_string();
            }
            let iso_code = fields[8].to_string();
            if name.is_empty() || iso_code.is_empty() {
                continue;
            }

            match (fields[4].parse::<f64>(), fields[5].parse::<f64>()) {
                (Ok(lat), Ok(lon)) => {
                    let place = Place {
                        name,
                        lat,
                        lon,
                        iso_code,
                        admin1: fields[10].to_string(),
                        admin2: fields[11].to_string(),
                    };

                    places.push(place);
                    place_count += 1;
                }
                (Err(_), Ok(_)) => {
                    warn!("Cannot parse lat: {}", fields[4]);
                }
                (Ok(_), Err(_)) => {
                    warn!("Cannot parse lon: {}", fields[5]);
                }
                (Err(_), Err(_)) => {
                    warn!("Cannot parse lat and lon: {}, {}", fields[4], fields[5]);
                }
            };
        }

        info!("Loaded {} places from {}", place_count, expected_filename);
        place_count = 0;
    }
}

fn download_file(options: &GeocodingOptions, filename: &str) -> PathBuf {
    let out_filename = options.get_output_path(filename);
    if options.force_download || !Path::exists(&out_filename) {
        let url = format!("https://download.geonames.org/export/dump/{}", filename);
        let resp = reqwest::blocking::get(&url)
            .expect(&format!("download_file: request for {} failed", filename));
        let body = resp
            .bytes()
            .expect(&format!("download_file: body of {} is invalid", filename));
        debug!("Starting writing to {:?}", &out_filename);
        let file = File::create(&out_filename)
            .expect(&format!("download_file: failed to create {}", filename));
        let mut writer = ByteCounter::new(file);
        writer.write_all(&body).expect(&format!(
            "download_file: failed to copy content to {}",
            filename
        ));

        writer.flush().unwrap();

        debug!(
            "Wrote {} bytes to {:?}",
            writer.bytes_written(),
            &out_filename
        );
    } else {
        debug!("File {:?} already exists, skipping download (specify --force-geonames-download to change this behaviour)", &out_filename);
    }

    out_filename
}

#[derive(Debug, Clone, Default)]
pub struct GeocodingOptions {
    /// Folder in which to place the downloaded files.
    /// If this is None, no downloading (and hence no geocoding) is
    /// done.
    pub download_folder: Option<PathBuf>,
    /// List of country isocodes to load.
    pub countries: Vec<String>,
    /// If true, forces a re-download of the data files even if they already
    /// exist. This is a good way of keeping them up to date.
    pub force_download: bool,
}

impl GeocodingOptions {
    fn disable_geocoding(&self) -> bool {
        self.download_folder.is_none() || self.countries.is_empty()
    }

    fn get_output_path<P: AsRef<Path>>(&self, filename: P) -> PathBuf {
        let mut out = self.download_folder.clone().unwrap();
        out.push(filename);
        out
    }

    fn include_country(&self, isocode: &str) -> bool {
        self.countries.iter().position(|c| c == isocode).is_some()
    }
}
