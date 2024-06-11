//! The Options that the pipeline and pipeline_sector programs need. Copied from the command line arguments.
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
 *   ops.rs - Global options (bad?)
 *   Author - bjb8@st-andrews.ac.uk
 *
 *
*/

use std::path::PathBuf;

pub struct MovesOps {
    /// What width are we aiming for?
    pub target_width: u32,
    /// Sonar IDs to include   
    pub sonar_ids: Vec<i32>,
    /// Upper limit on dataset size  
    pub dataset_limit: usize,
    /// Username for the database
    pub dbuser: String,
    /// Password for the database        
    pub dbpass: String,
    /// Name of the database
    pub dbname: String,
    /// Path to the FITS files
    pub fits_path: PathBuf,
    /// Path where the dataset is saved
    pub out_path: PathBuf,
    /// The number of frames / history time window length
    pub num_frames: u32,
    /// Number of threads to run in parallel
    pub num_threads: u32,
    /// Path to an optional SQLFilter file
    pub sqlfilter: Option<PathBuf>,
    /// If we are sectoring, what size the sectors?
    pub sector_size: u32,
    // Of the original images, what is the minimum height they should all be set to
    pub crop_height: u32,
    // The rejection rate for the track rejection function. 
    pub reject_rate: f32,
}
