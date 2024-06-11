//! pipeline_sector generates 'sectored/segmented' data from our *sealhits* project.
//! This program works with data collected by the [SealHits] project.
//! The sets created by CrabSeal can be used with the [OceanMotion] neural network.
//!
//! pipeline_sector creates small output images that represent the sectors of the
//! original image - a sector goes to 1 if movement occured in that sector.
//!
//! [SealHits]: https://github.com/onidaito/sealhits
//! [OceanMotion]: https://github.com/onidaito/oceanmotion

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
 *   pipeline.rs - generate datasets using a pipeline
 *   Author - Benjamin Blundell - bjb8@st-andrews.ac.uk
 *
*/
use clap::Parser;
use crabseal::files::create_image_dirs;
use crabseal::generators::GeneratorGroups;

use crabseal::nodes::{node_reject_on_no_mask_tiny, node_slice_datum, node_slice_datum_overlap};
use crabseal::nodes_tracks::{
    node_group_to_trackraw, node_track_kalman, node_trackraw_interpolate, node_trackraw_overlap,
};
use crabseal::nodes_volumes::{
    node_group_to_volume, node_trackraw_to_sectors, node_volume_crop_sector, node_volume_resize,
    node_volume_trim,
};

use crabseal::ops::MovesOps;
use crabseal::ptypes::{DatumT, VolumeT};
use crabseal::sinks::{sink_to_npz, sink_to_png, sink_to_txt};
use fern;
use humantime;
use image::imageops::FilterType::Lanczos3;
use log::info;
use pbr::ProgressBar;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;
use walkdir::WalkDir;

fn run_pipeline(ops: &MovesOps) {
    //! Run the basic pipeline
    let mut img_paths: HashMap<String, PathBuf> = HashMap::new();

    // Write to a cache file or read from it, if it already exits.
    let cache_path = "crabseal.cache";

    if Path::new(cache_path).exists() {
        let file = File::open(cache_path).unwrap();
        let reader = BufReader::new(file);

        for res in reader.lines() {
            let line = res.unwrap().replace('\n', "");
            let tokens = line.split(",").collect::<Vec<&str>>();
            img_paths.insert(tokens[0].to_string(), Path::new(tokens[1]).to_path_buf());
        }
    } else {
        for file in WalkDir::new(&ops.fits_path)
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

        // Now write the cache file
        let mut file = File::create(cache_path).unwrap();

        for key in img_paths.keys() {
            write!(file, "{},{}\n", key, img_paths[key].to_str().unwrap()).unwrap();
        }
    }

    // Dataset paths
    let path_train = ops.out_path.clone().join("images").join("train");
    let path_test = ops.out_path.clone().join("images").join("test");
    let path_val = ops.out_path.clone().join("images").join("val");

    // text file paths
    let path_train_txt = ops.out_path.clone().join("set_train.txt");
    let path_test_txt = ops.out_path.clone().join("set_test.txt");
    let path_val_txt = ops.out_path.clone().join("set_val.txt");

    // Make sure we have a code_to_class id file for outputting classes
    let code_class_path = ops.out_path.clone().join("code_to_class.csv");
    let mut code_to_id: HashMap<String, u8> = HashMap::new();

    if code_class_path.exists() {
        let file = File::open(code_class_path).unwrap();
        let reader = BufReader::new(file);

        for res in reader.lines() {
            let line = res.unwrap().replace('\n', "");
            let tokens = line.split(",").collect::<Vec<&str>>();
            code_to_id.insert(tokens[0].to_string(), tokens[1].parse::<u8>().unwrap());
        }
    }

    let generator = GeneratorGroups::new(
        &ops.dbuser,
        &ops.dbpass,
        &ops.dbname,
        &ops.sonar_ids,
        &img_paths,
        ops.num_frames as usize,
        ops.dataset_limit as usize,
        1632,
        &ops.sqlfilter,
        ops.num_threads,
        &code_to_id,
    );

    // Decide on the group sizes
    let num_groups = generator.size();
    let num_train = (num_groups as f32 / 100.0 * 80.0) as usize;
    let num_test = ((num_groups - num_train) as f32 / 100.0 * 80.0) as usize;
    let mut count: usize = 0;

    info!(
        "Set Sizes - Train: {}, Test: {}, Val: {}",
        num_train,
        num_test,
        num_groups - num_train - num_test
    );

    let mut pb = ProgressBar::new(num_groups as u64);
    pb.format("╢▌▌░╟");

    for group in generator {
        // The pipeline proper - nodes in order.
        assert!(group.points.len() > 0);
        let track_raw = node_group_to_trackraw(&group);
        assert!(track_raw.boxes.len() > 0);
        let interp_track = node_trackraw_interpolate(&track_raw);
        let filled_track = node_trackraw_overlap(&interp_track);
        let kalman_track = node_track_kalman(&filled_track);
        let rejected = crabseal::nodes::node_reject_on_trackraw(&kalman_track, ops.reject_rate);

        if !rejected {
            let overlap_track_second = node_trackraw_overlap(&kalman_track);
            let maybe_vol = node_group_to_volume(&group, &img_paths);
            let mask_volume =
                node_trackraw_to_sectors(&overlap_track_second, &group, ops.sector_size);

            if maybe_vol.is_some() {
                let data_volume = maybe_vol.unwrap();
                let data_cropped_sector = node_volume_crop_sector(&data_volume, ops.sector_size);
                let data_resized =
                    node_volume_resize(&data_cropped_sector, ops.target_width, Lanczos3);
                let datum: DatumT = DatumT::new(&data_resized, &mask_volume);

                // Split the datum and recombine after trim. Do a trim here to make things a bit tighter.
                let (trim_data, _) = node_volume_trim(
                    &VolumeT {
                        volume: datum.raw,
                        extents: datum.extents,
                        origin: datum.origin.clone(),
                    },
                    &overlap_track_second,
                );
                let (trim_mask, _) = node_volume_trim(
                    &VolumeT {
                        volume: datum.mask,
                        extents: datum.extents,
                        origin: datum.origin.clone(),
                    },
                    &overlap_track_second,
                );
                let (trim_og, _) = node_volume_trim(&data_volume, &overlap_track_second);

                let datum_trimed: DatumT = DatumT::new(&trim_data, &trim_mask);
                let og_datum_trimed: DatumT = DatumT::new(&trim_og, &trim_mask);

                if !node_reject_on_no_mask_tiny(&datum_trimed) {
                    // Decide which set this goes into.
                    // TODO - we need a proper node/sink or something for this
                    if count < num_train {
                        sink_to_png(&datum_trimed, &path_train);
                        sink_to_txt(&datum_trimed, &path_train_txt);

                        let slices =
                            node_slice_datum_overlap(&datum_trimed, ops.num_frames as usize);
                        if slices.is_some() {
                            sink_to_npz(slices.unwrap(), &path_train, "half");
                        }

                        let og_slices =
                            node_slice_datum_overlap(&og_datum_trimed, ops.num_frames as usize);
                        if og_slices.is_some() {
                            sink_to_npz(og_slices.unwrap(), &path_train, "large");
                        }
                    } else if count >= num_train && count < num_train + num_test {
                        sink_to_png(&datum_trimed, &path_test);
                        sink_to_txt(&datum_trimed, &path_test_txt);

                        let slices = node_slice_datum(&datum_trimed, ops.num_frames as usize);
                        if slices.is_some() {
                            sink_to_npz(slices.unwrap(), &path_test, "half");
                        }

                        let og_slices =
                            node_slice_datum_overlap(&og_datum_trimed, ops.num_frames as usize);
                        if og_slices.is_some() {
                            sink_to_npz(og_slices.unwrap(), &path_test, "large");
                        }
                    } else {
                        sink_to_png(&datum_trimed, &path_val);
                        sink_to_txt(&datum_trimed, &path_val_txt);

                        let slices = node_slice_datum(&datum_trimed, ops.num_frames as usize);
                        if slices.is_some() {
                            sink_to_npz(slices.unwrap(), &path_val, "half");
                        }

                        let og_slices =
                            node_slice_datum_overlap(&og_datum_trimed, ops.num_frames as usize);
                        if og_slices.is_some() {
                            sink_to_npz(og_slices.unwrap(), &path_val, "large");
                        }
                    }
                }
            }
        }

        pb.inc();
        count = count + 1;
    }
}

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = '.'.to_string())]
    fitspath: String,
    #[arg(short, long, default_value_t = '.'.to_string())]
    outpath: String,
    #[arg(long, default_value_t = String::from("sealhits"))]
    dbname: String,
    #[arg(long, default_value_t = String::from("sealhits"))]
    dbuser: String,
    #[arg(long, default_value_t = String::from("kissfromarose"))]
    dbpass: String,
    #[arg(long, default_value_t = 0)]
    width: u32,
    #[arg(short, long, default_value_t = String::from("853,854"))]
    sonarids: String,
    #[arg(short, long, default_value_t = 0)]
    limit: usize,
    #[arg(long, default_value_t = 16)]
    numframes: u32,
    #[arg(short, long, default_value_t = 6)]
    threads: u32,
    #[arg(long, default_value_t = String::from("none"))]
    sqlfilter: String,
    #[arg(long, default_value_t = 2)]
    sizefilter: i32,
    #[arg(long, default_value_t = 32)]
    sectorsize: u32,
    #[arg(long, default_value_t = 400.0)]
    rejectrate: f32,
}

