use core::{panic, str};
use model::Trackpoint;
use quick_xml::reader::Reader;
use quick_xml::{
    events::{attributes::Attribute, BytesStart, Event},
    name::QName,
    Writer,
};
use std::borrow::Cow;
use std::{
    fs::{read_dir, File},
    io::BufWriter,
    path::{Path, PathBuf},
};

mod model;

fn main() {
    let exe_dir = get_exe_dir();
    let input_files = get_list_of_input_files(&exe_dir);
    if input_files.is_empty() {
        println!("No .gpx files found");
        return;
    }

    for f in input_files {
        simplify(&f);
    }
}

fn simplify(input_file: &Path) {
    let mut output_file = input_file.to_owned();
    output_file.set_extension("simplified.gpx");
    if output_file.exists() {
        println!(
            "Simplified file {:?} already exists, skipping...",
            &output_file
        );
        return;
    } else {
        println!("Writing file {:?}", &output_file);
    }

    // Reading.
    let mut reader = Reader::from_file(input_file).expect("Could not create XML reader");
    let mut buf = Vec::with_capacity(8096);
    // Writing.
    let bw = BufWriter::new(File::create(&output_file).expect("Could not open output_file"));
    let mut writer = Writer::new_with_indent(bw, b' ', 2);

    let mut trackpoints = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Decl(decl)) => {
                writer.write_event(Event::Decl(decl)).unwrap();
            }
            Ok(Event::Start(e)) => {
                match e.name().as_ref() {
                    b"gpx" => {
                        println!("Found the gpx tag");
                        writer.write_event(Event::Start(e)).unwrap();
                    }
                    b"trk" => {
                        println!("Found a trk tag");
                        writer.write_event(Event::Start(e)).unwrap();
                    }
                    b"trkseg" => {
                        println!("Found a trkseg tag");
                        writer.write_event(Event::Start(e)).unwrap();
                    }
                    b"trkpt" => {
                        let lat = get_f32_attr(&e, "lat");
                        let lon = get_f32_attr(&e, "lon");
                        let ele: f32;
                        let time: String;

                        let eot1 = read_ele_or_time(&mut reader, &mut buf);
                        let eot2 = read_ele_or_time(&mut reader, &mut buf);

                        match (eot1, eot2) {
                            (EleOrTime::ele(e), EleOrTime::time(t)) => {
                                ele = e;
                                time = t;
                            }
                            (EleOrTime::time(t), EleOrTime::ele(e)) => {
                                ele = e;
                                time = t;
                            }
                            _ => panic!("Did not get both the <ele> and <time> tags"),
                        }

                        let tp = Trackpoint {
                            lat,
                            lon,
                            ele,
                            time,
                        };

                        println!("{:?}", tp);
                        trackpoints.push(tp);

                        // // The lat and lon attributes have an absurd number of decimal places.
                        // // Only 6 d.p. are needed to be precise to 11cm.
                        // // See https://en.wikipedia.org/wiki/Decimal_degrees
                        // let lat = e.try_get_attribute("lat").unwrap().unwrap().value;
                        // let trimmed_lat = trim_dp(&lat);
                        // let trimmed_lat = make_attr("lat", trimmed_lat);

                        // let lon = e.try_get_attribute("lon").unwrap().unwrap().value;
                        // let trimmed_lon = trim_dp(&lon);
                        // let trimmed_lon = make_attr("lon", trimmed_lon);

                        // let mut e2 = BytesStart::new("trkpt");
                        // e2.push_attribute(trimmed_lat);
                        // e2.push_attribute(trimmed_lon);

                        // writer.write_event(Event::Start(e2)).unwrap();
                    }
                    b"ele" => {
                        // Read again to get the text inside the <ele>...</ele> tags.
                        match reader.read_event_into(&mut buf) {
                            Ok(Event::Text(t)) => {
                                writer.create_element("ele").write_text_content(t).unwrap();
                            }
                            _ => panic!("Got unexpected XML node, document is probably corrupt"),
                        }
                    }
                    b"time" => {
                        // Read again to get the text inside the <time>...</time> tags.
                        match reader.read_event_into(&mut buf) {
                            Ok(Event::Text(t)) => {
                                writer.create_element("time").write_text_content(t).unwrap();
                            }
                            _ => panic!("Got unexpected XML node, document is probably corrupt"),
                        }
                    }
                    _ => (),
                }
            }
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"gpx" => {
                    writer.write_event(Event::End(e)).unwrap();
                }
                b"trk" => {
                    writer.write_event(Event::End(e)).unwrap();
                }
                b"trkseg" => {
                    writer.write_event(Event::End(e)).unwrap();
                }
                b"trkpt" => {
                    writer.write_event(Event::End(e)).unwrap();
                }
                _ => (),
            },
            Ok(Event::Eof) => break,
            Err(e) => panic!("Error at position {}: {:?}", reader.error_position(), e),
            _ => (),
        }

        buf.clear();
    }

    println!("Found {} trkpt nodes", trackpoints.len());
}

