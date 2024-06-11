//! Useful functions for dealing with the various filetypes we are interested in.

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
 *   files.rs - file wrangling functions
 *   Author - bjb8@st-andrews.ac.uk
 *   
 */

use std::collections::HashMap;
use std::io::{Error, BufReader, ErrorKind, BufRead};
use std::{fs::create_dir, fs::File, path::Path};
use std::path::PathBuf;


/// Read the bearing table file. Needed to properly convert the raw images.
pub fn read_bearing_table() -> Result<Vec<f32>, Error> {
    let file = File::open("btable.dat").unwrap();

    let br: BufReader<File> = BufReader::new(file);
    br.lines()
        .map(|line| line.and_then(|v| v.parse().map_err(|e| Error::new(ErrorKind::InvalidData, e))))
        .collect()
}

/// Read the file that maps codes to numbers for the classes.
/// * `map_path` - the path to the class file.
pub fn read_class_map(map_path: &Path,) -> Result<HashMap::<String, u32>, Box<dyn std::error::Error>> {
    let file = File::open(map_path).unwrap();
    let mut rdr = csv::Reader::from_reader(file);
    let mut map = HashMap::<String, u32>::new();
    
    for result in rdr.records() {
        let record = result?;
        let class_name: String = String::from(&record[0]);
        let class_number: u32 = record[1].parse()?;
        map.insert(class_name, class_number);
    }

    Ok(map)
}

/// Find out if the fits, compressed or otherwise, exists in the path
/// * `fits_path` - the path to the FITS files.
/// * `fits_name` - the name of the FITS file we want.
pub fn fits_in_path(fits_path: &Path, fits_name: &String) -> Option<PathBuf> { 
    // TODO - we should probably check the image size as well to see if it
    // matches what is expected.
    let compressed: String = fits_name.clone() + ".lz4";

    match fast_find(fits_path, &compressed) {
        Some(path) => return Some(path),
        None => match fast_find(fits_path, fits_name) {
            Some(path) => return Some(path),
            None => {
                return None;
            }
        },
    }
}


/// Find FITS files we are interested in, but using the file layout from *SealHits* to save time.
/// * `fits_path` - path to the FITS files.
/// * `fits_name` - the name of the FITS file we want.
pub fn fast_find(fits_path: &Path, fits_name: &String) -> Option<PathBuf> {
    //! Search for our fits images.
    use walkdir::WalkDir;

    for entry in WalkDir::new(fits_path)
    .follow_links(false)
    .into_iter()
    .filter_map(|e| e.ok()) {
        let f_name = entry.file_name().to_str()?;

        if f_name == fits_name.as_str() {
            let found = entry.into_path();
            return Some(found);
        }
    }

    None
    //panic!("No FITS found for path  {}", fits_path.to_string_lossy())
    // Err(format!("No FITS found for path  {}", fits_path.to_string_lossy()).into())
}


/// Create the required directories for each set - train, test and validation.
/// * `base_path_str` - base directory to make other directories under.
pub fn create_image_dirs(base_path_str: &String) {
    let base_path = Path::new(base_path_str);

    let images = "images";
    let images_path = base_path.join(images);
    _create_dir(&images_path);

    let train = "train";
    let train_path = images_path.join(train);
    _create_dir(&train_path);

    let test = "test";
    let test_path = images_path.join(test);
    _create_dir(&test_path);

    let val = "val";
    let val_path = images_path.join(val);
    _create_dir(&val_path);
}

/// Internal function to create 
pub fn _create_dir(dir_path: &PathBuf) {
    if let Err(e) = create_dir(dir_path) {
        if e.kind() != std::io::ErrorKind::AlreadyExists {
            panic!("Failed to create directory: {}", e);
        } else {
            println!("Directory already exists");
        }
    } else {
        println!("Directory created successfully");
    }
}