/// The various functions for the CrabSeal project.
/// This library contains the various nodes, bounding boxes,
/// generators and sinks that form the processing pipeline
/// in pipeline and pipeline_sector binaries.

/** ```rust,ignore
 * 
 *     /\
 *    ( /   @ @    ()
 *     \  __| |__  /
 *      -/   "   \-
 *     /-|       |-\
 *    / /-\     /-\ \
 *     / /-`---'-\ \     
 *      /         \ CRABSEAL
 * 
 *   lib.rs - rust lib declaration
 *   Author - bjb8@st-andrews.ac.uk
 *   ```
 */

pub mod bbs;
pub mod cache;
pub mod classdata;
pub mod constants;
pub mod db;
pub mod files;
pub mod generators;
pub mod groups;
pub mod image;
pub mod models;
pub mod nodes;
pub mod nodes_tracks;
pub mod nodes_volumes;
pub mod ops;
pub mod ptypes;
pub mod schema;
pub mod sinks;
pub mod track;
