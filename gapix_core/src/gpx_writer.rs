use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

use indent_write::io::IndentWriter;
use log::info;
use logging_timer::time;

use crate::{
    byte_counter::ByteCounter,
    dates::format_utc_date,
    error::GapixError,
    model::{
        Copyright, Email, Extensions, Gpx, Link, Metadata, Person, Route, Track, TrackSegment,
        Waypoint, XmlDeclaration,
    },
};

/// Controls what values get written when writing a GPX.
#[derive(Debug, Copy, Clone)]
pub enum OutputOptions {
    /// Writes all the detail that exists in the model.
    Full,
    /// Writes just enough data to be a valid Audax UK DIY submission. This will
    /// exclude GPX-level waypoints, routes, extensions and waypoints within
    /// tracks (aka trackpoints) will have the minimal required information
    /// instead of full detail. Most other things, such as the 'metadata', will
    /// be fully written because they are a small part of the overall file size
    /// (which is dominated by trackpoints).
    AudaxUKDIY,
}

impl Default for OutputOptions {
    /// Returns OutputOptions::Full.
    fn default() -> Self {
        Self::Full
    }
}

/// Writes a GPX to file with full-fidelity, i.e. everything we can write is
/// written.
pub fn write_gpx_to_file<P: AsRef<Path>>(
    output_file: P,
    gpx: &Gpx,
    options: OutputOptions,
) -> Result<(), GapixError> {
    let output_file = output_file.as_ref();

    let file = match File::create(output_file) {
        Ok(f) => f,
        Err(err) => {
            return Err(GapixError::CreateFile {
                path: output_file.to_owned(),
                source: err,
            })
        }
    };

    let w = BufWriter::new(file);
    let mut w = ByteCounter::new(w);
    write_gpx_to_writer(&mut w, gpx, options)?;
    info!(
        "Wrote GPX file {:?}, {} Kb",
        output_file,
        w.bytes_written() / 1024
    );
    Ok(())
}

/// Writes a GPX to the specified writer with full-fidelity, i.e. everything we
/// can write is written.
#[time]
pub fn write_gpx_to_writer<W: Write>(
    w: &mut W,
    gpx: &Gpx,
    output_options: OutputOptions,
) -> Result<(), GapixError> {
    let mut w = IndentWriter::new("  ", w);
    write_declaration(&mut w, &gpx.declaration)?;
    write_gpx_open(&mut w, gpx)?;
    w.indent();
    write_metadata(&mut w, &gpx.metadata)?;

    match output_options {
        OutputOptions::Full => {
            for wp in &gpx.waypoints {
                write_waypoint(&mut w, wp, "wpt", output_options)?;
            }
            for route in &gpx.routes {
                write_route(&mut w, route)?;
            }
        }
        OutputOptions::AudaxUKDIY => { /* Waypoints and routes not needed */ }
    };

    for track in &gpx.tracks {
        write_track(&mut w, track, output_options)?;
    }

    match output_options {
        OutputOptions::Full => {
            write_extensions(&mut w, &gpx.extensions)?;
        }
        OutputOptions::AudaxUKDIY => { /* Extensions not needed */ }
    }

    w.outdent();
    writeln!(w, "</gpx>")?;

    w.flush()?;
    Ok(())
}

fn write_declaration<W: Write>(w: &mut W, declaration: &XmlDeclaration) -> Result<(), GapixError> {
    write!(w, "<?xml version=\"{}\"", declaration.version)?;
    if let Some(encoding) = &declaration.encoding {
        write!(w, " encoding=\"{}\"", encoding)?;
    }
    if let Some(standalone) = &declaration.standalone {
        write!(w, " standalone=\"{}\"", standalone)?;
    }
    writeln!(w, "?>")?;
    Ok(())
}

fn write_gpx_open<W: Write>(w: &mut W, info: &Gpx) -> Result<(), GapixError> {
    writeln!(
        w,
        "<gpx creator=\"{}\" version=\"{}\"",
        info.creator, info.version
    )?;
    for (key, value) in &info.attributes {
        writeln!(w, "  {}=\"{}\"", key, value)?;
    }
    writeln!(w, ">")?;
    Ok(())
}

