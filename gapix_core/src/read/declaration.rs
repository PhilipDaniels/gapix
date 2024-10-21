use quick_xml::events::BytesDecl;

use crate::{error::GapixError, model::XmlDeclaration};

use super::XmlReaderConversions;

/// Parses an XML declaration, i.e. the very first line of the file which is:
///     <?xml version="1.0" encoding="UTF-8"?>
pub(crate) fn parse_declaration<C: XmlReaderConversions>(
    declaration: &BytesDecl<'_>,
    converter: &C,
) -> Result<XmlDeclaration, GapixError> {
    let version = converter.cow_to_string(declaration.version()?)?;

    let encoding = if let Some(enc) = declaration.encoding() {
        let enc = enc?;
        Some(converter.cow_to_string(enc)?)
    } else {
        None
    };

    let standalone = if let Some(sa) = declaration.standalone() {
        let sa = sa?;
        Some(converter.cow_to_string(sa)?)
    } else {
        None
    };

    Ok(XmlDeclaration {
        version,
        encoding,
        standalone,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use quick_xml::{events::Event, Reader};

    // This one is a bit different, returns a different type.
    fn start_parse_of_decl<'a>(
        xml_reader: &mut Reader<&'a [u8]>,
    ) -> quick_xml::events::BytesDecl<'a> {
        match xml_reader.read_event().unwrap() {
            Event::Decl(decl) => decl,
            _ => panic!("Failed to parse Event::Decl(_) element"),
        }
    }

    #[test]
    fn valid_declaration() {
        let mut xml_reader = Reader::from_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
        let start = start_parse_of_decl(&mut xml_reader);
        let result = parse_declaration(&start, &xml_reader).unwrap();
        assert_eq!(result.version, "1.0");
        assert_eq!(result.encoding, Some("UTF-8".to_string()));
        assert_eq!(result.standalone, None);
    }

    #[test]
    fn valid_declaration_with_standalone() {
        let mut xml_reader =
            Reader::from_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes" ?>"#);
        let start = start_parse_of_decl(&mut xml_reader);
        let result = parse_declaration(&start, &xml_reader).unwrap();
        assert_eq!(result.version, "1.0");
        assert_eq!(result.encoding, Some("UTF-8".to_string()));
        assert_eq!(result.standalone, Some("yes".to_string()));
    }

    #[test]
    fn valid_declaration_missing_encoding() {
        let mut xml_reader = Reader::from_str(r#"<?xml version="1.0" ?>"#);
        let start = start_parse_of_decl(&mut xml_reader);
        let result = parse_declaration(&start, &xml_reader).unwrap();
        assert_eq!(result.version, "1.0");
        assert_eq!(result.encoding, None);
        assert_eq!(result.standalone, None);
    }

    #[test]
    fn missing_version() {
        let mut xml_reader = Reader::from_str(r#"<?xml encoding="UTF-8"?>"#);
        let start = start_parse_of_decl(&mut xml_reader);
        match parse_declaration(&start, &mut xml_reader) {
            Err(GapixError::XmlError { .. }) => {}
            x => panic!("Unexpected result from parse(): {:?}", x),
        };
    }

    // TODO: Log this as a bug, it should not allow extra attributes.
    #[test]
    fn extra_attributes() {
        let mut xml_reader = Reader::from_str(r#"<?xml version="1.0" foo="bar"?>"#);
        let start = start_parse_of_decl(&mut xml_reader);
        let result = parse_declaration(&start, &xml_reader).unwrap();
        assert_eq!(result.version, "1.0");
        assert_eq!(result.encoding, None);
        assert_eq!(result.standalone, None);
    }
}
