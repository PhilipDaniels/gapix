//! Contains the functionality relating to Stages.
//! Detecting these is quite a bit of work. Once we get
//! the Stages determined we can calculate a lot of
//! other metrics fairly easily.

use core::{fmt, slice};
use std::ops::Index;

use geo::{GeodesicDistance, Point};
use log::{debug, info, warn};
use logging_timer::time;
use time::{Duration, OffsetDateTime};

use crate::model::{EnrichedGpx, EnrichedTrackPoint};

/// Calculates speed in kmh from metres and seconds.
pub fn speed_kmh(metres: f64, seconds: f64) -> f64 {
    (metres / seconds) * 3.6
}

/// Calculates speed in kmh from metres and a Duration.
pub fn speed_kmh_from_duration(metres: f64, time: Duration) -> f64 {
    speed_kmh(metres, time.as_seconds_f64())
}

/// These are the parameters that control the 'Stage-finding'
/// algorithm.
#[derive(Debug)]
pub struct StageDetectionParameters {
    /// You are considered "Stopped" if your speed drops below this.
    pub stopped_speed_kmh: f64,

    // You are considered to be "Moving Again" when you have moved
    // at least this many metres from the time you first stopped.
    pub min_metres_to_resume: f64,

    /// We want to eliminate tiny Stages caused by noisy data, for
    /// example these can occur when just starting off again.
    /// So set the minimum length of a stage, in seconds.
    pub min_duration_seconds: f64,
}

/// Represents a stage from a GPX track. The stage can represent
/// you moving, or stopped.
#[derive(Debug)]
pub struct Stage<'gpx> {
    pub stage_type: StageType,
    pub start: &'gpx EnrichedTrackPoint,
    pub end: &'gpx EnrichedTrackPoint,
    pub min_elevation: &'gpx EnrichedTrackPoint,
    pub max_elevation: &'gpx EnrichedTrackPoint,
    pub max_speed: &'gpx EnrichedTrackPoint,
    // The first point in the entire track. We could pass it
    // into the relevant methods, but storing it works ok too.
    // We will need this to calculate some metrics later.
    pub track_start_point: &'gpx EnrichedTrackPoint,
}

/// The type of a Stage.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum StageType {
    Moving,
    Stopped,
}

impl fmt::Display for StageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StageType::Moving => write!(f, "Moving"),
            StageType::Stopped => write!(f, "Stopped"),
        }
    }
}

impl StageType {
    fn toggle(self) -> Self {
        match self {
            StageType::Moving => StageType::Stopped,
            StageType::Stopped => StageType::Moving,
        }
    }
}

impl<'gpx> Stage<'gpx> {
    /// Returns the duration of the stage.
    pub fn duration(&self) -> Duration {
        // Be careful to use the time that the 'start' TrackPoint
        // began, not when it was recorded. They are very different
        // for TrackPoints written when you are stopped. A TrackPoint
        // may not be written for many minutes in that situation.
        self.end.time - self.start.start_time()
    }

    /// Returns the running duration to the end of the stage from
    /// the 'track_start_point' (the first point in the track).
    pub fn running_duration(&self) -> Duration {
        self.end.time - self.track_start_point.start_time()
    }

    /// Returns the distance (length) of the stage, in metres.
    pub fn distance_metres(&self) -> f64 {
        self.end.running_metres - self.start.running_metres
    }

    /// Returns the distance of the stage, in km.
    pub fn distance_km(&self) -> f64 {
        self.distance_metres() / 1000.0
    }

    /// Returns the cumulative distance to the end of the stage
    /// from the start of the entire track.
    pub fn running_distance_km(&self) -> f64 {
        self.end.running_metres / 1000.0
    }

    /// Returns the average speed of the stage, in kmh.
    pub fn average_speed_kmh(&self) -> f64 {
        speed_kmh_from_duration(self.distance_metres(), self.duration())
    }

