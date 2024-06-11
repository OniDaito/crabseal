//! A small number of caching functions to help speed up multiple runs
//! of dataset processing.

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
 *   cache.rs - functions for caching and loading the fits images.
 *   Author - bjb8@st-andrews.ac.uk
 *   
 */

use fitsio::errors::Error as FitsError;
use lzzzz::lz4f::{WriteCompressor, Preferences};
use chrono::{DateTime, Utc, Datelike};
use image::Luma;
use std::io::prelude::*;
use std::fs::{File, self};
use std::path::{Path, PathBuf};
use crate::image::img_to_fits;


/// Save an image to a directory, compressed.
/// 
/// * `cache_path` - the path to the cache directory.
/// * `fits_name` - filename for the fits file.
/// * `fits_time` - the time for this fits file.
/// * `image_data` - The data for the image itself.
pub fn img_to_cache_compressed(
    cache_path: &Path,
    fits_name: &String,
    fits_time: &DateTime<Utc>,
    img_data: &image::ImageBuffer<Luma<u8>, Vec<u8>>,
) -> Result<PathBuf, FitsError> {
    //! Save an img to cache as a compressed fits with lz4
    // First create a subdir if it doesn't exist already
    let y = format!("{:02}", fits_time.year());
    let m = format!("{:02}", fits_time.month());
    let d = format!("{:02}", fits_time.day());
    let dpath: PathBuf = cache_path.join(y + "_" + &m + "_" + &d); 
    
    if !dpath.exists() {
        std::fs::create_dir(&dpath)?;
    }

    let fpath: PathBuf = dpath.join(&fits_name);
    let cpath: PathBuf = dpath.join(fits_name.clone() + ".lz4");
    let cached = img_to_fits(&fpath, img_data)?;
    // Now compress
    // Delete temp file if it exists - ignore error
    let _ = fs::remove_file(&cpath);
    let mut stream_in = File::open(cached).unwrap();
    let mut buf: Vec<u8> = vec![];
    stream_in.read_to_end(&mut buf)?;

    let mut stream_out = File::create(&cpath).unwrap();
    let mut w = WriteCompressor::new(&mut stream_out, Preferences::default()).unwrap(); // Ignore error?
    w.write_all(&buf)?;

    fs::remove_file(fpath)?;
    Ok(cpath)
}
