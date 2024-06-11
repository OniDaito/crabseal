//! The node functions. Functions that process and pass on data through the pipeline.
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
 *   nodes.rs - nodes take an object and transform it.
 *   Author - bjb8@st-andrews.ac.uk
 *   
 */
extern crate nalgebra as na;
use crate::image::{reject_mask, reject_mask_tiny};
use crate::{
    bbs::Area,
    bbs::RefChange,
    image::ImageVolume,
    models::{Images, Points},
    ptypes::{DatumT, GroupT, SlicedDatumT, TrackRawT, VolumeT},
};

/// Split a volume into smaller, overlapping volumes with random placement.
///
/// * `track` - the TrackRawT to convert
/// * `group` - the corresponding GroupT object.
/// * `sector_size` - The dimension of the square sector.
pub fn node_trim_group(group: &GroupT) -> GroupT {
    //! Remove frames before the first track and any after.
    let images: Vec<Images> = vec![];
    let points: Vec<Vec<Points>> = vec![];

    let mut start: usize = 0;
    let mut end: usize = group.points.len() - 1;

    for pidx in 0..group.points.len() {
        if points[pidx].len() > 0 {
            start = pidx;
            break;
        }
    }

    for pidx in group.points.len() - 1..0 {
        if points[pidx].len() > 0 {
            end = pidx;
            break;
        }
    }

    GroupT {
        origin: group.origin.clone(),
        images: images[start..end].to_vec(),
        points: points[start..end].to_vec(),
    }
}

/// Combine the two volumes (data and mask) into a single DatumT.
///
/// * `data` - the data VolumeT.
/// * `mask` - the mask VolumeT.
pub fn node_combine_datum_mask(data: &VolumeT, mask: &VolumeT) -> DatumT {
    //! Combine the two halve of the datum
    assert!(data.volume.0[0].width() == mask.volume.0[0].width());
    assert!(data.volume.0[0].height() == mask.volume.0[0].height());
    DatumT::new(data, mask)
}

/// Combine the two sectored volumes (data and mask) into a single DatumT.
///
/// * `data` - the data VolumeT.
/// * `mask` - the mask VolumeT.
pub fn node_combine_datum_sector(data: &VolumeT, mask: &VolumeT) -> DatumT {
    //! Combine the two halve of the datum
    assert!(data.volume.0[0].width() > mask.volume.0[0].width());
    assert!(data.volume.0[0].height() > mask.volume.0[0].height());
    DatumT::new(data, mask)
}

/// Slice a DatumT into shorter DatumTs
///
/// * `data` - the data VolumeT to slice.
/// * `window` - the length of the slices in number of frames.
pub fn node_slice_datum(datum: &DatumT, window: usize) -> Option<SlicedDatumT> {
    //! Slice the datum up into parts based on window size. This node does not overlap nodes
    //! We return a sliced type that we can then pass on to another node.
    let mut slices: Vec<DatumT> = vec![];

    if datum.mask.0.len() != datum.raw.0.len() || datum.mask.0.len() < window {
        return None;
    }

    let mut sidx: usize = 0;

    while sidx + window < datum.mask.0.len() {
        let mut nraw = ImageVolume(vec![]);

        for v in &datum.raw.0[sidx..sidx + window] {
            nraw.0.push(v.clone());
        }

        assert!(nraw.0.len() == window);

        let mut nmask = ImageVolume(vec![]);

        for v in &datum.mask.0[sidx..sidx + window] {
            nmask.0.push(v.clone());
        }

        assert!(nmask.0.len() == window);

        let newd = DatumT {
            raw: nraw,
            mask: nmask,
            origin: datum.origin.clone(),
            extents: datum.extents,
        };

        slices.push(newd);
        sidx += window;
    }

    Some(SlicedDatumT { slices: slices })
}