    /// Returns the average speed, calculated over the distance from
    /// the start of the track to the end of the stage.
    pub fn running_average_speed_kmh(&self) -> f64 {
        speed_kmh_from_duration(self.end.running_metres, self.running_duration())
    }

    /// Returns the total ascent in metres over the stage.
    pub fn ascent_metres(&self) -> f64 {
        self.end.running_ascent_metres - self.start.running_ascent_metres
    }

    /// Returns the total ascent to the end of the stage from
    /// the beginning of the track.
    pub fn running_ascent_metres(&self) -> f64 {
        self.end.running_ascent_metres
    }

    /// Returns the total descent in metres over the stage.
    pub fn descent_metres(&self) -> f64 {
        self.end.running_descent_metres - self.start.running_descent_metres
    }

    /// Returns the total descent to the end of the stage from
    /// the beginning of the track.
    pub fn running_descent_metres(&self) -> f64 {
        self.end.running_descent_metres
    }
}

#[derive(Default)]
pub struct StageList<'gpx>(Vec<Stage<'gpx>>);

impl<'gpx> Index<usize> for StageList<'gpx> {
    type Output = Stage<'gpx>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl<'gpx> StageList<'gpx> {
    // TODO: Implement Iterator properly.
    pub fn iter(&self) -> slice::Iter<Stage> {
        self.0.iter()
    }

    /// Returns the first point in the first stage.
    pub fn first_point(&self) -> &EnrichedTrackPoint {
        self.0[0].start
    }

    /// Returns the last point in the last stage.
    pub fn last_point(&self) -> &EnrichedTrackPoint {
        self.0[self.len() - 1].end
    }

