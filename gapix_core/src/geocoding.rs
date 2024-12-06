use std::{
    collections::HashMap,
    fs::{self, File},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    sync::{LazyLock, OnceLock},
};

use chrono_tz::Tz;
use geo::{point, Distance, Geodesic};
use log::{debug, error, info, warn};
use logging_timer::{stime, time};
use rstar::{PointDistance, RTree, RTreeObject, AABB};
use tzf_rs::DefaultFinder;

use crate::byte_counter::ByteCounter;

static OPTIONS: OnceLock<GeocodingOptions> = OnceLock::new();

/// A map of isocode -> Country.
/// TODO: Currently unused.
static COUNTRIES: LazyLock<HashMap<String, Country>> = LazyLock::new(load_countries);

/// Given key, return the first-level country subdivision.
/// e.g. for "GB.ENG" return "England".
/// In the US this would be a state.
/// TODO: Currently unused.
static ADMIN_1_CODES: LazyLock<HashMap<String, String>> = LazyLock::new(load_admin_1_codes);

/// Given key, return the second-level country subdivision.
/// e.g. for "GB.ENG.J9" return "Nottinghamshire".
static ADMIN_2_CODES: LazyLock<HashMap<String, String>> = LazyLock::new(load_admin_2_codes);

/// The lowest level of places - towns, mountains, parks etc.
/// This is where reverse-geocoding begins.
static PLACES: LazyLock<RTree<Place>> = LazyLock::new(load_places);

/// This is used to lookup a timezone (in string form such as "Europe/London")
/// from a (lat,lon) pair.
static TIMEZONES: LazyLock<tzf_rs::DefaultFinder> = LazyLock::new(DefaultFinder::new);

/// Initialises the geocoding system. This involves downloading, filtering and
/// loading various files from geonames.org. This is done on background threads
/// so that hopefully the structures will be available as soon as they are
/// needed, which is when the first stage analysis is done.
#[time]
pub fn initialise_geocoding(mut options: GeocodingOptions) {
    // Try and create the download folder once. If this fails, carry on so that
    // the statics get initialised, but set a flag so that the download process
    // is disabled: the statics will get initialised to empty sets (sentinels),
    // which effectively disables geocoding while keeping a simple API.
    if let Some(p) = &options.download_folder {
        match fs::create_dir_all(p) {
            Ok(_) => options.download_folder_exists = true,
            Err(_) => {
                error!("Could not create download folder {:?}", p);
                options.download_folder_exists = false;
            }
        }
    }

    // Stuff `options` somewhere that other threads can use it. This runs on the
    // main thread and is very quick, so we can assume in the spawned loading
    // threads that the value is set (i.e. we can just unwrap).
    //
    // This is to work around the fact that OnceLock::get() does not block, see
    // the unstabilized API: `wait()` which we would prefer to use instead.
    // Since that doesn't exist, we need to replace our static OnceLocks with
    // LazyLocks which *do* block while they are being initialized. And to do
    // that we need to find some way to pass the options into the LazyLock's
    // `new` function, which is a closure which doesn't normally accept
    // parameters. And this is how we do that!
    OPTIONS
        .set(options)
        .expect("Setting global GeocodingOptions should always work");
    let options = OPTIONS.get();
    assert!(options.is_some());

    // Spawn some background threads to "get" these statics right away. This
    // will cause the load process to actually run as soon as possible,
    // hopefully before the data is actually needed by the main thread.
    std::thread::spawn(|| {
        LazyLock::force(&PLACES);
    });

    std::thread::spawn(|| {
        LazyLock::force(&TIMEZONES);
    });

    std::thread::spawn(|| {
        LazyLock::force(&ADMIN_2_CODES);
    });
}

/// Given a (lat, lon) finds the nearest place and returns a description of it.
pub fn reverse_geocode_latlon(point: RTreePoint) -> Option<String> {
    let place = PLACES.nearest_neighbor(&point)?;

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
    PLACES.nearest_neighbor(&point)
}

