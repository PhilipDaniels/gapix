use anyhow::{bail, Result};
use quick_xml::{events::Event, Reader};

use crate::model::Waypoint;

use super::{
    attributes::Attributes, bytes_to_string, extensions::parse_extensions, link::parse_link,
    XmlReaderExtensions
};

/// Parses a waypoint. Waypoints can appear under the 'gpx' tag, as part of a
/// route or as part of a track.
pub(crate) fn parse_waypoint(
    mut attributes: Attributes,
    xml_reader: &mut Reader<&[u8]>,
    expected_end_tag: &[u8], // Possible ending tags: wpt, rtept, trkpt
) -> Result<Waypoint> {
    let lat = attributes.get("lat")?;
    let lon = attributes.get("lon")?;
    if !attributes.is_empty() {
        bail!(
            "Found extra attributes while parsing waypoint {:?}",
            attributes
        );
    }

    let mut wp = Waypoint::with_lat_lon(lat, lon);

    loop {
        match xml_reader.read_event() {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"ele" => {
                    wp.ele = Some(xml_reader.read_inner_as()?);
                }
                b"time" => {
                    wp.time = Some(xml_reader.read_inner_as_time()?);
                }
                b"magvar" => {
                    wp.magvar = Some(xml_reader.read_inner_as()?);
                }
                b"geoidheight" => {
                    wp.geoid_height = Some(xml_reader.read_inner_as()?);
                }
                b"name" => {
                    wp.name = Some(xml_reader.read_inner_as()?);
                }
                b"cmt" => {
                    wp.comment = Some(xml_reader.read_inner_as()?);
                }
                b"desc" => {
                    wp.description = Some(xml_reader.read_inner_as()?);
                }
                b"src" => {
                    wp.source = Some(xml_reader.read_inner_as()?);
                }
                b"link" => {
                    let link = parse_link(Attributes::new(&e)?, xml_reader)?;
                    wp.links.push(link);
                }
                b"sym" => {
                    wp.source = Some(xml_reader.read_inner_as()?);
                }
                b"type" => {
                    wp.r#type = Some(xml_reader.read_inner_as()?);
                }
                b"fix" => {
                    let fix: String = xml_reader.read_inner_as()?;
                    wp.fix = Some(fix.try_into()?);
                }
                b"sat" => {
                    wp.num_satellites = Some(xml_reader.read_inner_as()?);
                }
                b"hdop" => {
                    wp.hdop = Some(xml_reader.read_inner_as()?);
                }
                b"vdop" => {
                    wp.vdop = Some(xml_reader.read_inner_as()?);
                }
                b"pdop" => {
                    wp.pdop = Some(xml_reader.read_inner_as()?);
                }
                b"ageofdgpsdata" => {
                    wp.age_of_dgps_data = Some(xml_reader.read_inner_as()?);
                }
                b"dgpsid" => {
                    wp.dgps_id = Some(xml_reader.read_inner_as()?);
                }
                b"extensions" => {
                    wp.extensions = Some(parse_extensions(xml_reader)?);
                }
                e => bail!("Unexpected element {:?}", bytes_to_string(e)),
            },
            Ok(Event::End(e)) => {
                if e.name().as_ref() == expected_end_tag {
                    return Ok(wp);
                } else {
                    // TODO: Check for all valid ends.
                }
            }
            // Ignore spurious Event::Text, I think they are newlines.
            Ok(Event::Text(_)) => {}
            e => bail!("Unexpected element {:?}", e),
        }
    }
}
