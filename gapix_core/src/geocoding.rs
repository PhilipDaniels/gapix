use std::{
    collections::HashMap,
    fs::{self, File},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    sync::OnceLock,
    thread::JoinHandle,
};

use geo::{point, GeodesicDistance};
use log::{debug, info, warn};
use logging_timer::time;
use rstar::{PointDistance, RTree, RTreeObject, AABB};

use crate::byte_counter::ByteCounter;

#[derive(Debug, Clone)]
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

/// Initialises the geocoding system. This involves downloading, filtering and
/// loading various files from geonames.org. This is done on background threads
/// so that hopefully the structures will be available as soon as they are
/// needed, which is when the first stage analysis is done.
#[time]
pub fn initialise_geocoding(options: &GeocodingOptions) {
    if let Some(p) = &options.download_folder {
        fs::create_dir_all(p).unwrap();
    }

    let h1 = load_countries(options.clone());
    let h2 = load_admin_1_codes(options.clone());
    let h3 = load_admin_2_codes(options.clone());
    let h4 = load_places(options.clone());

    // TODO: For now, join all the threads so that the time library doesn't blow
    // up (it only works in single-threaded programs).
    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
    h4.join().unwrap();
}

static COUNTRIES: OnceLock<HashMap<String, Country>> = OnceLock::new();
static ADMIN_1_CODES: OnceLock<HashMap<String, String>> = OnceLock::new();
static ADMIN_2_CODES: OnceLock<HashMap<String, String>> = OnceLock::new();
static PLACES: OnceLock<RTree<Place>> = OnceLock::new();

/// Given a (lat, lon) finds the nearest place and returns a description of it.
#[time]
pub fn reverse_geocode_latlon((lat, lon): (f64, f64)) -> Option<String> {
    // Lookup this (lat,lon) in the R*Tree to find the Place.
    let tree = PLACES.get().unwrap();
    let place = tree.nearest_neighbor(&[lat, lon]);
    if place.is_none() {
        return None;
    }

    let place = place.unwrap();
    // Now use the attributes of the Place to find the country name, county etc.

    Some(place.name.clone())
}

/// Given a 2-letter ISOCode, return the country.
/// n.b. The code for the UK is "GB".
pub fn get_country(iso_code: &str) -> Option<&Country> {
    COUNTRIES.get()?.get(iso_code)
}

/// Given key, return the first-level country subdivision.
/// e.g. for "GB.ENG" return "England".
/// In the US this would be a state.
pub fn get_admin1_code(key: &str) -> Option<&String> {
    ADMIN_1_CODES.get()?.get(key)
}

/// Given key, return the second-level country subdivision.
/// e.g. for "GB.ENG.J9" return "Nottinghamshire".
pub fn get_admin2_code(key: &str) -> Option<&String> {
    ADMIN_2_CODES.get()?.get(key)
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
    pub timezone: String,
}

impl RTreeObject for Place {
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_point([self.lat, self.lon])
    }
}

impl PointDistance for Place {
    fn distance_2(
        &self,
        point: &<Self::Envelope as rstar::Envelope>::Point,
    ) -> <<Self::Envelope as rstar::Envelope>::Point as rstar::Point>::Scalar {
        // These must match what we set in `impl RTreeObject::envelope()`.
        let lat = point[0];
        let lon = point[1];
        // Note that points are constructed this way round: see
        // `EnrichedTrackPoint::as_geo_point()`.
        let p1 = point! { x: lon, y: lat };
        let p2 = point! { x: self.lon, y: self.lat };
        p1.geodesic_distance(&p2)
    }
}

fn load_countries(options: GeocodingOptions) -> JoinHandle<()> {
    assert!(COUNTRIES.get().is_none());
    // Spawn a thread to do the actual work of downloading and populating the
    // countries table. We don't join the thread (ever) to avoid blocking the
    // main thread here.
    std::thread::spawn(|| {
        COUNTRIES.set(load_countries_inner(options)).unwrap();
    })
}

#[time]
fn load_countries_inner(options: GeocodingOptions) -> HashMap<String, Country> {
    if options.disable_geocoding() {
        return HashMap::new();
    }

    let filename = download_file(&options, "countryInfo.txt");

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

fn load_admin_1_codes(options: GeocodingOptions) -> JoinHandle<()> {
    assert!(ADMIN_1_CODES.get().is_none());
    // Spawn a thread to do the actual work of downloading and populating the
    // ADMIN_1_CODES table. We don't join the thread (ever) to avoid blocking
    // the main thread here.
    std::thread::spawn(|| {
        ADMIN_1_CODES
            .set(load_admin_1_codes_inner(options))
            .unwrap();
    })
}

#[time]
fn load_admin_1_codes_inner(options: GeocodingOptions) -> HashMap<String, String> {
    if options.disable_geocoding() {
        return HashMap::new();
    }

    let filename = download_file(&options, "admin1CodesASCII.txt");
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

fn load_admin_2_codes(options: GeocodingOptions) -> JoinHandle<()> {
    assert!(ADMIN_2_CODES.get().is_none());
    // Spawn a thread to do the actual work of downloading and populating the
    // ADMIN_2_CODES table. We don't join the thread (ever) to avoid blocking
    // the main thread here.
    std::thread::spawn(|| {
        ADMIN_2_CODES
            .set(load_admin_2_codes_inner(options))
            .unwrap();
    })
}

#[time]
fn load_admin_2_codes_inner(options: GeocodingOptions) -> HashMap<String, String> {
    if options.disable_geocoding() {
        return HashMap::new();
    }

    let filename = download_file(&options, "admin2Codes.txt");
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

fn load_places(options: GeocodingOptions) -> JoinHandle<()> {
    assert!(PLACES.get().is_none());
    // Spawn a thread to do the actual work of downloading and populating the
    // ADMIN_2_CODES table. We don't join the thread (ever) to avoid blocking
    // the main thread here.
    std::thread::spawn(|| {
        PLACES.set(load_places_inner(options)).unwrap();
    })
}

#[time]
fn load_places_inner(options: GeocodingOptions) -> RTree<Place> {
    if options.disable_geocoding() {
        return RTree::new();
    }

    let mut places = Vec::with_capacity(2048);

    for iso_code in &options.countries {
        load_place(&mut places, &options, iso_code);
    }

    let tree = RTree::bulk_load(places);

    info!(
        "Loaded {} places from {} country files into the RTree",
        tree.size(),
        options.countries.len()
    );

    tree
}

#[time]
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
            let fc = fields[6]; // feature_class
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
                        timezone: fields[17].to_string(),
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