fn setup_logger(args: &Args) -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                humantime::format_rfc3339_seconds(SystemTime::now()),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .chain(std::io::stdout())
        .chain(fern::log_file(args.outpath.clone() + "/output.log")?)
        .apply()?;
    Ok(())
}

fn main() {
    let args = Args::parse();
    setup_logger(&args).unwrap();

    // Spit out the git tag and url and date and such to the log
    let output = Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .output()
        .unwrap();
    let git_hash = String::from_utf8(output.stdout).unwrap();
    info!("GitTag:{}", git_hash);
    info!("Args:{:?}", args);

    // Create the output directories
    create_image_dirs(&args.outpath);

    // Set the SQLFilter file
    let mut sqlfilter: Option<PathBuf> = None;

    if args.sqlfilter != "none" {
        let np = PathBuf::from(args.sqlfilter);
        match np.try_exists() {
            Ok(_) => {
                sqlfilter = Some(np);
            }
            Err(_) => {
                sqlfilter = None;
            }
        }
    }

    let mut sonarids: Vec<i32> = vec![];
    let splits = args.sonarids.split(",");

    for split in splits {
        let sonar_id = split.parse::<i32>().unwrap();
        sonarids.push(sonar_id);
    }

    let gops = MovesOps {
        target_width: args.width,
        sonar_ids: sonarids,
        dataset_limit: args.limit,
        dbuser: args.dbuser,
        dbpass: args.dbpass,
        dbname: args.dbname,
        fits_path: PathBuf::from(&args.fitspath),
        out_path: PathBuf::from(&args.outpath),
        num_frames: args.numframes,
        num_threads: args.threads,
        sqlfilter: sqlfilter,
        sector_size: args.sectorsize,
        crop_height: 1632,
        reject_rate: args.rejectrate,
    };

    if args.width < 32 {
        println!("--width must be set manually to 32 or greater.");
        return;
    }

    run_pipeline(&gops);
}
