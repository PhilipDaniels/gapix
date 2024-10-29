use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
};

use model::{Continent, Country, Place};

mod model;

fn main() {
    // Build
    // 0. Determine which data to include. User can specify
    //    CountryIsoCodes = [GB, FR, US].  aka REQUIRED_COUNTRIES
    //    Continents      = [EU, SA].      aka REQUIRED_CONTINENTS
    // The union of all matching sets will be included.
    //
    // 1. Preprocess the files in 'raw_assets' into 'processed_assets'
    //    a. countryInfo.txt - not necessary, is small. Include all countries, irrespective of filter.
    //    b. admin1CodesASCI.txt - can be cut in half. not necessary. Currently 57kb zipped.
    //       Only include if the first part of the key matches a REQUIRED_COUNTRY or is on a REQUIRED_CONTINENT.
    //    c. admin2Codes.txt - can be cut in half. Currently 680kb zipped.
    //       Only include if the first part of the key matches a REQUIRED_COUNTRY or is on a REQUIRED_CONTINENT.
    //    d. allCountries.txt - Currently 374MB zipped.

    let countries = read_countries();
    // Level 1 is UK countries (ENG, WLS, SCT, NIR) or US states or French departments.
    let level1 = read_level1_codes();
    // Level 2 is UK counties (Notts) or US counties (Miami-Dade County)
    let level2 = read_level2_codes();
    dbg!(&level2);
    // Place names (Bothamsall, New York)
    let places = read_places(&countries);
    for p in &places {
        print_place(p, &level1, &level2);
    }
}

fn print_place(place: &Place, level1: &HashMap<String, String>, level2: &HashMap<String, String>) {
    let name = &place.name;
    let state = level1
        .get(&format!("{}.{}", place.country.iso_code, place.admin1))
        .unwrap();
    let county = level2
        .get(&format!(
            "{}.{}.{}",
            place.country.iso_code, place.admin1, place.admin2
        ))
        .unwrap();

    /*
    Marseille, Bouches-du-Rhône, Provence-Alpes-Côte d'Azur, France
    Marseille, Bouches-du-Rhône, Provence-Alpes-Côte d'Azur, FR
    Bothamsall, Nottinghamshire, England, United Kingdom
    Bothamsall, Nottinghamshire, England, GB
    Houston, Winston County, Alabama, United States
    Houston, Winston County, Alabama, US
    */
    //println!("{name}, {county}, {state}, {}", place.country.name);
    println!("{name}, {county}, {state}, {}", place.country.iso_code);
}

/// Returns a mapping of ISOCode -> Country, e.g. "GB" -> { "GB", "United
/// Kingdom", Europe }. This is read from the file "countryInfo.txt".
fn read_countries() -> HashMap<String, Country> {
    let mut countries = HashMap::new();

    let f = File::open("/home/phil/repos/mine/gapix/geonames/assets/countryInfo.txt").unwrap();
    let rdr = BufReader::new(f);
    for line in rdr.lines() {
        let line = line.unwrap();
        if line.starts_with('#') {
            continue;
        }

        let fields: Vec<_> = line.split('\t').collect();
        let iso_code = fields[0];
        let name = fields[4];
        let continent_code = fields[8];

        let country = Country {
            iso_code: iso_code.to_string(),
            name: name.to_string(),
            continent: Continent::try_from(continent_code).unwrap(),
        };

        countries.insert(iso_code.to_string(), country);
    }

    countries
}

/// Returns a mapping of "GB.ENG" -> "England", or "US.TX" -> "Texas". This is
/// read from the file "admin1CodesASCII.txt".
fn read_level1_codes() -> HashMap<String, String> {
    let mut result = HashMap::new();

    let f = File::open("/home/phil/repos/mine/gapix/geonames/assets/admin1CodesASCII.txt").unwrap();
    let rdr = BufReader::new(f);
    for line in rdr.lines() {
        let line = line.unwrap();
        let fields: Vec<_> = line.split('\t').collect();
        let key = fields[0];
        let name = fields[1];
        result.insert(key.to_string(), name.to_string());
    }

    result
}

/// Returns a mapping of "GB.ENG.J9" -> "Nottinghamshire". This is read from the
/// file "admin2Codes.txt".
fn read_level2_codes() -> HashMap<String, String> {
    let mut result = HashMap::new();

    let f = File::open("/home/phil/repos/mine/gapix/geonames/assets/admin2Codes.txt").unwrap();
    let rdr = BufReader::new(f);
    for line in rdr.lines() {
        let line = line.unwrap();
        let fields: Vec<_> = line.split('\t').collect();
        let key = fields[0];
        let name = fields[1];
        result.insert(key.to_string(), name.to_string());
    }

    result
}

/// Returns a list of all places. This is read from the file "allCountries.txt".
fn read_places(countries: &HashMap<String, Country>) -> Vec<Place> {
    let mut result = Vec::new();

    let f = File::open("/home/phil/repos/mine/gapix/geonames/assets/allCountries.txt").unwrap();
    let rdr = BufReader::new(f);
    for line in rdr.lines() {
        let line = line.unwrap();
        let fields: Vec<_> = line.split('\t').collect();
        let geoname_id: u32 = fields[0].parse().unwrap();
        let mut name = fields[1].to_string();
        if name.is_empty() {
            name = fields[2].to_string();
        }
        let lat: f64 = fields[4].parse().unwrap();
        let lon: f64 = fields[5].parse().unwrap();
        let _feature_class = fields[6];
        let _feature_code = fields[7];
        let country_code = fields[8];
        let country = match countries.get(country_code) {
            Some(c) => c.clone(),
            None => {
                //eprintln!("Unknown country code '{country_code}'");
                continue;
            }
        };

        let admin1 = fields[10];
        let admin2 = fields[11];
        let timezone = fields[17];

        if !((country_code == "FR" && name == "Marseille")
            || (country_code == "GB" && name == "Bothamsall")
            || (country_code == "US" && name == "Dallas"))
        {
            continue;
        }

        let p = Place {
            geoname_id,
            name,
            lat,
            lon,
            country: country.clone(),
            admin1: admin1.to_string(),
            admin2: admin2.to_string(),
            timezone: timezone.to_string(),
        };

        dbg!(&p);
        result.push(p);
    }

    result
}