fn write_metadata<W: Write>(
    w: &mut IndentWriter<W>,
    metadata: &Metadata,
) -> Result<(), GapixError> {
    writeln!(w, "<metadata>")?;
    w.indent();
    if let Some(name) = &metadata.name {
        writeln!(w, "<name>{}</name>", name)?;
    }
    if let Some(desc) = &metadata.description {
        writeln!(w, "<desc>{}</desc>", desc)?;
    }
    if let Some(author) = &metadata.author {
        write_person(w, author, "author")?;
    }
    if let Some(copyright) = &metadata.copyright {
        write_copyright(w, copyright)?;
    }
    for link in &metadata.links {
        write_link(w, link)?;
    }
    if let Some(time) = &metadata.time {
        writeln!(w, "<time>{}</time>", format_utc_date(time))?;
    }
    if let Some(keywords) = &metadata.keywords {
        writeln!(w, "<keywords>{}</keywords>", keywords)?;
    }
    if let Some(bounds) = &metadata.bounds {
        writeln!(
            w,
            "<bounds minlat=\"{}\" maxlat=\"{}\" minlon=\"{}\" maxlon=\"{}\"/>",
            bounds.min_lat, bounds.max_lat, bounds.min_lon, bounds.max_lon
        )?;
    }
    write_extensions(w, &metadata.extensions)?;

    w.outdent();
    writeln!(w, "</metadata>")?;
    Ok(())
}

fn write_person<W: Write>(
    w: &mut IndentWriter<W>,
    person: &Person,
    element_name: &str,
) -> Result<(), GapixError> {
    writeln!(w, "<{}>", element_name)?;
    w.indent();
    if let Some(name) = &person.name {
        writeln!(w, "<name>{}</name>", name)?;
    }
    if let Some(email) = &person.email {
        write_email(w, email)?;
    }
    if let Some(link) = &person.link {
        write_link(w, link)?;
    }
    w.outdent();
    writeln!(w, "</{}>", element_name)?;
    Ok(())
}

fn write_copyright<W: Write>(
    w: &mut IndentWriter<W>,
    copyright: &Copyright,
) -> Result<(), GapixError> {
    writeln!(w, "<copyright>")?;
    w.indent();
    if let Some(year) = &copyright.year {
        writeln!(w, "<year>{}</year>", year)?;
    }
    if let Some(license) = &copyright.license {
        writeln!(w, "<license>{}</license>", license)?;
    }
    writeln!(w, "<author>{}</author>", copyright.author)?;
    w.outdent();
    writeln!(w, "</copyright>")?;
    Ok(())
}

fn write_email<W: Write>(w: &mut W, email: &Email) -> Result<(), GapixError> {
    writeln!(
        w,
        "<email id=\"{}\" domain=\"{}\" />",
        email.id, email.domain
    )?;
    Ok(())
}

fn write_link<W: Write>(w: &mut IndentWriter<W>, link: &Link) -> Result<(), GapixError> {
    writeln!(w, "<link href=\"{}\">", link.href)?;
    w.indent();
    if let Some(text) = &link.text {
        writeln!(w, "<text>{}</text>", text)?;
    }
    if let Some(r#type) = &link.r#type {
        writeln!(w, "<type>{}</type>", r#type)?;
    }
    w.outdent();
    writeln!(w, "</link>")?;
    Ok(())
}

fn write_route<W: Write>(w: &mut IndentWriter<W>, route: &Route) -> Result<(), GapixError> {
    writeln!(w, "<rte>")?;
    w.indent();
    if let Some(name) = &route.name {
        writeln!(w, "<name>{}</name>", name)?;
    }
    if let Some(comment) = &route.comment {
        writeln!(w, "<cmt>{}</cmt>", comment)?;
    }
    if let Some(desc) = &route.description {
        writeln!(w, "<desc>{}</desc>", desc)?;
    }
    if let Some(source) = &route.source {
        writeln!(w, "<src>{}</src>", source)?;
    }
    for link in &route.links {
        write_link(w, link)?;
    }
    if let Some(number) = &route.number {
        writeln!(w, "<number>{}</number>", number)?;
    }
    if let Some(route_type) = &route.r#type {
        writeln!(w, "<type>{}</type>", route_type)?;
    }
    write_extensions(w, &route.extensions)?;
    for pt in &route.points {
        write_waypoint(w, pt, "rtept", OutputOptions::Full)?;
    }
    w.outdent();
    writeln!(w, "</rte>")?;
    Ok(())
}

