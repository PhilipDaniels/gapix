# TODO
- FIT parsing: do we need to worry about unit conversion?
- Move model into its own crate.
- Track splitting. Put file-level waypoints on the nearest split track.
- Waypoint processing for warnings etc.
- XLSX: Create images to represent the stage profiles.
- XLSX: Display is wrong when time goes over 24 hours.
- Fastest KM, 5KM, 10KM
- Segment detection and storage: SQLite?
- Fix the location of the round-trip test file - use a Vec<u8>
- Geocoding point types

# Design Questions
- Consider using a new type for DGBSStationType (0..=1023) on waypoint. The
  current design validates when reading a document, but does not validate that
  it is set to a valid value at runtime. The newtype pattern would require a lot
  of boilerplate though, and derive_more doesn't really help with a lot of it.
- Other possible newtypes with the same issues: lat/lon on waypoint and bounds,
- and degrees on waypoint.magvar.

# Performance Database


# Links
- [GPX XSD](https://www.topografix.com/GPX/1/1/gpx.xsd)
- [Trackpoint extensions XSD](https://www8.garmin.com/xmlschemas/TrackPointExtensionv1.xsd)- []
- [FIT file SDK](https://developer.garmin.com/fit/overview/)
