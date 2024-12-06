use quick_xml::{events::BytesStart, Reader};

use crate::{error::GapixError, model::Extensions};

use super::attributes::Attributes;

pub(crate) fn parse_extensions(
    start_element: &BytesStart<'_>,
    xml_reader: &mut Reader<&[u8]>,
) -> Result<Extensions, GapixError> {
    Attributes::check_is_empty(start_element, xml_reader)?;

    let start = BytesStart::new("extensions");
    let end = start.to_end();
    let text = xml_reader.read_text(end.name())?;
    let ext = Extensions::new(text.trim());
    Ok(ext)
}

#[cfg(test)]
mod tests {
    use crate::read::xml_reader_extensions::start_parse;

    use super::*;
    use quick_xml::Reader;

    #[test]
    fn valid_empty() {
        let mut xml_reader = Reader::from_str(
            r#"<extensions>  
               </extensions>"#,
        );

        let start = start_parse(&mut xml_reader);
        let result = parse_extensions(&start, &mut xml_reader).unwrap();
        assert!(result.raw_xml.is_empty());
    }

    #[test]
    fn valid_extensions_and_preserves_newlines() {
        let mut xml_reader = Reader::from_str(
            r#"<extensions>  
                  <foo bar="42">inner text</foo>
<plod>12</plod>
               </extensions>"#,
        );

        let start = start_parse(&mut xml_reader);
        let result = parse_extensions(&start, &mut xml_reader).unwrap();
        assert_eq!(
            result.raw_xml,
            r#"<foo bar="42">inner text</foo>
<plod>12</plod>"#
        );
    }
}
