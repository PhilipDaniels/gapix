use time::format_description::well_known;
use time::{OffsetDateTime, UtcOffset};

use crate::error::GapixError;

/// Convert 'utc_date' to a local date by applying the current local offset of the
/// user at the specified time.
/// TODO: It would be better to determine the offset to apply based on the
/// lat-lon of the trackpoint. We need a time-zone database to do that.
pub fn to_local_date(utc_date: OffsetDateTime) -> Result<OffsetDateTime, GapixError> {
    assert!(utc_date.offset().is_utc());

    match UtcOffset::local_offset_at(utc_date) {
        Ok(local_offset) => Ok(utc_date.to_offset(local_offset)),
        Err(err) => Err(GapixError::DateFormatFailure(err.to_string())),
    }
}

/// Formats 'utc_date' into a string like "2024-09-01T05:10:44Z".
/// This is the format that GPX files contain.
pub fn format_utc_date(utc_date: &OffsetDateTime) -> Result<String, GapixError> {
    assert!(utc_date.offset().is_utc());

    let mut buf = Vec::with_capacity(20);
    match utc_date.format_into(&mut buf, &well_known::Rfc3339) {
        Ok(_) => { /* Don't care about the number of bytes. */ }
        Err(err) => return Err(GapixError::DateFormatFailure(err.to_string())),
    }

    String::from_utf8(buf).map_err(|err| GapixError::DateFormatFailure(err.to_string()))
}
