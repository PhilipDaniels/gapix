# TODO
- FIT parsing: do we need to worry about unit conversion?
- Move model into its own crate.
- Track splitting. Put file-level waypoints on the nearest split track.
- Waypoint processing for warnings etc.
- XLSX: Create images to represent the stage profiles.
- XLSX: Display is wrong when time goes over 24 hours.
- Stage should be an enum
 
# Design Questions
- Consider using a new type for DGBSStationType (0..=1023) on waypoint. The
  current design validates when reading a document, but does not validate that
  it is set to a valid value at runtime. The newtype pattern would require a lot
  of boilerplate though, and derive_more doesn't really help with a lot of it.
- Other possible newtypes with the same issues: lat/lon on waypoint and bounds,
- and degrees on waypoint.magvar.


# Links
- [GPX XSD](https://www.topografix.com/GPX/1/1/gpx.xsd)
- [Trackpoint extensions XSD](https://www8.garmin.com/xmlschemas/TrackPointExtensionv1.xsd)- []
- [FIT file SDK](https://developer.garmin.com/fit/overview/)



# 'Ridden' web site

## Stack
- https://tokio.rs/ + https://github.com/tokio-rs/axum
  Axum is written by the Tokio team
- https://www.sea-ql.org/SeaORM/
- https://maud.lambda.xyz/
- https://htmx.org/
- https://tailwindcss.com/

## Examples
- Example FIT app: https://github.com/karaul/fitplotter  
- Example Htmx + Maud TODO app: https://github.com/hadamove/todo-maud-htmx
  This looks very good, the templates are simple.
  It is server-side rendered.
  You can see how htmx calls a handler easily.
- Example Svelte app: https://github.com/svelterust/todo/tree/master
- https://rust-api.dev/docs/front-matter/authors/ LOOKS GOOD

## Alternatives (Rejected)
- https://v2.tauri.app/ : Basically a webview, you have to use front-end techniques to make an app
- https://leptos.dev/ : Similar to React. Uses signals and closures. Nasty front-end/back-end split necessary.
- https://dioxuslabs.com/ : Inspired by React. Uses signals and closures.

## The Database
- https://www.reddit.com/r/rust/comments/1e8ld5d/my_take_on_databases_with_rust_seaorm_vs_diesel/
- Configuration table instead of command line parameters
- Store entire GPX or FIT file in the db
- Use sha256 for duplicate file detection
- Segment type: Moving, Control, Climb, Descent, Segment, Ride
- Segment end: lock to point within a radius of 50m
- Entities: Controls, Segments, Files, FileTypes
- Should we store the individual points in a ride in a table? i.e. decompose the file?
  
## Questions/Decisions
- How to do styling with tailwind
- How to do logging or tracing? https://crates.io/crates/tracing See tracing-log in particular.
  And https://crates.io/crates/tracing-appender can take a Writer.
  The log needs to be in State.
- How do we have multiple separate states in Axum? In one big AppState, with FromRef substates
  See "Understand Axum" https://rust-api.dev/docs/part-1/tokio-hyper-axum/
- We need a database to do trend analysis, so an "online" version is out of scope for now
- If using SQLite, we need to deal with the fact the multiple requests may be posted
  simultaneously. Single thread them using an actor mechanism?
- How to deal with enums in SeaORM?


## Features
- Copy the database to a backup upon close
- CVE: Cardiovascular efficiency, being beats per kmh with climb compensation
- Fastest KM, 5KM, 10KM
- Replay and "race" multiple replays at once
- Trend analysis
- Download as spreadsheet
- Km ridden per month per year

## Plan
- Do something with htmx and an API call
- Get tracing and logging working into RAM
- Shutdown axum when the window is closed: https://github.com/tokio-rs/axum/discussions/1500
- Favicon

## Tables
- TAG: Name (PK), 200,300,400,500,600,1000,1200 etc. DIY, Calendar, Perm, Audax
  Auto-generate and auto-copy based on previous rides.
- RIDE_TAG: Many-to-many for TAG to RIDE.
- FILE: The raw data from a file upload. Contains a sha256 hash for uniqueness checking.
- CONFIGURATION: Key,Value.
  Saving configuration may recalculate all data.
- VALIDATION_TYPE: DIY-MAND-GPS, DIY, e-Brevet, Brevet
- RIDE: Id, FileId (FK), Name, Km, Points, Climb, StartTime etc.
  EntryCost, EventNo, EventLink, Organiser
  + Other derived fields currently in the XLSX.
- RIDE_POINT: RideId, lat,long etc. - waypoints.
- CONTROL: Id, Name, lat, long, Description, ToleranceCircle
- RIDE_CONTROL: Many-to-many for CONTROL to RIDE_POINT
- SEGMENT: Start, End, Name, SegmentType
  A portion of a ride.
  Master list of segments, either created by the user or previously detected.
  Rides are split into segments. 
- SEGMENT_TYPE: WholeRide, Stage, Climb, Descent, Custom
- RIDE_SEGMENT: Many-to-many for RIDE to SEGMENT.
  This is what appears on the "Stages" tab in my XLSX.
- RIDE_GROUP: Groups similar rides, e.g. all Lincs 200 Bardney rides.
  Has an "exemplar" ride (probably the first identified) and a similarity rating.
  
## Job System
- JOB: Id, JobType, Data.
  JobType says that type of processing is required. Data is the key of the data.
  Processing a job may create further jobs.
  Doing something such as uploading a file creates a job entry, which is then detected by
  the job processing system. Jobs are run in order. We want the ability to run in parallel too
  if at all possible. Probably some sort of actor system.
- JOBTYPE (enum):
  - ProcessAllFiles, ProcessFile. These take a raw FILE and produce a RIDE.
    Maybe have extracting the points as a separate step from the analysis?
    Analysis can be changed based on CONFIGURATION, but extracting the points is
    always the same.