//! Sinks are the endpoints of the pipeline. Writing NPZ files, PNGs and textfiles.
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
 *   sinks.rs - final output nodes
 *   Author - bjb8@st-andrews.ac.uk
 *   
 */

use crate::ptypes::VolumeT;

use crate::ptypes::{DatumT, SlicedDatumT};
use image::{GrayImage, Luma, Rgb, Rgb32FImage, RgbImage};
use npyz::WriterBuilder;
use std::io::Write;
use std::path::PathBuf;
use std::{
    fs::{File, OpenOptions},
    io,
    path::Path,
};


/// Save a datum as a couple of PNG files.
/// 
/// * `datum` - the DatumT to save.
/// * `outpath` - The path to save the PNGs.
pub fn sink_to_png(datum: &DatumT, out_path: &PathBuf) {
    //! Save PNGs out to disk by squashing the volume.
    let bp: Rgb<f32> = Rgb([0.1, 0.2, 0.0]);
    let bq: Rgb<u8> = Rgb([0, 0, 0]);
    let num_frames = datum.raw.0.len();
    let mut tee_raw: image::ImageBuffer<Rgb<f32>, Vec<f32>> =
        Rgb32FImage::from_pixel(datum.raw.0[0].width(), datum.raw.0[0].height(), bp);
    let mut final_raw: image::ImageBuffer<Rgb<u8>, Vec<u8>> =
        RgbImage::from_pixel(datum.raw.0[0].width(), datum.raw.0[0].height(), bq);

    let ex = datum.extents.0;
    let ey = datum.extents.1;
    let ew = datum.extents.2;
    let eh = datum.extents.3;

    for image in &datum.raw.0 {
        let mut y = 0;

        for row in image.rows() {
            let mut x = 0;
            for og_pixel in row {
                let mut new_pixel = tee_raw.get_pixel(x, y).clone();
                new_pixel.0[2] += og_pixel.0[0] as f32;
                tee_raw.put_pixel(x, y, new_pixel);
                x += 1;
            }
            y += 1;
        }
    }

    for (x, y, pixel) in tee_raw.enumerate_pixels() {
        let new_pixel = Rgb([10, 20, (pixel.0[2] / num_frames as f32).min(255.0) as u8]);
        final_raw.put_pixel(x, y, new_pixel);
    }

    if datum.origin.is_some() {
        let dpath = out_path.clone();
        let pname = datum.origin.clone().unwrap().group.huid.to_string().clone()
            + &format!("{:02}", ex)
            + "-"
            + &format!("{:02}", ey)
            + "-"
            + &format!("{:02}", ew)
            + "-"
            + &format!("{:02}", eh)
            + "_base.png"; // lol! What a line!
        let pstr = dpath.join(&pname);
        let ppath: &Path = Path::new(&pstr);
        final_raw.save(ppath).unwrap();
    } else {
        // TODO - not implemented yet
        assert!(false)
    }

    let bp: Luma<u8> = Luma([0]);
    let mut final_mask: image::ImageBuffer<Luma<u8>, Vec<u8>> =
        GrayImage::from_pixel(datum.mask.0[0].width(), datum.mask.0[0].height(), bp);

    for image in &datum.mask.0 {
        let mut y = 0;

        for row in image.rows() {
            let mut x = 0;

            for og_pixel in row {
                let mut new_pixel = final_mask.get_pixel(x, y).clone();
                new_pixel.0[0] = (og_pixel[0] + new_pixel.0[0]).min(1);
                final_mask.put_pixel(x, y, new_pixel);
                x += 1;
            }
            y += 1;
        }
    }

    for pixel in final_mask.pixels_mut() {
        pixel.0[0] = pixel.0[0] * 255;
    }

    if datum.origin.is_some() {
        let dpath = out_path.clone();
        let pname = datum.origin.clone().unwrap().group.huid.to_string().clone()
            + &format!("{:02}", ex)
            + "-"
            + &format!("{:02}", ey)
            + "-"
            + &format!("{:02}", ex + ew)
            + "-"
            + &format!("{:02}", ey + eh)
            + "_mask.png"; // lol! What a line!
        let pstr = dpath.join(&pname);
        let ppath: &Path = Path::new(&pstr);
        final_mask.save(ppath).unwrap();
    } else {
        // TODO - not implemented yet
        assert!(false)
    }
}


