use chrono::{DateTime, SecondsFormat, Utc};

use crate::{error::GapixError, geocoding::{get_timezone, RTreePoint}};

use chrono_tz::Tz;

/// Convert 'utc_date' to a local date by applying the current local offset of the
/// user at the specified time.
pub fn to_local_date(utc_date: DateTime<Utc>) -> Result<DateTime<Tz>, GapixError> {
    let tz: Tz = "Europe/London".parse().unwrap();
    let x = utc_date.with_timezone(&tz);
    Ok(x)
}

pub fn to_local_date2(utc_date: DateTime<Utc>, timezone: Tz) -> Result<DateTime<Tz>, GapixError> {
    let x = utc_date.with_timezone(&timezone);
    Ok(x)
}

pub fn to_local_date3(utc_date: DateTime<Utc>, point: RTreePoint) -> Result<DateTime<Tz>, GapixError> {
    match get_timezone(point) {
        Some(tz) => {
            to_local_date2(utc_date, tz)
        }
        None => Err(GapixError::DateFormatFailure(format!("Cannot determine time zone of point {:?}", point)))
    }

}

/// Formats 'utc_date' into a string like "2024-09-01T05:10:44Z".
/// This is the format that GPX files contain.
pub fn format_utc_date(utc_date: &DateTime<Utc>) -> String {
    utc_date.to_rfc3339_opts(SecondsFormat::Secs, true)
}
