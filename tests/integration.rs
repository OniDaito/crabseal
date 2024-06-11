use chrono::NaiveDate;
use chrono::NaiveTime;
use crabseal::files::*;
use crabseal::image::read_fits;
use crabseal::cache::img_to_cache_compressed;
use chrono::NaiveDateTime;
use std::env;
use std::path::{Path, PathBuf};

// Files tests
#[test]
fn test_fast_find() {
    let mut d = PathBuf::from(env::var("SEALHITS_TESTDATA_DIR").unwrap());
    d.push("fits");
    
    match fits_in_path(&d.to_path_buf(), &String::from("2023_05_28_22_52_42_130_854.fits.lz4")) {
        Some(_path) =>  assert!(true),
        None => { assert!(false); }
    }

    let mut e = PathBuf::from(env::var("SEALHITS_TESTDATA_DIR").unwrap());
    e.push("src");
    
    match fits_in_path(&e.to_path_buf(), &String::from("2023_05_28_22_52_42_130_854.fits.lz4")) {
        Some(_path) =>  assert!(false),
        None => { assert!(true); }
    }
}


#[test]
fn test_cache_save() {
    use chrono::{DateTime, Utc};

    let mut d = PathBuf::from(env::var("SEALHITS_TESTDATA_DIR").unwrap());
    d.push("fits/2023_05_28/2023_05_28_22_52_27_352_853.fits.lz4");
    
    let naive_date = NaiveDate::from_ymd_opt(2021, 10, 1).unwrap();
    let naive_time = NaiveTime::from_hms_opt(0, 0, 0).unwrap();

    let test_date = DateTime::from_naive_utc_and_offset(NaiveDateTime::new(naive_date, naive_time), Utc);
    let fits = read_fits(Path::new(&d)).unwrap();
    assert!(
        img_to_cache_compressed(Path::new("./tests/"), &String::from("test.fits"), &test_date, &fits)
            .is_ok()
    );
}
