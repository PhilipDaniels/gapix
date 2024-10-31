use std::{
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
struct OwnedCountry {
    iso_code: String,
    continent: String
}

fn main() {
    // Tell Cargo that if the given file changes, to rerun this build script.
    println!("cargo::rerun-if-changed=assets");
    let countries = pre_process_country_info_file();
    let geo_filter = get_geo_filter(countries);
    pre_process_admin1codes_file(&geo_filter);
    pre_process_admin2codes_file(&geo_filter);
    pre_process_all_countries_file(&geo_filter);
}

fn pre_process_country_info_file() -> Vec<OwnedCountry> {
    let mut countries = Vec::new();

    let src_path = input_path("countryInfo.txt");
    let src_file = File::open(&src_path).unwrap();
    let rdr = BufReader::new(src_file);

    let dest_path = output_path("countries.rs");
    let dest_file = File::create(&dest_path).unwrap();
    let mut writer = BufWriter::new(dest_file);
    println!("Processing input file {src_path:?} to {dest_path:?}");

    writeln!(
        &mut writer,
        "static COUNTRIES: Map<&'static str, crate::Country> = phf_map! {{"
    )
    .unwrap();

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
            //let continent = Continent::try_from(continent_code).unwrap();
            let continent_str = match continent_code {
                "AF" => "Continent::Africa",
                "AS" => "Continent::Asia",
                "EU" => "Continent::Europe",
                "NA" => "Continent::NorthAmerica",
                "OC" => "Continent::Oceania",
                "SA" => "Continent::SouthAmerica",
                "AN" => "Continent::Antarctica",
                _ => panic!("Unknown continent {continent_code}"),
            };

            writeln!(&mut writer, r#"    "{iso_code}" => Country {{ iso_code: "{iso_code}", name: "{name}", continent: {continent_str} }},"#).unwrap();

            countries.push(OwnedCountry {
                iso_code: iso_code.to_string(),
                continent: continent_code.to_string(),
            });
        }
    }

    writeln!(&mut writer, "}};").unwrap();

    countries
}

fn pre_process_admin1codes_file(geo_filter: &GeoFilter) {
    let src_path = input_path("admin1CodesASCII.txt");
    let src_file = File::open(&src_path).unwrap();
    let rdr = BufReader::new(src_file);

    let dest_path = output_path("admin1CodesASCII.rs");
    let dest_file = File::create(&dest_path).unwrap();
    let mut writer = BufWriter::new(dest_file);
    println!("Processing input file {src_path:?} to {dest_path:?}");

    writeln!(&mut writer, "use ::phf::{{Map, phf_map}};").unwrap();
    writeln!(&mut writer).unwrap();

    writeln!(
        &mut writer,
        "static ADMIN_1_CODES: Map<&'static str, &'static str> = phf_map! {{"
    )
    .unwrap();

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
            writeln!(&mut writer, r#"    "{key}" => "{name}","#).unwrap();
        }
    }

    writeln!(&mut writer, "}};").unwrap();
}

fn pre_process_admin2codes_file(geo_filter: &GeoFilter) {
    let src_path = input_path("admin2Codes.txt");
    let src_file = File::open(&src_path).unwrap();
    let rdr = BufReader::new(src_file);

    let dest_path = output_path("admin2Codes.rs");
    let dest_file = File::create(&dest_path).unwrap();
    let mut writer = BufWriter::new(dest_file);
    println!("Processing input file {src_path:?} to {dest_path:?}");

    writeln!(&mut writer).unwrap();

    writeln!(
        &mut writer,
        "static ADMIN_2_CODES: Map<&'static str, &'static str> = phf_map! {{"
    )
    .unwrap();

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
            writeln!(&mut writer, r#"    "{key}" => "{name}","#).unwrap();
        }
    }

    writeln!(&mut writer, "}};").unwrap();
}

fn get_geo_filter(countries: Vec<OwnedCountry>) -> GeoFilter {
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
    country_list: Vec<OwnedCountry>,
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
                    if *rc == country.continent {
                        return true;
                    }
                }
            }
        }

        false
    }
}
