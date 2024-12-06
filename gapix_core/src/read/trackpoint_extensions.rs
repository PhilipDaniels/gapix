use quick_xml::{events::Event, Reader};

use crate::{error::GapixError, model::GarminTrackpointExtensions};

use super::xml_reader_extensions::XmlReaderExtensions;

/// This function is a little different to the other parse() functions - it expects
/// to be fed the inner Xml from an &lt;extensions&gt; tag, so it won't find the usual
/// &lt;/extensions&gt; ending tag.
pub(crate) fn parse_garmin_trackpoint_extensions(
    s: &str,
) -> Result<Option<GarminTrackpointExtensions>, GapixError> {
    let mut gext = GarminTrackpointExtensions::default();
    let mut xml_reader = Reader::from_str(s);

    loop {
        match xml_reader.read_event() {
            Ok(Event::Start(e)) => match e.local_name().as_ref() {
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
                _ => { /* Ignore any other elements, there can be ANYTHING in an extensions tag */ }
            },
            // Ignore spurious Event::Text, I think they are newlines.
            Ok(Event::Text(_)) => {}
            Ok(Event::End(_)) => {}
            Ok(Event::Eof) => {
                if gext.air_temp.is_some()
                    || gext.water_temp.is_some()
                    || gext.depth.is_some()
                    || gext.heart_rate.is_some()
                    || gext.cadence.is_some()
                {
                    return Ok(Some(gext));
                } else {
                    return Ok(None);
                }
            }
            e => return Err(GapixError::bad_event(e)),
        }
    }

    //Ok(Some(gext))
}
