//! Node functions that deal specifically with volumes of different kinds (images and masks).
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
 *   nodes_volumes.rs - nodes that apply to volumes.
 *   Author - bjb8@st-andrews.ac.uk
 *   
 */
extern crate nalgebra as na;

use crate::bbs::FrameBoxRaw;

use crate::ptypes::Dimensions;
use crate::{
    image::read_fits,
    image::ImageVolume,
    ptypes::{DatumT, GroupT, TrackRawT, VolumeT},
};
use image::imageops::crop;
use image::imageops::resize;
use image::imageops::FilterType;
use image::{GrayImage, ImageBuffer, Luma};

use std::collections::HashMap;
use std::path::PathBuf;
use rand::prelude::*;


/// Trim down a volume so it matches the start and end frames of a track.
/// Return a VolumeT and a new Track with adjusted frames.
/// 
/// * `vol` - the volume to trim down.
/// * `track` - the TrackRawT to trim the volume to.
pub fn node_volume_trim(vol: &VolumeT, track: &TrackRawT) -> (VolumeT, TrackRawT) {
    let mut first_frame = u32::max_value();
    let mut last_frame = 0;

    for t in &track.boxes {
        if t.frame < first_frame {
            first_frame = t.frame;
        }

        if t.frame > last_frame {
            last_frame = t.frame;
        }
    }

    let mut new_vol: ImageVolume = ImageVolume {
        0: vec![]
    };
    let mut new_boxes: Vec<FrameBoxRaw> = vec![];

    for idx in first_frame..last_frame+1 {
        new_vol.0.push(vol.volume.0[idx as usize].clone());
    }

    for bbox in &track.boxes {
        let new_fb :FrameBoxRaw = FrameBoxRaw {
            frame: bbox.frame - first_frame,
            bbox: bbox.bbox.clone()
        };
        new_boxes.push(new_fb);
    }  

    (VolumeT {
        volume: new_vol,
        extents: vol.extents, // Assumes there is at least one in the vol!
        origin: vol.origin.clone(),
    },

    TrackRawT {
        boxes: new_boxes,
        origin: track.origin.clone()
    })
}


/// Convert a GroupT to an image VolumeT
/// 
/// * `group` - the GroupT to convert.
/// * `image_path_cache` - the cache of FITS paths.
pub fn node_group_to_volume(
    group: &GroupT,
    image_path_cache: &HashMap<String, PathBuf>,
) -> Option<VolumeT> {
    //! Given a GroupT, get all the images and output a VolumeT
    let mut final_img = ImageVolume(vec![]);
    let num_frames = group.images.len();
    let bp: Luma<u8> = Luma([0]);

    for _i in 0..num_frames {
        final_img.0.push(GrayImage::from_pixel(
            group.origin.crop_size.width as u32, // Origin img_sizes are already cropped!
            group.origin.crop_size.height as u32,
            bp,
        ));
    }

    for _i in 0..num_frames {
        let image = &group.images[_i as usize];
        let image_path = image_path_cache.get(&image.filename).unwrap();
        let mut img_data: ImageBuffer<Luma<u8>, Vec<u8>> = read_fits(&image_path).unwrap();

        // Crop to height to remove all the variability in the images. Reject if any are too small
        if img_data.height() < group.origin.crop_size.height {
            return None
        }

        img_data = crop(&mut img_data, 0, 0, group.origin.crop_size.width, group.origin.crop_size.height).to_image();
        final_img.0[_i] = img_data;
    }

    Some(VolumeT::new(final_img, Option::Some(group.origin.clone())))
}


