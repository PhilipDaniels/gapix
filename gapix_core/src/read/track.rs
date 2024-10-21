use quick_xml::{
    events::{BytesStart, Event},
    Reader,
};

use crate::{error::GapixError, model::Track};

use super::{
    attributes::Attributes, extensions::parse_extensions, link::parse_link,
    track_segment::parse_track_segment, XmlReaderExtensions,
};

pub(crate) fn parse_track(
    start_element: &BytesStart<'_>,
    xml_reader: &mut Reader<&[u8]>,
) -> Result<Track, GapixError> {
    Attributes::check_is_empty(start_element, xml_reader)?;

    let mut track = Track::default();

    loop {
        match xml_reader.read_event() {
            Ok(Event::Start(start)) => match start.name().as_ref() {
                b"name" => {
                    track.name = Some(xml_reader.read_inner_as()?);
                }
                b"cmt" => {
                    track.comment = Some(xml_reader.read_inner_as()?);
                }
                b"desc" => {
                    track.description = Some(xml_reader.read_inner_as()?);
                }
                b"src" => {
                    track.source = Some(xml_reader.read_inner_as()?);
                }
                b"link" => {
                    let link = parse_link(&start, xml_reader)?;
                    track.links.push(link);
                }
                b"number" => {
                    track.number = Some(xml_reader.read_inner_as()?);
                }
                b"type" => {
                    track.r#type = Some(xml_reader.read_inner_as_string()?);
                }
                b"extensions" => {
                    track.extensions = Some(parse_extensions(&start, xml_reader)?);
                }
                b"trkseg" => {
                    track
                        .segments
                        .push(parse_track_segment(&start, xml_reader)?);
                }
                e => return Err(GapixError::bad_start(e, xml_reader)),
            },
            Ok(Event::End(e)) => {
                let n = e.name();
                let n = n.as_ref();
                if n == start_element.name().as_ref() {
                    return Ok(track);
                } else if n == b"name"
                    || n == b"cmt"
                    || n == b"desc"
                    || n == b"src"
                    || n == b"link"
                    || n == b"number"
                    || n == b"type"
                    || n == b"extensions"
                    || n == b"trkseg"
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
    use crate::read::start_parse;
    use quick_xml::Reader;

    #[test]
    fn valid_track_all_fields() {
        let mut xml_reader = Reader::from_str(
            r#"<trk>
                 <name>Route name</name>
                 <cmt>Route comment</cmt>
                 <desc>Route description</desc>
                 <src>Route source</src>
                 <link href="http://example.com">
                    <text>Some text here</text>
                    <type>jpeg</type>
                 </link>
                 <link href="http://example2.com">
                    <text>Some text here2</text>
                    <type>jpeg2</type>
                 </link>
                 <number>42</number>
                 <type>Route type</type>
                 <extensions><foo><ex:ex1>extended data</ex:ex1></foo></extensions>
                 <trkseg>
                    <trkpt lat="253.20625" lon="-11.450350">
                    </trkpt>
                    <trkpt lat="253.20625" lon="-11.450350">
                    </trkpt>
                 </trkseg>
                 <trkseg>
                    <trkpt lat="253.20625" lon="-11.450350">
                    </trkpt>
                 </trkseg>
               </trk>"#,
        );

        let start = start_parse(&mut xml_reader);
        let result = parse_track(&start, &mut xml_reader).unwrap();
        assert_eq!(result.name, Some("Route name".to_string()));
        assert_eq!(result.comment, Some("Route comment".to_string()));
        assert_eq!(result.description, Some("Route description".to_string()));
        assert_eq!(result.source, Some("Route source".to_string()));
        assert_eq!(result.links.len(), 2);
        assert_eq!(result.links[0].href, "http://example.com");
        assert_eq!(result.links[1].href, "http://example2.com");
        assert_eq!(result.number, Some(42));
        assert_eq!(result.r#type, Some("Route type".to_string()));
        let ext = result.extensions.unwrap();
        assert_eq!(ext.raw_xml, "<foo><ex:ex1>extended data</ex:ex1></foo>");
        assert_eq!(result.segments.len(), 2);
    }

    #[test]
    fn extra_elements() {
        let mut xml_reader = Reader::from_str(
            r#"<trk>
                 <foo>bar</foo>
               </trk>"#,
        );

        let start = start_parse(&mut xml_reader);
        match parse_track(&start, &mut xml_reader) {
            Err(GapixError::UnexpectedStartElement(_)) => {}
            x => panic!("Unexpected result from parse(): {:?}", x),
        };
    }

    #[test]
    fn extra_attributes() {
        let mut xml_reader = Reader::from_str(
            r#"<trk foo="bar">
               </trk>"#,
        );

        let start = start_parse(&mut xml_reader);
        match parse_track(&start, &mut xml_reader) {
            Err(GapixError::UnexpectedAttributes { .. }) => {}
            x => panic!("Unexpected result from parse(): {:?}", x),
        };
    }
}