fn write_track<W: Write>(
    w: &mut IndentWriter<W>,
    track: &Track,
    output_options: OutputOptions,
) -> Result<(), GapixError> {
    writeln!(w, "<trk>")?;
    w.indent();
    if let Some(name) = &track.name {
        writeln!(w, "<name>{}</name>", name)?;
    }
    if let Some(comment) = &track.comment {
        writeln!(w, "<cmt>{}</cmt>", comment)?;
    }
    if let Some(desc) = &track.description {
        writeln!(w, "<desc>{}</desc>", desc)?;
    }
    if let Some(source) = &track.source {
        writeln!(w, "<src>{}</src>", source)?;
    }
    for link in &track.links {
        write_link(w, link)?;
    }
    if let Some(number) = &track.number {
        writeln!(w, "<number>{}</number>", number)?;
    }
    if let Some(track_type) = &track.r#type {
        writeln!(w, "<type>{}</type>", track_type)?;
    }
    write_extensions(w, &track.extensions)?;
    for segment in &track.segments {
        write_track_segment(w, segment, output_options)?;
    }
    w.outdent();
    writeln!(w, "</trk>")?;
    Ok(())
}

fn write_track_segment<W: Write>(
    w: &mut IndentWriter<W>,
    segment: &TrackSegment,
    output_options: OutputOptions,
) -> Result<(), GapixError> {
    writeln!(w, "<trkseg>")?;
    w.indent();
    for p in &segment.points {
        write_waypoint(w, p, "trkpt", output_options)?;
    }
    write_extensions(w, &segment.extensions)?;
    w.outdent();
    writeln!(w, "</trkseg>")?;
    Ok(())
}

fn write_waypoint<W: Write>(
    w: &mut IndentWriter<W>,
    point: &Waypoint,
    element_name: &str,
    output_options: OutputOptions,
) -> Result<(), GapixError> {
    match output_options {
        OutputOptions::AudaxUKDIY => {
            writeln!(
                w,
                "<{element_name} lat=\"{:.6}\" lon=\"{:.6}\">",
                point.lat, point.lon
            )?;
            w.indent();
            if let Some(ele) = point.ele {
                writeln!(w, "<ele>{:.1}</ele>", ele)?;
            }
            if let Some(t) = point.time {
                writeln!(w, "<time>{}</time>", format_utc_date(&t))?;
            }
            w.outdent();
            writeln!(w, "</{element_name}>")?;
            return Ok(());
        }
        OutputOptions::Full => { /* Drop through */ }
    }

    writeln!(
        w,
        "<{element_name} lat=\"{}\" lon=\"{}\">",
        point.lat, point.lon
    )?;
    w.indent();
    if let Some(ele) = point.ele {
        writeln!(w, "<ele>{}</ele>", ele)?;
    }
    if let Some(t) = point.time {
        writeln!(w, "<time>{}</time>", format_utc_date(&t))?;
    }
    if let Some(magvar) = point.magvar {
        writeln!(w, "<magvar>{}</magvar>", magvar)?;
    }
    if let Some(geoid_height) = point.geoid_height {
        writeln!(w, "<geoidheight>{}</geoidheight>", geoid_height)?;
    }
    if let Some(name) = &point.name {
        writeln!(w, "<name>{name}</name>")?;
    }
    if let Some(comment) = &point.comment {
        writeln!(w, "<cmt>{comment}</cmt>")?;
    }
    if let Some(desc) = &point.description {
        writeln!(w, "<desc>{desc}</desc>")?;
    }
    if let Some(src) = &point.source {
        writeln!(w, "<src>{src}</src>")?;
    }
    for link in &point.links {
        write_link(w, link)?;
    }
    if let Some(sym) = &point.symbol {
        writeln!(w, "<sym>{sym}</sym>")?;
    }
    if let Some(point_type) = &point.r#type {
        writeln!(w, "<type>{point_type}</type>")?;
    }
    if let Some(fix) = &point.fix {
        writeln!(w, "<fix>{fix}</fix>")?;
    }
    if let Some(sat) = &point.num_satellites {
        writeln!(w, "<sat>{sat}</sat>")?;
    }
    if let Some(hdop) = point.hdop {
        writeln!(w, "<hdop>{}</hdop>", hdop)?;
    }
    if let Some(vdop) = point.vdop {
        writeln!(w, "<vdop>{}</vdop>", vdop)?;
    }
    if let Some(pdop) = point.pdop {
        writeln!(w, "<pdop>{}</pdop>", pdop)?;
    }
    if let Some(age) = point.age_of_dgps_data {
        writeln!(w, "<ageofdgpsdata>{}</ageofdgpsdata>", age)?;
    }
    if let Some(id) = point.dgps_id {
        writeln!(w, "<dgpsid>{id}</dgpsid>")?;
    }
    write_extensions(w, &point.extensions)?;
    w.outdent();
    writeln!(w, "</{element_name}>")?;
    Ok(())
}

