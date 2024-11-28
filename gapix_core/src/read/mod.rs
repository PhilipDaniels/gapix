#![allow(clippy::single_match)]

use core::str;
use std::{borrow::Cow, path::Path, str::FromStr};

use chrono::{DateTime, Utc};
use declaration::parse_declaration;
use fit::read_fit_from_slice;
use gpx::parse_gpx;
use log::info;
use logging_timer::time;
use quick_xml::{events::Event, Reader};

use crate::{
    error::GapixError,
    model::{Gpx, XmlDeclaration},
};

mod attributes;
mod bounds;
mod copyright;
mod declaration;
mod email;
mod extensions;
mod fit;
mod gpx;
mod link;
mod metadata;
mod person;
mod route;
mod track;
mod track_segment;
mod trackpoint_extensions;
mod waypoint;

/// Reads an input file (either FIT or GPX). The file type is determined by
/// checking the extension: if its "fit" we read it as a FIT file, otherwise we
/// assume it's a GPX and try and read it as such.
pub fn read_input_file<P: AsRef<Path>>(input_file: P) -> Result<Gpx, GapixError> {
    let input_file = input_file.as_ref();
    match input_file.extension() {
        Some(ext) => if ext.to_ascii_lowercase() == "fit" {
            read_fit_from_file(input_file)
        } else {
            // Assume gpx.
            read_gpx_from_file(input_file)
        }
        None => read_gpx_from_file(input_file)
    }
}

#[time]
pub fn read_fit_from_file<P: AsRef<Path>>(input_file: P) -> Result<Gpx, GapixError> {
    let input_file = input_file.as_ref();
    info!("Reading FIT file {:?}", input_file);
    let contents = std::fs::read(input_file)?;
    let mut gpx = read_fit_from_slice(&contents)?;
    gpx.filename = Some(input_file.to_owned());
    Ok(gpx)
}

/// The XSD, which defines the format of a GPX file, is at https://www.topografix.com/GPX/1/1/gpx.xsd
#[time]
pub fn read_gpx_from_file<P: AsRef<Path>>(input_file: P) -> Result<Gpx, GapixError> {
    let input_file = input_file.as_ref();
    info!("Reading GPX file {:?}", input_file);
    let contents = std::fs::read(input_file)?;
    let mut gpx = read_gpx_from_slice(&contents)?;
    gpx.filename = Some(input_file.to_owned());
    Ok(gpx)
}

pub fn read_gpx_from_slice(data: &[u8]) -> Result<Gpx, GapixError> {
    let xml_reader = Reader::from_reader(data);
    read_gpx_from_reader(xml_reader)
}

#[time]
pub fn read_gpx_from_reader(mut xml_reader: Reader<&[u8]>) -> Result<Gpx, GapixError> {
    let mut xml_declaration: Option<XmlDeclaration> = None;
    let mut gpx: Option<Gpx> = None;

    loop {
        match xml_reader.read_event() {
            Ok(Event::Decl(decl)) => {
                xml_declaration = Some(parse_declaration(&decl, &xml_reader)?);
            }
            Ok(Event::Start(start)) => match start.name().as_ref() {
                b"gpx" => {
                    gpx = Some(parse_gpx(&start, &mut xml_reader)?);
                }
                e => {
                    let name = xml_reader.bytes_to_string(e)?;
                    return Err(GapixError::UnexpectedStartElement(name));
                }
            },
            Ok(Event::Eof) => {
                // We should already have consumed the closing '<gpx>' tag in parse_gpx().
                // So the next thing will be EOF.
                if gpx.is_none() {
                    return Err(GapixError::ElementNotFound("gpx".to_string()));
                }
                let mut gpx = gpx.unwrap();
                if xml_declaration.is_none() {
                    return Err(GapixError::ElementNotFound("xml".to_string()));
                }

                gpx.declaration = xml_declaration.unwrap();
                return Ok(gpx);
            }
            Err(e) => return Err(e.into()),
            _ => (),
        }
    }
}

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
fn start_parse<'a>(xml_reader: &mut Reader<&'a [u8]>) -> quick_xml::events::BytesStart<'a> {
    match xml_reader.read_event().unwrap() {
        Event::Start(start) => start,
        _ => panic!("Failed to parse Event::Start(_) element"),
    }
}
