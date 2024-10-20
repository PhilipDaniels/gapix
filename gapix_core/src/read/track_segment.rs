use anyhow::{bail, Result};
use quick_xml::{
    events::{BytesStart, Event},
    Reader,
};

use crate::model::TrackSegment;

use super::{
    check_no_attributes, extensions::parse_extensions, waypoint::parse_waypoint,
    XmlReaderConversions,
};

pub(crate) fn parse_track_segment(
    start_element: &BytesStart<'_>,
    xml_reader: &mut Reader<&[u8]>,
) -> Result<TrackSegment> {
    check_no_attributes(&start_element, xml_reader)?;

    let mut segment = TrackSegment::default();

    loop {
        match xml_reader.read_event() {
            Ok(Event::Start(start)) => match start.name().as_ref() {
                b"trkpt" => {
                    let point = parse_waypoint(&start, xml_reader)?;
                    segment.points.push(point);
                }
                b"extensions" => {
                    segment.extensions = Some(parse_extensions(&start, xml_reader)?);
                }
                e => bail!("Unexpected Start element {:?}", xml_reader.bytes_to_cow(e)),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::read::start_parse;
    use quick_xml::Reader;

    #[test]
    fn valid_track_segment_all_fields() {
        let mut xml_reader = Reader::from_str(
            r#"<trkseg>
                 <extensions><foo><ex:ex1>extended data</ex:ex1></foo></extensions>
               </trkseg>"#,
        );

        let start = start_parse(&mut xml_reader).unwrap();
        let result = parse_track_segment(&start, &mut xml_reader).unwrap();
        let ext = result.extensions.unwrap();
        assert_eq!(ext.raw_xml, "<foo><ex:ex1>extended data</ex:ex1></foo>");
    }

    #[test]
    fn extra_elements() {
        let mut xml_reader = Reader::from_str(
            r#"<trkseg>
                 <foo>bar</foo>
               </trkseg>"#,
        );

        let start = start_parse(&mut xml_reader).unwrap();
        let result = parse_track_segment(&start, &mut xml_reader);
        assert!(result.is_err());
    }

    #[test]
    fn extra_attributes() {
        let mut xml_reader = Reader::from_str(
            r#"<trkseg foo="bar">
               </trkseg>"#,
        );

        let start = start_parse(&mut xml_reader).unwrap();
        let result = parse_track_segment(&start, &mut xml_reader);
        assert!(result.is_err());
    }
}
