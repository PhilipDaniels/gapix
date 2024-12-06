#![allow(clippy::single_match)]

use std::{fs::File, io::{BufReader, Cursor, Read}, path::Path};

use declaration::parse_declaration;
use fit::read_fit_from_reader_inner;
use gpx::parse_gpx;
use log::info;
use logging_timer::time;
use quick_xml::{events::Event, Reader};
use xml_reader_extensions::XmlReaderConversions;

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
pub(crate) mod xml_reader_extensions;

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

/// Reads a GPX from a file.
/// 
/// Note: This ultimately calls [`read_gpx_from_xml_reader`], and to do so it
/// first needs to read the entire file into RAM.
#[time]
pub fn read_gpx_from_file<P: AsRef<Path>>(input_file: P) -> Result<Gpx, GapixError> {
    let input_file = input_file.as_ref();
    info!("Reading GPX file {:?}", input_file);
    let contents = std::fs::read(input_file)?;
    let mut gpx = read_gpx_from_slice(&contents)?;
    gpx.filename = Some(input_file.to_owned());
    Ok(gpx)
}

/// Reads a GPX from a slice of bytes.
pub fn read_gpx_from_slice(data: &[u8]) -> Result<Gpx, GapixError> {
    let xml_reader = Reader::from_reader(data);
    read_gpx_from_xml_reader(xml_reader)
}

/// Reads a GPX from a Quick-Xml Reader. We rely on various methods that are
/// only implemented for readers over `[u8]`.
#[time]
pub fn read_gpx_from_xml_reader(mut xml_reader: Reader<&[u8]>) -> Result<Gpx, GapixError> {
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


/// Reads a FIT file.
/// 
/// Note: Unlike the equivalent for GPX files, this function does NOT load the
/// entire file into memory. It is therefore preferable in memory-constrained
/// environments.
#[time]
pub fn read_fit_from_file<P: AsRef<Path>>(input_file: P) -> Result<Gpx, GapixError> {
    let input_file = input_file.as_ref();
    info!("Reading FIT file {:?}", input_file);
    let reader = BufReader::new(File::open(input_file)?);
    let mut gpx = read_fit_from_reader(reader)?;
    gpx.filename = Some(input_file.to_owned());
    Ok(gpx)
}

/// Reads a FIT file from a slice of bytes.
#[time]
pub fn read_fit_from_slice(data: &[u8]) -> Result<Gpx, GapixError> {
    let reader = Cursor::new(data);
    read_fit_from_reader(reader)
}

/// Reads a FIT file from a reader.
/// #[time]
pub fn read_fit_from_reader<R: Read>(reader: R) -> Result<Gpx, GapixError> {
    read_fit_from_reader_inner(reader)
}
