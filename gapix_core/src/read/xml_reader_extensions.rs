use std::{borrow::Cow, str::FromStr};

use chrono::{DateTime, Utc};
use quick_xml::{events::Event, Reader};

use crate::error::GapixError;

/// An extension trait for quick_xml::Reader that converts the underlying bytes
/// into usable str and String values.
pub(crate) trait XmlReaderConversions {
    fn bytes_to_cow<'a>(&self, bytes: &'a [u8]) -> Result<Cow<'a, str>, GapixError>;
    fn bytes_to_string(&self, bytes: &[u8]) -> Result<String, GapixError>;
    fn cow_to_string(&self, bytes: Cow<'_, [u8]>) -> Result<String, GapixError>;
}

impl<R> XmlReaderConversions for Reader<R> {
    #[inline]
    fn bytes_to_cow<'a>(&self, bytes: &'a [u8]) -> Result<Cow<'a, str>, GapixError> {
        // It is important to pass the bytes through decode() in order to do a
        // proper conversion.
        Ok(self.decoder().decode(bytes)?)
    }

    #[inline]
    fn bytes_to_string(&self, bytes: &[u8]) -> Result<String, GapixError> {
        // Ensure everything goes through decode().
        Ok(self.bytes_to_cow(bytes)?.into())
    }

    #[inline]
    fn cow_to_string(&self, bytes: Cow<'_, [u8]>) -> Result<String, GapixError> {
        match bytes {
            // Ensure everything goes through decode().
            Cow::Borrowed(slice) => Ok(self.bytes_to_string(slice)?),
            Cow::Owned(vec) => Ok(self.bytes_to_string(&vec)?),
        }
    }
}

/// An extension trait for quick_xml::Reader that makes it convenient to read
/// inner text and convert it to a specific type.
pub(crate) trait XmlReaderExtensions {
    fn read_inner_as_string(&mut self) -> Result<String, GapixError>;
    fn read_inner_as_time(&mut self) -> Result<DateTime<Utc>, GapixError>;
    fn read_inner_as<T: FromStr>(&mut self) -> Result<T, GapixError>;
}

impl XmlReaderExtensions for Reader<&[u8]> {
    #[inline]
    fn read_inner_as_string(&mut self) -> Result<String, GapixError> {
        match self.read_event() {
            Ok(Event::Text(text)) => Ok(self.bytes_to_string(&text)?),
            event => {
                let s = format!("{:?}", event);
                Err(GapixError::MissingText(self.buffer_position(), s))
            }
        }
    }

    #[inline]
    fn read_inner_as_time(&mut self) -> Result<DateTime<Utc>, GapixError> {
        let t = self.read_inner_as_string()?;
        // Do not allow errors from the time library to surface in our API, as
        // we may eventually allow a choice of time libraries between time and
        // chrono.
        match DateTime::parse_from_rfc3339(&t) {
            Ok(dt) => Ok(dt.to_utc()),
            Err(e) => Err(GapixError::DateParseFailure(e.to_string()))
        }
    }

    #[inline]
    fn read_inner_as<T: FromStr>(&mut self) -> Result<T, GapixError> {
        let value = self.read_inner_as_string()?;

        value.parse::<T>().map_err(|_| GapixError::ParseFailure {
            from: value,
            dest_type: std::any::type_name::<T>().to_string(),
        })
    }
}

/// A helper method to simplify tests. Often we need to get the contents of an
/// 'Event::Start' event type.
#[cfg(test)]
pub(crate) fn start_parse<'a>(xml_reader: &mut Reader<&'a [u8]>) -> quick_xml::events::BytesStart<'a> {
    match xml_reader.read_event().unwrap() {
        Event::Start(start) => start,
        _ => panic!("Failed to parse Event::Start(_) element"),
    }
}
