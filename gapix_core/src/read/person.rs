use anyhow::{bail, Result};
use quick_xml::{
    events::{BytesStart, Event},
    Reader,
};

use crate::model::Person;

use super::{
    check_no_attributes, email::parse_email, link::parse_link, XmlReaderConversions,
    XmlReaderExtensions,
};

pub(crate) fn parse_person(
    start_element: &BytesStart<'_>,
    xml_reader: &mut Reader<&[u8]>,
) -> Result<Person> {
    check_no_attributes(start_element, xml_reader)?;

    let mut person = Person::default();

    loop {
        match xml_reader.read_event() {
            Ok(Event::Start(start)) => match start.name().as_ref() {
                b"name" => {
                    person.name = Some(xml_reader.read_inner_as()?);
                }
                b"email" => {
                    person.email = Some(parse_email(&start, xml_reader)?);
                }
                b"link" => {
                    person.link = Some(parse_link(&start, xml_reader)?);
                }
                e => bail!("Unexpected Start element {:?}", xml_reader.bytes_to_cow(e)),
            },
            Ok(Event::End(e)) => {
                if e.name().as_ref() == start_element.name().as_ref() {
                    return Ok(person);
                } else {
                    // TODO: Check for all valid ends.
                }
            }
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
    fn valid_person_all_fields() {
        let mut xml_reader = Reader::from_str(
            r#"<person>
                 <name>Homer Simpson</name>
                 <email id="phil" domain="gmail.com"></email>
                 <link href="http://example.com">
                    <text>Some text here</text>
                    <type>jpeg</type>
                </link>
               </person>"#,
        );

        let start = start_parse(&mut xml_reader).unwrap();
        let result = parse_person(&start, &mut xml_reader).unwrap();
        assert_eq!(result.name, Some("Homer Simpson".to_string()));
        let email = result.email.unwrap();
        assert_eq!(email.id, "phil");
        assert_eq!(email.domain, "gmail.com");
        let link = result.link.unwrap();
        assert_eq!(link.href, "http://example.com");
    }

    #[test]
    fn valid_person_no_fields() {
        let mut xml_reader = Reader::from_str(r#"<person></person>"#);

        let start = start_parse(&mut xml_reader).unwrap();
        let result = parse_person(&start, &mut xml_reader).unwrap();
        assert_eq!(result.name, None);
        assert!(result.email.is_none());
        assert!(result.link.is_none());
    }

    #[test]
    fn extra_attributes() {
        let mut xml_reader = Reader::from_str(r#"<person foo="bar"></person>"#);

        let start = start_parse(&mut xml_reader).unwrap();
        let result = parse_person(&start, &mut xml_reader);
        assert!(result.is_err());
    }

    #[test]
    fn extra_fields() {
        let mut xml_reader = Reader::from_str(r#"<person><foo>bar</foo></person>"#);

        let start = start_parse(&mut xml_reader).unwrap();
        let result = parse_person(&start, &mut xml_reader);
        assert!(result.is_err());
    }
}