    pub fn push(&mut self, stage: Stage<'gpx>) {
        self.0.push(stage);
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns the start time of the first Stage.
    pub fn start_time(&self) -> OffsetDateTime {
        self.first_point().start_time()
    }

    /// Returns the end time of the last Stage.
    pub fn end_time(&self) -> OffsetDateTime {
        self.last_point().time
    }

    /// Returns the total duration between the start of the first
    /// stage and the end of the last stage.
    pub fn duration(&self) -> Duration {
        self.end_time() - self.start_time()
    }

    /// Returns the total time Moving across all the stages.
    pub fn total_moving_time(&self) -> Duration {
        self.duration() - self.total_stopped_time()
    }

    /// Returns the total time Stopped across all the stages.
    pub fn total_stopped_time(&self) -> Duration {
        self.0
            .iter()
            .filter_map(|stage| match stage.stage_type {
                StageType::Moving => None,
                StageType::Stopped => Some(stage.duration()),
            })
            .sum()
    }

    /// Returns the total distance of all the stages in metres.
    pub fn distance_metres(&self) -> f64 {
        self.0.iter().map(|s| s.distance_metres()).sum()
    }

    /// Returns the total distance of all the stages in km.
    pub fn distance_km(&self) -> f64 {
        self.distance_metres() / 1000.0
    }

    /// Returns the average moving speed over the whole track,
    /// this excludes stopped time.
    pub fn average_moving_speed(&self) -> f64 {
        speed_kmh_from_duration(self.distance_metres(), self.total_moving_time())
    }

    /// Returns the overall average moving speed over the whole track,
    /// this includes stopped time.
    pub fn average_overall_speed(&self) -> f64 {
        speed_kmh_from_duration(self.distance_metres(), self.duration())
    }

    /// Returns the point of minimum elevation across all the stages.
    pub fn min_elevation(&self) -> &EnrichedTrackPoint {
        self.0
            .iter()
            .map(|stage| &stage.min_elevation)
            .min_by(|a, b| a.ele.total_cmp(&b.ele))
            .unwrap()
    }

    /// Returns the point of maximum elevation across all the stages.
    pub fn max_elevation(&self) -> &EnrichedTrackPoint {
        self.0
            .iter()
            .map(|stage| &stage.max_elevation)
            .max_by(|a, b| a.ele.total_cmp(&b.ele))
            .unwrap()
    }

    /// Returns the total ascent in metres across all the stages.
    pub fn total_ascent_metres(&self) -> f64 {
        self.0.iter().map(|stage| stage.ascent_metres()).sum()
    }

    /// Returns the total descent in metres across all the stages.
    pub fn total_descent_metres(&self) -> f64 {
        self.0.iter().map(|stage| stage.descent_metres()).sum()
    }

    /// Returns the point of maximum speed across all the stages.
    pub fn max_speed(&self) -> &EnrichedTrackPoint {
        self.0
            .iter()
            .map(|stage| &stage.max_speed)
            .max_by(|a, b| a.speed_kmh.total_cmp(&b.speed_kmh))
            .unwrap()
    }
}

/// Calculate a set of enriched TrackPoint information (distances, speed, climb).
#[time]
pub fn enrich_trackpoints(gpx: &mut EnrichedGpx) {
    let start_time = gpx.points[0].time;
    let mut cum_ascent_metres = 0.0;
    let mut cum_descent_metres = 0.0;

    let mut p1 = gpx.points[0].as_geo_point();

    for idx in 1..gpx.points.len() {
        let p2 = gpx.points[idx].as_geo_point();
        gpx.points[idx].delta_metres = distance_between_points_metres(p1, p2);
        assert!(gpx.points[idx].delta_metres >= 0.0);

        gpx.points[idx].running_metres =
            gpx.points[idx - 1].running_metres + gpx.points[idx].delta_metres;
        assert!(gpx.points[idx].running_metres >= 0.0);

        // Time delta. Don't really need this stored, but is handy to spot
        // points that took more than usual when scanning the CSV.
        gpx.points[idx].delta_time = gpx.points[idx].time - gpx.points[idx - 1].time;
        assert!(gpx.points[idx].delta_time.is_positive());

        // Speed. Based on the distance we just calculated.
        gpx.points[idx].speed_kmh =
            speed_kmh_from_duration(gpx.points[idx].delta_metres, gpx.points[idx].delta_time);
        assert!(gpx.points[idx].speed_kmh >= 0.0);

        // How long it took to get here.
        gpx.points[idx].running_delta_time = gpx.points[idx].time - start_time;
        assert!(gpx.points[idx].running_delta_time.is_positive());

        // Ascent and descent.
        let ele_delta_metres = gpx.points[idx].ele - gpx.points[idx - 1].ele;
        gpx.points[idx].ele_delta_metres = ele_delta_metres;

        if ele_delta_metres > 0.0 {
            cum_ascent_metres += ele_delta_metres;
        } else {
            cum_descent_metres += ele_delta_metres.abs();
        }

        gpx.points[idx].running_ascent_metres = cum_ascent_metres;
        assert!(gpx.points[idx].running_ascent_metres >= 0.0);
        gpx.points[idx].running_descent_metres = cum_descent_metres;
        assert!(gpx.points[idx].running_descent_metres >= 0.0);

        p1 = p2;
    }
}

/// Detects the stages in the GPX and returns them as a list.
///
/// Invariants: the first stage starts at TrackPoint 0
/// and goes to TrackPoint N. The next stage starts at
/// Trackpoint N + 1 and goes to TrackPoint M. The last stage
/// ends at the last TrackPoint. In other words, there are no gaps,
/// all TrackPoints are in a stage, and no TrackPoint is in
/// two stages. TrackPoints are cloned as part of this construction.
///
/// A Stage is a Stopped stage if you speed drops below
/// a (very low) limit and you don't move a certain distance
/// for a 'min_stop_time' length of time.
///
/// All non-Stopped stages are considered Moving stages.
#[time]
pub fn detect_stages(gpx: &EnrichedGpx, params: StageDetectionParameters) -> StageList {
    if gpx.points.len() < 2 {
        warn!("GPX {:?} does not have any points", gpx.filename);
        return Default::default();
    }

    info!(
        "Detecting stages in {:?} using stopped_speed_kmh={}, min_duration_seconds={}, min_metres_to_resume={}",
        gpx.filename,
        params.stopped_speed_kmh,
        params.min_duration_seconds,
        params.min_metres_to_resume
    );

    let mut stages = StageList::default();

    // Note 1: The first TrackPoint always has a speed of 0, but it is unlikely
    // that you are actually in a Stopped stage. However, it's not impossible,
    // see Note 2 for why.

    // Note 2: We need to deal with the slightly bizarre situation where you turn
    // the GPS on and then don't go anywhere for a while - so your first stage
    // may be a Stopped stage!

    let mut start_idx = 0;

    // We will alternate stage types - Moving-Stopped-Moving-Stopped etc.
    // But instead of assuming that we start off Moving, we try and figure it out.
    let mut stage_type = get_starting_stage_type(gpx, &params);
    info!("Determined type of the first stage to be {}", stage_type);

    while let Some(stage) = get_next_stage(stage_type, start_idx, gpx, &params) {
        // Stages do not share points, the next stage starts on the next point.
        start_idx = stage.end.index + 1;

        info!(
            "Adding {} stage from point {} to {}, length={:.3}km, duration={}",
            stage.stage_type,
            stage.start.index,
            stage.end.index,
            stage.distance_km(),
            stage.duration(),
        );
        stages.push(stage);

        stage_type = stage_type.toggle();
    }

    info!("Detection finished, found {} stages", stages.len());

    // Should include all TrackPoints and start/end indexes overlap.
    assert_eq!(
        stages[0].start.index, 0,
        "Should always start with the first point"
    );

    assert_eq!(
        stages[stages.len() - 1].end.index,
        gpx.points.len() - 1,
        "Should always end with the last point"
    );

    for idx in 0..stages.len() - 1 {
        assert_eq!(
            stages[idx].end.index + 1,
            stages[idx + 1].start.index,
            "The next stage should always start on the next index"
        );
    }

    stages
}

fn get_next_stage<'gpx>(
    stage_type: StageType,
    start_idx: usize,
    gpx: &'gpx EnrichedGpx,
    params: &StageDetectionParameters,
) -> Option<Stage<'gpx>> {
    // Get this out into a variable to avoid off-by-one errors (hopefully).
    let last_valid_idx = gpx.last_valid_idx();

    info!(
        "Finding stage of type {} starting at index {}",
        stage_type, start_idx
    );

    // Termination condition, we reached the end of the TrackPoints.
    if start_idx >= last_valid_idx {
        info!("get_next_stage(start_idx={start_idx}, stage_type={stage_type}) All TrackPoints exhausted, returning None. last_valid_index={last_valid_idx}");
        return None;
    }

    // A Moving stage ends on the point before the speed drops below the limit.
    // A Stopped stage ends when we have moved some distance.
    let end_idx = match stage_type {
        StageType::Moving => find_stop_index(gpx, start_idx, last_valid_idx, params),
        StageType::Stopped => {
            find_resume_index(gpx, start_idx, last_valid_idx, params.min_metres_to_resume)
        }
    };

    assert!(end_idx <= last_valid_idx);
    assert!(
        end_idx >= start_idx,
        "A stage must contain at least 1 TrackPoint"
    );

    let (min_ele, max_ele) = find_min_and_max_elevation_points(gpx, start_idx, end_idx);

    let stage = Stage {
        stage_type,
        start: &gpx.points[start_idx],
        end: &gpx.points[end_idx],
        min_elevation: min_ele,
        max_elevation: max_ele,
        max_speed: find_max_speed(gpx, start_idx, end_idx),
        track_start_point: &gpx.points[0],
    };

    // Just check we created everything correctly.
    assert_eq!(stage.start.index, start_idx);
    assert_eq!(stage.end.index, end_idx);
    assert!(stage.end.index >= stage.start.index);
    assert!(stage.end.time >= stage.start.time);
    assert!(stage.start.index >= stage.track_start_point.index);

    return Some(stage);
}