/// Returns the name of the timezone at point, such as "Europe/London".
pub fn get_timezone_name(point: RTreePoint) -> &'static str {
    TIMEZONES.get_tz_name(point[1], point[0])
}

/// Given a point, finds the timezone.
pub fn get_timezone(point: RTreePoint) -> Option<Tz> {
    // I suppose parse() might fail, we are passing timezone names from tzf-rs
    // into chrono-tz. They SHOULD be the same though.
    let tz: Tz = get_timezone_name(point).parse().ok()?;
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
    pub admin2: String,
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
        Geodesic::distance(p1, p2)
    }
}

#[stime]
fn load_countries() -> HashMap<String, Country> {
    let options = OPTIONS
        .get()
        .expect("OPTIONS should be set early on the main thread");

    let mut countries = HashMap::new();

    if options.disable_geocoding() {
        return countries;
    }

    let rdr = match download_file_and_open(options, "countryInfo.txt") {
        Some(rdr) => rdr,
        None => return countries,
    };

    for line in rdr.lines() {
        let line = match line {
            Ok(line) => line,
            Err(err) => {
                error!("load_countries: Error while reading line: {}", err);
                return countries;
            }
        };

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
            match Continent::try_from(continent_code) {
                Ok(continent) => {
                    let country = Country {
                        iso_code: iso_code.into(),
                        name: name.into(),
                        continent,
                    };
                    countries.insert(iso_code.to_string(), country);
                }
                Err(_) => {
                    error!(
                        "load_countries: Not a valid continent code, skipping line {}",
                        continent_code
                    );
                }
            };
        }
    }

    info!("Loaded {} countries from countryInfo.txt", countries.len());
    countries
}

