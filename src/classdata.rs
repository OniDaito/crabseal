//! Functions for writing which class a datum belongs to.
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
 *   classdata.rs - functions for writing which class a datum belongs to.
 *   Author - bjb8@st-andrews.ac.uk
 *   
 */
use crate::bbs::XYBox;
use crate::files::read_class_map;
use crate::models::Groups;
use chrono::{DateTime, Utc};
use std::fs::File;
use std::io::Write;
use std::path::Path;

use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// A useful struct for storing the group, tbox and times output from export_frames
/// Represents a single frame in our output datum.
pub struct FrameInfo {
    pub group: Uuid,
    pub tbox: XYBox,
    pub frame: u32,
    pub image_time: DateTime<Utc>,
}

/// A useful struct for storing the group to strip_name output from export_frames
pub struct GroupStrip {
    pub group: Uuid,
    pub strip_name: String,
}


/// Write a class annotation to an external file.
/// 
/// * `group` - the Groups object we are referring to.
/// * `gname` - the name of the group (usually the huid).
/// * `wrap_anno` - the file we are writing to.
/// * `map_path` - the mapping of String to number for the class.
fn write_class_anno(
    group: &Groups,
    gname: &String,
    wrap_anno: &Arc<Mutex<File>>,
    map_path: &Path,
) {
    // We should decide on the class. We map the codes, keeping them all
    // as they are for now.
    // Load the classmapping - TODO we don't need to do this every time
    let classmap = read_class_map(map_path).unwrap(); // Ignore error?
    let gcode = group.code.to_lowercase();

    // Assuming then 'none' or 'other' class is zero here :S or that 0 is default
    let mut classid = 0;

    match classmap.get(&gcode) {
        Some(res) => {
            classid = *res;
        }
        None => {
            println!("No class found for group {} with code {}", group.uid, gcode);
            classid = 0;
        }
    }

    let mut anno_file = wrap_anno.lock().unwrap();

    // Finally, write out to the CSV file
    write!(anno_file, "{},{}\n", gname, classid).unwrap();
}


/// Write a group and it's bounding boxes to a CSV file.
/// 
/// * `group` - the Groups object we are referring to.
/// * `xybox` - the bounding box in question.
/// * `frame` - the frame number within this group track.
/// * `time` - the real datetime for this frame.
/// * `wrap_data` - the file we are writing to.
fn write_data(
    group: &Groups,
    xybox: &XYBox,
    frame: u32,
    time: DateTime<Utc>,
    wrap_data: &Arc<Mutex<File>>,
) {
    let mut data_file = wrap_data.lock().unwrap();

    // Finally, write out to the CSV file
    write!(
        data_file,
        "{},{},{},{},{},{},{},{}\n",
        group.uid,
        group.code,
        frame,
        time,
        xybox.x_min,
        xybox.y_min,
        xybox.x_max - xybox.x_min,
        xybox.y_max - xybox.y_min
    )
    .unwrap();
}
