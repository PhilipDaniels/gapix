use quick_xml::{
    events::{BytesStart, Event},
    Reader,
};

use crate::{error::GapixError, model::Metadata};

use super::{
    attributes::Attributes, bounds::parse_bounds, copyright::parse_copyright,
    extensions::parse_extensions, link::parse_link, person::parse_person, xml_reader_extensions::XmlReaderExtensions,
};

pub(crate) fn parse_metadata(
    start_element: &BytesStart<'_>,
    xml_reader: &mut Reader<&[u8]>,
) -> Result<Metadata, GapixError> {
    Attributes::check_is_empty(start_element, xml_reader)?;

    let mut metadata = Metadata::default();

    loop {
        match xml_reader.read_event() {
            Ok(Event::Start(start)) => match start.name().as_ref() {
                b"name" => {
                    metadata.name = Some(xml_reader.read_inner_as()?);
                }
                b"desc" => {
                    metadata.description = Some(xml_reader.read_inner_as()?);
                }
                b"author" => {
                    metadata.author = Some(parse_person(&start, xml_reader)?);
                }
                b"copyright" => {
                    metadata.copyright = Some(parse_copyright(&start, xml_reader)?);
                }
                b"link" => {
                    let link = parse_link(&start, xml_reader)?;
                    metadata.links.push(link);
                }
                b"time" => {
                    metadata.time = Some(xml_reader.read_inner_as_time()?);
                }
                b"keywords" => {
                    metadata.keywords = Some(xml_reader.read_inner_as()?);
                }
                b"bounds" => {
                    // Bounds can come as <bounds></bounds> which will trigger this case.
                    metadata.bounds = Some(parse_bounds(&start, xml_reader)?);
                }
                b"extensions" => {
                    metadata.extensions = Some(parse_extensions(&start, xml_reader)?);
                }
                e => return Err(GapixError::bad_start(e, xml_reader)),
            },
            Ok(Event::Empty(start)) => {
                // Bounds can come as <bounds /> which will trigger this case.
                if start.name().as_ref() == b"bounds" {
                    metadata.bounds = Some(parse_bounds(&start, xml_reader)?);
                } else {
                    return Err(GapixError::bad_event(Ok(Event::Empty(start))));
                }
            }
            Ok(Event::End(e)) => {
                let n = e.name();
                let n = n.as_ref();
                if n == start_element.name().as_ref() {
                    return Ok(metadata);
                } else if n == b"name"
                    || n == b"desc"
                    || n == b"author"
                    || n == b"copyright"
                    || n == b"link"
                    || n == b"time"
                    || n == b"keywords"
                    || n == b"bounds"
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
    use crate::read::xml_reader_extensions::start_parse;

    use super::*;
    use chrono::DateTime;
    use quick_xml::Reader;

    #[test]
    fn valid_metadata_all_fields() {
        let mut xml_reader = Reader::from_str(
            r#"<metadata>
                 <name>Homer Simpson</name>
                 <desc>description</desc>
                 <author>
                    <name>Sauron</name>
                    <email id="bigeye" domain="mordor.com"></email>
                    <link href="http://example2.com">
                    </link>
                 </author>
                 <copyright>
                    <year>2024</year>
                    <license>MIT</license>
                    <author>Homer Simpson</author>
                 </copyright>
                 <link href="http://example.com">
                    <text>Some text here</text>
                    <type>jpeg</type>
                 </link>
                 <link href="http://example2.com">
                    <text>Some text here2</text>
                    <type>jpeg2</type>
                 </link>
                 <time>2024-02-02T10:10:54.000Z</time>
                 <keywords>keyword1, keyword2</keywords>
                 <bounds minlat="-1.1" maxlat="1.1" minlon="-53.1111" maxlon="88.88"></bounds>
                 <extensions><foo><ex:ex1>extended data</ex:ex1></foo></extensions>
               </metadata>"#,
        );

        let start = start_parse(&mut xml_reader);
        let result = parse_metadata(&start, &mut xml_reader).unwrap();
        assert_eq!(result.name, Some("Homer Simpson".to_string()));
        assert_eq!(result.description, Some("description".to_string()));
        let author = result.author.unwrap();
        assert_eq!(author.name, Some("Sauron".to_string()));
        let email = author.email.unwrap();
        assert_eq!(email.id, "bigeye");
        assert_eq!(email.domain, "mordor.com");
        assert_eq!(author.link.unwrap().href, "http://example2.com");
        let copyright = result.copyright.unwrap();
        assert_eq!(copyright.year, Some(2024));
        assert_eq!(result.links.len(), 2);
        assert_eq!(result.links[0].href, "http://example.com");
        assert_eq!(result.links[1].href, "http://example2.com");
        assert_eq!(
            result.time,
            Some(DateTime::parse_from_rfc3339("2024-02-02T10:10:54.000Z").unwrap().to_utc())
        );
        assert_eq!(result.keywords, Some("keyword1, keyword2".to_string()));
        let bounds = result.bounds.unwrap();
        assert_eq!(bounds.min_lat, -1.1);
        assert_eq!(bounds.max_lat, 1.1);
        assert_eq!(bounds.min_lon, -53.1111);
        assert_eq!(bounds.max_lon, 88.88);
        let ext = result.extensions.unwrap();
        assert_eq!(ext.raw_xml, "<foo><ex:ex1>extended data</ex:ex1></foo>");
    }

    #[test]
    fn valid_metadata_self_closing_bounds() {
        let mut xml_reader = Reader::from_str(
            r#"<metadata>
                 <name>Homer Simpson</name>
                 <desc>description</desc>
                 <author>
                    <name>Sauron</name>
                    <email id="bigeye" domain="mordor.com"></email>
                    <link href="http://example2.com">
                    </link>
                 </author>
                 <copyright>
                    <year>2024</year>
                    <license>MIT</license>
                    <author>Homer Simpson</author>
                 </copyright>
                 <link href="http://example.com">
                    <text>Some text here</text>
                    <type>jpeg</type>
                 </link>
                 <link href="http://example2.com">
                    <text>Some text here2</text>
                    <type>jpeg2</type>
                 </link>
                 <time>2024-02-02T10:10:54.000Z</time>
                 <keywords>keyword1, keyword2</keywords>
                 <bounds minlat="-1.1" maxlat="1.1" minlon="-53.1111" maxlon="88.88" />
                 <extensions><foo><ex:ex1>extended data</ex:ex1></foo></extensions>
               </metadata>"#,
        );

        let start = start_parse(&mut xml_reader);
        let result = parse_metadata(&start, &mut xml_reader).unwrap();
        assert_eq!(result.name, Some("Homer Simpson".to_string()));
        assert_eq!(result.description, Some("description".to_string()));
        let author = result.author.unwrap();
        assert_eq!(author.name, Some("Sauron".to_string()));
        let email = author.email.unwrap();
        assert_eq!(email.id, "bigeye");
        assert_eq!(email.domain, "mordor.com");
        assert_eq!(author.link.unwrap().href, "http://example2.com");
        let copyright = result.copyright.unwrap();
        assert_eq!(copyright.year, Some(2024));
        assert_eq!(result.links.len(), 2);
        assert_eq!(result.links[0].href, "http://example.com");
        assert_eq!(result.links[1].href, "http://example2.com");
        assert_eq!(
            result.time,
            Some(DateTime::parse_from_rfc3339("2024-02-02T10:10:54.000Z").unwrap().to_utc())
        );
        assert_eq!(result.keywords, Some("keyword1, keyword2".to_string()));
        let bounds = result.bounds.unwrap();
        assert_eq!(bounds.min_lat, -1.1);
        assert_eq!(bounds.max_lat, 1.1);
        assert_eq!(bounds.min_lon, -53.1111);
        assert_eq!(bounds.max_lon, 88.88);
        let ext = result.extensions.unwrap();
        assert_eq!(ext.raw_xml, "<foo><ex:ex1>extended data</ex:ex1></foo>");
    }

    #[test]
    fn extra_elements() {
        let mut xml_reader = Reader::from_str(
            r#"<metadata>
                 <foo>bar</foo>
               </metadata>"#,
        );

        let start = start_parse(&mut xml_reader);
        match parse_metadata(&start, &mut xml_reader) {
            Err(GapixError::UnexpectedStartElement(_)) => {}
            x => panic!("Unexpected result from parse(): {:?}", x),
        };
    }

    #[test]
    fn extra_attributes() {
        let mut xml_reader = Reader::from_str(
            r#"<metadata foo="bar">
               </metadata>"#,
        );

        let start = start_parse(&mut xml_reader);
        match parse_metadata(&start, &mut xml_reader) {
            Err(GapixError::UnexpectedAttributes { .. }) => {}
            x => panic!("Unexpected result from parse(): {:?}", x),
        };
    }
}
