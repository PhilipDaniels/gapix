use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

use log::info;
use logging_timer::time;

use crate::{
    byte_counter::ByteCounter,
    error::GapixError,
    formatting::format_utc_date,
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
    /// instead of full detail.
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
        "Wrote PX file {:?}, {} Kb",
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
    write_declaration_element(w, &gpx.declaration)?;
    write_gpxinfo_element_open(w, gpx)?;
    write_metadata_element(w, &gpx.metadata)?;

    match output_options {
        OutputOptions::Full => {
            for wp in &gpx.waypoints {
                write_waypoint(w, wp, "wpt", output_options)?;
            }
            for route in &gpx.routes {
                write_route(w, route)?;
            }
        }
        OutputOptions::AudaxUKDIY => { /* Waypoints and routes not needed */ }
    };

    for track in &gpx.tracks {
        write_track(w, track, output_options)?;
    }

    match output_options {
        OutputOptions::Full => {
            write_extensions(w, &gpx.extensions)?;
        }
        OutputOptions::AudaxUKDIY => { /* Extensions not needed */ }
    }

    write_gpxinfo_element_close(w)?;

    w.flush()?;
    Ok(())
}

fn write_declaration_element<W: Write>(
    w: &mut W,
    declaration: &XmlDeclaration,
) -> Result<(), GapixError> {
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

fn write_gpxinfo_element_open<W: Write>(w: &mut W, info: &Gpx) -> Result<(), GapixError> {
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

fn write_gpxinfo_element_close<W: Write>(w: &mut W) -> Result<(), GapixError> {
    writeln!(w, "</gpx>")?;
    Ok(())
}

fn write_metadata_element<W: Write>(w: &mut W, metadata: &Metadata) -> Result<(), GapixError> {
    writeln!(w, "  <metadata>")?;
    if let Some(name) = &metadata.name {
        writeln!(w, "    <name>{}</name>", name)?;
    }
    if let Some(desc) = &metadata.description {
        writeln!(w, "    <desc>{}</desc>", desc)?;
    }
    if let Some(author) = &metadata.author {
        write_person_element(w, author, "author")?;
    }
    if let Some(copyright) = &metadata.copyright {
        write_copyright_element(w, copyright)?;
    }
    for link in &metadata.links {
        write_link_element(w, link)?;
    }
    if let Some(time) = &metadata.time {
        writeln!(w, "    <time>{}</time>", format_utc_date(time)?)?;
    }
    if let Some(keywords) = &metadata.keywords {
        writeln!(w, "    <keywords>{}</keywords>", keywords)?;
    }
    if let Some(bounds) = &metadata.bounds {
        writeln!(
            w,
            "    <bounds min_lat=\"{:.6}\" max_lat=\"{:.6}\" min_lon=\"{:.6}\" max_lon=\"{:.6}\"/>",
            bounds.min_lat, bounds.max_lat, bounds.min_lon, bounds.max_lon
        )?;
    }
    write_extensions(w, &metadata.extensions)?;

    writeln!(w, "  </metadata>")?;
    Ok(())
}

fn write_person_element<W: Write>(
    w: &mut W,
    person: &Person,
    element_name: &str,
) -> Result<(), GapixError> {
    writeln!(w, "<{}>", element_name)?;
    if let Some(name) = &person.name {
        writeln!(w, "  <name>{}</name>", name)?;
    }
    if let Some(email) = &person.email {
        write_email_element(w, email)?;
    }
    if let Some(link) = &person.link {
        write_link_element(w, link)?;
    }
    writeln!(w, "</{}>", element_name)?;
    Ok(())
}

fn write_copyright_element<W: Write>(w: &mut W, copyright: &Copyright) -> Result<(), GapixError> {
    writeln!(w, "<copyright>")?;
    if let Some(year) = &copyright.year {
        writeln!(w, "  <year>{}</year>", year)?;
    }
    if let Some(license) = &copyright.license {
        writeln!(w, "  <license>{}</license>", license)?;
    }
    writeln!(w, "  <author>{}</author>", copyright.author)?;
    writeln!(w, "</copyright>")?;
    Ok(())
}

fn write_email_element<W: Write>(w: &mut W, email: &Email) -> Result<(), GapixError> {
    writeln!(
        w,
        "<email id=\"{}\" domain=\"{}\" />",
        email.id, email.domain
    )?;
    Ok(())
}

fn write_link_element<W: Write>(w: &mut W, link: &Link) -> Result<(), GapixError> {
    writeln!(w, "    <link href=\"{}\">", link.href)?;
    if let Some(text) = &link.text {
        writeln!(w, "      <text>{}</text>", text)?;
    }
    if let Some(r#type) = &link.r#type {
        writeln!(w, "      <type>{}</type>", r#type)?;
    }
    writeln!(w, "    </link>")?;
    Ok(())
}

fn write_route<W: Write>(w: &mut W, route: &Route) -> Result<(), GapixError> {
    writeln!(w, "  <rte>")?;
    if let Some(name) = &route.name {
        writeln!(w, "    <name>{}</name>", name)?;
    }
    if let Some(comment) = &route.comment {
        writeln!(w, "    <cmt>{}</cmt>", comment)?;
    }
    if let Some(desc) = &route.description {
        writeln!(w, "    <desc>{}</desc>", desc)?;
    }
    if let Some(source) = &route.source {
        writeln!(w, "    <src>{}</src>", source)?;
    }
    for link in &route.links {
        write_link_element(w, link)?;
    }
    if let Some(number) = &route.number {
        writeln!(w, "    <number>{}</number>", number)?;
    }
    if let Some(route_type) = &route.r#type {
        writeln!(w, "    <type>{}</type>", route_type)?;
    }
    write_extensions(w, &route.extensions)?;
    for pt in &route.points {
        write_waypoint(w, pt, "rtept", OutputOptions::Full)?;
    }
    writeln!(w, "  </rte>")?;
    Ok(())
}

fn write_track<W: Write>(
    w: &mut W,
    track: &Track,
    output_options: OutputOptions,
) -> Result<(), GapixError> {
    writeln!(w, "  <trk>")?;
    if let Some(name) = &track.name {
        writeln!(w, "    <name>{}</name>", name)?;
    }
    if let Some(comment) = &track.comment {
        writeln!(w, "    <cmt>{}</cmt>", comment)?;
    }
    if let Some(desc) = &track.description {
        writeln!(w, "    <desc>{}</desc>", desc)?;
    }
    if let Some(source) = &track.source {
        writeln!(w, "    <src>{}</src>", source)?;
    }
    for link in &track.links {
        write_link_element(w, link)?;
    }
    if let Some(number) = &track.number {
        writeln!(w, "    <number>{}</number>", number)?;
    }
    if let Some(track_type) = &track.r#type {
        writeln!(w, "    <type>{}</type>", track_type)?;
    }
    write_extensions(w, &track.extensions)?;
    for segment in &track.segments {
        write_track_segment(w, segment, output_options)?;
    }
    writeln!(w, "  </trk>")?;
    Ok(())
}

fn write_track_segment<W: Write>(
    w: &mut W,
    segment: &TrackSegment,
    output_options: OutputOptions,
) -> Result<(), GapixError> {
    writeln!(w, "    <trkseg>")?;
    for p in &segment.points {
        write_waypoint(w, p, "trkpt", output_options)?;
    }
    write_extensions(w, &segment.extensions)?;
    writeln!(w, "    </trkseg>")?;
    Ok(())
}

fn write_waypoint<W: Write>(
    w: &mut W,
    point: &Waypoint,
    element_name: &str,
    output_options: OutputOptions,
) -> Result<(), GapixError> {
    writeln!(
        w,
        "      <{element_name} lat=\"{:.6}\" lon=\"{:.6}\">",
        point.lat, point.lon
    )?;
    if let Some(ele) = point.ele {
        writeln!(w, "        <ele>{:.1}</ele>", ele)?;
    }
    if let Some(t) = point.time {
        writeln!(w, "        <time>{}</time>", format_utc_date(&t)?)?;
    }
    if let Some(magvar) = point.magvar {
        writeln!(w, "        <magvar>{:.6}</magvar>", magvar)?;
    }
    if let Some(geoid_height) = point.geoid_height {
        writeln!(w, "        <geoidheight>{:.6}</geoidheight>", geoid_height)?;
    }
    if let Some(name) = &point.name {
        writeln!(w, "        <name>{name}</name>")?;
    }
    if let Some(comment) = &point.comment {
        writeln!(w, "        <cmt>{comment}</cmt>")?;
    }
    if let Some(desc) = &point.description {
        writeln!(w, "        <desc>{desc}</desc>")?;
    }
    if let Some(src) = &point.source {
        writeln!(w, "        <src>{src}</src>")?;
    }
    for link in &point.links {
        write_link_element(w, link)?;
    }
    if let Some(sym) = &point.symbol {
        writeln!(w, "        <sym>{sym}</sym>")?;
    }
    if let Some(point_type) = &point.r#type {
        writeln!(w, "        <type>{point_type}</type>")?;
    }
    if let Some(fix) = &point.fix {
        writeln!(w, "        <fix>{fix}</fix>")?;
    }
    if let Some(sat) = &point.num_satellites {
        writeln!(w, "        <sat>{sat}</sat>")?;
    }
    if let Some(hdop) = point.hdop {
        writeln!(w, "        <hdop>{:.6}</hdop>", hdop)?;
    }
    if let Some(vdop) = point.vdop {
        writeln!(w, "        <vdop>{:.6}</vdop>", vdop)?;
    }
    if let Some(pdop) = point.pdop {
        writeln!(w, "        <pdop>{:.6}</pdop>", pdop)?;
    }
    if let Some(age) = point.age_of_dgps_data {
        writeln!(w, "        <ageofdgpsdata>{:.6}</ageofdgpsdata>", age)?;
    }
    if let Some(id) = point.dgps_id {
        writeln!(w, "        <dgpsid>{id}</dgpsid>")?;
    }
    match output_options {
        OutputOptions::Full => {
            write_extensions(w, &point.extensions)?;
        }
        OutputOptions::AudaxUKDIY => { /* Not needed */ }
    }
    writeln!(w, "      </{element_name}>")?;
    Ok(())
}

fn write_extensions<W: Write>(
    w: &mut W,
    extensions: &Option<Extensions>,
) -> Result<(), GapixError> {
    // TODO: Need to be careful of the namespace. Can get it from the GPX tag.

    if let Some(ext) = extensions {
        writeln!(w, "<extensions>{}</extensions>", ext.raw_xml)?;
    }

    Ok(())
}
