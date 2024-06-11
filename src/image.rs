//! Functions for dealing with our particular images, such as creating volumes from multiple images,
//! building polar / fan images from rectangles, compression and so-on.
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
 *   image.rs - image load, save, compression and manipulation functions
 *   Author - bjb8@st-andrews.ac.uk
 *   
 */
use lzzzz::lz4f::ReadDecompressor;
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::{assert, fs::File, path::Path};
use uuid::Uuid;
use crate::models::{Groups, Images};
use fitsio::errors::Error as FitsError;
use fitsio::hdu::HduInfo;
use fitsio::{errors::check_status, sys, FileOpenMode, FitsFile};
use image::{GrayImage, ImageBuffer, Luma};
use libc;
use log::info;

// Type alias for a 3D image volume
#[derive(Clone)]
pub struct ImageVolume(pub Vec<ImageBuffer<Luma<u8>, Vec<u8>>>);

// The iterator type for the ImageVolume
#[derive(Clone)]
pub struct ImageVolumeIntoIterator {
    img_vol: ImageVolume,
    index: u32, // Hopefully big enough
}

impl IntoIterator for ImageVolume {
    type Item = u8;
    type IntoIter = ImageVolumeIntoIterator;

    fn into_iter(self) -> Self::IntoIter {
        ImageVolumeIntoIterator {
            img_vol: self,
            index: 0,
        }
    }
}

impl Iterator for ImageVolumeIntoIterator {
    type Item = u8;

    // It's depth, height, width progression
    fn next(&mut self) -> Option<u8> {
        let depth = self.img_vol.0.len() as u32; // Weird we have to use the 0 here :/ It's because we can't really type alias
        let height = self.img_vol.0[0].height();
        let width = self.img_vol.0[0].width();

        let z = self.index / (width * height);
        let y = (self.index - z * width * height) / width;
        let x = (self.index - z * width * height) % width;

        let mut result = None;

        if z < depth {
            if y < height {
                if x < width {
                    self.index += 1;
                    result = Some(self.img_vol.0[z as usize].get_pixel(x, y)[0]);
                }
            }
        }

        result
    }
}

use log::error;

/// Type that records an image's size in pixels, width then height.
#[derive(Clone)]
pub struct ImageSize {
    pub width: u32,
    pub height: u32,
}

/// Check that all these images can be loaded correctly.
///
/// * `group_images` - list of Images objects.
/// * `img_path_cache` - cache of image paths.
pub fn check_all_images(
    group_images: &Vec<Images>,
    img_path_cache: &HashMap<String, PathBuf>,
) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    // Loop through all the images found for this group, making sure each image, compressed or otherwise, exists.
    let mut paths: Vec<PathBuf> = vec![];
    assert!(group_images.len() > 0);

    for image in group_images {
        match img_path_cache.get(&image.filename) {
            Some(path) => {
                paths.push(path.clone());
            }
            None => {
                return Err(format!(
                    "Could not find image {} in check_all_images.",
                    image.filename
                )
                .into());
            }
        }
    }

    Ok(paths)
}

/// Read an image for this group.
///
/// * `group` - the Groups object we are interested in.
/// * `group_image` - the Images object we want.
/// * `image_path_cache` - the cache of image paths.
pub fn read_group_image(
    group: &Groups,
    group_image: &Images,
    image_path_cache: &HashMap<String, PathBuf>,
) -> Result<image::ImageBuffer<Luma<u8>, Vec<u8>>, Box<dyn std::error::Error>> {
    // Go through each group, creating our cropped images for ingest.
    info!("Reading group image details for: {}", group.uid);
    let fitspath: PathBuf = image_path_cache
        .get(&group_image.filename)
        .unwrap()
        .to_path_buf();

    // Load one of the images in the group to get hold of sizes
    let mut image_size = ImageSize {
        width: 0,
        height: 0,
    };

    let rimg: ImageBuffer<Luma<u8>, Vec<u8>>;
    /*info!(
        "Reading FITS {} for group {} details.",
        fitspath.to_str().unwrap(),
        group.uid
    );*/

    match read_fits(&fitspath) {
        Ok(img) => {
            image_size.width = img.width();
            image_size.height = img.height();
            rimg = img;
        }
        Err(err) => {
            error!(
                "Failed to read FITS images for group {} - {}",
                group.uid, err
            );
            return Err(err.into());
        }
    }

    Ok(rimg)
}

