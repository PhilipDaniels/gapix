use quick_xml::{
    events::{BytesStart, Event},
    Reader,
};

use crate::{error::GapixError, model::Link};

use super::{attributes::Attributes, XmlReaderExtensions};

pub(crate) fn parse_link(
    start_element: &BytesStart<'_>,
    xml_reader: &mut Reader<&[u8]>,
) -> Result<Link, GapixError> {
    let mut attributes = Attributes::new(start_element, xml_reader)?;
    let mut link = Link {
        href: attributes.get("href")?,
        ..Default::default()
    };
    attributes.check_is_empty_now()?;

    loop {
        match xml_reader.read_event() {
            Ok(Event::Start(start)) => match start.name().as_ref() {
                b"text" => {
                    link.text = Some(xml_reader.read_inner_as()?);
                }
                b"type" => {
                    link.r#type = Some(xml_reader.read_inner_as()?);
                }
                e => return Err(GapixError::bad_start(e, xml_reader)),
            },
            Ok(Event::End(e)) => {
                let n = e.name();
                let n = n.as_ref();
                if n == start_element.name().as_ref() {
                    return Ok(link);
                } else if n == b"text" || n == b"type" {
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
    fn valid_link_all_fields() {
        let mut xml_reader = Reader::from_str(
            r#"<link href="http://example.com">
                 <text>Some text here</text>
                 <type>jpeg</type>
               </link>"#,
        );

        let start = start_parse(&mut xml_reader);
        let result = parse_link(&start, &mut xml_reader).unwrap();
        assert_eq!(result.href, "http://example.com");
        assert_eq!(result.text, Some("Some text here".to_string()));
        assert_eq!(result.r#type, Some("jpeg".to_string()));
    }

    #[test]
    fn valid_link_href_only() {
        let mut xml_reader = Reader::from_str(r#"<link href="http://example.com"></link>"#);

        let start = start_parse(&mut xml_reader);
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

        let start = start_parse(&mut xml_reader);
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

        let start = start_parse(&mut xml_reader);
        match parse_link(&start, &mut xml_reader) {
            Err(GapixError::UnexpectedStartElement(_)) => {}
            x => panic!("Unexpected result from parse(): {:?}", x),
        };
    }

    #[test]
    fn extra_attributes() {
        let mut xml_reader = Reader::from_str(
            r#"<link href="http://example.com" foo="bar">
               </link>"#,
        );

        let start = start_parse(&mut xml_reader);
        match parse_link(&start, &mut xml_reader) {
            Err(GapixError::UnexpectedAttributes { .. }) => {}
            x => panic!("Unexpected result from parse(): {:?}", x),
        };
    }
}