/// Resize an image volume
/// 
/// * `volume` - the VolumeT to resize.
/// * `width` - the width to resize to. Height is calculate so the ratio is maintained.
/// * `filter` - which filter to use (Lanczos3 or similar).
pub fn node_volume_resize(volume: &VolumeT, width: u32, filter: FilterType) -> VolumeT {
    // Resize, starting with the extents this volume goes over.
    let ratio = volume.extents.2 as f32 / width as f32;
    let nx = (volume.extents.0 as f32 * ratio) as u32;
    let ny = (volume.extents.1 as f32 * ratio) as u32;
    let nw = width;
    let nh = (volume.extents.3 as f32 * ratio) as u32;

    let mut new_volume = VolumeT {
        origin: volume.origin.clone(),
        extents: (nx, ny, nw, nh),
        volume: ImageVolume(vec![]),
    };

    for frame in &volume.volume.0 {
        let height = (frame.height() as f32 *(width as f32 / frame.width() as f32)) as u32;

        if frame.width() != width || frame.height() != height {
            let img_data = resize(&frame.clone(), width, height, filter);
            new_volume.volume.0.push(img_data);
        }
    }
    new_volume
}


/// Crop a volume in width and/or height.
/// 
/// * `volume` - the VolumeT to crop.
/// * `x` - starting x position for the crop.
/// * `y` - starting y position for the crop.
/// * `width` - the width to crop to.
/// * `height` - the height to crop to.
pub fn node_volume_crop(volume: &VolumeT, x: u32, y: u32, width: u32, height: u32) -> VolumeT {
    // Extend the extents as the extents must always reflect the origin.
    // So a crop of a crop still has extents relative to the original image.
    let nx = volume.extents.0 + x;
    let ny = volume.extents.1 + y;
    let nw = width;
    let nh = height;

    let mut new_volume = VolumeT {
        origin: volume.origin.clone(),
        extents: (nx, ny, nw, nh),
        volume: ImageVolume(vec![]),
    };

    for frame in &volume.volume.0 {
        let img_data = crop(&mut frame.clone(), x, y, width, height).to_image();
        new_volume.volume.0.push(img_data);
    }
    new_volume
}


/// Crop a volume in width and/or height but to the nearest amount of sector_size
/// 
/// * `volume` - the VolumeT to crop.
/// * `sector_size` - the size of the sectors we want (e.g 64 pixels).
pub fn node_volume_crop_sector(volume: &VolumeT, sector_size: u32) -> VolumeT {
    // Crop to the nearest power of two.
    let nx = volume.extents.0;
    let ny = volume.extents.1;

    let mut width = 0;
    while width + sector_size <= volume.width() as u32 {
        width = width + sector_size;
    } 

    let mut height = 0;
    while height + sector_size <= volume.height() as u32 {
        height = height + sector_size;
    }
    
    let nw = width as u32;
    let nh = height as u32;

    let mut new_volume = VolumeT {
        origin: volume.origin.clone(),
        extents: (nx, ny, nw, nh),
        volume: ImageVolume(vec![]),
    };

    for frame in &volume.volume.0 {
        let img_data = crop(&mut frame.clone(), 0, 0, nw as u32, nh as u32).to_image();
        new_volume.volume.0.push(img_data);
    }
    new_volume
}


/// Split a volume into smaller, overlapping volumes.
/// 
/// * `volume` - the VolumeT to split.
pub fn node_volume_split(volume: &VolumeT) -> Vec<VolumeT> {
    // Split a volume into smaller volumes, with some level of overlap.
    // To keep things simple, we go with a 2 x 3 split.
    let mut new_vols: Vec<VolumeT> = vec![];
    let w: i32 = (volume.extents.2 as f32 / 2.0 + 64.0) as i32;
    let h: i32 = (volume.extents.3 as f32 / 3.0 + 64.0) as i32;

    for i in 0..2 {
        for j in 0..3 {
            let nx: i32 = (((i * (volume.extents.2 as f32 / 2.0) as i32)) - 32).max(0);
            let ny: i32 = (((j * (volume.extents.3 as f32 / 3.0) as i32)) - 32).max(0);
            let mut nw = w;

            if  nx + nw >= volume.extents.2 as i32{
                nw = volume.extents.2 as i32 - nx - 1;
            }

            let mut nh = h;

            if  ny + nh >= volume.extents.3 as i32 {
                nh = volume.extents.3 as i32 - ny - 1;
            }

            assert!(nx >= 0);
            assert!(ny >= 0);
            assert!(nw > 0);
            assert!(nh > 0);

            // Now cut the volume
            let mut new_volume = VolumeT {
                origin: volume.origin.clone(),
                extents: (nx as u32, ny as u32, nw as u32, nh as u32),
                volume: ImageVolume(vec![]),
            };
        
            for frame in &volume.volume.0 {
                let mut fcopy = frame.clone();
                let img_data = crop(&mut fcopy, nx as u32, ny as u32, nw as u32, nh as u32);
                new_volume.volume.0.push(img_data.to_image());
            }

            new_vols.push(new_volume);
        }
    }

    new_vols
}