#[stime]
fn load_admin_1_codes() -> HashMap<String, String> {
    let options = OPTIONS
        .get()
        .expect("OPTIONS should be set early on the main thread");

    let mut codes = HashMap::new();

    if options.disable_geocoding() {
        return codes;
    }

    let rdr = match download_file_and_open(options, "admin1CodesASCII.txt") {
        Some(rdr) => rdr,
        None => return codes,
    };

    for line in rdr.lines() {
        let line = match line {
            Ok(line) => line,
            Err(err) => {
                error!("load_admin_1_codes: Error while reading line: {}", err);
                return codes;
            }
        };

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
    let options = OPTIONS
        .get()
        .expect("OPTIONS should be set early on the main thread");

    let mut codes = HashMap::new();

    if options.disable_geocoding() {
        return codes;
    }

    let rdr = match download_file_and_open(options, "admin2Codes.txt") {
        Some(rdr) => rdr,
        None => return codes,
    };

    for line in rdr.lines() {
        let line = match line {
            Ok(line) => line,
            Err(err) => {
                error!("load_admin_2_codes: Error while reading line: {}", err);
                return codes;
            }
        };

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
    let options = OPTIONS
        .get()
        .expect("OPTIONS should be set early on the main thread");

    if options.disable_geocoding() {
        return RTree::new();
    }

    let mut places = Vec::with_capacity(2048);

    for iso_code in &options.countries {
        load_places_for_country(&mut places, options, iso_code);
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
fn load_places_for_country(places: &mut Vec<Place>, options: &GeocodingOptions, iso_code: &str) {
    let src_filename = format!("{}.zip", iso_code);

    let rdr = match download_file_and_open(options, &src_filename) {
        Some(rdr) => rdr,
        None => return,
    };

    let mut zip = zip::ZipArchive::new(rdr).expect("ZIP can be opened");

    let expected_filename = format!("{}.txt", iso_code);
    let mut place_count = 0;
    for i in 0..zip.len() {
        let zip_entry = match zip.by_index(i) {
            Ok(entry) => entry,
            Err(err) => {
                error!("load_place({iso_code}): Could not extract zip entry {i} due to error {err}, ignoring entry but continuing to scan the zip");
                continue;
            }
        };

        if zip_entry.name() != expected_filename {
            continue;
        }

        debug!("Reading {} from {:?}", zip_entry.name(), &src_filename);
        let rdr = BufReader::new(zip_entry);
        for line in rdr.lines() {
            let line = match line {
                Ok(line) => line,
                Err(err) => {
                    error!("load_place({iso_code}): Error while reading line: {err}",);
                    return;
                }
            };

            let fields: Vec<_> = line.split('\t').collect();
            let fc = fields[6];

            // feature_class
            // A = country, state, region
            // H = stream, lake
            // L = parks, area
            // R = road, railroad
            // P = city, village
            // S = spot, building, farm
            // T = mountain, hill, rock
            // U = undersea
            // V = forest, heath
            if !["P", "T"].contains(&fc) {
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

fn download_file_and_open(options: &GeocodingOptions, filename: &str) -> Option<BufReader<File>> {
    match download_file(options, filename) {
        Some(filename) => match File::open(&filename) {
            Ok(src_file) => {
                let rdr = BufReader::new(src_file);
                Some(rdr)
            }
            Err(_) => {
                error!("Could not open downloaded file {:?}", filename);
                None
            }
        },
        None => None,
    }
}

/// Downloads the specified file from geonames.org. If an error occurs during
/// any part of the process it is logged and None is returned.
fn download_file(options: &GeocodingOptions, filename: &str) -> Option<PathBuf> {
    assert!(!options.disable_geocoding());
    
    let out_filename = options.get_output_path(filename);
    if options.force_download || !Path::exists(&out_filename) {
        let url = format!("https://download.geonames.org/export/dump/{}", filename);

        let resp = match reqwest::blocking::get(&url) {
            Ok(resp) => resp,
            Err(_) => {
                error!("download_file: request for {} failed", filename);
                return None;
            }
        };

        let body = match resp.bytes() {
            Ok(body) => body,
            Err(_) => {
                error!("download_file: body of {} is invalid", filename);
                return None;
            }
        };

        debug!("Starting writing to {:?}", &out_filename);

        let file = match File::create(&out_filename) {
            Ok(file) => file,
            Err(_) => {
                error!("download_file: failed to create {}", filename);
                return None;
            }
        };

        let mut writer = ByteCounter::new(file);

        match writer.write_all(&body) {
            Ok(_) => {}
            Err(_) => {
                error!(
                    "download_file: failed to write download content to {}",
                    filename
                );
                return None;
            }
        };

        match writer.flush() {
            Ok(_) => {}
            Err(_) => {
                error!("download_file: failed to flush file {}", filename);
                return None;
            }
        }

        debug!(
            "Wrote {} bytes to {:?}",
            writer.bytes_written(),
            &out_filename
        );
    } else {
        debug!("File {:?} already exists, skipping download (specify --force-geonames-download to change this behaviour)", &out_filename);
    }

    Some(out_filename)
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
    /// If false, indicates we were not able to create the download folder,
    /// and hence geocoding should be disabled.
    download_folder_exists: bool,
}

impl GeocodingOptions {
    pub fn new(
        download_folder: Option<PathBuf>,
        countries: Vec<String>,
        force_download: bool,
    ) -> Self {
        Self {
            download_folder,
            countries,
            force_download,
            download_folder_exists: false,
        }
    }

    fn disable_geocoding(&self) -> bool {
        self.download_folder.is_none() || self.countries.is_empty() || !self.download_folder_exists
    }

    fn get_output_path<P: AsRef<Path>>(&self, filename: P) -> PathBuf {
        let mut out = self.download_folder.clone().unwrap();
        out.push(filename);
        out
    }

    fn include_country(&self, isocode: &str) -> bool {
        self.countries.iter().any(|c| c == isocode)
    }
}