/// Read a FITS image from disk.
///
/// * `fits_path` - full path to the FITS file, including the .lz4 if compressed.
pub fn read_fits(
    fits_path: &Path,
) -> Result<image::ImageBuffer<Luma<u8>, Vec<u8>>, Box<dyn std::error::Error>> {
    //! Read a FITS file but check if we are passing in an LZ4 path and decompress if we do.
    // TODO - should be an Option!
    // TODO - code duplication here
    let mut width: u32 = 1;
    let mut height: u32 = 1;

    // info!("Reading FITS {} for track.", fits_path.to_str().unwrap());

    let extension: String;

    match fits_path.extension() {
        Some(ext) => {
            extension = String::from(ext.to_str().unwrap());
        }
        None => {
            return Err(Box::from("Failed to get extension."));
        }
    }

    if extension == "lz4" {
        let mut stream = File::open(fits_path)?;
        let mut r = ReadDecompressor::new(&mut stream)?;
        let mut decompressed = Vec::new();
        let mut num_bytes = r.read_to_end(&mut decompressed)?;
        let mut ptr = decompressed.as_ptr();
        let tuid = Uuid::new_v4().to_string();
        let t_filename = std::ffi::CString::new(tuid + ".fits").unwrap();
        let mut fptr = std::ptr::null_mut();
        let mut status = 0;

        unsafe {
            sys::ffomem(
                &mut fptr as *mut *mut _,
                t_filename.as_ptr(),
                sys::READONLY as _,
                &mut ptr as *const _ as *mut *mut libc::c_void,
                &mut num_bytes as *mut _,
                0,
                None,
                &mut status,
            );
        }

        check_status(status)?;

        let mut fits_file = unsafe { FitsFile::from_raw(fptr, FileOpenMode::READONLY) }?;
        //fits_file.pretty_print().expect("pretty printing fits file");
        let hdu = fits_file.primary_hdu().unwrap();

        // Assumption that this is a u8 - it is for our data but still
        let image_data: Vec<u8> = hdu.read_image(&mut fits_file).unwrap();
        if let HduInfo::ImageInfo { shape, .. } = hdu.info {
            width = shape[1] as u32;
            height = shape[0] as u32;

            match GrayImage::from_vec(width, height, image_data) {
                Some(img) => {
                    return Ok(img);
                }
                None => {
                    return Err(Box::from("Failed to make GrayImage from LZ4 in read_fits."));
                }
            }
        }
    }

    let mut fits_file = FitsFile::open(fits_path)?;
    let hdu = fits_file.primary_hdu()?;

    // Assumption that this is a u8 - it is for our data but still
    let image_data: Vec<u8> = hdu.read_image(&mut fits_file)?;

    if let HduInfo::ImageInfo { shape, .. } = hdu.info {
        width = shape[1] as u32;
        height = shape[0] as u32;
    }

    match GrayImage::from_vec(width, height, image_data) {
        Some(img) => {
            return Ok(img);
        }
        None => {
            return Err(Box::from("Failed to make GrayImage in read_fits."));
        }
    }
}

/// Save an image to a FITS file
///
/// * `fits_path` - full path to the FITS file, including the .lz4 if compressed.
/// * `image_data` - the image data.
pub fn img_to_fits(
    fits_path: &Path,
    img_data: &image::ImageBuffer<Luma<u8>, Vec<u8>>,
) -> Result<PathBuf, FitsError> {
    //! Given an ImageBuffer, write this out to a fits file. If the file already exists,
    //! it is removed
    let _ = fs::remove_file(fits_path); // Ignore errors
    let mut fptr = FitsFile::create(fits_path).open()?;

    let hdu = fptr.hdu(0)?;
    hdu.resize(
        &mut fptr,
        &[
            img_data.dimensions().1 as usize,
            img_data.dimensions().0 as usize,
        ],
    )?;
    let hdu2 = fptr.hdu(0)?;
    hdu2.write_image(&mut fptr, img_data)?;
    Ok(fits_path.to_path_buf())
}

/// Given the height of a polar image, return it's width.
///
/// * `height` - height in pixels.
pub fn width_from_height(height: u32) -> u32 {
    //! The fan distortion function. We choose a height that works as our scaling ratio (1.732).
    ((1.732 * height as f32).floor()) as u32
}

/// Reject a mask ImageVolume. Rejects a mask with fewer than 50 pixels set to one.
///
/// * `mask` - the image volume that represents a mask
pub fn reject_mask(mask: &ImageVolume) -> bool {
    let border = 32;
    let mut total: u32 = 0;

    for i in 0..mask.0.len() {
        let frame: &ImageBuffer<Luma<u8>, Vec<u8>> = &mask.0[i];

        for y in border..frame.height() - border {
            for x in border..frame.width() - border {
                let pixel = frame.get_pixel(x, y);
                if pixel.0[0] > 0 {
                    total += 1;
                }
            }
        }
    }

    if total > 50 {
        // TODO - Arbitrary but for now, better than just a single pixel
        return false;
    }

    return true;
}

/// Reject a mask ImageVolume. Rejects a mask with fewer than 1 pixel set to one.
///
/// * `mask` - the image volume that represents a mask
pub fn reject_mask_tiny(mask: &ImageVolume) -> bool {
    let mut total: u32 = 0;

    for i in 0..mask.0.len() {
        let frame: &ImageBuffer<Luma<u8>, Vec<u8>> = &mask.0[i];

        for y in 0..frame.height() {
            for x in 0..frame.width() {
                let pixel = frame.get_pixel(x, y);
                if pixel.0[0] > 0 {
                    total += 1;
                }
            }
        }
    }

    if total > 1 {
        // TODO - Arbitrary
        return false;
    }

    return true;
}

// *** TESTS ***
#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_fits_compressed() {
        let mut d = PathBuf::from(env::var("SEALHITS_TESTDATA_DIR").unwrap());
        d.push("fits/2023_05_28/2023_05_28_22_52_40_711_854.fits.lz4");
        let fits = read_fits(Path::new(&d)).unwrap();
        assert_eq!(fits.dimensions().0, 512);
    }

    #[test]
    fn test_load_save() {
        let mut d = PathBuf::from(env::var("SEALHITS_TESTDATA_DIR").unwrap());
        d.push("fits/2023_05_28/2023_05_28_22_52_40_711_854.fits.lz4");
        let fits = read_fits(Path::new(&d)).unwrap();
        assert!(img_to_fits(Path::new("test.fits"), &fits).is_ok());
        let saved_fits = read_fits(Path::new("test.fits")).unwrap();
        assert_eq!(saved_fits.dimensions().0, 512);
    }
}
