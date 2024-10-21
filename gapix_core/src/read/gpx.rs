use quick_xml::{
    events::{BytesStart, Event},
    Reader,
};

use crate::{error::GapixError, model::Gpx};

use super::{
    attributes::Attributes, extensions::parse_extensions, metadata::parse_metadata,
    route::parse_route, track::parse_track, waypoint::parse_waypoint,
};

/// Parses the 'gpx' element itself.
pub(crate) fn parse_gpx(
    start_element: &BytesStart<'_>,
    xml_reader: &mut Reader<&[u8]>,
) -> Result<Gpx, GapixError> {
    let mut attributes = Attributes::new(start_element, xml_reader)?;

    let mut gpx = Gpx {
        creator: attributes.get("creator")?,
        version: attributes.get("version")?,
        attributes: attributes.into_inner(),
        ..Default::default()
    };

    loop {
        match xml_reader.read_event() {
            Ok(Event::Start(start)) => match start.name().as_ref() {
                b"metadata" => {
                    gpx.metadata = parse_metadata(&start, xml_reader)?;
                }
                b"wpt" => {
                    let waypoint = parse_waypoint(&start, xml_reader)?;
                    gpx.waypoints.push(waypoint);
                }
                b"rte" => {
                    let route = parse_route(&start, xml_reader)?;
                    gpx.routes.push(route);
                }
                b"trk" => {
                    let track = parse_track(&start, xml_reader)?;
                    gpx.tracks.push(track);
                }
                b"extensions" => {
                    gpx.extensions = Some(parse_extensions(&start, xml_reader)?);
                }
                _ => (),
            },
            Ok(Event::End(e)) => {
                let n = e.name();
                let n = n.as_ref();

                if n == start_element.name().as_ref() {
                    return Ok(gpx);
                } else if n == b"metadata"
                    || n == b"wpt"
                    || n == b"rte"
                    || n == b"trk"
                    || n == b"extensions"
                {
                    // These are expected endings, do nothing.
                } else {
                    return Err(GapixError::bad_end(n, xml_reader));
                }
            }
            Ok(Event::Eof) => {
                return Err(GapixError::UnexpectedEof);
            }
            Err(e) => return Err(e.into()),
            //bail!("Error at position {}: {:?}", xml_reader.error_position(), e),
            _ => (),
        }
    }
}
