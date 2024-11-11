use std::{
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
};

/// This program builds the places.txt file. It includes all places,
/// there is no filtering. The resultant file is about 700MB. Not done
/// in build.rs because doing that tends to crash Rust Analyzer.
fn main() {
    pre_process_all_countries_file();
}

fn pre_process_all_countries_file() {
    let src_path = input_path("allCountries.txt");
    let src_file = File::open(&src_path).unwrap();
    let rdr = BufReader::new(src_file);
    let dest_path = output_path("places.txt");
    println!("Writing output file {dest_path:?}");
    let dest_file = File::create(&dest_path).unwrap();
    let mut writer = BufWriter::new(dest_file);

    for line in rdr.lines() {
        let line = line.unwrap();
        let fields: Vec<_> = line.split('\t').collect();
        let mut name = fields[1].to_string();
        if name.is_empty() {
            name = fields[2].to_string();
        }
        let iso_code = fields[8];
        if name.is_empty() || iso_code.is_empty() {
            continue;
        }

        let lat: f64 = fields[4].parse().unwrap();
        let lon: f64 = fields[5].parse().unwrap();
        let _feature_class = fields[6];
        let _feature_code = fields[7];

        let admin1 = fields[10];
        let admin2 = fields[11];
        let timezone = fields[17];

        let name = name.replace("\"", "\\\"");

        writeln!(
            &mut writer,
            "{name}\t{lat}\t{lon}\t{iso_code}\t{admin1}\t{admin2}\t{timezone}"
        )
        .unwrap();
    }

    writer.flush().unwrap();
}

/// Returns the output path for a particular filename.
fn output_path<P: AsRef<Path>>(filename: P) -> PathBuf {
    let out_dir = get_exe_dir();
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

fn get_exe_dir() -> PathBuf {
    let mut exe_path = std::env::current_exe().unwrap();
    exe_path.pop();
    exe_path
}
