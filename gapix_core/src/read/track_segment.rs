use quick_xml::{
    events::{BytesStart, Event},
    Reader,
};

use crate::{error::GapixError, model::TrackSegment};

use super::{attributes::Attributes, extensions::parse_extensions, waypoint::parse_waypoint};

pub(crate) fn parse_track_segment(
    start_element: &BytesStart<'_>,
    xml_reader: &mut Reader<&[u8]>,
) -> Result<TrackSegment, GapixError> {
    Attributes::check_is_empty(start_element, xml_reader)?;

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
                e => return Err(GapixError::bad_start(e, xml_reader)),
            },
            Ok(Event::End(e)) => {
                let n = e.name();
                let n = n.as_ref();
                if n == start_element.name().as_ref() {
                    return Ok(segment);
                } else if n == b"trkpt" || n == b"extensions" {
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
    use crate::read::xml_reader_extensions::start_parse;

    use super::*;
    use quick_xml::Reader;

    #[test]
    fn valid_track_segment_all_fields() {
        let mut xml_reader = Reader::from_str(
            r#"<trkseg>
                 <extensions><foo><ex:ex1>extended data</ex:ex1></foo></extensions>
               </trkseg>"#,
        );

        let start = start_parse(&mut xml_reader);
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

        let start = start_parse(&mut xml_reader);
        match parse_track_segment(&start, &mut xml_reader) {
            Err(GapixError::UnexpectedStartElement(_)) => {}
            x => panic!("Unexpected result from parse(): {:?}", x),
        };
    }

    #[test]
    fn extra_attributes() {
        let mut xml_reader = Reader::from_str(
            r#"<trkseg foo="bar">
               </trkseg>"#,
        );

        let start = start_parse(&mut xml_reader);
        match parse_track_segment(&start, &mut xml_reader) {
            Err(GapixError::UnexpectedAttributes { .. }) => {}
            x => panic!("Unexpected result from parse(): {:?}", x),
        };
    }
}