/// Slice a DatumT into shorter DatumTs but allowing overlap so nothing is missed.
///
/// * `data` - the data VolumeT to slice.
/// * `window` - the length of the slices in number of frames.
pub fn node_slice_datum_overlap(datum: &DatumT, window: usize) -> Option<SlicedDatumT> {
    //! Slice the datum up into parts based on window size. This node allows overlapping slices.
    //! We return a sliced type that we can then pass on to another node.
    //! We add the slices together, then shift each one until we match the size required
    let mut slices: Vec<DatumT> = vec![];

    if datum.mask.0.len() != datum.raw.0.len() || datum.mask.0.len() < window {
        return None;
    }

    // Find the starting positions for each slice, trying to keep the overlaps as
    // even as possible.
    let max_len = datum.mask.0.len() as i32;
    let num_slices = (datum.raw.0.len() as f32 / window as f32).ceil() as i32;
    let mut stretch_len = num_slices * window as i32;

    let mut positions: Vec<i32> = vec![];

    for i in 0..num_slices {
        positions.push(i * window as i32);
    }

    let mut widx = 1;

    while stretch_len > max_len {
        for idx in widx..num_slices {
            positions[idx as usize] = positions[idx as usize] - 1;
        }
        let last = positions.len() - 1;
        stretch_len = positions[last] + window as i32;

        widx += 1;

        if widx >= num_slices {
            widx = 1
        }
    }

    // Now do the slicing
    for posi in positions {
        let pos = posi as usize;

        let mut nraw = ImageVolume(vec![]);

        for v in &datum.raw.0[pos..pos + window] {
            nraw.0.push(v.clone());
        }

        assert!(nraw.0.len() == window);

        let mut nmask = ImageVolume(vec![]);

        for v in &datum.mask.0[pos..pos + window] {
            nmask.0.push(v.clone());
        }

        assert!(nmask.0.len() == window);

        let newd = DatumT {
            raw: nraw,
            mask: nmask,
            origin: datum.origin.clone(),
            extents: datum.extents,
        };

        slices.push(newd);
    }

    Some(SlicedDatumT { slices: slices })
}

/// Reject a TrackRawT if the standard deviation of position or area exceeds a certain level.
/// Returns true if this TrackRawT should be rejected.
///
/// * `track` - the TrackRawT to reject.
/// * `reject_rate` - the number to be under to not be rejected.
pub fn node_reject_on_trackraw(track: &TrackRawT, reject_rate: f32) -> bool {
    // TODO - other nodes will need to check for options and reject. It allows for early rejection at
    // any stage of the pipeline. Perhaps, a node like this, when injected into a pipeline can reach
    // out to the pipeline class and call an early stop?

    // Reject a datum if the mask is bobbins. We want small areas of low variation across the board
    // and movement.

    // TODO - this function isn't perfect as we aren't checking whether or not the boxes are one per frame.

    // TODO - same reject_rate for area AND position?

    // Start with dists.
    let mut dists: Vec<f32> = vec![];

    for idx in 0..track.boxes.len() - 1 {
        let bbox = track.boxes[idx].bbox;
        let nbox = track.boxes[idx + 1].bbox;
        let (bx, by) = bbox.centre();
        let (nx, ny) = nbox.centre();

        let d = ((ny as f32 - by as f32) * (ny as f32 - by as f32))
            + ((nx as f32 - bx as f32) * (nx as f32 - bx as f32));
        dists.push(d as f32);
    }

    let mean = dists.iter().sum::<f32>() / dists.len() as f32;
    let mut vary: Vec<f32> = vec![];

    for d in dists {
        let v: f32 = (mean - d) * (mean - d);
        vary.push(v);
    }

    let variance = vary.iter().sum::<f32>() / vary.len() as f32;
    let stddev = variance.sqrt();

    if stddev > reject_rate {
        // TODO - very basic rejection
        return true;
    }

    // Now areas
    let mut areas: Vec<f32> = vec![];

    for idx in 0..track.boxes.len() {
        let bbox = track.boxes[idx].bbox;
        areas.push(bbox.area() as f32);
    }

    let mean = areas.iter().sum::<f32>() / areas.len() as f32;
    let mut vary: Vec<f32> = vec![];

    for d in areas {
        let v: f32 = (mean - d) * (mean - d);
        vary.push(v);
    }

    let variance = vary.iter().sum::<f32>() / vary.len() as f32;
    let stddev = variance.sqrt();

    if stddev > reject_rate {
        // TODO - very basic rejection
        return true;
    }

    // println!("areas stddev of {} is {}", torigin.unwrap().group.huid, stddev);
    false
}

/// Reject if this datum has a bad mask.
/// Returns true if this DatumT should be rejected.
///
/// * `datum` - the DatumT to reject.
pub fn node_reject_on_no_mask(datum: &DatumT) -> bool {
    // Reject a datum if it has no mask in it (can happen post split)
    reject_mask(&datum.mask)
}

