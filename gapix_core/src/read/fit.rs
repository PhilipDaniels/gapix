use std::io::Read;

use chrono::{DateTime, Utc};
use fitparser::{profile::MesgNum, FitDataField, FitDataRecord, Value};
use log::{error, info, warn};

use crate::{
    error::GapixError,
    model::{GarminTrackpointExtensions, Gpx, Metadata, Track, TrackSegment, Waypoint, XmlDeclaration},
};

pub(crate) fn read_fit_from_reader_inner<R: Read>(mut reader: R) -> Result<Gpx, GapixError> {
    let declaration = XmlDeclaration::default();
    let metadata = Metadata::default();
    let mut gpx = Gpx::new(declaration, metadata);
    gpx.creator = env!("CARGO_PKG_NAME").to_string();
    gpx.set_default_garmin_attributes();
    gpx.metadata.description = Some("Parsed from a FIT file".to_string());
    gpx.metadata.time = None;

    let mut num_activity_messages = 0;
    let mut num_session_messages = 0;
    let mut num_record_messages = 0;
    gpx.tracks.clear();

    let fit_data = fitparser::from_reader(&mut reader)?;
    for d in fit_data {
        match d.kind() {
            // See https://developer.garmin.com/fit/file-types/activity/
            // for the types of messages we can expect in an Activity.

            // Interesting fields: type=activity, manufacturer=garmin, garmin_product=edge_1040
            MesgNum::FileId => {
                let file_type = get_field_string(d.fields(), "type")?;
                if file_type != "activity" {
                    error!("FIT file is not of type 'activity', instead it is '{file_type}'");
                    return Err(GapixError::MultipleTracksFound);
                }
                let ts = get_field_timestamp(d.fields(), "time_created")?;
                gpx.metadata.time = Some(ts);
            }

            MesgNum::Activity => {
                num_activity_messages += 1;
            }
           
            MesgNum::Session => {
                num_session_messages += 1;
                parse_session_message(&d, &mut gpx)?;
            }

            MesgNum::Record => {
                num_record_messages += 1;
                let _error_already_logged = parse_record_message(&d, &mut gpx);
            }
            _ => {}
        }
    }

    if num_activity_messages != 1 {
        warn!("FIT file contains {num_activity_messages} Activity Messages, expected 1");
    }

    info!("Parsed FIT file contained {num_session_messages} Session Messages and {num_record_messages} Record Messages");
    Ok(gpx)
}

/// Parses a Record. Note that, arbitrarily, certain fields can be missing. If
/// we don't get enough to form a valid waypoint, ignore it and carry on processing.
/// There tend to be a lot of these, so don't log anything.
fn parse_record_message(data: &FitDataRecord, gpx: &mut Gpx) -> Result<(), GapixError> {
    ensure_default_track(gpx);

    // Interesting fields: position_lat/long, heart_rate(EXT), distance, temperature (EXT), enhanced_speed,
    // enhanced_altitude, enhanced_respiration_rate, timestamp (UTC),
    //debug!("{:?}", data);
    
    let lat = match get_latlon(data.fields(), "position_lat") {
        Ok(v) => v,
        Err(err) => {
            return Err(err);
        }
    };

    let lon = match get_latlon(data.fields(), "position_long") {
        Ok(v) => v,
        Err(err) => {
            return Err(err);
        }
    };

    let mut tp = Waypoint::with_lat_lon(lat, lon)?;
    let ele = match get_field_f64(data.fields(), "enhanced_altitude") {
        Ok(alt) => alt,
        Err(_) => get_field_f64(data.fields(), "altitude")?
    };

    tp.ele = Some(ele);
    tp.time = Some(get_field_timestamp(data.fields(), "timestamp")?);

    let mut extensions = GarminTrackpointExtensions::default();
    match get_field_f64(data.fields(), "temperature") {
        Ok(temp) => extensions.air_temp = Some(temp),
        Err(_) => { /* ignore */},
    };
    match get_field_f64(data.fields(), "heart_rate") {
        Ok(hr) => extensions.heart_rate = Some(hr as u8),
        Err(_) => { /* ignore */},
    };
    if extensions.air_temp.is_some() || extensions.heart_rate.is_some() {
        tp.garmin_extensions = Some(extensions);
    }

    let idx = gpx.tracks.len() - 1;
    gpx.tracks[idx].segments[0].points.push(tp);

    Ok(())
}