/// Save a sliced datum as a series of NPZ files for numpy.
/// 
/// * `sliced` - the SlicedDatumT to save.
/// * `out_path` - the path to save the NPZ files.
/// * `suffix` - a common suffix to all the files.
pub fn sink_to_npz(sliced: SlicedDatumT, out_path: &PathBuf, suffix: &str) {
    // Take the datum ownership and send it to npz files

    for sidx in 0..sliced.slices.len() {
        let datum = sliced.slices[sidx].clone(); // TODO - Using a lot of clones around here :/
        let origin = &datum.origin;

        // TODO - assuming an origin here. Also the clone seems unnecessary
        let fname = origin.clone().unwrap().group.huid;
        let sonar_id = origin.clone().unwrap().sonar_id;

        let ex = datum.extents.0;
        let ey = datum.extents.1;
        let ew = datum.extents.2;
        let eh = datum.extents.3;

        // Save out the final image to a subdir 'images' of the dataset
        let gname = fname.to_string()
            + "_"
            + &format!("{:02}", sidx)
            + "_"
            + &format!("{:02}", ex)
            + "-"
            + &format!("{:02}", ey)
            + "-"
            + &format!("{:02}", ex + ew)
            + "-"
            + &format!("{:02}", ey + eh)
            + "_"
            + &format!("{}", sonar_id)
            + "_"
            + suffix
            + "_base.npz";
        let mname = fname.to_string()
            + "_"
            + &format!("{:02}", sidx)
            + "_"
            + &format!("{:02}", ex)
            + "-"
            + &format!("{:02}", ey)
            + "-"
            + &format!("{:02}", ex + ew)
            + "-"
            + &format!("{:02}", ey + eh)
            + "_"
            + &format!("{}", sonar_id)
            + "_"
            + suffix
            + "_mask.npz";

        if !std::path::Path::new(&out_path).exists() {
            std::fs::create_dir(&out_path).unwrap();
        }

        let fstr = out_path.join(&gname);
        let fpath: &Path = Path::new(&fstr);

        let mstr = out_path.join(&mname);
        let mpath: &Path = Path::new(&mstr);

        // Write out the volume (3D image / Video etc) as a numpy NPZ file for use in Python
        let file_image: io::BufWriter<File> = io::BufWriter::new(File::create(fpath).unwrap());
        let shape = [
            datum.raw.0.len() as u64,
            datum.raw.0[0].height() as u64,
            datum.raw.0[0].width() as u64,
        ];
        let mut writer = npyz::WriteOptions::new()
            .default_dtype()
            .shape(&shape)
            .writer(file_image)
            .begin_nd()
            .unwrap();

        writer.extend(datum.raw.into_iter()).unwrap();
        writer.finish().unwrap();

        let file_mask: io::BufWriter<File> = io::BufWriter::new(File::create(mpath).unwrap());
        let shape = [
            datum.mask.0.len() as u64,
            datum.mask.0[0].height() as u64,
            datum.mask.0[0].width() as u64,
        ];
        let mut writer = npyz::WriteOptions::new()
            .default_dtype()
            .shape(&shape)
            .writer(file_mask)
            .begin_nd()
            .unwrap();
        writer.extend(datum.mask.into_iter()).unwrap();
        writer.finish().unwrap();
    }
}


/// Save a single VolumeT to an NPZ file.
/// 
/// * `volume` - the VolumeT to save.
/// * `out_path` - the path to save the NPZ files.
/// * `suffix` - a common suffix to all the files.
pub fn sink_to_npz_volume(volume: VolumeT, out_path: &PathBuf, suffix: &str) {
    //! Sink a volume to an NPZ
    let origin = &volume.origin;

    let fname = origin.clone().unwrap().group.huid;
    let sonar_id = origin.clone().unwrap().sonar_id;

    let ex = volume.extents.1;
    let ey = volume.extents.1;
    let ew = volume.extents.2;
    let eh = volume.extents.3;

    // Save out the final image to a subdir 'images' of the dataset
    let gname = fname.to_string()
        + "_"
        + &format!("{:02}", 0)
        + "_"
        + &format!("{:02}", ex)
        + "-"
        + &format!("{:02}", ey)
        + "-"
        + &format!("{:02}", ex + ew)
        + "-"
        + &format!("{:02}", ey + eh)
        + "_"
        + &format!("{}", sonar_id)
        + suffix + ".npz";
   
    if !std::path::Path::new(&out_path).exists() {
        std::fs::create_dir(&out_path).unwrap();
    }

    let fstr = out_path.join(&gname);
    let fpath: &Path = Path::new(&fstr);

    // Write out the volume (3D image / Video etc) as a numpy NPZ file for use in Python
    let file_image: io::BufWriter<File> = io::BufWriter::new(File::create(fpath).unwrap());
    let shape = [
        volume.volume.0.len() as u64,
        volume.volume.0[0].height() as u64,
        volume.volume.0[0].width() as u64,
    ];
    let mut writer = npyz::WriteOptions::new()
        .default_dtype()
        .shape(&shape)
        .writer(file_image)
        .begin_nd()
        .unwrap();

    writer.extend(volume.volume.into_iter()).unwrap();
    writer.finish().unwrap();
}


/// Save a datum to a text file.
/// 
/// * `datum` - the DatumT to save.
/// * `out_path` - the path to the text file to save this DatumT to.
pub fn sink_to_txt(datum: &DatumT, out_path: &PathBuf) {
    //! Record the HUID to a text file
    let mut file = OpenOptions::new()
        .read(true)
        .append(true)
        .create(true)
        .open(out_path)
        .unwrap();

    let line = datum.origin.clone().unwrap().group.huid;
    write!(file, "{}\n", line).unwrap();
}
