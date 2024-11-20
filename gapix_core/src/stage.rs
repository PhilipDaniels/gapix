//! Contains the functionality relating to Stages.
//! Detecting these is quite a bit of work. Once we get
//! the Stages determined we can calculate a lot of
//! other metrics fairly easily.

use core::{fmt, slice};
use std::{collections::HashSet, ops::Index};

use chrono::{DateTime, TimeDelta, Utc};
use geo::{GeodesicDistance, Point};
use log::{debug, info, warn};
use logging_timer::time;
use rayon::prelude::*;

use crate::{
    geocoding::reverse_geocode_latlon,
    model::{EnrichedGpx, EnrichedTrackPoint},
};

/// Calculates speed in km/h from metres and seconds.
pub fn speed_kmh(metres: f64, seconds: f64) -> f64 {
    (metres / seconds) * 3.6
}

/// Calculates speed in km/h from metres and a duration.
pub fn speed_kmh_from_duration(metres: f64, duration: TimeDelta) -> f64 {
    let millis = duration.num_milliseconds() as f64;
    speed_kmh(metres, millis / 1000.0)
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
/// you moving, or controlling.
#[derive(Debug)]
pub struct Stage {
    pub stage_type: StageType,
    // The first point in the entire track. We need this to calculate various
    // running totals. We could pass it into the relevant methods, but storing
    // it works ok too.
    pub track_start_point: EnrichedTrackPoint,
    pub start: EnrichedTrackPoint,
    pub end: EnrichedTrackPoint,
    pub min_elevation: Option<EnrichedTrackPoint>,
    pub max_elevation: Option<EnrichedTrackPoint>,
    pub max_speed: Option<EnrichedTrackPoint>,
    pub avg_heart_rate: Option<f64>,
    pub max_heart_rate: Option<EnrichedTrackPoint>,
    pub avg_air_temp: Option<f64>,
    pub min_air_temp: Option<EnrichedTrackPoint>,
    pub max_air_temp: Option<EnrichedTrackPoint>,
}

/// The type of a Stage.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum StageType {
    Moving,
    Control,
}

impl fmt::Display for StageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StageType::Moving => write!(f, "Moving"),
            StageType::Control => write!(f, "Control"),
        }
    }
}

impl StageType {
    fn toggle(self) -> Self {
        match self {
            StageType::Moving => StageType::Control,
            StageType::Control => StageType::Moving,
        }
    }
}

impl Stage {
    /// Returns the indexes of all the TrackPoints that have been
    /// highlighted as 'special' in some way, e.g. the point
    /// of min elevation. This is so we can force a hyperlink
    /// to Google Maps for the special points.
    pub fn highlighted_trackpoints(&self) -> Vec<usize> {
        let mut idxs = vec![
            self.track_start_point.index,
            self.start.index,
            self.end.index,
        ];

        if let Some(p) = &self.min_elevation {
            idxs.push(p.index);
        }

        if let Some(p) = &self.max_elevation {
            idxs.push(p.index);
        }

        if let Some(p) = &self.max_speed {
            idxs.push(p.index);
        }

        if let Some(p) = &self.max_heart_rate {
            idxs.push(p.index);
        }

        if let Some(p) = &self.max_air_temp {
            idxs.push(p.index);
        }

        idxs.sort();

        idxs
    }

    /// Reverse geocodes the stage, i.e. looks up the place name from
    /// the (lat,lon) coordinates and returns it. For a Control stage, this
    /// is just "Name", for a Moving stage, returns "Name1 to Name2".
    pub fn reverse_geocode(&self) -> Option<String> {
        let start_desc = reverse_geocode_latlon(self.start.as_rtree_point());

        match self.stage_type {
            StageType::Moving => {
                let end_desc = reverse_geocode_latlon(self.end.as_rtree_point());
                match (start_desc, end_desc) {
                    (Some(mut sd), Some(ed)) => {
                        sd.push_str(" to ");
                        sd.push_str(&ed);
                        Some(sd)
                    }
                    _ => None,
                }
            }
            StageType::Control => start_desc,
        }
    }

