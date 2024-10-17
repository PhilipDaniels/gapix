use anyhow::{bail, Result};
use quick_xml::{events::Event, Reader};

use crate::model::TrackSegment;

use super::{
    attributes::Attributes, bytes_to_string, extensions::parse_extensions, waypoint::parse_waypoint,
};

pub(crate) fn parse_track_segment(
    buf: &mut Vec<u8>,
    xml_reader: &mut Reader<&[u8]>,
) -> Result<TrackSegment> {

    let mut segment = TrackSegment::default();

    loop {
        match xml_reader.read_event_into(buf) {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"trkpt" => {
                    let point = parse_waypoint(Attributes::new(&e)?, buf, xml_reader, b"trkpt")?;
                    segment.points.push(point);
                }
                b"extensions" => {
                    segment.extensions = Some(parse_extensions(xml_reader)?);
                }
                e => bail!("Unexpected element {:?}", bytes_to_string(e)?),
            },
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"trkseg" => {
                    return Ok(segment);
                }
                _ => {}
            },
            // Ignore spurious Event::Text, I think they are newlines.
            Ok(Event::Text(_)) => {}
            e => bail!("Unexpected element {:?}", e),
        }
    }
}
