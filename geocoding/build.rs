use std::{
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
};

fn main() {
    // Tell Cargo that if the given file changes, to rerun this build script.
    println!("cargo::rerun-if-changed=assets");
    let countries = pre_process_country_info_file();
    let geo_filter = get_geo_filter(countries);
    pre_process_admin1codes_file(&geo_filter);
    pre_process_admin2codes_file(&geo_filter);
    pre_process_all_countries_file(&geo_filter);
}

fn get_geo_filter(countries: Vec<Country>) -> GeoFilter {
    let ctry_list = std::env::var_os("GAPIX_COUNTRIES")
        .unwrap_or_default()
        .to_string_lossy()
        .split(',')
        .filter_map(|s| {
            if s.is_empty() {
                None
            } else {
                Some(s.to_owned())
            }
        })
        .collect();

    let cont_list: Vec<String> = std::env::var_os("GAPIX_CONTINENTS")
        .unwrap_or_default()
        .to_string_lossy()
        .split(',')
        .filter_map(|s| {
            if s.is_empty() {
                None
            } else {
                Some(s.to_owned())
            }
        })
        .collect();

    GeoFilter {
        required_countries: ctry_list,
        required_continents: cont_list,
        country_list: countries,
    }
}

fn pre_process_all_countries_file(geo_filter: &GeoFilter) {
    let src_path = input_path("allCountries.txt");
    let src_file = File::open(src_path).unwrap();
    let dest_path = output_path("allCountries.txt");
    println!("Writing output file {dest_path:?}");
    let dest_file = File::create(dest_path).unwrap();
    let mut writer = BufWriter::new(dest_file);
    let rdr = BufReader::new(src_file);

    for line in rdr.lines() {
        let line = line.unwrap();
        let fields: Vec<_> = line.split('\t').collect();
        let isocode = fields[8];
        if !geo_filter.include_country(isocode) {
            continue;
        }

        let mut name = fields[1].to_string();
        if name.is_empty() {
            name = fields[2].to_string();
        }
        let lat: f64 = fields[4].parse().unwrap();
        let lon: f64 = fields[5].parse().unwrap();
        let _feature_class = fields[6];
        let _feature_code = fields[7];

        let admin1 = fields[10];
        let admin2 = fields[11];
        let timezone = fields[17];

        if name.is_empty() || isocode.is_empty() {
            //println!("allCountries.txt: Skipping line due to empty name or country_code. name={name}, isocode={isocode}, admin1={admin1}, admin2={admin2}, timezone={timezone}");
        } else {
            writeln!(
                &mut writer,
                "{name}\t{lat}\t{lon}\t{isocode}\t{admin1}\t{admin2}\t{timezone}"
            )
            .unwrap();
        }
    }
}

fn pre_process_admin2codes_file(geo_filter: &GeoFilter) {
    let src_path = input_path("admin2Codes.txt");
    let src_file = File::open(src_path).unwrap();
    let dest_path = output_path("admin2Codes.txt");
    println!("Writing output file {dest_path:?}");
    let dest_file = File::create(dest_path).unwrap();
    let mut writer = BufWriter::new(dest_file);
    let rdr = BufReader::new(src_file);

    for line in rdr.lines() {
        let line = line.unwrap();
        let fields: Vec<_> = line.split('\t').collect();
        let key = fields[0];
        let isocode = &key[0..2];
        if !geo_filter.include_country(isocode) {
            continue;
        }

        let name = fields[1];
        if key.is_empty() || name.is_empty() {
            println!("Admin2Codes.txt: Skipping line due to one or more empty fields. key={key}, name={name}");
        } else {
            writeln!(&mut writer, "{key}\t{name}").unwrap();
        }
    }
}

fn pre_process_admin1codes_file(geo_filter: &GeoFilter) {
    let src_path = input_path("admin1CodesASCII.txt");
    let src_file = File::open(src_path).unwrap();
    let dest_path = output_path("admin1CodesASCII.txt");
    println!("Writing output file {dest_path:?}");
    let dest_file = File::create(dest_path).unwrap();
    let mut writer = BufWriter::new(dest_file);
    let rdr = BufReader::new(src_file);

    for line in rdr.lines() {
        let line = line.unwrap();
        let fields: Vec<_> = line.split('\t').collect();
        let key = fields[0];
        let isocode = &key[0..2];
        if !geo_filter.include_country(isocode) {
            continue;
        }

        let name = fields[1];
        if key.is_empty() || name.is_empty() {
            println!("Admin1Codes.txt: Skipping line due to one or more empty fields. key={key}, name={name}");
        } else {
            writeln!(&mut writer, "{key}\t{name}").unwrap();
        }
    }
}

fn pre_process_country_info_file() -> Vec<Country> {
    let mut countries = Vec::new();

    let src_path = input_path("countryInfo.txt");
    let src_file = File::open(src_path).unwrap();
    let dest_path = output_path("countryInfo.txt");
    println!("Writing output file {dest_path:?}");
    let dest_file = File::create(dest_path).unwrap();
    let mut writer = BufWriter::new(dest_file);
    let rdr = BufReader::new(src_file);

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
            println!("CountryInfo.txt: Skipping line due to one or more empty fields. iso_code={iso_code}, name={name}, continent_code={continent_code}");
        } else {
            writeln!(&mut writer, "{iso_code}\t{name}\t{continent_code}").unwrap();

            countries.push(Country {
                iso_code: iso_code.to_string(),
                name: name.to_string(),
                continent: Continent::try_from(continent_code).unwrap(),
            });
        }
    }

    countries
}

/// Returns the output path for a particular filename.
fn output_path<P: AsRef<Path>>(filename: P) -> PathBuf {
    let out_dir = std::env::var_os("OUT_DIR").unwrap();
    Path::new(&out_dir).join(filename)
}

/// Returns the input path for a particular filename.
fn input_path<P: AsRef<Path>>(filename: P) -> PathBuf {
    assets_dir().join(filename)
}

/// Returns the location of the assets directory.
fn assets_dir() -> PathBuf {
    let dir = std::env::var_os("CARGO_MANIFEST_DIR").unwrap();
    Path::new(&dir).join("assets")
}

#[derive(Debug)]
struct GeoFilter {
    required_countries: Vec<String>,
    required_continents: Vec<String>,
    country_list: Vec<Country>,
}

impl GeoFilter {
    fn include_country(&self, isocode: &str) -> bool {
        // No filter was set.
        if self.required_countries.is_empty() && self.required_continents.is_empty() {
            return true;
        }

        // Is the list of required countries set and contains it?
        if !self.required_countries.is_empty() {
            if self.required_countries.iter().any(|rc| rc == isocode) {
                return true;
            }
        }

        // Is the list of required continents set and contains that country?
        if !self.required_continents.is_empty() {
            if let Some(country) = self.country_list.iter().find(|c| c.iso_code == isocode) {
                for rc in &self.required_continents {
                    if rc == country.continent.as_str() {
                        return true;
                    }
                }
            }
        }

        false
    }
}

#[derive(Debug, Copy, Clone)]
enum Continent {
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
    fn as_str(&self) -> &'static str {
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
struct Country {
    iso_code: String,
    name: String,
    continent: Continent,
}
