use quick_xml::events::BytesStart;

use crate::{error::GapixError, model::Bounds};

use super::{attributes::Attributes, XmlReaderConversions};

pub(crate) fn parse_bounds<C: XmlReaderConversions>(
    start_element: &BytesStart<'_>,
    converter: &C,
) -> Result<Bounds, GapixError> {
    let mut attributes = Attributes::new(start_element, converter)?;
    let bounds = Bounds::new(
        attributes.get("minlat")?,
        attributes.get("minlon")?,
        attributes.get("maxlat")?,
        attributes.get("maxlon")?,
    )?;
    attributes.check_is_empty_now()?;
    Ok(bounds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::read::start_parse;
    use quick_xml::Reader;

    #[test]
    fn valid_bounds() {
        let mut xml_reader = Reader::from_str(
            r#"<bounds minlat="-1.1" maxlat="1.1" minlon="-53.1111" maxlon="88.88">"#,
        );
        let start = start_parse(&mut xml_reader);
        let result = parse_bounds(&start, &xml_reader).unwrap();
        assert_eq!(result.min_lat, -1.1);
        assert_eq!(result.max_lat, 1.1);
        assert_eq!(result.min_lon, -53.1111);
        assert_eq!(result.max_lon, 88.88);
    }

    #[test]
    fn missing_min_lat() {
        let mut xml_reader =
            Reader::from_str(r#"<bounds maxlat="1.1" minlon="-53.1111" maxlon="88.88">"#);
        let start = start_parse(&mut xml_reader);
        match parse_bounds(&start, &mut xml_reader) {
            Err(GapixError::MandatoryAttributeNotFound(a)) if a == "minlat" => {}
            x => panic!("Unexpected result from parse(): {:?}", x),
        };
    }

    #[test]
    fn missing_max_lat() {
        let mut xml_reader =
            Reader::from_str(r#"<bounds minlat="-1.1" minlon="-53.1111" maxlon="88.88">"#);
        let start = start_parse(&mut xml_reader);
        match parse_bounds(&start, &mut xml_reader) {
            Err(GapixError::MandatoryAttributeNotFound(a)) if a == "maxlat" => {}
            x => panic!("Unexpected result from parse(): {:?}", x),
        };
    }

    #[test]
    fn missing_min_lon() {
        let mut xml_reader =
            Reader::from_str(r#"<bounds minlat="-1.1" maxlat="1.1" maxlon="88.88">"#);
        let start = start_parse(&mut xml_reader);
        match parse_bounds(&start, &mut xml_reader) {
            Err(GapixError::MandatoryAttributeNotFound(a)) if a == "minlon" => {}
            x => panic!("Unexpected result from parse(): {:?}", x),
        };
    }

    #[test]
    fn missing_max_lon() {
        let mut xml_reader =
            Reader::from_str(r#"<bounds minlat="-1.1" maxlat="1.1" minlon="-53.1111">"#);
        let start = start_parse(&mut xml_reader);
        match parse_bounds(&start, &mut xml_reader) {
            Err(GapixError::MandatoryAttributeNotFound(a)) if a == "maxlon" => {}
            x => panic!("Unexpected result from parse(): {:?}", x),
        };
    }

    #[test]
    fn missing_all() {
        let mut xml_reader = Reader::from_str(r#"<bounds>"#);
        let start = start_parse(&mut xml_reader);
        let result = parse_bounds(&start, &xml_reader);
        assert!(result.is_err());
    }

    #[test]
    fn extra_attributes() {
        let mut xml_reader = Reader::from_str(
            r#"<bounds minlat="-1.1" maxlat="1.1" minlon="-53.1111" maxlon="88.88" foo="bar">"#,
        );
        let start = start_parse(&mut xml_reader);
        match parse_bounds(&start, &mut xml_reader) {
            Err(GapixError::UnexpectedAttributes { .. }) => {}
            x => panic!("Unexpected result from parse(): {:?}", x),
        };
    }

    #[test]
    fn invalid_min_lat() {
        let mut xml_reader = Reader::from_str(
            r#"<bounds minlat="360.1" maxlat="1.1" minlon="-53.1111" maxlon="88.88">"#,
        );
        let start = start_parse(&mut xml_reader);
        match parse_bounds(&start, &mut xml_reader) {
            Err(GapixError::InvalidLatitude(360.1)) => {}
            x => panic!("Unexpected result from parse(): {:?}", x),
        };
    }

    #[test]
    fn invalid_max_lat() {
        let mut xml_reader = Reader::from_str(
            r#"<bounds minlat="-1.1" maxlat="360.1" minlon="-53.1111" maxlon="88.88">"#,
        );
        let start = start_parse(&mut xml_reader);
        match parse_bounds(&start, &mut xml_reader) {
            Err(GapixError::InvalidLatitude(360.1)) => {}
            x => panic!("Unexpected result from parse(): {:?}", x),
        };
    }

    #[test]
    fn invalid_min_lon() {
        let mut xml_reader = Reader::from_str(
            r#"<bounds minlat="-1.1" maxlat="1.1" minlon="-5453.1111" maxlon="88.88">"#,
        );
        let start = start_parse(&mut xml_reader);
        match parse_bounds(&start, &mut xml_reader) {
            Err(GapixError::InvalidLongitude(-5453.1111)) => {}
            x => panic!("Unexpected result from parse(): {:?}", x),
        };
    }

    #[test]
    fn invalid_max_lon() {
        let mut xml_reader = Reader::from_str(
            r#"<bounds minlat="-1.1" maxlat="1.1" minlon="-53.1111" maxlon="888.88">"#,
        );
        let start = start_parse(&mut xml_reader);
        match parse_bounds(&start, &mut xml_reader) {
            Err(GapixError::InvalidLongitude(888.88)) => {}
            x => panic!("Unexpected result from parse(): {:?}", x),
        };
    }
}
