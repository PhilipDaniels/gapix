use std::{
    collections::{hash_map::Entry, HashMap},
    str::FromStr,
};

use quick_xml::events::BytesStart;

use crate::error::GapixError;

use super::XmlReaderConversions;

#[derive(Debug)]
pub(crate) struct Attributes {
    data: HashMap<String, String>,
    start_element_name: String,
}

impl Attributes {
    /// Creates a new Attributes object by extracting all the attributes of the
    /// specified tag and holding them as attr=value pairs for later use.
    pub(crate) fn new<C: XmlReaderConversions>(
        start_element: &BytesStart<'_>,
        converter: &C,
    ) -> Result<Self, GapixError> {
        let start_element_name = converter
            .bytes_to_string(start_element.name().into_inner())?
            .to_owned();

        let mut data = HashMap::new();

        for attr in start_element.attributes() {
            let attr = attr?;
            let key = attr.key.into_inner();
            let key = converter.bytes_to_string(key)?;
            let value = converter.cow_to_string(attr.value)?;

            data.insert(key, value);
        }

        Ok(Self {
            data,
            start_element_name,
        })
    }

    /// Helper method. Checks to see if an element has any attributes and bails with
    /// an error if it does.
    pub(crate) fn check_is_empty<C: XmlReaderConversions>(
        start_element: &BytesStart<'_>,
        converter: &C,
    ) -> Result<(), GapixError> {
        if start_element.attributes().count() == 0 {
            return Ok(());
        }

        let attrs = Attributes::new(start_element, converter)?;
        attrs.check_is_empty_now()
    }

    /// Checks to see whether an attribute set is now empty.
    pub(crate) fn check_is_empty_now(&self) -> Result<(), GapixError> {
        if self.is_empty() {
            return Ok(());
        }

        let mut joined_attributes = String::new();

        let mut keys = self.data.keys();
        if let Some(item) = keys.next() {
            joined_attributes.push_str(item);
        }

        for item in keys {
            joined_attributes.push(',');
            joined_attributes.push_str(item);
        }

        Err(GapixError::UnexpectedAttributes {
            element: self.start_element_name.clone(),
            attributes: joined_attributes,
        })
    }

    /// Returns the number of attributes.
    pub(crate) fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns true if the attribute set is empty.
    pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the underlying hashmap of attr=value pairs and consumes Self.
    pub(crate) fn into_inner(self) -> HashMap<String, String> {
        self.data
    }

    /// Gets a mandatory attribute. The attribute is removed from the list
    /// of attributes and returned to the caller.
    pub(crate) fn get<S, T>(&mut self, key: S) -> Result<T, GapixError>
    where
        S: Into<String>,
        T: FromStr,
    {
        let key = key.into();

        let value = match self.data.entry(key.clone()) {
            Entry::Occupied(occupied_entry) => occupied_entry.remove(),
            _ => return Err(GapixError::MandatoryAttributeNotFound(key)),
        };

        value.parse::<T>().map_err(|_| GapixError::ParseFailure {
            from: value,
            dest_type: std::any::type_name::<T>().to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::read::start_parse;
    use quick_xml::Reader;

    #[test]
    fn get_works_for_extant_attributes() {
        let mut xml_reader = Reader::from_str(
            r#"<bounds minlat="-1.1" maxlat="1.1" minlon="-53.1111" maxlon="88.88">"#,
        );
        let start = start_parse(&mut xml_reader);
        let mut attrs = Attributes::new(&start, &xml_reader).unwrap();
        assert_eq!(attrs.len(), 4);
        let max_lat: f64 = attrs.get("maxlat").unwrap();
        assert_eq!(max_lat, 1.1);
        assert_eq!(attrs.len(), 3);
    }

    #[test]
    fn get_returns_error_for_non_existing_attributes() {
        let mut xml_reader = Reader::from_str(
            r#"<bounds minlat="-1.1" maxlat="1.1" minlon="-53.1111" maxlon="88.88">"#,
        );
        let start = start_parse(&mut xml_reader);
        let mut attrs = Attributes::new(&start, &xml_reader).unwrap();
        let result: Result<_, GapixError> = attrs.get::<&str, String>("blah");
        match result {
            Err(GapixError::MandatoryAttributeNotFound(a)) if a == "blah" => {}
            x => panic!("Unexpected result from parse(): {:?}", x),
        };
    }
}
