use quick_xml::{
    events::{BytesStart, Event},
    Reader,
};

use crate::{error::GapixError, model::Route};

use super::{
    attributes::Attributes, extensions::parse_extensions, link::parse_link,
    waypoint::parse_waypoint, xml_reader_extensions::XmlReaderExtensions,
};

pub(crate) fn parse_route(
    start_element: &BytesStart<'_>,
    xml_reader: &mut Reader<&[u8]>,
) -> Result<Route, GapixError> {
    Attributes::check_is_empty(start_element, xml_reader)?;

    let mut route = Route::default();

    loop {
        match xml_reader.read_event() {
            Ok(Event::Start(start)) => match start.name().as_ref() {
                b"name" => {
                    route.name = Some(xml_reader.read_inner_as()?);
                }
                b"cmt" => {
                    route.comment = Some(xml_reader.read_inner_as()?);
                }
                b"desc" => {
                    route.description = Some(xml_reader.read_inner_as()?);
                }
                b"src" => {
                    route.source = Some(xml_reader.read_inner_as()?);
                }
                b"link" => {
                    let link = parse_link(&start, xml_reader)?;
                    route.links.push(link);
                }
                b"number" => {
                    route.number = Some(xml_reader.read_inner_as()?);
                }
                b"type" => {
                    route.r#type = Some(xml_reader.read_inner_as()?);
                }
                b"extensions" => {
                    route.extensions = Some(parse_extensions(&start, xml_reader)?);
                }
                b"rtept" => {
                    let point = parse_waypoint(&start, xml_reader)?;
                    route.points.push(point);
                }
                e => return Err(GapixError::bad_start(e, xml_reader)),
            },
            Ok(Event::End(e)) => {
                let n = e.name();
                let n = n.as_ref();
                if n == start_element.name().as_ref() {
                    return Ok(route);
                } else if n == b"name"
                    || n == b"cmt"
                    || n == b"desc"
                    || n == b"src"
                    || n == b"link"
                    || n == b"number"
                    || n == b"type"
                    || n == b"extensions"
                    || n == b"rtept"
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
    use crate::read::xml_reader_extensions::start_parse;

    use super::*;
    use quick_xml::Reader;

    #[test]
    fn valid_route_all_fields() {
        let mut xml_reader = Reader::from_str(
            r#"<rte>
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
               </rte>"#,
        );

        let start = start_parse(&mut xml_reader);
        let result = parse_route(&start, &mut xml_reader).unwrap();
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
    }

    #[test]
    fn extra_elements() {
        let mut xml_reader = Reader::from_str(
            r#"<rte>
                 <foo>bar</foo>
               </rte>"#,
        );

        let start = start_parse(&mut xml_reader);
        match parse_route(&start, &mut xml_reader) {
            Err(GapixError::UnexpectedStartElement(_)) => {}
            x => panic!("Unexpected result from parse(): {:?}", x),
        };
    }

    #[test]
    fn extra_attributes() {
        let mut xml_reader = Reader::from_str(
            r#"<rte foo="bar">
               </rte>"#,
        );

        let start = start_parse(&mut xml_reader);
        match parse_route(&start, &mut xml_reader) {
            Err(GapixError::UnexpectedAttributes { .. }) => {}
            x => panic!("Unexpected result from parse(): {:?}", x),
        };
    }
}