/// Reject if this datum has a bad mask.
/// Returns true if this DatumT should be rejected.
///
/// * `datum` - the DatumT to reject.
pub fn node_reject_on_no_mask_tiny(datum: &DatumT) -> bool {
    reject_mask_tiny(&datum.mask)
}

#[cfg(test)]
mod tests {
    // TODO - these tests are super inefficient! Setup and teardown needs to be split out as the DB stuff
    // needs to be done just once and always, even if tests fail. Don't need to keep doing it. Would allow
    // us to run in parallel.
    use super::*;
    use crate::sinks::sink_to_png;
    use crate::{generators::GeneratorGroups, sinks::sink_to_npz};
    use image::imageops::FilterType::Lanczos3;
    use postgres::{Client, NoTls};
    use serial_test::serial;
    use std::collections::HashMap;
    use std::env;
    use std::panic;
    use std::path::PathBuf;
    use std::{
        fs::File,
        io::{BufRead, BufReader},
        path::Path,
        str::FromStr,
    };
    use walkdir::WalkDir;

    use crate::nodes_tracks::{
        node_group_to_trackraw, node_track_kalman, node_trackraw_interpolate, node_trackraw_overlap,
    };
    use crate::nodes_volumes::{
        node_group_to_volume, node_trackraw_to_sectors, node_trackraw_to_volume,
        node_volume_crop_sector, node_volume_resize, node_volume_split_random, node_volume_trim,
    };

    // This closure and catch lets us catch failures but always fire off the teardown.
    // https://medium.com/@ericdreichert/test-setup-and-teardown-in-rust-without-a-framework-ba32d97aa5ab
    // Can't put setup inside this sadly, but that's okay for now.
    fn run_test<T>(test: T) -> ()
    where T: FnOnce() -> () + panic::UnwindSafe {

        let result = panic::catch_unwind(|| {
            test()
        });

        teardown();
        assert!(result.is_ok())
    }

    fn setup(fits_path: &str) -> (HashMap<String, PathBuf>, HashMap<String, u8>) {
        // Load the test database into PostgreSQL just like with Sealhits
        let mut d = PathBuf::from(env::var("SEALHITS_TESTDATA_DIR").unwrap());
        d.push("testseals.sql");
        let sql_file_content = std::fs::read_to_string(d.as_path()).unwrap();
        let pg_user = env::var("SEALHITS_TESTDATA_PGUSER").unwrap();
        let pg_pass = env::var("SEALHITS_TESTDATA_PGPASS").unwrap();
        let pg_str: String = String::from_str("host=localhost user=").unwrap();
        let conn_string = pg_str + &pg_user + " password=" + &pg_pass;
        let mut client = Client::connect(conn_string.as_str(), NoTls).unwrap();
        client
            .batch_execute("CREATE USER testseals WITH PASSWORD 'testseals';")
            .unwrap();
        client.batch_execute("CREATE DATABASE testseals WITH OWNER testseals TEMPLATE = template0 ENCODING = 'UTF8' LOCALE_PROVIDER = libc LOCALE = 'C.UTF-8'").unwrap();
        client.close().unwrap();

        let conn_string2 = conn_string + " dbname=testseals";
        client = Client::connect(conn_string2.as_str(), NoTls).unwrap();
        client.batch_execute(&sql_file_content).unwrap();
        client.close().unwrap();

        // Start with a generator
        let mut img_paths: HashMap<String, PathBuf> = HashMap::new();

        for file in WalkDir::new(fits_path)
            .into_iter()
            .filter_map(|file| file.ok())
        {
            if file.metadata().unwrap().is_file() {
                // This conversion to string from osstr is absolutely stupid!
                let mut key = file.file_name().to_str().map(|s| s.to_string()).unwrap();
                key = key.replace(".lz4", "");
                img_paths.insert(key, file.path().to_path_buf());
            }
        }

        let mut code_to_id: HashMap<String, u8> = HashMap::new();
        let code_class_path = "code_to_class.csv";

        if Path::new(code_class_path).exists() {
            let file = File::open(code_class_path).unwrap();
            let reader = BufReader::new(file);

            for res in reader.lines() {
                let line = res.unwrap().replace('\n', "");
                let tokens = line.split(",").collect::<Vec<&str>>();
                code_to_id.insert(tokens[0].to_string(), tokens[1].parse::<u8>().unwrap());
            }
        }

        (img_paths, code_to_id)
    }

