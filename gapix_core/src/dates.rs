use chrono::{DateTime, SecondsFormat, Utc};
use chrono_tz::Tz;

use crate::{
    error::GapixError,
    geocoding::{get_timezone, RTreePoint},
};

/// Convert `utc_date` to a date in the specified `timezone`.
pub fn utc_to_timezone(utc_date: DateTime<Utc>, timezone: Tz) -> DateTime<Tz> {
    utc_date.with_timezone(&timezone)
}

/// Convert `utc_date` to a date in the timezone that `point` is in.
pub fn utc_to_appropriate_timezone(
    utc_date: DateTime<Utc>,
    point: RTreePoint,
) -> Result<DateTime<Tz>, GapixError> {
    match get_timezone(point) {
        Some(tz) => Ok(utc_to_timezone(utc_date, tz)),
        None => Err(GapixError::DateFormatFailure(format!(
            "Cannot determine time zone of point {:?}",
            point
        ))),
    }
}

/// Formats 'utc_date' into a string like "2024-09-01T05:10:44Z".
/// This is the format that GPX files contain.
pub fn format_utc_date(utc_date: &DateTime<Utc>) -> String {
    utc_date.to_rfc3339_opts(SecondsFormat::Secs, true)
}