/// A Moving stage is ended when we stop. This occurs when we drop below the
/// 'stopped_speed_kmh' and do not attain 'resume_speed_kmh' for at least
/// 'min_duration_seconds'. Find the index of that point.
fn find_stop_index(
    gpx: &EnrichedGpx,
    start_idx: usize,
    last_valid_idx: usize,
    params: &StageDetectionParameters,
) -> usize {
    let mut end_idx = start_idx + 1;

    while end_idx <= last_valid_idx {
        // Find the first time we drop below 'stopped_speed_kmh'
        while end_idx <= last_valid_idx && gpx.points[end_idx].speed_kmh > params.stopped_speed_kmh
        {
            end_idx += 1;
        }

        debug!(
            "find_stop_index(start_idx={start_idx}) Dropped below stopped_speed_kmh of {} at {}",
            params.stopped_speed_kmh, end_idx
        );

        // It's possible we exhausted all the TrackPoints - we were in a moving
        // Stage that went right to the end of the track. Note that the line
        // above which increments end_index means that it is possible that
        // end_index is GREATER than last_valid_index at this point.
        if end_idx >= last_valid_idx {
            debug!(
                "find_stop_index(start_idx={start_idx}) (1) Returning last_valid_idx {last_valid_idx} due to TrackPoint exhaustion"
            );

            return last_valid_idx;
        }

        // Now take note of this point as a *possible* index of the
        // end of a moving stage. We want the stage to end on the point
        // BEFORE we stopped moving.
        let possible_end_point = &gpx.points[end_idx - 1];

        // Scan forward until we have moved 'min_metres_to_resume'.
        while end_idx <= last_valid_idx
            && gpx.points[end_idx].running_metres - possible_end_point.running_metres
                < params.min_metres_to_resume
        {
            end_idx += 1;
        }

        debug!(
            "find_stop_index(start_idx={start_idx}) Scanned forward to index {}, which is {:.2} metres from the possible stop",
            end_idx,
            gpx.points[end_idx].running_metres - possible_end_point.running_metres
        );

        // Same logic as above.
        if end_idx >= last_valid_idx {
            debug!(
                "find_stop_index(start_idx={start_idx}) (2) Returning last_valid_idx {last_valid_idx} due to TrackPoint exhaustion"
            );
            return last_valid_idx;
        }

        // Is that a stop of sufficient length? If so, the point found above is a valid
        // end for this current stage (which is a Moving Stage, remember).

        // Recall that 'possible_end_point' is 1 TrackPoint BEFORE the stop starts - stages
        // do not share points. So you may think that we need to calculate the length
        // of time based on the NEXT TrackPoint. However, a stop can be a single trackpoint
        // long, with an elapsed time of say 25 minutes, and the 'time' field of the
        // TrackPoint will be the time at the END of that 25 minute period. We need to
        // include that 25 minutes in the calculation, so we calculate the duration based on
        // the 'possible_end_point' as a starting point.
        let stop_duration = gpx.points[end_idx].time - possible_end_point.time;

        if stop_duration.as_seconds_f64() >= params.min_duration_seconds {
            debug!(
                "find_stop_index(start_idx={start_idx}) Found valid stop at index {}, duration = {}",
                possible_end_point.index,
                stop_duration
            );
            return possible_end_point.index;
        } else {
            debug!(
                "find_stop_index(start_idx={start_idx}) Rejecting stop at {} because it is too short, duration = {}",
                possible_end_point.index,
                stop_duration
            );
        }

        // If that's not a valid stop (because it's too short),
        // we need to continue searching. Start again from the
        // point we have already reached.
        end_idx += 1;
    }

    // If we get here then we exhausted all the TrackPoints.
    debug!("find_stop_index(start_idx={start_idx}) (2) Returning last_valid_idx {last_valid_idx} due to TrackPoint exhaustion");
    last_valid_idx
}