    fn teardown() {
        // Remove the testseals database and user
        let pg_user = env::var("SEALHITS_TESTDATA_PGUSER").unwrap();
        let pg_pass = env::var("SEALHITS_TESTDATA_PGPASS").unwrap();
        let pg_str: String = String::from_str("host=localhost user=").unwrap();
        let conn_string = pg_str + &pg_user + " password=" + &pg_pass;
        let mut client = Client::connect(conn_string.as_str(), NoTls).unwrap();
        client.batch_execute("drop database testseals;").unwrap();
        client.batch_execute("drop user testseals;").unwrap();
    }

    #[test]
    #[serial]
    fn test_nodes() {
        let mut d = PathBuf::from(env::var("SEALHITS_TESTDATA_DIR").unwrap());
        d.push("fits");
        let dbuser = "testseals";
        let dbpass = "testseals";
        let dbname = "testseals";
        let sonar_ids = vec![853, 854];
        let fits_path = &d.to_str().unwrap();
        let dataset_limit = 10;
        let minimum_window = 4;
        let (img_paths, code_to_id) = setup(fits_path);

        run_test(|| {
            let generator = GeneratorGroups::new(
                dbuser,
                dbpass,
                dbname,
                &sonar_ids,
                &img_paths,
                minimum_window,
                dataset_limit,
                1632,
                &None,
                4,
                &code_to_id,
            );

            for group in generator {
                assert!(group.points.len() > 0);
                let track_raw = node_group_to_trackraw(&group);
                assert!(track_raw.boxes.len() > 0);
                let interp_track = node_trackraw_interpolate(&track_raw);
                let data_volume = node_group_to_volume(&group, &img_paths).unwrap();
                let (trimed_volume, _) = node_volume_trim(&data_volume, &interp_track);

                let mut max_pixel = 0;

                for pixel in trimed_volume.volume.0[0].pixels() {
                    if pixel[0] > max_pixel {
                        max_pixel = pixel[0];
                    }
                }

                assert!(max_pixel > 0);
                let mask_volume = node_trackraw_to_volume(&interp_track, &group);
                let (trimmed_mask, _) = node_volume_trim(&mask_volume, &interp_track);

                let datum = node_combine_datum_mask(&trimed_volume, &trimmed_mask);
                sink_to_png(&datum, &PathBuf::from_str("./tests/").unwrap());

                let slices = node_slice_datum(&datum, minimum_window);

                if slices.is_some() {
                    sink_to_npz(
                        slices.unwrap(),
                        &PathBuf::from_str("./tests/").unwrap(),
                        "og",
                    );
                }
            }
        })
    }

    #[test]
    #[serial]
    fn test_kalman() {
        let mut d = PathBuf::from(env::var("SEALHITS_TESTDATA_DIR").unwrap());
        d.push("fits");
        let dbuser = "testseals";
        let dbpass = "testseals";
        let dbname = "testseals";
        let sonar_ids = vec![853, 854];
        let fits_path = d.to_str().unwrap();
        let dataset_limit = 100;
        let minimum_window = 16;

        let (img_paths, code_to_id) = setup(fits_path);

        run_test(|| {
            let generator = GeneratorGroups::new(
                dbuser,
                dbpass,
                dbname,
                &sonar_ids,
                &img_paths,
                minimum_window,
                dataset_limit,
                1632,
                &None,
                6,
                &code_to_id,
            );

            for group in generator {
                assert!(group.points.len() > 0);
                let track_raw = node_group_to_trackraw(&group);
                assert!(track_raw.boxes.len() > 0);
                let interp_track = node_trackraw_interpolate(&track_raw);
                let overlap_track = node_trackraw_overlap(&interp_track);
                let kalman_track = node_track_kalman(&overlap_track);
                let rejected = node_reject_on_trackraw(&kalman_track, 400.0);

                if !rejected {
                    let overlap_track_second = node_trackraw_overlap(&kalman_track);
                    let data_volume = node_group_to_volume(&group, &img_paths).unwrap();
                    let mut max_pixel = 0;

                    for pixel in data_volume.volume.0[0].pixels() {
                        if pixel[0] > max_pixel {
                            max_pixel = pixel[0];
                        }
                    }

                    assert!(max_pixel > 0);
                    let mask_volume = node_trackraw_to_volume(&overlap_track_second, &group);
                    let datum = node_combine_datum_mask(&data_volume, &mask_volume);
                    sink_to_png(&datum, &PathBuf::from_str("./tests/").unwrap());
                    let slices = node_slice_datum(&datum, minimum_window);

                    if slices.is_some() {
                        sink_to_npz(
                            slices.unwrap(),
                            &PathBuf::from_str("./tests/").unwrap(),
                            "og",
                        );
                    }
                }
            }
        })
    }

