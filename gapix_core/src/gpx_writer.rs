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
    model::{Extensions, Gpx, Link, Metadata, Track, TrackSegment, Waypoint, XmlDeclaration},
};

/// Writes a GPX to file with full-fidelity, i.e. everything we can write is
/// written.
pub fn write_gpx_to_file<P: AsRef<Path>>(output_file: P, gpx: &Gpx) -> Result<(), GapixError> {
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
    write_gpx_to_writer(&mut w, gpx)?;
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
pub fn write_gpx_to_writer<W: Write>(w: &mut W, gpx: &Gpx) -> Result<(), GapixError> {
    write_declaration_element(w, &gpx.declaration)?;
    write_gpxinfo_element_open(w, gpx)?;
    write_metadata_element(w, &gpx.metadata)?;
    for track in &gpx.tracks {
        write_track(w, track)?;
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
    for link in &metadata.links {
        write_link_element(w, link)?;
    }
    if let Some(time) = &metadata.time {
        writeln!(w, "    <time>{}</time>", format_utc_date(time)?)?;
    }
    if let Some(desc) = &metadata.description {
        writeln!(w, "    <desc>{}</desc>", desc)?;
    }
    writeln!(w, "  </metadata>")?;
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

fn write_track<W: Write>(w: &mut W, track: &Track) -> Result<(), GapixError> {
    writeln!(w, "  <trk>")?;
    if let Some(track_name) = &track.name {
        writeln!(w, "    <name>{}</name>", track_name)?;
    }
    if let Some(track_type) = &track.r#type {
        writeln!(w, "    <type>{}</type>", track_type)?;
    }
    if let Some(desc) = &track.description {
        writeln!(w, "    <desc>{}</desc>", desc)?;
    }

    for segment in &track.segments {
        write_track_segment(w, segment)?;
    }

    writeln!(w, "  </trk>")?;
    Ok(())
}

fn write_track_segment<W: Write>(w: &mut W, segment: &TrackSegment) -> Result<(), GapixError> {
    writeln!(w, "    <trkseg>")?;
    for p in &segment.points {
        write_trackpoint(w, p)?;
    }
    writeln!(w, "    </trkseg>")?;
    Ok(())
}

fn write_trackpoint<W: Write>(w: &mut W, point: &Waypoint) -> Result<(), GapixError> {
    writeln!(
        w,
        "      <trkpt lat=\"{:.6}\" lon=\"{:.6}\">",
        point.lat, point.lon
    )?;

    if let Some(ele) = point.ele {
        writeln!(w, "        <ele>{:.1}</ele>", ele)?;
    }

    if let Some(t) = point.time {
        writeln!(w, "        <time>{}</time>", format_utc_date(&t)?)?;
    }

    if let Some(ext) = &point.extensions {
        write_extensions(w, ext)?;
    }

    writeln!(w, "      </trkpt>")?;

    Ok(())
}

fn write_extensions<W: Write>(_w: &mut W, _ext: &Extensions) -> Result<(), GapixError> {
    // TODO: Need to be careful of the namespace. Can get it from the GPX tag.
    Ok(())
}