enum EleOrTime {
    ele(f32),
    time(String),
}

/// Read the <ele> or <time> sub-node. I am not assuming which comes first in the file,
/// (so we return an enum) but I am assuming they are the first sub-nodes, e.g. before
/// any <extensions>.
fn read_ele_or_time(reader: &mut Reader<std::io::BufReader<File>>, buf: &mut Vec<u8>) -> EleOrTime {
    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(e)) => {
                match e.name().as_ref() {
                    b"ele" => {
                        // Read again to get the text inside the <ele>...</ele> tags.
                        match reader.read_event_into(buf) {
                            Ok(Event::Text(ele)) => {
                                let ele = ele.as_ref();
                                let ele = str::from_utf8(ele).expect("The bytes should be ASCII, therefore valid UTF-8");
                                let ele: f32 = ele.parse().expect("The string should be a valid number");
                                // All the ele's will come out to 1 d.p., because that is all that my Garmin
                                // Edge 1040 can actually manage, even though it records them as
                                // "151.1999969482421875" or "149.8000030517578125".
                                return EleOrTime::ele(ele)
                            }
                            _ => panic!("Got unexpected XML node, document is probably corrupt"),
                        }
                    }
                    b"time" => {
                        // Read again to get the text inside the <time>...</time> tags.
                        match reader.read_event_into(buf) {
                            Ok(Event::Text(time)) => {
                                let time = time.as_ref();
                                let time = String::from_utf8_lossy(time).into_owned();
                                return EleOrTime::time(time)
                            }
                            _ => panic!("Got unexpected XML node, document is probably corrupt"),
                        }
                    }
                    _ => panic!("Unexpected element")
                }
            }
            _ => {}
        }
    }
}

fn get_f32_attr(e: &BytesStart, arg: &str) -> f32 {
    let lat2 = e
        .try_get_attribute(arg)
        .expect("Unless the file is corrupt the attributes we asl for always exist")
        .expect("And always have values")
        .value;
    let lat2 = lat2.as_ref();
    let lat2 = str::from_utf8(lat2).expect("The bytes should be ASCII, therefore valid UTF-8");
    let lat2: f32 = lat2.parse().expect("The string should be a valid number");
    lat2
}

// Get a list of all files in the exe_dir that have the ".gpx" extension.
// Be careful to exclude files that actually end in ".simplified.gpx" -
// they are output files we already created! If we don't exclude them here,
// we end up generating ".simplified.simplified.gpx", etc.
fn get_list_of_input_files(exe_dir: &PathBuf) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = read_dir(exe_dir) else {
        return Vec::new();
    };

    for entry in entries {
        let entry = entry.unwrap();
        let meta = entry.metadata().unwrap();
        if meta.is_file() {
            let s = &entry.file_name();
            let p = Path::new(s);
            if let Some(ext) = p.extension() {
                if ext.to_ascii_lowercase() == "gpx" {
                    let s = s.to_string_lossy().to_ascii_lowercase();
                    if !s.ends_with(".simplified.gpx") {
                        println!("Found GPX input file {:?}", entry.path());
                        files.push(entry.path());
                    }
                }
            }
        }
    }

    files
}

fn get_exe_dir() -> PathBuf {
    let mut exe_path = std::env::current_exe().unwrap();
    exe_path.pop();
    exe_path
}

fn trim_dp(v: &[u8]) -> &[u8] {
    let mut idx = 0;
    while v[idx] != b'.' {
        idx += 1;
    }

    &v[0..idx + 7]
}

// fn trim_dp<'a>(v: &'a [u8]) -> Cow<'a, [u8]> {
//     let mut idx = 0;
//     while v[idx] != b'.' {
//         idx += 1;
//     }

//     Cow::Borrowed(&v[0..idx + 7])
// }

fn make_attr<'a, 'b: 'a>(name: &'b str, value: &'a [u8]) -> Attribute<'a> {
    Attribute {
        key: QName(name.as_bytes()),
        value: Cow::Borrowed(value),
    }
}
