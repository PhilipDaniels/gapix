use quick_xml::{
    events::{BytesStart, Event},
    Reader,
};

use crate::{error::GapixError, model::Copyright};

use super::{attributes::Attributes, XmlReaderExtensions};

pub(crate) fn parse_copyright(
    start_element: &BytesStart<'_>,
    xml_reader: &mut Reader<&[u8]>,
) -> Result<Copyright, GapixError> {
    Attributes::check_is_empty(start_element, xml_reader)?;

    let mut copyright = Copyright::default();

    loop {
        match xml_reader.read_event() {
            Ok(Event::Start(start)) => match start.name().as_ref() {
                b"year" => {
                    copyright.year = Some(xml_reader.read_inner_as()?);
                }
                b"license" => {
                    copyright.license = Some(xml_reader.read_inner_as()?);
                }
                b"author" => {
                    copyright.author = xml_reader.read_inner_as()?;
                }
                e => return Err(GapixError::bad_start(e, xml_reader)),
            },
            Ok(Event::End(e)) => {
                let n = e.name();
                let n = n.as_ref();

                if n == start_element.name().as_ref() {
                    if copyright.author.is_empty() {
                        return Err(GapixError::MandatoryElementNotFound("author".to_string()));
                    }

                    return Ok(copyright);
                } else if n == b"year" || n == b"license" || n == b"author" {
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
    fn valid_copyright_all_fields() {
        let mut xml_reader = Reader::from_str(
            r#"<copyright>
                 <year>2024</year>
                 <license>MIT</license>
                 <author>Homer Simpson</author>
               </copyright>"#,
        );

        let start = start_parse(&mut xml_reader);
        let result = parse_copyright(&start, &mut xml_reader).unwrap();
        assert_eq!(result.year, Some(2024));
        assert_eq!(result.license, Some("MIT".to_string()));
        assert_eq!(result.author, "Homer Simpson");
    }

    #[test]
    fn valid_copyright_author_only() {
        let mut xml_reader = Reader::from_str(
            r#"<copyright>
                 <author>Homer Simpson</author>
               </copyright>"#,
        );

        let start = start_parse(&mut xml_reader);
        let result = parse_copyright(&start, &mut xml_reader).unwrap();
        assert_eq!(result.year, None);
        assert_eq!(result.license, None);
        assert_eq!(result.author, "Homer Simpson");
    }

    #[test]
    fn valid_copyright_missing_license() {
        let mut xml_reader = Reader::from_str(
            r#"<copyright>
                 <year>2024</year>
                 <author>Homer Simpson</author>
               </copyright>"#,
        );

        let start = start_parse(&mut xml_reader);
        let result = parse_copyright(&start, &mut xml_reader).unwrap();
        assert_eq!(result.year, Some(2024));
        assert_eq!(result.license, None);
        assert_eq!(result.author, "Homer Simpson");
    }

    #[test]
    fn missing_author() {
        let mut xml_reader = Reader::from_str(
            r#"<copyright>
                 <year>2024</year>
                 <license>MIT</license>
               </copyright>"#,
        );

        let start = start_parse(&mut xml_reader);
        let result = parse_copyright(&start, &mut xml_reader);
        assert!(result.is_err());
    }

    #[test]
    fn extra_elements() {
        let mut xml_reader = Reader::from_str(
            r#"<copyright>
                 <author>Homer Simpson</author>
                 <foo>bar</foo>
               </copyright>"#,
        );

        let start = start_parse(&mut xml_reader);
        let result = parse_copyright(&start, &mut xml_reader);
        assert!(result.is_err());
    }

    #[test]
    fn extra_attributes() {
        let mut xml_reader = Reader::from_str(
            r#"<copyright foo="bar">
                 <author>Homer Simpson</author>
               </copyright>"#,
        );

        let start = start_parse(&mut xml_reader);
        let result = parse_copyright(&start, &mut xml_reader);
        assert!(result.is_err());
    }
}
