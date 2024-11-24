use anyhow::{Context, Ok, Result};
use args::{get_required_outputs, parse_args, Args, RequiredOutputFiles};
use clap::builder::styling::AnsiColor;
use directories::ProjectDirs;
use env_logger::Builder;
use gapix_core::{
    excel::{create_summary_xlsx, write_summary_to_file, Hyperlink},
    geocoding::{initialise_geocoding, GeocodingOptions},
    gpx_writer::{write_gpx_to_file, OutputOptions},
    model::Gpx,
    read::{read_fit_from_file, read_gpx_from_file},
    simplification::{metres_to_epsilon, reduce_trackpoints_by_rdp},
    stage::{detect_stages, StageDetectionParameters},
};
use join::join_input_files;
use log::{debug, error, info, logger, warn};
use logging_timer::time;
use rayon::prelude::*;
use std::{io::Write, path::PathBuf};

mod args;
mod join;

pub const PROGRAM_NAME: &str = env!("CARGO_PKG_NAME");
pub const AUTHOR: &str = env!("CARGO_PKG_AUTHORS");

#[time]
fn main() -> Result<()> {
    configure_logging();
    main2()?;
    logger().flush();
    Ok(())
}

fn get_geocoding_options(args: &Args) -> GeocodingOptions {
    let project_dirs = ProjectDirs::from("", "", env!("CARGO_PKG_NAME"));
    GeocodingOptions::new(
        project_dirs.map(|d| d.config_local_dir().to_path_buf()),
        args.countries.clone(),
        args.force_geonames_download
    )

}

fn read_fits(files: Vec<PathBuf>) {
    for f in files {
        let gpx = read_fit_from_file(f).unwrap();
    }
}

#[time]
fn main2() -> Result<()> {
    info!("Starting {PROGRAM_NAME}");

    let args = parse_args();
    debug!("{:?}", &args);
    if args.force {
        info!("'--force' specified, all existing output files will be overwritten");
    }

    // If we are running in "join mode" then we need to load all the
    // input files into RAM and merge them into a single file.
    let input_files = args.files();
    if input_files.is_empty() {
        warn!("No .gpx or .fit files specified, exiting");
        return Ok(());
    }

    read_fits(input_files);
    std::process::exit(0);

    let geo_opt = get_geocoding_options(&args);
    initialise_geocoding(geo_opt);

    // In join mode we join all the input files into a single file
    // and then process it. There is nothing to be done after that.
    if args.join {
        let rof = get_required_outputs(&args, &input_files[0]);
        debug!("In join mode: {:?}", &rof);

        if let Some(joined_filename) = &rof.joined_file {
            let mut gpx = join_input_files(&input_files)?;
            gpx.filename = Some(joined_filename.clone());
            write_gpx_to_file(joined_filename, &gpx, OutputOptions::Full)?;
            analyse_gpx(&gpx, &args, &rof)?;
            simplify_gpx(gpx, &args, rof)?;
        }

        return Ok(());
    }

    // The other modes break down to 'process each file separately'.
    debug!("In per-file mode");

    input_files.par_iter().for_each(|f| {
        let rof = get_required_outputs(&args, f);
        debug!("Required Output Files: {:?}", &rof);

        if let Err(err) = read_gpx_from_file(f).map(|gpx| {
            let gpx = gpx.into_single_track();
            analyse_gpx(&gpx, &args, &rof).map(|_| simplify_gpx(gpx, &args, rof))
        }) {
            error!("Error while processing file {:?}: {}", f, err)
        };
    });

    Ok(())
}

fn analyse_gpx(gpx: &Gpx, args: &Args, rof: &RequiredOutputFiles) -> Result<()> {
    assert!(gpx.is_single_track());

    if let Some(analysis_file) = &rof.analysis_file {
        assert!(args.analyse);

        // Analysis requires us to enrich the GPX data with some
        // derived data such as speed and running distance.
        let mut enriched_gpx = gpx.to_enriched_gpx()?;
        let params = StageDetectionParameters {
            stopped_speed_kmh: args.control_speed,
            min_metres_to_resume: args.control_resumption_distance,
            min_duration_seconds: args.min_control_time * 60.0,
        };

        let stages = detect_stages(&mut enriched_gpx, params);

        let tp_hyper = if args.trackpoint_hyperlinks {
            Hyperlink::Yes
        } else {
            Hyperlink::No
        };

        let workbook = create_summary_xlsx(tp_hyper, &enriched_gpx, &stages)?;
        write_summary_to_file(analysis_file, workbook)?;
    }

    Ok(())
}

fn simplify_gpx(mut gpx: Gpx, args: &Args, rof: RequiredOutputFiles) -> Result<()> {
    assert!(gpx.is_single_track());

    if let Some(simplified_file) = &rof.simplified_file {
        let metres = args
            .metres
            .context("The 'metres' argument should be specified if we are simplifying")?;
        let epsilon = metres_to_epsilon(metres);
        let start_count = gpx.num_points();
        reduce_trackpoints_by_rdp(&mut gpx.tracks[0].segments[0].points, epsilon);
        let end_count = gpx.num_points();

        info!(
            "Using Ramer-Douglas-Peucker with a precision of {metres}m (epsilon={epsilon}) reduced the trackpoint count from {start_count} to {end_count} for {:?}",
            gpx.filename
            );

        write_gpx_to_file(simplified_file, &gpx, OutputOptions::AudaxUKDIY)?;
    }

    Ok(())
}

fn configure_logging() {
    let mut builder = Builder::from_default_env();

    builder.format(|buf, record| {
        let level_style = buf.default_level_style(record.level());
        let level_style = match record.level() {
            log::Level::Error => level_style.fg_color(Some(AnsiColor::Red.into())),
            log::Level::Warn => level_style.fg_color(Some(AnsiColor::Yellow.into())),
            log::Level::Info => level_style.fg_color(Some(AnsiColor::Green.into())),
            log::Level::Debug => level_style.fg_color(Some(AnsiColor::Blue.into())),
            log::Level::Trace => level_style.fg_color(Some(AnsiColor::Magenta.into())),
        };

        let line_number_style = buf.default_level_style(record.level())
            .fg_color(Some(AnsiColor::Cyan.into()));

        match (record.file(), record.line()) {
            (Some(file), Some(line)) => writeln!(
                buf,
                "[{} {level_style}{}{level_style:#} [{}] {}/{line_number_style}{}{line_number_style:#}] {}",
                buf.timestamp_micros(),
                record.level(),
                record.target(),
                file,
                line,
                record.args()
            ),
            (Some(file), None) => writeln!(
                buf,
                "[{} {level_style}{}{level_style:#} {}] {}",
                buf.timestamp_micros(),
                record.level(),
                file,
                record.args()
            ),
            (None, Some(_line)) => writeln!(
                buf,
                "[{} {level_style}{}{level_style:#}] {}",
                buf.timestamp_micros(),
                record.level(),
                record.args()
            ),
            (None, None) => writeln!(
                buf,
                "[{} {level_style}{}{level_style:#}] {}",
                buf.timestamp_micros(),
                record.level(),
                record.args()
            ),
        }
    });

    builder.init();
}