/// A Stopped stage is ended when we have moved at least 'min_metres_to_resume'.
/// GPX readings can be very noisy when stopped, especially if you move the bike
/// around or take the GPX in a shop with you, so it is better to rely on distance
/// moved rather than speed. We do this using an "as the crow flies" measurement
/// to hopefully avoid nonsense such as parking the bike in a secure spot near
/// the shop...
fn find_resume_index(
    gpx: &EnrichedGpx,
    start_idx: usize,
    last_valid_idx: usize,
    min_metres_to_resume: f64,
) -> usize {
    let start_pt = gpx.points[start_idx].as_geo_point();

    let mut end_index = start_idx + 1;

    while end_index <= last_valid_idx {
        let moved_metres =
            distance_between_points_metres(start_pt, gpx.points[end_index].as_geo_point());
        if moved_metres > min_metres_to_resume {
            debug!("find_resume_index(start_idx={start_idx}) Returning end_idx={end_index} due to having moved {moved_metres:.2}m");
            return end_index;
        }
        end_index += 1;
    }

    // If we get here then we exhausted all the TrackPoints.
    debug!("find_resume_index(start_idx={start_idx}) Returning last_valid_idx={last_valid_idx} due to TrackPoint exhaustion");

    last_valid_idx
}

/// Within a given range of trackpoints, finds the ones with the minimum
/// and maximum elevation.
fn find_min_and_max_elevation_points<'gpx>(
    gpx: &'gpx EnrichedGpx,
    start_idx: usize,
    end_idx: usize,
) -> (&'gpx EnrichedTrackPoint, &'gpx EnrichedTrackPoint) {
    let mut min = &gpx.points[start_idx];
    let mut max = &gpx.points[start_idx];

    for tp in &gpx.points[start_idx..=end_idx] {
        if tp.ele < min.ele {
            min = tp;
        } else if tp.ele > max.ele {
            max = tp;
        }
    }

    assert!(max.ele >= min.ele);

    (min, max)
}

