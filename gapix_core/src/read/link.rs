use anyhow::{bail, Result};
use quick_xml::{
    events::{BytesStart, Event},
    Reader,
};

use crate::model::Link;

use super::{attributes::Attributes, XmlReaderConversions, XmlReaderExtensions};

pub(crate) fn parse_link(
    start_element: &BytesStart<'_>,
    xml_reader: &mut Reader<&[u8]>,
) -> Result<Link> {
    let mut attributes = Attributes::new(start_element, xml_reader)?;
    let mut link = Link::default();
    link.href = attributes.get("href")?;
    if !attributes.is_empty() {
        bail!("Found extra attributes on 'link' element");
    }

    loop {
        match xml_reader.read_event() {
            Ok(Event::Start(start)) => match start.name().as_ref() {
                b"text" => {
                    link.text = Some(xml_reader.read_inner_as()?);
                }
                b"type" => {
                    link.r#type = Some(xml_reader.read_inner_as()?);
                }
                e => bail!("Unexpected Start element {:?}", xml_reader.bytes_to_cow(e)),
            },
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"link" => {
                    return Ok(link);
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
    fn valid_link_all_fields() {
        let mut xml_reader = Reader::from_str(
            r#"<link href="http://example.com">
                 <text>Some text here</text>
                 <type>jpeg</type>
               </link>"#,
        );

        let start = start_parse(&mut xml_reader).unwrap();
        let result = parse_link(&start, &mut xml_reader).unwrap();
        assert_eq!(result.href, "http://example.com");
        assert_eq!(result.text, Some("Some text here".to_string()));
        assert_eq!(result.r#type, Some("jpeg".to_string()));
    }

    #[test]
    fn valid_link_href_only() {
        let mut xml_reader = Reader::from_str(r#"<link href="http://example.com"></link>"#);

        let start = start_parse(&mut xml_reader).unwrap();
        let result = parse_link(&start, &mut xml_reader).unwrap();
        assert_eq!(result.href, "http://example.com");
        assert_eq!(result.text, None);
        assert_eq!(result.r#type, None);
    }

    #[test]
    fn missing_href() {
        let mut xml_reader = Reader::from_str(
            r#"<link>
                 <text>Some text here</text>
                 <type>jpeg</type>
               </link>"#,
        );

        let start = start_parse(&mut xml_reader).unwrap();
        let result = parse_link(&start, &mut xml_reader);
        assert!(result.is_err());
    }

    #[test]
    fn extra_elements() {
        let mut xml_reader = Reader::from_str(
            r#"<link href="http://example.com">
                 <text>Some text here</text>
                 <type>jpeg</type>
                 <foo>bar</foo>
               </link>"#,
        );

        let start = start_parse(&mut xml_reader).unwrap();
        let result = parse_link(&start, &mut xml_reader);
        assert!(result.is_err());
    }

    #[test]
    fn extra_attributes() {
        let mut xml_reader = Reader::from_str(
            r#"<link href="http://example.com" foo="bar">
               </link>"#,
        );

        let start = start_parse(&mut xml_reader).unwrap();
        let result = parse_link(&start, &mut xml_reader);
        assert!(result.is_err());
    }
}
