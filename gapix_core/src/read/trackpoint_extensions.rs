use quick_xml::{events::Event, Reader};

use crate::{error::GapixError, model::GarminTrackpointExtensions};

use super::XmlReaderExtensions;

pub(crate) fn parse_garmin_trackpoint_extensions(
    xml_reader: &mut Reader<&[u8]>,
) -> Result<GarminTrackpointExtensions, GapixError> {
    let mut gext = GarminTrackpointExtensions::default();

    loop {
        match xml_reader.read_event() {
            Ok(Event::Start(e)) => match e.local_name().as_ref() {
                b"TrackPointExtension" => { /* ignore, just a container element */ }
                b"atemp" => {
                    gext.air_temp = Some(xml_reader.read_inner_as()?);
                }
                b"wtemp" => {
                    gext.water_temp = Some(xml_reader.read_inner_as()?);
                }
                b"depth" => {
                    gext.depth = Some(xml_reader.read_inner_as()?);
                }
                b"hr" => {
                    gext.heart_rate = Some(xml_reader.read_inner_as()?);
                }
                b"cad" => {
                    gext.cadence = Some(xml_reader.read_inner_as()?);
                }
                e => return Err(GapixError::bad_start(e, xml_reader)),
            },
            Ok(Event::End(e)) => match e.local_name().as_ref() {
                b"TrackPointExtension" => { /* ignore, just a container element */ }
                b"extensions" => {
                    return Ok(gext);
                }
                b"atemp" | b"wtemp" | b"depth" | b"hr" | b"cad" => { /* ignore, just the closing tags */
                }
                e => return Err(GapixError::bad_end(e, xml_reader)),
            },
            // Ignore spurious Event::Text, I think they are newlines.
            Ok(Event::Text(_)) => {}
            e => return Err(GapixError::bad_event(e)),
        }
    }
}