/// Split a volume into smaller, overlapping volumes with random placement.
/// 
/// * `volume_base` - the Image VolumeT to split.
/// * `volume_mask` - the corresponding Track/ Mask VolumeT to split.
/// * `split_size` - The dimension of these square volumes.
pub fn node_volume_split_random(volume_base: &VolumeT, volume_mask: &VolumeT, split_size: i32) -> Vec<DatumT> {
    // Split a volume into smaller volumes, but with a fixed size, randomly.
    // We choose via a poisson sampling.
    let mut new_vols: Vec<DatumT> = vec![];
    
    // Between 0 and 1
    let bw = volume_base.volume.0[0].width() as i32 - split_size;
    let bh = volume_base.volume.0[0].height() as i32 - split_size;

    let mut rng = rand::thread_rng();
   
    let mut pp: Vec<[f64; 2]> = vec![];

    for _ in 0..128 {
        let x: f64 = rng.gen();
        let y: f64 = rng.gen();
        pp.push([x, y]);
    }

    for p in pp {
        let mut nx: i32 = (p[0] * bw as f64) as i32;
        let mut ny: i32 = (p[1] * bh as f64) as i32;
       
        if  nx + split_size >= volume_base.extents.2 as i32 {
            nx = nx - (nx + split_size - volume_base.extents.2 as i32);
        }

        if  ny + split_size >= volume_base.extents.3 as i32 {
            ny = ny - (ny + split_size - volume_base.extents.3 as i32);
        }

        //println!("Box {},{} {},{}", nx, ny, nx + split_size, ny + split_size);

        // Now cut the volumes
        let mut new_base = VolumeT {
            origin: volume_base.origin.clone(),
            extents: (nx as u32, ny as u32, split_size as u32, split_size as u32),
            volume: ImageVolume(vec![]),
        };
    
        for frame in &volume_base.volume.0 {
            let mut fcopy = frame.clone();
            let img_data = crop(&mut fcopy, nx as u32, ny as u32, split_size as u32, split_size as u32);
            new_base.volume.0.push(img_data.to_image());
        }

        let mut new_mask = VolumeT {
            origin: volume_mask.origin.clone(),
            extents: (nx as u32, ny as u32, split_size as u32, split_size as u32),
            volume: ImageVolume(vec![]),
        };
    
        for frame in &volume_mask.volume.0 {
            let mut fcopy = frame.clone();
            let img_data = crop(&mut fcopy, nx as u32, ny as u32, split_size as u32, split_size as u32);
            new_mask.volume.0.push(img_data.to_image());
        }

        assert!(new_base.volume.0[0].width() == new_mask.volume.0[0].width());
        assert!(new_base.volume.0[0].height() == new_mask.volume.0[0].height());
        let d = DatumT::new(&new_base, &new_mask);

        new_vols.push(d);
    }

    new_vols
}


