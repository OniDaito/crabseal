//! This module contains all the nodes functions that deal with tracks.
/**
 *     /\
 *    ( /   @ @    ()
 *     \  __| |__  /
 *      -/   "   \-
 *     /-|       |-\
 *    / /-\     /-\ \
 *     / /-`---'-\ \     
 *      /         \ CRABSEAL
 *
 *   nodes_tracks.rs - nodes that apply to tracks
 *   Author - bjb8@st-andrews.ac.uk
 *   
 */
extern crate nalgebra as na;

use crate::bbs::{points_to_bb, FrameBoxRaw, RawCoords};
use crate::image::ImageSize;
use crate::track::{interpolate_track_raw, smooth_track};
use crate::{
    files::read_bearing_table,
    ptypes::{GroupT, TrackRawT},
    track::overlap_track_raw,
};


/// Convert a GroupT object to a TrackRawT object
///
/// * `group` - the GroupT object to convert.
pub fn node_group_to_trackraw(group: &GroupT) -> TrackRawT {
    //! Extract the track raw from a group
    let mut boxes: Vec<FrameBoxRaw> = vec![];
    let btable = read_bearing_table().unwrap();

    // Loop over the images in the group, exporting the tracks and indices
    for i in 0..group.points.len() {
        let sonar_range = group.images[i].range as f32;
        let image_points = &group.points[i];

        // Now find the original Bounding boxes
        if image_points.len() > 0 {
            // We have some points in this image so lets make a BoundingBox
            let bb = points_to_bb(&image_points, sonar_range);
            let rawbb = bb.to_raw(&group.origin.img_size, &btable);

            boxes.push(FrameBoxRaw {
                frame: i as u32,
                bbox: rawbb,
            });
        }
    }

    TrackRawT::new(boxes, Option::Some(group.origin.clone()))
}


/// Interpolate a TrackRawT object, filling in any blank frames.
///
/// * `track` - the TrackRawT object to interpolate.
pub fn node_trackraw_interpolate(track: &TrackRawT) -> TrackRawT {
    //! Interpolate the track object
    let torigin = track.origin.clone();

    if torigin.is_some() {
        let origin = torigin.unwrap();
        let img_size: &ImageSize = &origin.img_size;
        let new_track = interpolate_track_raw(&track.boxes, img_size);
        return TrackRawT::new(new_track, Some(origin.clone()))
    }
    TrackRawT::new(track.boxes.clone(), torigin)
}


/// Make sure all points in the track overlap per frame.
///
/// * `track` - the TrackRawT object to modify.
pub fn node_trackraw_overlap(track: &TrackRawT) -> TrackRawT {
    //! Make sure neighbouring frames of tracks overlap
    let torigin = track.origin.clone();

    if track.origin.is_some() {
        let origin = torigin.unwrap();
        let new_track = overlap_track_raw(&track.boxes, &origin.img_size);
        return TrackRawT::new(new_track, track.origin.clone())
    }
    TrackRawT::new(track.boxes.clone(), torigin)
}


/// Smooth this track with a Kalman Filter.
///
/// * `track` - the TrackRawT object to smooth.
pub fn node_track_kalman(track: &TrackRawT) -> TrackRawT {
    //! Attempt to smooth this track with a Kalman filter
    let torigin = track.origin.clone();
    
    if track.origin.is_some() {
        let origin = torigin.unwrap();
        let smoothed = smooth_track(&track.boxes, &origin.img_size);

        return TrackRawT{
            boxes: smoothed,
            origin: Some(origin.clone())
        }
    }

    return TrackRawT{
        boxes: track.boxes.clone(),
        origin: torigin
    }
}