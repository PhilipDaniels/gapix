use std::{
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
};

fn main() {
    // Tell Cargo that if the given file changes, to rerun this build script.
    println!("cargo::rerun-if-changed=assets");

    pre_process_country_info_file();
    pre_process_admin1codes_file();
    pre_process_admin2codes_file();
    pre_process_all_countries_file();
}

fn pre_process_all_countries_file() {
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
        let _geoname_id: u32 = fields[0].parse().unwrap();
        let mut name = fields[1].to_string();
        if name.is_empty() {
            name = fields[2].to_string();
        }
        let lat: f64 = fields[4].parse().unwrap();
        let lon: f64 = fields[5].parse().unwrap();
        let _feature_class = fields[6];
        let _feature_code = fields[7];
        let country_code = fields[8];
        let admin1 = fields[10];
        let admin2 = fields[11];
        let timezone = fields[17];

        if name.is_empty() || country_code.is_empty() {
            println!("allCountries.txt: Skipping line due to empty name or country_code. name={name}, country_code={country_code}, admin1={admin1}, admin2={admin2}, timezone={timezone}");
        } else {
            writeln!(
                &mut writer,
                "{name}\t{lat}\t{lon}\t{country_code}\t{admin1}\t{admin2}\t{timezone}"
            )
            .unwrap();
        }
    }
}

fn pre_process_admin2codes_file() {
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
        let name = fields[1];
        if key.is_empty() || name.is_empty() {
            println!("Admin2Codes.txt: Skipping line due to one or more empty fields. key={key}, name={name}");
        } else {
            writeln!(&mut writer, "{key}\t{name}").unwrap();
        }
    }
}

fn pre_process_admin1codes_file() {
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
        let name = fields[1];
        if key.is_empty() || name.is_empty() {
            println!("Admin1Codes.txt: Skipping line due to one or more empty fields. key={key}, name={name}");
        } else {
            writeln!(&mut writer, "{key}\t{name}").unwrap();
        }
    }
}

fn pre_process_country_info_file() {
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
