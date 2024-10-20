use quick_xml::events::BytesStart;

use crate::{error::GapixError, model::Email};

use super::{attributes::Attributes, XmlReaderConversions};

/// Parses an element of the form: <email id="phil" domain="gmail.com">
pub(crate) fn parse_email<C: XmlReaderConversions>(
    start_element: &BytesStart<'_>,
    converter: &C,
) -> Result<Email, GapixError> {
    let mut attributes = Attributes::new(start_element, converter)?;
    let id: String = attributes.get("id")?;
    let domain: String = attributes.get("domain")?;
    attributes.check_is_empty_now()?;
    Ok(Email::new(id, domain))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::read::start_parse;
    use quick_xml::Reader;

    #[test]
    fn valid_email() {
        let mut xml_reader = Reader::from_str(r#"<email id="phil" domain="gmail.com">"#);
        let start = start_parse(&mut xml_reader);
        let result = parse_email(&start, &xml_reader).unwrap();
        assert_eq!(result.id, "phil");
        assert_eq!(result.domain, "gmail.com");
    }

    #[test]
    fn missing_domain() {
        let mut xml_reader = Reader::from_str(r#"<email id="phil">"#);
        let start = start_parse(&mut xml_reader);
        let result = parse_email(&start, &xml_reader);
        assert!(result.is_err());
    }

    #[test]
    fn missing_id() {
        let mut xml_reader = Reader::from_str(r#"<email domain="gmail.com">"#);
        let start = start_parse(&mut xml_reader);
        let result = parse_email(&start, &xml_reader);
        assert!(result.is_err());
    }

    #[test]
    fn missing_both() {
        let mut xml_reader = Reader::from_str(r#"<email>"#);
        let start = start_parse(&mut xml_reader);
        let result = parse_email(&start, &xml_reader);
        assert!(result.is_err());
    }

    #[test]
    fn extras() {
        let mut xml_reader = Reader::from_str(r#"<email id="phil" domain="gmail.com" foo="bar">"#);
        let start = start_parse(&mut xml_reader);
        let result = parse_email(&start, &xml_reader);
        assert!(result.is_err());
    }
}