    /// Returns the duration of the stage.
    pub fn duration(&self) -> Option<TimeDelta> {
        // Be careful to use the time that the 'start' TrackPoint
        // began, not when it was recorded. They are very different
        // for TrackPoints written when you are stopped. A TrackPoint
        // may not be written for many minutes in that situation.
        match (self.end.time, self.start.start_time()) {
            (Some(et), Some(st)) => Some(et - st),
            _ => None,
        }
    }

    /// Returns the running duration to the end of the stage from
    /// the 'track_start_point' (the first point in the track).
    pub fn running_duration(&self) -> Option<TimeDelta> {
        match (self.end.time, self.track_start_point.start_time()) {
            (Some(et), Some(st)) => Some(et - st),
            _ => None,
        }
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
    pub fn average_speed_kmh(&self) -> Option<f64> {
        self.duration()
            .map(|dur| speed_kmh_from_duration(self.distance_metres(), dur))
    }

    /// Returns the average speed, calculated over the distance from
    /// the start of the track to the end of the stage.
    pub fn running_average_speed_kmh(&self) -> Option<f64> {
        self.running_duration()
            .map(|dur| speed_kmh_from_duration(self.end.running_metres, dur))
    }

    /// Returns the total ascent in metres over the stage.
    pub fn ascent_metres(&self) -> Option<f64> {
        match (
            self.end.running_ascent_metres,
            self.start.running_ascent_metres,
        ) {
            (Some(m1), Some(m2)) => Some(m1 - m2),
            _ => None,
        }
    }

    /// Returns the total ascent to the end of the stage from
    /// the beginning of the track.
    pub fn running_ascent_metres(&self) -> Option<f64> {
        self.end.running_ascent_metres
    }

    /// Returns the ascent rate in m/km over the stage.
    pub fn ascent_rate_per_km(&self) -> Option<f64> {
        self.ascent_metres().map(|a| a / self.distance_km())
    }

    /// Returns the total descent in metres over the stage.
    pub fn descent_metres(&self) -> Option<f64> {
        match (
            self.end.running_descent_metres,
            self.start.running_descent_metres,
        ) {
            (Some(m1), Some(m2)) => Some(m1 - m2),
            _ => None,
        }
    }

    /// Returns the total descent to the end of the stage from
    /// the beginning of the track.
    pub fn running_descent_metres(&self) -> Option<f64> {
        self.end.running_descent_metres
    }

    /// Returns the descent rate in m/km over the stage.
    pub fn descent_rate_per_km(&self) -> Option<f64> {
        self.descent_metres().map(|a| a / self.distance_km())
    }
}

#[derive(Default)]
pub struct StageList(Vec<Stage>);

impl Index<usize> for StageList {
    type Output = Stage;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl<'sl> IntoIterator for &'sl StageList {
    type Item = &'sl Stage;
    type IntoIter = slice::Iter<'sl, Stage>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl StageList {
    /// Returns a Rayon parallel iterator over the stages.
    pub fn par_iter(&self) -> rayon::slice::Iter<'_, Stage> {
        self.0.par_iter()
    }

    /// Returns a Rayon mutable parallel iterator over the stages.
    pub fn par_iter_mut(&mut self) -> rayon::slice::IterMut<'_, Stage> {
        self.0.par_iter_mut()
    }

    /// Returns the indexes of all the TrackPoints that have been
    /// highlighted as 'special' in some way, e.g. the point
    /// of min elevation. This is so we can force a hyperlink
    /// to Google Maps for the special points.
    pub fn highlighted_trackpoints(&self) -> HashSet<usize> {
        let mut idxs = HashSet::new();

        for s in self.iter() {
            for i in s.highlighted_trackpoints() {
                idxs.insert(i);
            }
        }

        idxs
    }

    pub fn iter(&self) -> slice::Iter<Stage> {
        self.0.iter()
    }

    /// Returns the first point in the first stage.
    pub fn first_point(&self) -> &EnrichedTrackPoint {
        &self.0[0].start
    }

    /// Returns the last point in the last stage.
    pub fn last_point(&self) -> &EnrichedTrackPoint {
        &self.0[self.len() - 1].end
    }

