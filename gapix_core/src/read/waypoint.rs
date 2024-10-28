use quick_xml::{
    events::{BytesStart, Event},
    Reader,
};

use crate::{
    error::GapixError,
    model::Waypoint,
    model_impls::{validate_degrees, validate_dgps_station_id},
};

use super::{
    attributes::Attributes, extensions::parse_extensions, link::parse_link,
    trackpoint_extensions::parse_garmin_trackpoint_extensions, XmlReaderExtensions,
};

/// Parses a waypoint. Waypoints can appear under the 'gpx' tag, as part of a
/// route or as part of a track.
pub(crate) fn parse_waypoint(
    start_element: &BytesStart<'_>,
    xml_reader: &mut Reader<&[u8]>,
) -> Result<Waypoint, GapixError> {
    let mut attributes = Attributes::new(start_element, xml_reader)?;
    let lat = attributes.get("lat")?;
    let lon = attributes.get("lon")?;
    attributes.check_is_empty_now()?;

    let mut wp = Waypoint::with_lat_lon(lat, lon)?;

    loop {
        match xml_reader.read_event() {
            Ok(Event::Start(start)) => match start.name().as_ref() {
                b"ele" => {
                    wp.ele = Some(xml_reader.read_inner_as()?);
                }
                b"time" => {
                    wp.time = Some(xml_reader.read_inner_as_time()?);
                }
                b"magvar" => {
                    let degrees = validate_degrees(xml_reader.read_inner_as()?)?;
                    wp.magvar = Some(degrees);
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
                    let link = parse_link(&start, xml_reader)?;
                    wp.links.push(link);
                }
                b"sym" => {
                    wp.symbol = Some(xml_reader.read_inner_as()?);
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
                    let id = validate_dgps_station_id(xml_reader.read_inner_as()?)?;
                    wp.dgps_id = Some(id);
                }
                b"extensions" => {
                    wp.extensions = Some(parse_extensions(&start, xml_reader)?);
                    if start_element.name().as_ref() == b"trkpt" {
                        // TP Extensions only exist on waypoints within tracks.
                        // Even then, they are optional.
                        let s = &wp.extensions.as_ref()
                            .expect("unwrap is safe due to just setting extensions to Some in line above")
                            .raw_xml;
                        wp.garmin_extensions = parse_garmin_trackpoint_extensions(s)?;
                    }
                }
                e => return Err(GapixError::bad_start(e, xml_reader)),
            },
            Ok(Event::End(e)) => {
                let n = e.name();
                let n = n.as_ref();
                if n == start_element.name().as_ref() {
                    return Ok(wp);
                } else if n == b"ele"
                    || n == b"time"
                    || n == b"magvar"
                    || n == b"geoidheight"
                    || n == b"name"
                    || n == b"cmt"
                    || n == b"desc"
                    || n == b"src"
                    || n == b"link"
                    || n == b"sym"
                    || n == b"type"
                    || n == b"fix"
                    || n == b"sat"
                    || n == b"hdop"
                    || n == b"vdop"
                    || n == b"pdop"
                    || n == b"ageofdgpsdata"
                    || n == b"dgpsid"
                    || n == b"extensions"
                {
                    // These are expected endings, do nothing.
                } else {
                    return Err(GapixError::bad_end(n, xml_reader));
                }
            }
            // Ignore spurious Event::Text, I think they are newlines.
            Ok(Event::Text(_)) => {}
            e => return Err(GapixError::bad_event(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{model::FixType, read::start_parse};
    use quick_xml::Reader;

    #[test]
    fn valid_waypoint_all_fields() {
        let mut xml_reader = Reader::from_str(
            r#"<trkpt lat="55.20625" lon="-11.450350">
                 <ele>158.399993896484375</ele>
                 <time>2024-02-02T10:10:54.000Z</time>
                 <magvar>52.3</magvar>
                 <geoidheight>100.7</geoidheight>
                 <name>Waypoint name</name>
                 <cmt>Waypoint comment</cmt>
                 <desc>Waypoint description</desc>
                 <src>Waypoint source</src>
                 <link href="http://example.com">
                    <text>Some text here</text>
                    <type>jpeg</type>
                 </link>
                 <link href="http://example2.com">
                    <text>Some text here2</text>
                    <type>jpeg2</type>
                 </link>
                 <sym>Waypoint symbol</sym>
                 <type>Waypoint type</type>
                 <fix>3d</fix>
                 <sat>12</sat>
                 <hdop>120.2</hdop>
                 <vdop>130.3</vdop>
                 <pdop>140.4</pdop>
                 <ageofdgpsdata>1234.1234</ageofdgpsdata>
                 <dgpsid>89</dgpsid>
                 <extensions><foo><ex:ex1>extended data</ex:ex1></foo></extensions>
               </trkpt>"#,
        );

        let start = start_parse(&mut xml_reader);
        let result = parse_waypoint(&start, &mut xml_reader).unwrap();
        assert_eq!(result.lat, 55.20625);
        assert_eq!(result.lon, -11.450350);
        assert_eq!(result.magvar, Some(52.3));
        assert_eq!(result.geoid_height, Some(100.7));
        assert_eq!(result.name, Some("Waypoint name".to_string()));
        assert_eq!(result.comment, Some("Waypoint comment".to_string()));
        assert_eq!(result.description, Some("Waypoint description".to_string()));
        assert_eq!(result.source, Some("Waypoint source".to_string()));

        assert_eq!(result.links.len(), 2);
        assert_eq!(result.links[0].href, "http://example.com");
        assert_eq!(result.links[1].href, "http://example2.com");
        assert_eq!(result.symbol, Some("Waypoint symbol".to_string()));
        assert_eq!(result.r#type, Some("Waypoint type".to_string()));
        assert_eq!(result.fix, Some(FixType::ThreeDimensional));
        assert_eq!(result.num_satellites, Some(12));
        assert_eq!(result.hdop, Some(120.2));
        assert_eq!(result.vdop, Some(130.3));
        assert_eq!(result.pdop, Some(140.4));
        assert_eq!(result.age_of_dgps_data, Some(1234.1234));
        assert_eq!(result.dgps_id, Some(89));

        let ext = result.extensions.unwrap();
        assert_eq!(ext.raw_xml, "<foo><ex:ex1>extended data</ex:ex1></foo>");
    }

    #[test]
    fn invalid_negative_dgps_id() {
        let mut xml_reader = Reader::from_str(
            r#"<trkpt lat="55.20625" lon="-11.450350">
                 <dgpsid>-1</dgpsid>
               </trkpt>"#,
        );

        let start = start_parse(&mut xml_reader);
        match parse_waypoint(&start, &mut xml_reader) {
            Err(GapixError::InvalidDGPSStationId(-1)) => {}
            x => panic!("Unexpected result from parse(): {:?}", x),
        };
    }

    #[test]
    fn invalid_positive_dgps_id() {
        let mut xml_reader = Reader::from_str(
            r#"<trkpt lat="55.20625" lon="-11.450350">
                 <dgpsid>1024</dgpsid>
               </trkpt>"#,
        );

        let start = start_parse(&mut xml_reader);
        match parse_waypoint(&start, &mut xml_reader) {
            Err(GapixError::InvalidDGPSStationId(1024)) => {}
            x => panic!("Unexpected result from parse(): {:?}", x),
        };
    }

    #[test]
    fn invalid_magvar() {
        let mut xml_reader = Reader::from_str(
            r#"<trkpt lat="55.20625" lon="-11.450350">
                 <magvar>360.1</magvar>
               </trkpt>"#,
        );

        let start = start_parse(&mut xml_reader);
        match parse_waypoint(&start, &mut xml_reader) {
            Err(GapixError::InvalidDegrees(360.1)) => {}
            x => panic!("Unexpected result from parse(): {:?}", x),
        };
    }

    #[test]
    fn invalid_latitude() {
        let mut xml_reader = Reader::from_str(
            r#"<trkpt lat="1234" lon="-11.450350">
               </trkpt>"#,
        );

        let start = start_parse(&mut xml_reader);
        match parse_waypoint(&start, &mut xml_reader) {
            Err(GapixError::InvalidLatitude(1234.0)) => {}
            x => panic!("Unexpected result from parse(): {:?}", x),
        };
    }

    #[test]
    fn invalid_longitude() {
        let mut xml_reader = Reader::from_str(
            r#"<trkpt lat="34" lon="-1111.45">
               </trkpt>"#,
        );

        let start = start_parse(&mut xml_reader);
        match parse_waypoint(&start, &mut xml_reader) {
            Err(GapixError::InvalidLongitude(-1111.45)) => {}
            x => panic!("Unexpected result from parse(): {:?}", x),
        };
    }

    #[test]
    fn extra_elements() {
        let mut xml_reader = Reader::from_str(
            r#"<trkpt lat="55.20625" lon="-11.450350">
                 <foo>bar</foo>
               </trkpt>"#,
        );

        let start = start_parse(&mut xml_reader);
        match parse_waypoint(&start, &mut xml_reader) {
            Err(GapixError::UnexpectedStartElement(_)) => {}
            x => panic!("Unexpected result from parse(): {:?}", x),
        };
    }

    #[test]
    fn extra_attributes() {
        let mut xml_reader = Reader::from_str(
            r#"<trkpt lat="55.20625" lon="-11.450350" foo="bar">
               </trkpt>"#,
        );

        let start = start_parse(&mut xml_reader);
        match parse_waypoint(&start, &mut xml_reader) {
            Err(GapixError::UnexpectedAttributes { .. }) => {}
            x => panic!("Unexpected result from parse(): {:?}", x),
        };
    }
}