fn write_extensions<W: Write>(
    w: &mut W,
    extensions: &Option<Extensions>,
) -> Result<(), GapixError> {
    if let Some(ext) = extensions {
        writeln!(w, "<extensions>{}</extensions>", ext.raw_xml)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;

    use super::*;
    use crate::{model::*, read::read_gpx_from_slice};
    use std::{collections::HashMap, iter::zip};

    /// We construct 'gpx1', then write it to a buffer. We then
    /// deserialize the buffer into 'gpx2'. The two models should be identical
    /// if we did everything correctly.
    #[test]
    fn round_trip_entire_model() {
        let gpx1 = make_fully_populated_gpx();
        let mut buffer = Vec::new();
        write_gpx_to_writer(&mut buffer, &gpx1, OutputOptions::Full).unwrap();

        // write_gpx_to_file(
        //     "/home/phil/repos/mine/gapix/target/release/round_trip.gpx",
        //     &gpx1,
        //     OutputOptions::Full,
        // )
        // .unwrap();

        let gpx2 = read_gpx_from_slice(&buffer).unwrap();

        // Filename is not written to or parsed from the buffer.

        assert_eq!(gpx1.declaration, gpx2.declaration);
        assert_eq!(gpx1.version, gpx2.version);
        assert_eq!(gpx1.creator, gpx2.creator);
        assert_eq!(gpx1.attributes, gpx2.attributes);
        compare_metadata(&gpx1.metadata, &gpx2.metadata);
        for (wp1, wp2) in zip(&gpx1.waypoints, &gpx2.waypoints) {
            compare_waypoint(wp1, wp2);
        }
        for (route1, route2) in zip(&gpx1.routes, &gpx2.routes) {
            compare_route(route1, route2);
        }
        for (track1, track2) in zip(&gpx1.tracks, &gpx2.tracks) {
            compare_track(track1, track2);
        }
        assert_eq!(gpx1.extensions, gpx2.extensions);
    }

    fn compare_metadata(md1: &Metadata, md2: &Metadata) {
        assert_eq!(md1.name, md2.name);
        assert_eq!(md1.description, md2.description);
        assert_eq!(md1.author, md2.author);
        assert_eq!(md1.copyright, md2.copyright);
        assert_eq!(md1.links, md2.links);
        assert_eq!(md1.time, md2.time);
        assert_eq!(md1.keywords, md2.keywords);
        let b1 = md1.bounds.as_ref().unwrap();
        let b2 = md2.bounds.as_ref().unwrap();
        assert_eq!(b1.min_lat, b2.min_lat);
        assert_eq!(b1.min_lon, b2.min_lon);
        assert_eq!(b1.max_lat, b2.max_lat);
        assert_eq!(b1.max_lon, b2.max_lon);
        assert_eq!(md1.extensions, md2.extensions);
    }

    fn compare_route(route1: &Route, route2: &Route) {
        assert_eq!(route1.name, route2.name);
        assert_eq!(route1.comment, route2.comment);
        assert_eq!(route1.description, route2.description);
        assert_eq!(route1.source, route2.source);
        assert_eq!(route1.links, route2.links);
        assert_eq!(route1.number, route2.number);
        assert_eq!(route1.r#type, route2.r#type);
        assert_eq!(route1.extensions, route2.extensions);
        for (wp1, wp2) in zip(&route1.points, &route2.points) {
            compare_waypoint(wp1, wp2);
        }
    }
    fn compare_track(track1: &Track, track2: &Track) {
        assert_eq!(track1.name, track2.name);
        assert_eq!(track1.comment, track2.comment);
        assert_eq!(track1.description, track2.description);
        assert_eq!(track1.source, track2.source);
        assert_eq!(track1.links, track2.links);
        assert_eq!(track1.number, track2.number);
        assert_eq!(track1.r#type, track2.r#type);
        assert_eq!(track1.extensions, track2.extensions);
        for (segment1, segment2) in zip(&track1.segments, &track2.segments) {
            assert_eq!(segment1.extensions, segment2.extensions);
            for (wp1, wp2) in zip(&segment1.points, &segment2.points) {
                compare_waypoint(wp1, wp2);
            }
        }
    }

    fn compare_waypoint(wp1: &Waypoint, wp2: &Waypoint) {
        assert_eq!(wp1.lat, wp2.lat);
        assert_eq!(wp1.lon, wp2.lon);
        assert_eq!(wp1.ele, wp2.ele);
        assert_eq!(wp1.time, wp2.time);
        assert_eq!(wp1.magvar, wp2.magvar);
        assert_eq!(wp1.geoid_height, wp2.geoid_height);
        assert_eq!(wp1.name, wp2.name);
        assert_eq!(wp1.comment, wp2.comment);
        assert_eq!(wp1.description, wp2.description);
        assert_eq!(wp1.source, wp2.source);
        assert_eq!(wp1.links, wp2.links);
        assert_eq!(wp1.symbol, wp2.symbol);
        assert_eq!(wp1.r#type, wp2.r#type);
        assert_eq!(wp1.fix, wp2.fix);
        assert_eq!(wp1.num_satellites, wp2.num_satellites);
        assert_eq!(wp1.hdop, wp2.hdop);
        assert_eq!(wp1.vdop, wp2.vdop);
        assert_eq!(wp1.pdop, wp2.pdop);
        assert_eq!(wp1.age_of_dgps_data, wp2.age_of_dgps_data);
        assert_eq!(wp1.dgps_id, wp2.dgps_id);
        assert_eq!(wp1.extensions, wp2.extensions);
    }

    fn make_fully_populated_gpx() -> Gpx {
        let declaration = XmlDeclaration {
            version: "1.0".to_string(),
            encoding: Some("UTF-8".to_string()),
            standalone: Some("false".to_string()),
        };

        let copyright = Copyright {
            year: Some(2024),
            license: Some("MIT".to_string()),
            author: "Homer Simpson".to_string(),
        };

        let author = Person {
            name: Some("First Person".to_string()),
            email: Some(Email {
                id: "first_person".to_string(),
                domain: "gmail.com".to_string(),
            }),
            link: Some(make_link("Author Link", "txt", "http://example.com")),
        };

        let bounds = Bounds::new(10.0, 20.0, 30.0, 40.0).unwrap();

        let metadata = Metadata {
            name: Some("My GPX".to_string()),
            description: Some("My GPX Description".to_string()),
            author: Some(author),
            copyright: Some(copyright),
            links: vec![
                make_link("Metadata Link 1", "jpeg", "http://jpeg1.com"),
                make_link("Metadata Link 2", "mp3", "http://mp3.com"),
            ],
            time: Some(
                DateTime::parse_from_rfc3339("2024-02-02T10:10:54.000Z")
                    .unwrap()
                    .to_utc(),
            ),
            keywords: Some("keyword1, keyword2".to_string()),
            bounds: Some(bounds),
            extensions: Some(Extensions::new(
                "The raw inner text of some metadata extensions",
            )),
        };

        let mut gpx = Gpx::new(declaration, metadata);
        gpx.version = "1.1".to_string();
        gpx.creator = "GAPIX".to_string();
        gpx.attributes = HashMap::new();
        gpx.attributes
            .insert("key1".to_string(), "value1".to_string());
        gpx.attributes
            .insert("key2".to_string(), "value2".to_string());
        gpx.waypoints = make_waypoints(5, "Main", -20.0);
        gpx.routes = make_routes(7, 20.0);
        gpx.tracks = make_tracks(2, 40.0);
        gpx.extensions = Some(Extensions::new("The raw inner text of some GPX extensions"));

        gpx
    }

    fn make_link(text: &str, r#type: &str, href: &str) -> Link {
        Link {
            text: Some(text.to_string()),
            r#type: Some(r#type.to_string()),
            href: href.to_string(),
        }
    }

    fn make_routes(count: usize, latlon_base: f64) -> Vec<Route> {
        (0..count)
            .map(|i| Route {
                name: Some(format!("Route {i}")),
                comment: Some(format!("Route {i} Comment")),
                description: Some(format!("Route {i} Description")),
                source: Some(format!("Route {i} Source")),
                links: vec![
                    make_link(&format!("Route {i} Link 1"), "jpeg", "http://jpeg1.com"),
                    make_link(&format!("Route {i} Link 2"), "mp3", "http://mp3.com"),
                    make_link(&format!("Route {i} Link 3"), "txt", "http://txt.com"),
                ],
                number: Some(i as u32),
                r#type: Some(format!("Route {i} Type")),
                extensions: Some(Extensions::new(format!("Route {i} Extensions"))),
                points: make_waypoints(4, "Route", latlon_base),
            })
            .collect()
    }

    fn make_tracks(count: usize, latlon_base: f64) -> Vec<Track> {
        (0..count)
            .map(|i| Track {
                name: Some(format!("Route {i}")),
                comment: Some(format!("Route {i} Comment")),
                description: Some(format!("Route {i} Description")),
                source: Some(format!("Route {i} Source")),
                links: vec![
                    make_link(&format!("Route {i} Link 1"), "jpeg", "http://jpeg1.com"),
                    make_link(&format!("Route {i} Link 2"), "mp3", "http://mp3.com"),
                    make_link(&format!("Route {i} Link 3"), "txt", "http://txt.com"),
                ],
                number: Some(i as u32),
                r#type: Some(format!("Route {i} Type")),
                extensions: Some(Extensions::new(format!("Route {i} Extensions"))),
                segments: vec![
                    TrackSegment {
                        points: make_waypoints(20, "Track", latlon_base),
                        extensions: Some(Extensions::new(format!(
                            "Track Segment {i}.1 Extensions"
                        ))),
                    },
                    TrackSegment {
                        points: make_waypoints(30, "Track", latlon_base),
                        extensions: Some(Extensions::new(format!(
                            "Track Segment {i}.2 Extensions"
                        ))),
                    },
                    TrackSegment {
                        points: make_waypoints(50, "Track", latlon_base),
                        extensions: Some(Extensions::new(format!(
                            "Track Segment {i}.2 Extensions"
                        ))),
                    },
                ],
            })
            .collect()
    }

    /// Generate 'count' waypoints, arranging for them to have unique names
    /// (sub-elements) and lat-lon (attributes) values.
    fn make_waypoints(count: usize, name: &str, latlon_base: f64) -> Vec<Waypoint> {
        (0..count)
            .map(|i| {
                // Generate some waypo
                let fullname = format!("{name} Waypoint {i}");
                make_waypoint(i, fullname, latlon_base)
            })
            .collect()
    }

    fn make_waypoint<S: Into<String>>(i: usize, name: S, latlon_base: f64) -> Waypoint {
        let lat = latlon_base + i as f64;
        let lon = latlon_base - i as f64;
        let mut wp = Waypoint::with_lat_lon(lat, lon).unwrap();

        wp.ele = Some(12.3);
        wp.time = Some(
            DateTime::parse_from_rfc3339("2024-02-02T10:10:54.000Z")
                .unwrap()
                .to_utc(),
        );
        wp.magvar = Some(98.121242354365);
        wp.geoid_height = Some(123.8487);
        wp.name = Some(name.into());
        wp.comment = Some(format!("Waypoint {i} Comment"));
        wp.description = Some(format!("Waypoint {i} Description"));
        wp.source = Some(format!("Waypoint {i} Source"));
        wp.links = vec![
            make_link(&format!("Waypoint {i} Link 1"), "jpeg", "http://jpeg1.com"),
            make_link(&format!("Waypoint {i} Link 2"), "mp3", "http://mp3.com"),
            make_link(&format!("Waypoint {i} Link 3"), "txt", "http://txt.com"),
        ];
        wp.symbol = Some(format!("Waypoint {i} Symbol"));
        wp.r#type = Some(format!("Waypoint {i} Type"));
        wp.fix = Some(FixType::PPS);
        wp.num_satellites = Some(12 + i as u16);
        wp.hdop = Some(1.1 + i as f64);
        wp.vdop = Some(1.1 - i as f64);
        wp.pdop = Some(100.1 + i as f64);
        wp.age_of_dgps_data = Some(20.0 + i as f64);
        wp.dgps_id = Some(200 + i as u16);
        wp.extensions = Some(Extensions::new(format!("Waypoint {i}Extensions")));

        wp
    }
}
