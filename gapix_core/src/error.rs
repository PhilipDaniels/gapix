use std::{num::TryFromIntError, path::PathBuf};

use quick_xml::events::attributes::AttrError;
use thiserror::Error;

use crate::read::XmlReaderConversions;

#[derive(Debug, Error)]
pub enum GapixError {
    #[error(transparent)]
    XmlError(#[from] quick_xml::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Xlsx(#[from] rust_xlsxwriter::XlsxError),

    #[error("Mandatory attribute {0} was not found on the element")]
    MandatoryAttributeNotFound(String),
    #[error("Mandatory element {0} was not found")]
    MandatoryElementNotFound(String),
    #[error("Could not parse {from} into type {dest_type}")]
    ParseFailure { from: String, dest_type: String },
    #[error("Unexpected Start element {0}")]
    UnexpectedStartElement(String),
    #[error("Unexpected End element {0}")]
    UnexpectedEndElement(String),
    #[error("Did not find the {0} element")]
    ElementNotFound(String),
    #[error("Element {element} has unexpected extra attributes {attributes}")]
    UnexpectedAttributes { element: String, attributes: String },
    #[error("Did not find an Event::Text element, buffer position = {0}, event={1}")]
    MissingText(u64, String),
    #[error("Date could not be parsed: {0}")]
    DateParseFailure(String),
    #[error("Multiple tracks were found when the operation requires a single track")]
    MultipleTracksFound,
    #[error("{0} is not a valid fix type. Valid values are 'none', '2d', '3d', 'dgps', 'pps'")]
    InvalidFixType(String),
    #[error("Date could not be formatted: {0}")]
    DateFormatFailure(String),
    #[error("Could not create file {path:?}")]
    CreateFile {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("Could not perform a numeric conversion: {0}")]
    NumericConversionError(String),
    #[error("Unexpected event received from Xml parser: {0}")]
    UnexpectedEvent(String),
    #[error("Unexpected EOF. Check file for corruption")]
    UnexpectedEof,
    #[error("Invalid DGPS station Id of {0}. Valid range is 0..=1023")]
    InvalidDGPSStationId(i64),
    #[error("Invalid latitude of {0}. Valid range is -90.0..=90.0")]
    InvalidLatitude(f64),
    #[error("Invalid longitude of {0}. Valid range is -180.0..=180.0")]
    InvalidLongitude(f64),
    #[error("Invalid degrees of {0}. Valid range is 0.0..=360")]
    InvalidDegrees(f64),
}

impl From<AttrError> for GapixError {
    fn from(value: AttrError) -> Self {
        Self::XmlError(value.into())
    }
}

impl From<TryFromIntError> for GapixError {
    fn from(value: TryFromIntError) -> Self {
        Self::NumericConversionError(value.to_string())
    }
}

impl GapixError {
    pub(crate) fn bad_start<C: XmlReaderConversions>(bytes: &[u8], converter: &C) -> Self {
        match converter.bytes_to_string(bytes) {
            Ok(s) => Self::UnexpectedStartElement(s),
            Err(err) => err,
        }
    }

    pub(crate) fn bad_end<C: XmlReaderConversions>(bytes: &[u8], converter: &C) -> Self {
        match converter.bytes_to_string(bytes) {
            Ok(s) => Self::UnexpectedEndElement(s),
            Err(err) => err,
        }
    }

    pub(crate) fn bad_event(event: Result<quick_xml::events::Event<'_>, quick_xml::Error>) -> Self {
        let s = format!("{:?}", event);
        Self::NumericConversionError(s)
    }
}