/// Convert a TrackRawT to a VolumeT
/// 
/// * `track` - the TrackRawT to convert.
/// * `group` - the corresponding GroupT.
pub fn node_trackraw_to_volume(track: &TrackRawT, group: &GroupT) -> VolumeT {
    // Convert a track to our image mask
    // If multiclass, export the class into the volume, else just write a 1.0.
    let mut final_mask = ImageVolume(vec![]);
    let bp: Luma<u8> = Luma([0]);

    for _i in 0..group.images.len() {
        let mask = GrayImage::from_pixel(
            group.origin.crop_size.width as u32,
            group.origin.crop_size.height as u32,
            bp,
        );
        final_mask.0.push(mask);
    }

    let corigin = track.origin.clone();
    let classid = corigin.unwrap().classid;
    let wp: Luma<u8> = Luma([classid]);

    for _i in 0..track.boxes.len() {
        let fidx = track.boxes[_i].frame as usize;
        // Set the area in the bounding box to full white.
        let mask = &mut final_mask.0[fidx];

        let tbox = track.boxes[_i].bbox;
        for x in tbox.x_min..tbox.x_max {
            for y in tbox.y_min..tbox.y_max {
                // Check that the box, which is in img_size, does not exceed crop_size
                if x < group.origin.crop_size.width as i32 && y < group.origin.crop_size.height as i32 {
                    mask.put_pixel(x as u32, y as u32, wp);
                }
            }
        }
    }

    VolumeT::new(final_mask, Option::Some(group.origin.clone()))
}


/// Split a volume into smaller, overlapping volumes with random placement.
/// 
/// * `track` - the TrackRawT to convert
/// * `group` - the corresponding GroupT object.
/// * `sector_size` - The dimension of the square sector.
pub fn node_trackraw_to_sectors(track: &TrackRawT, group: &GroupT, sector_size: u32) -> VolumeT {
    // Convert a track to our image mask but sectored - so more pixelated in effect.
    // We don't pixelate in the time dimension though.
    // We look at the base and try to match that as best as we can.
    // sector_size is related to the original image size as that is the space the tracks are in.
    // Base image height is the height of all our images before any rescaling but before the intitial crop
    let mut final_mask = ImageVolume(vec![]);

    // We use a value equal to the original class of this group
    // TODO - we should check this corigin unwrap!
    let corigin = track.origin.clone();
    let bp: Luma<u8> = Luma([0]);
    let wp: Luma<u8> = Luma([corigin.unwrap().classid]);

    let iwidth = group.origin.crop_size.width as u32;
    let iheight = group.origin.crop_size.height as u32;

    // Scale down and move the extents if necessary. Cropping the matching node seems alright
    let nums_width = iwidth / sector_size;
    let nums_height = iheight / sector_size;
    let new_extents = (0, 0, nums_width * sector_size, nums_height * sector_size);

    for _i in 0..group.images.len() {
        let mask = GrayImage::from_pixel(
            nums_width as u32,
            nums_height as u32,
            bp,
        );
        final_mask.0.push(mask);
    }
    
    // TODO - could probably just do this directly without intervening sectors
    let mut sectors : Vec<Vec<Vec<u8>>> = vec![];
    
    for _d in 0..group.images.len() {
        let mut sector : Vec<Vec<u8>> = vec![];
        
        for _ in 0..nums_height {
            let mut row : Vec<u8> = vec![];

            for _ in 0..nums_width {
                row.push(0);
            }

            sector.push(row);
        }
        sectors.push(sector);
    }

    for i in 0..track.boxes.len() {
        let d = track.boxes[i].frame as usize;
        let tbox = track.boxes[i].bbox;

        for y in tbox.y_min..tbox.y_max {
            for x in tbox.x_min..tbox.x_max {
                // Convert to sector. Bit wasteful as we can go step size, not every pixel.
                let sx= ((x as f32 / iwidth as f32) * nums_width as f32).floor() as usize;
                let sy= ((y as f32 / iheight as f32) * nums_height as f32).floor() as usize;
                if sy < nums_height as usize && sx < nums_width as usize {
                    sectors[d][sy][sx] = 1;
                }
            }
        }
    }

    // Now we have a sector map, lets translate that to our pixelated image.
    // TODO - might be a better way of doing this more simply? Nearest neighbour resize?
    for d in 0..group.images.len() {
        let mask: &mut ImageBuffer<Luma<u8>, Vec<u8>> = &mut final_mask.0[d];

        for y in 0..nums_height as usize {
            for x in 0..nums_width as usize {
                if sectors[d][y][x] == 1 {
                    mask.put_pixel(x as u32, y as u32, wp);
                }
            }
        }
    }
   
    let mut fvol = VolumeT::new(final_mask, Option::Some(group.origin.clone()));
    fvol.extents = new_extents;
    fvol
}