/// We map sessions to tracks.
fn parse_session_message(data: &FitDataRecord, gpx: &mut Gpx) -> Result<(), GapixError> {
    let mut track = Track {
        name: Some(format!("Track {}", gpx.tracks.len() + 1)),
        ..Default::default()
    };

    let sport = get_field_string(data.fields(), "sport");
    let sub_sport = get_field_string(data.fields(), "sub_sport");
    track.r#type = match (sport, sub_sport) {
        (Ok(sport), Ok(sub_sport)) => Some(format!("{sport} - {sub_sport}")),
        (Ok(sport), Err(_)) => Some(sport.to_string()),
        (Err(_), Ok(sub_sport)) => Some(sub_sport.to_string()),
        (Err(_), Err(_)) => Some("unknown".to_string()),
    };

    track.segments.push(TrackSegment::default());
    gpx.tracks.push(track);
    Ok(())
}

/// My reading of the FIT spec says that a Session Message should precede 1 or
/// more Record Messages. However, sometimes it seems to come AFTER the Record
/// Messages. This causes a problem because we attach the Records as points onto
/// track segments whose creation is triggered by the parsing of a Session
/// Message. If the order is wrong, rather than complain make a default track in
/// the GPX to which we can attach the points. If the Session Message does show
/// up at the end this will mean that the parsed GPX structure will end up with
/// 1 extra, empty track. This is not really a problem, it will probably get
/// eliminated by into_single_track() anyway.
fn ensure_default_track(gpx: &mut Gpx) {
    if !gpx.tracks.is_empty() {
        return;
    }

    let mut track = Track {
        name: Some("Track 0".to_string()),
        ..Default::default()
    };
    track.segments.push(TrackSegment::default());
    track.r#type = Some("unknown".to_string());
    gpx.tracks.push(track);
}

fn get_field<'a>(fields: &'a [FitDataField], name: &str) -> Option<&'a FitDataField> {
    fields.iter().find(|f| f.name() == name)
}

fn get_field_value<'a>(fields: &'a [FitDataField], name: &str) -> Result<&'a Value, GapixError> {
    match get_field(fields, name) {
        Some(field) => Ok(field.value()),
        None => Err(GapixError::FieldNotFound(name.to_string()))
    }
}

fn get_field_string<'a>(fields: &'a [FitDataField], name: &str) -> Result<&'a String, GapixError> {
    let value = get_field_value(fields, name)?;
    if let Value::String(val) = value {
        Ok(val)
    } else {
        Err(GapixError::NumericConversionError(format!("get_field_string: Field value is of wrong type {}", value)))
    }
}

fn get_field_timestamp(
    fields: &[FitDataField],
    name: &str,
) -> Result<DateTime<Utc>, GapixError> {
    let value = get_field_value(fields, name)?;
    if let Value::Timestamp(val) = value {
        Ok(val.to_utc())
    } else {
        Err(GapixError::NumericConversionError(format!("get_field_timestamp: Field value is of wrong type {}", value)))
    }
}

/// Gets a field value as an f64, converting it if possible.
fn get_field_f64(fields: &[FitDataField], name: &str) -> Result<f64, GapixError> {
    let value = get_field_value(fields, name)?;
    match value {
        Value::Byte(v) => Ok(f64::from(*v)),
        Value::SInt8(v) => Ok(f64::from(*v)),
        Value::UInt8(v) => Ok(f64::from(*v)),
        Value::SInt16(v) => Ok(f64::from(*v)),
        Value::UInt16(v) => Ok(f64::from(*v)),
        Value::SInt32(v) => Ok(f64::from(*v)),
        Value::UInt32(v) => Ok(f64::from(*v)),
        Value::Float32(v) => Ok(f64::from(*v)),
        Value::Float64(v) => Ok(*v),
        Value::UInt8z(v) => Ok(f64::from(*v)),
        Value::UInt16z(v) => Ok(f64::from(*v)),
        Value::UInt32z(v) => Ok(f64::from(*v)),
        Value::SInt64(v) => {
            let v_try = *v as f64;
            if *v == v_try as i64 {
                Ok(v_try)
            } else {
                Err(GapixError::NumericConversionError(format!("Cannot accurately convert {} to f64", *v)))
            }
        }
        Value::UInt64(v) => {
            let v_try = *v as f64;
            if *v == v_try as u64 {
                Ok(v_try)
            } else {
                Err(GapixError::NumericConversionError(format!("Cannot accurately convert {} to f64", *v)))
            }
        }
        Value::UInt64z(v) => {
            let v_try = *v as f64;
            if *v == v_try as u64 {
                Ok(v_try)
            } else {
                Err(GapixError::NumericConversionError(format!("Cannot accurately convert {} to f64", *v)))
            }
        }
        _ => Err(GapixError::NumericConversionError(format!("get_field_f64: Field value is of wrong type {}", value)))
    }
}

fn get_latlon(fields: &[FitDataField], name: &str) -> Result<f64, GapixError> {
    let semicircles = get_field_f64(fields, name)?;
    // For the magic value, see
    // https://forums.garmin.com/developer/fit-sdk/f/discussion/301824/newbie-how-to-dump-raw-fit-data-to-text-not-fittocsv-bat
    Ok(semicircles / 11930465.0)
}
