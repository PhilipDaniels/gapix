use anyhow::{bail, Result};
use quick_xml::{
    events::{BytesStart, Event},
    Reader,
};

use crate::model::Metadata;

use super::{
    bounds::parse_bounds, check_no_attributes, copyright::parse_copyright,
    extensions::parse_extensions, link::parse_link, person::parse_person, XmlReaderConversions,
    XmlReaderExtensions,
};

pub(crate) fn parse_metadata(
    start_element: &BytesStart<'_>,
    xml_reader: &mut Reader<&[u8]>,
) -> Result<Metadata> {
    check_no_attributes(start_element, xml_reader)?;

    let mut md = Metadata::default();

    loop {
        match xml_reader.read_event() {
            Ok(Event::Start(start)) => match start.name().as_ref() {
                b"name" => {
                    md.name = Some(xml_reader.read_inner_as()?);
                }
                b"desc" => {
                    md.description = Some(xml_reader.read_inner_as()?);
                }
                b"author" => {
                    md.author = Some(parse_person(&start, xml_reader)?);
                }
                b"copyright" => {
                    md.copyright = Some(parse_copyright(&start, xml_reader)?);
                }
                b"link" => {
                    let link = parse_link(&start, xml_reader)?;
                    md.links.push(link);
                }
                b"time" => {
                    md.time = Some(xml_reader.read_inner_as_time()?);
                }
                b"keywords" => {
                    md.keywords = Some(xml_reader.read_inner_as()?);
                }
                b"bounds" => {
                    md.bounds = Some(parse_bounds(&start, xml_reader)?);
                }
                b"extensions" => {
                    md.extensions = Some(parse_extensions(&start, xml_reader)?);
                }
                e => bail!("Unexpected Start element {:?}", xml_reader.bytes_to_cow(e)),
            },
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"metadata" => {
                    return Ok(md);
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
    use time::{format_description::well_known, OffsetDateTime};

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

        let start = start_parse(&mut xml_reader).unwrap();
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
            Some(OffsetDateTime::parse("2024-02-02T10:10:54.000Z", &well_known::Rfc3339).unwrap())
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

        let start = start_parse(&mut xml_reader).unwrap();
        let result = parse_metadata(&start, &mut xml_reader);
        assert!(result.is_err());
    }

    #[test]
    fn extra_attributes() {
        let mut xml_reader = Reader::from_str(
            r#"<metadata foo="bar">
               </metadata>"#,
        );

        let start = start_parse(&mut xml_reader).unwrap();
        let result = parse_metadata(&start, &mut xml_reader);
        assert!(result.is_err());
    }
}