    #[test]
    #[serial]
    fn test_splits() {
        let mut d = PathBuf::from(env::var("SEALHITS_TESTDATA_DIR").unwrap());
        d.push("fits");
        let dbuser = "testseals";
        let dbpass = "testseals";
        let dbname = "testseals";
        let sonar_ids = vec![853, 854];
        let fits_path = &d.to_str().unwrap();
        let dataset_limit = 10;
        let minimum_window = 16;
        let (img_paths, code_to_id) = setup(fits_path);

        run_test(|| {
            let generator = GeneratorGroups::new(
                dbuser,
                dbpass,
                dbname,
                &sonar_ids,
                &img_paths,
                minimum_window,
                dataset_limit,
                1632,
                &None,
                6,
                &code_to_id,
            );

            for group in generator {
                assert!(group.points.len() > 0);
                let track_raw = node_group_to_trackraw(&group);
                assert!(track_raw.boxes.len() > 0);

                let data_volume = node_group_to_volume(&group, &img_paths).unwrap();
                let mask_volume = node_trackraw_to_volume(&track_raw, &group);
                let new_dats = node_volume_split_random(&data_volume, &mask_volume, 128);

                for datum in new_dats {
                    if !node_reject_on_no_mask(&datum) {
                        let (trim_data, _) = node_volume_trim(
                            &VolumeT {
                                volume: datum.raw,
                                extents: datum.extents,
                                origin: datum.origin.clone(),
                            },
                            &track_raw,
                        );
                        let (trim_mask, _) = node_volume_trim(
                            &VolumeT {
                                volume: datum.mask,
                                extents: datum.extents,
                                origin: datum.origin.clone(),
                            },
                            &track_raw,
                        );
                        let datum_trimed: DatumT = node_combine_datum_mask(&trim_data, &trim_mask);

                        assert!(!node_reject_on_no_mask(&datum_trimed));
                        let _ = node_slice_datum(&datum_trimed, 16 as usize);

                        sink_to_png(&datum_trimed, &PathBuf::from_str("./tests/").unwrap());
                        let slices = node_slice_datum(&datum_trimed, minimum_window);
                        sink_to_npz(
                            slices.unwrap(),
                            &PathBuf::from_str("./tests/").unwrap(),
                            "og",
                        );
                    }
                }
            }
        })
    }

    #[test]
    #[serial]
    fn test_sectors() {
        let mut d = PathBuf::from(env::var("SEALHITS_TESTDATA_DIR").unwrap());
        d.push("fits");
        let dbuser = "testseals";
        let dbpass = "testseals";
        let dbname = "testseals";
        let sonar_ids = vec![853, 854];
        let fits_path = &d.to_str().unwrap();
        let dataset_limit = 10;
        let minimum_window = 16;
        let (img_paths, code_to_id) = setup(fits_path);

        run_test(|| {
            let generator = GeneratorGroups::new(
                dbuser,
                dbpass,
                dbname,
                &sonar_ids,
                &img_paths,
                minimum_window,
                dataset_limit,
                1632,
                &None,
                6,
                &code_to_id,
            );

            for group in generator {
                assert!(group.points.len() > 0);
                let track_raw = node_group_to_trackraw(&group);
                assert!(track_raw.boxes.len() > 0);

                let data_volume = node_group_to_volume(&group, &img_paths).unwrap();
                // Crop to the nearest power of two, then resize
                let data_cropped = node_volume_crop_sector(&data_volume, 32);
                let data_resized = node_volume_resize(&data_cropped, 256, Lanczos3);

                let mask_volume = node_trackraw_to_sectors(&track_raw, &group, 32);
                let datum: DatumT = DatumT::new(&data_resized, &mask_volume);

                sink_to_png(&datum, &PathBuf::from_str("./tests/").unwrap());
                let slices = node_slice_datum(&datum, 16 as usize);
                sink_to_npz(
                    slices.unwrap(),
                    &PathBuf::from_str("./tests/").unwrap(),
                    "half",
                );
            }
        })
    }
}