    /// Adds another stage to the end of the list.
    pub fn push(&mut self, stage: Stage) {
        self.0.push(stage);
    }

    /// Returns the number of stages.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns true if there are no stages.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the start time of the first Stage.
    pub fn start_time(&self) -> Option<DateTime<Utc>> {
        self.first_point().start_time()
    }

    /// Returns the end time of the last Stage.
    pub fn end_time(&self) -> Option<DateTime<Utc>> {
        self.last_point().time
    }

    /// Returns the total duration between the start of the first
    /// stage and the end of the last stage.
    pub fn duration(&self) -> Option<TimeDelta> {
        match (self.end_time(), self.start_time()) {
            (Some(et), Some(st)) => Some(et - st),
            _ => None,
        }
    }

    /// Returns the total time Moving across all the stages.
    pub fn total_moving_time(&self) -> Option<TimeDelta> {
        match (self.duration(), self.total_control_time()) {
            (Some(dur), Some(tst)) => Some(dur - tst),
            _ => None,
        }
    }

    /// Returns the total time Control across all the stages.
    pub fn total_control_time(&self) -> Option<TimeDelta> {
        self.0
            .iter()
            .filter_map(|stage| match stage.stage_type {
                StageType::Moving => None,
                StageType::Control => Some(stage.duration()),
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
    pub fn average_moving_speed(&self) -> Option<f64> {
        self.total_moving_time()
            .map(|tmt| speed_kmh_from_duration(self.distance_metres(), tmt))
    }

    /// Returns the overall average moving speed over the whole track,
    /// this includes stopped time.
    pub fn average_overall_speed(&self) -> Option<f64> {
        self.duration()
            .map(|dur| speed_kmh_from_duration(self.distance_metres(), dur))
    }

    /// Returns the point of minimum elevation across all the stages.
    pub fn min_elevation(&self) -> Option<&EnrichedTrackPoint> {
        let mut current_min = &self.0[0];

        for stage in self.iter() {
            match (
                current_min.min_elevation.as_ref(),
                stage.min_elevation.as_ref(),
            ) {
                (Some(min_ele), Some(stg)) => {
                    if stg.ele < min_ele.ele {
                        current_min = stage;
                    }
                }
                (None, Some(_)) => {
                    current_min = stage;
                }
                _ => {}
            }
        }

        current_min.min_elevation.as_ref()
    }

    /// Returns the point of maximum elevation across all the stages.
    pub fn max_elevation(&self) -> Option<&EnrichedTrackPoint> {
        let mut current_max = &self.0[0];

        for stage in self.iter() {
            match (
                current_max.max_elevation.as_ref(),
                stage.max_elevation.as_ref(),
            ) {
                (Some(max_ele), Some(stg)) => {
                    if stg.ele > max_ele.ele {
                        current_max = stage;
                    }
                }
                (None, Some(_)) => {
                    current_max = stage;
                }
                _ => {}
            }
        }

        current_max.max_elevation.as_ref()
    }

    /// Returns the total ascent in metres across all the stages.
    pub fn total_ascent_metres(&self) -> Option<f64> {
        self.0.iter().map(|stage| stage.ascent_metres()).sum()
    }

    /// Returns the total descent in metres across all the stages.
    pub fn total_descent_metres(&self) -> Option<f64> {
        self.0.iter().map(|stage| stage.descent_metres()).sum()
    }

    /// Returns the point of maximum speed across all the stages.
    pub fn max_speed(&self) -> Option<&EnrichedTrackPoint> {
        let mut current_max = &self.0[0];

        for stage in self.iter() {
            match (current_max.max_speed.as_ref(), stage.max_speed.as_ref()) {
                (Some(max_sp), Some(stg)) => {
                    if stg.speed_kmh > max_sp.speed_kmh {
                        current_max = stage;
                    }
                }
                (None, Some(_)) => {
                    current_max = stage;
                }
                _ => {}
            }
        }

        current_max.max_speed.as_ref()
    }

    /// Returns the point of maximum heart rate across all the stages.
    pub fn max_heart_rate(&self) -> Option<&EnrichedTrackPoint> {
        self.0
            .iter()
            .filter_map(|s| s.max_heart_rate.as_ref())
            // The unwraps are safe because of the use of filter_map().
            .max_by(|a, b| a.heart_rate().unwrap().cmp(&b.heart_rate().unwrap()))
    }

    /// Returns the point of minimum temperature across all the stages.
    pub fn min_temperature(&self) -> Option<&EnrichedTrackPoint> {
        self.0
            .iter()
            .filter_map(|s| s.min_air_temp.as_ref())
            // The unwraps are safe because of the use of filter_map().
            .min_by(|a, b| a.air_temp().unwrap().total_cmp(&b.air_temp().unwrap()))
    }

    /// Returns the point of maximum temperature across all the stages.
    pub fn max_temperature(&self) -> Option<&EnrichedTrackPoint> {
        self.0
            .iter()
            .filter_map(|s| s.max_air_temp.as_ref())
            // The unwraps are safe because of the use of filter_map().
            .max_by(|a, b| a.air_temp().unwrap().total_cmp(&b.air_temp().unwrap()))
    }

    /// Returns the total moving time as a percentage of the total duration
    /// across all the stages.
    pub fn moving_percent(&self) -> Option<f64> {
        match (self.total_moving_time(), self.duration()) {
            (Some(tmt), Some(dur)) => {
                let tmt = tmt.num_milliseconds() as f64;
                let dur = dur.num_milliseconds() as f64;
                Some(tmt / dur)
            }
            _ => None,
        }
    }

    /// Returns the total controlling time as a percentage of the total duration
    /// across all the stages.
    pub fn controlling_percent(&self) -> Option<f64> {
        self.moving_percent().map(|mp| 1.0 - mp)
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
pub fn detect_stages(gpx: &mut EnrichedGpx, params: StageDetectionParameters) -> StageList {
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

    // If we don't have time there is nothing we can do.
    if gpx.points.iter().any(|p| p.time.is_none()) {
        return stages;
    }

    // Note 1: The first TrackPoint always has a speed of 0, but it is unlikely
    // that you are actually in a Control stage. However, it's not impossible,
    // see Note 2 for why.

    // Note 2: We need to deal with the slightly bizarre situation where you turn
    // the GPS on and then don't go anywhere for a while - so your first stage
    // may be a Control stage!

    let mut start_idx = 0;

    // We will alternate stage types - Moving-Stopped-Moving-Stopped etc.
    // But instead of assuming that we start off Moving, we try and figure it out.
    let mut stage_type = get_starting_stage_type(gpx, &params);
    info!("Determined type of the first stage to be {}", stage_type);

    while let Some(stage) = get_next_stage(stage_type, start_idx, gpx, &params) {
        // Stages do not share points, the next stage starts on the next point.
        start_idx = stage.end.index + 1;

        if let Some(dur) = stage.duration() {
            info!(
                "Adding {} stage from point {} to {}, length={:.3}km, duration={}",
                stage.stage_type,
                stage.start.index,
                stage.end.index,
                stage.distance_km(),
                dur,
            );
        } else {
            info!(
                "Adding {} stage from point {} to {}, length={:.3}km, duration=unknown",
                stage.stage_type,
                stage.start.index,
                stage.end.index,
                stage.distance_km()
            );
        }

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

fn get_next_stage(
    stage_type: StageType,
    start_idx: usize,
    gpx: &mut EnrichedGpx,
    params: &StageDetectionParameters,
) -> Option<Stage> {
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
        StageType::Control => {
            find_resume_index(gpx, start_idx, last_valid_idx, params.min_metres_to_resume)
        }
    };

    assert!(end_idx <= last_valid_idx);
    assert!(
        end_idx >= start_idx,
        "A stage must contain at least 1 TrackPoint"
    );

    let (min_elevation, max_elevation) = find_min_and_max_elevation_points(gpx, start_idx, end_idx);
    let (max_heart_rate, avg_heart_rate) = find_heart_rates(gpx, start_idx, end_idx);
    let (min_air_temp, max_air_temp, avg_air_temp) = find_air_temps(gpx, start_idx, end_idx);

    // Reverse geocode all these points of interest before we clone them into
    // the stage. This ensures that they will show up in the TrackPoints tab
    // with the location descriptions set.
    // reverse_geocode_point(&mut gpx.points[0]);
    // reverse_geocode_point(&mut gpx.points[start_idx]);
    // reverse_geocode_point(&mut gpx.points[end_idx]);
    // reverse_geocode_point_option(min_elevation.borrow_mut());
    // reverse_geocode_point(&mut gpx.points[end_idx]);

    let stage = Stage {
        stage_type,
        track_start_point: gpx.points[0].clone(),
        start: gpx.points[start_idx].clone(),
        end: gpx.points[end_idx].clone(),
        min_elevation,
        max_elevation,
        max_speed: find_max_speed(gpx, start_idx, end_idx),
        avg_heart_rate,
        max_heart_rate,
        min_air_temp,
        max_air_temp,
        avg_air_temp,
    };

    // Just check we created everything correctly.
    assert_eq!(stage.start.index, start_idx);
    assert_eq!(stage.end.index, end_idx);
    assert!(stage.end.index >= stage.start.index);
    assert!(stage.end.time >= stage.start.time);
    assert!(stage.start.index >= stage.track_start_point.index);

    Some(stage)
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
        while end_idx <= last_valid_idx
            && gpx.points[end_idx]
                .speed_kmh
                .expect("speed exists due to check in detect_stages")
                > params.stopped_speed_kmh
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

        // Same logic as above.
        if end_idx >= last_valid_idx {
            debug!(
                "find_stop_index(start_idx={start_idx}) (2) Returning last_valid_idx {last_valid_idx} due to TrackPoint exhaustion"
            );
            return last_valid_idx;
        }

        debug!(
            "find_stop_index(start_idx={start_idx}) Scanned forward to index {}, which is {:.2} metres from the possible stop",
            end_idx,
            gpx.points[end_idx].running_metres - possible_end_point.running_metres
        );

        // Is that a stop of sufficient length? If so, the point found above is a valid
        // end for this current stage (which is a Moving Stage, remember).

        // Recall that 'possible_end_point' is 1 TrackPoint BEFORE the stop starts - stages
        // do not share points. So you may think that we need to calculate the length
        // of time based on the NEXT TrackPoint. However, a stop can be a single trackpoint
        // long, with an elapsed time of say 25 minutes, and the 'time' field of the
        // TrackPoint will be the time at the END of that 25 minute period. We need to
        // include that 25 minutes in the calculation, so we calculate the duration based on
        // the 'possible_end_point' as a starting point.
        let stop_duration = gpx.points[end_idx]
            .time
            .expect("time exists due to check in detect_stages")
            - possible_end_point
                .time
                .expect("time exists due to check in detect_stages");

        let secs = stop_duration.num_milliseconds() as f64 / 1000.0;
        if secs >= params.min_duration_seconds {
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
            debug!("find_resume_index(start_idx={start_idx}) Returning end_idx={end_index} due to having moved {moved_metres:.2}m as the crow flies");
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
fn find_min_and_max_elevation_points(
    gpx: &EnrichedGpx,
    start_idx: usize,
    end_idx: usize,
) -> (Option<EnrichedTrackPoint>, Option<EnrichedTrackPoint>) {
    let mut min = &gpx.points[start_idx];
    let mut max = &gpx.points[start_idx];

    for tp in &gpx.points[start_idx..=end_idx] {
        // Any missing elevation invalidates the calculation.
        if tp.ele.is_none() {
            return (None, None);
        }

        if tp.ele < min.ele {
            min = tp;
        } else if tp.ele > max.ele {
            max = tp;
        }
    }

    assert!(max.ele >= min.ele);

    (Some(min.clone()), Some(max.clone()))
}

/// Within a given range of trackpoints, finds the one with the
/// maximum speed.
fn find_max_speed(
    gpx: &EnrichedGpx,
    start_idx: usize,
    end_idx: usize,
) -> Option<EnrichedTrackPoint> {
    let mut max = &gpx.points[start_idx];

    for tp in &gpx.points[start_idx..=end_idx] {
        // Any missing speed invalidates the calculation.
        tp.speed_kmh?;

        if tp.speed_kmh > max.speed_kmh {
            max = tp;
        }
    }

    Some(max.clone())
}

/// Within a given range of trackpoints, finds the point of
/// maximum heart rate and the average heart rate.
fn find_heart_rates(
    gpx: &EnrichedGpx,
    start_idx: usize,
    end_idx: usize,
) -> (Option<EnrichedTrackPoint>, Option<f64>) {
    let mut sum: f64 = 0.0;
    let mut count = 0;
    let mut max: Option<EnrichedTrackPoint> = None;

    for point in &gpx.points[start_idx..=end_idx] {
        if let Some(hr) = point.heart_rate() {
            sum += hr as f64;
            count += 1;

            if let Some(m) = max.as_ref() {
                let mhr = m.heart_rate().expect("Should be safe to unwrap because 'max' is only set for points that have heart rates");
                if hr > mhr {
                    max = Some(point.clone());
                }
            } else {
                // No current max, this point has a hr so use it.
                max = Some(point.clone())
            }
        }
    }

    let avg = if sum == 0.0 {
        None
    } else {
        Some(sum / count as f64)
    };

    (max, avg)
}

/// Finds the min, max and avg air temp over the stage.
fn find_air_temps(
    gpx: &EnrichedGpx,
    start_idx: usize,
    end_idx: usize,
) -> (
    Option<EnrichedTrackPoint>,
    Option<EnrichedTrackPoint>,
    Option<f64>,
) {
    let mut sum: Option<f64> = None;
    let mut min: Option<EnrichedTrackPoint> = None;
    let mut max: Option<EnrichedTrackPoint> = None;
    let mut count = 0;

    for idx in start_idx..=end_idx {
        if let Some(at) = gpx.points[idx].air_temp() {
            count += 1;
            sum = Some(sum.unwrap_or_default() + at);

            if let Some(m) = min.as_ref() {
                let mht = m.air_temp().expect("Should be safe to unwrap because 'min' is only set for points that have an air temp");
                if at < mht {
                    min = Some(gpx.points[idx].clone());
                }
            } else {
                // No current min, this point has an air temp so use it.
                min = Some(gpx.points[idx].clone());
            }

            if let Some(m) = max.as_ref() {
                let mht = m.air_temp().expect("Should be safe to unwrap because 'max' is only set for points that have an air temp");
                if at > mht {
                    max = Some(gpx.points[idx].clone());
                }
            } else {
                // No current max, this point has an air temp so use it.
                max = Some(gpx.points[idx].clone());
            }
        }
    }

    let avg = sum.map(|s| s / count as f64);

    (min, max, avg)
}

/// Calculate distance between two points in metres.
pub fn distance_between_points_metres(p1: Point, p2: Point) -> f64 {
    p1.geodesic_distance(&p2)
}

/// Try and figure out whether we are starting Moving or Stopped
/// by looking at the average speed over the first 3 minutes.
fn get_starting_stage_type(gpx: &EnrichedGpx, _params: &StageDetectionParameters) -> StageType {
    // The first point has no start_time() since it does not have
    // a delta time. We can safely skip it.
    let start = &gpx.points[1];
    let mut end_idx = 1;

    while end_idx <= gpx.last_valid_idx() {
        let duration = gpx.points[end_idx]
            .time
            .expect("time exists due to check in detect_stages")
            - start
                .start_time()
                .expect("time exists due to check in detect_stages");

        let secs = duration.num_milliseconds() as f64 / 1000.0;
        if secs >= 180.0 {
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

    let time = last_point
        .time
        .expect("time exists due to check in detect_stages")
        - start_point
            .time
            .expect("time exists due to check in detect_stages");

    let speed = speed_kmh_from_duration(distance_metres, time);

    if speed < 5.0 {
        // Less than walking pace? Assume you're stopped.
        StageType::Control
    } else {
        StageType::Moving
    }
}