/// Within a given range of trackpoints, finds the one with the
/// maximum speed.
fn find_max_speed<'gpx>(
    gpx: &'gpx EnrichedGpx,
    start_idx: usize,
    end_idx: usize,
) -> &'gpx EnrichedTrackPoint {
    let mut max = &gpx.points[start_idx];

    for tp in &gpx.points[start_idx..=end_idx] {
        if tp.speed_kmh > max.speed_kmh {
            max = tp;
        }
    }

    max
}

/// Calculate distance between two points in metres.
fn distance_between_points_metres(p1: Point, p2: Point) -> f64 {
    p1.geodesic_distance(&p2)
}

/// Try and figure out whether we are starting Moving or Stopped
/// by looking at the average speed over the first 3 minutes.
fn get_starting_stage_type(gpx: &EnrichedGpx, _params: &StageDetectionParameters) -> StageType {
    // Get this out into a variable to avoid off-by-one errors (hopefully).
    let last_valid_idx = gpx.last_valid_idx();

    let start = &gpx.points[0];
    let mut end_idx = 1;
    while end_idx < last_valid_idx {
        let duration = gpx.points[end_idx].time - start.start_time();
        if duration.as_seconds_f64() >= 180.0 {
            return classify_stage(start, &gpx.points[end_idx]);
        } else {
            end_idx += 1;
        }
    }

    let end = &gpx.points[end_idx];
    classify_stage(start, end)
}

/// Classifies a stage, based on the average speed within that stage.
fn classify_stage(start_point: &EnrichedTrackPoint, last_point: &EnrichedTrackPoint) -> StageType {
    let distance_metres = last_point.running_metres - start_point.running_metres;
    let time = last_point.time - start_point.time;
    let speed = speed_kmh_from_duration(distance_metres, time);
    if speed < 5.0 {
        // Less than walking pace? Assume you're stopped.
        StageType::Stopped
    } else {
        StageType::Moving
    }
}