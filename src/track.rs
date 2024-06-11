//! Functions that deal with tracks.
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
 *   tracks.rs - functions dealing with tracks
 *   Author - bjb8@st-andrews.ac.uk
 *   
 *   uses https://github.com/OpenRR/trajectory to interpolate the tracks
 */

use crate::{bbs::{distance_rawbox, overlap_rawbox, Expand, FrameBox, FrameBoxRaw, RawBox, XYBox}, image::ImageSize};
use enterpolation::{linear::Linear, Generator};
use similari::utils::bbox::{BoundingBox, Universal2DBox};
use similari::utils::kalman::kalman_2d_box::Universal2DBoxKalmanFilter;


/// A Three dimensional bounding box.
pub struct BBThree {
    min_x: i32,
    min_y: i32,
    min_z: i32,
    max_x: i32,
    max_y: i32,
    max_z: i32
}

pub trait AsThree {
    fn frames_to_3d(&self) -> BBThree;
}

impl AsThree for &Vec<FrameBoxRaw> {

    /// Convert a Vector of FrameBoxRaw to a BBThree
    fn frames_to_3d(&self) -> BBThree {
        let mut bb  = BBThree {
            min_x: i32::MAX,
            min_y: i32::MAX,
            min_z: i32::MAX,
            max_x: 0,
            max_y: 0,
            max_z: 0,
        };
    
        for frame in *self {
            if frame.bbox.x_min < bb.min_x { bb.min_x = frame.bbox.x_min; }
            if frame.bbox.y_min < bb.min_y { bb.min_y = frame.bbox.y_min; }
            if (frame.frame as i32) < bb.min_z { bb.min_z = frame.frame as i32; }
    
            if frame.bbox.x_max > bb.max_x { bb.max_x = frame.bbox.x_max; }
            if frame.bbox.y_max > bb.max_y { bb.max_y = frame.bbox.y_max; }
            if (frame.frame as i32) > bb.max_z { bb.min_z = frame.frame as i32; }
        }
    
        bb
    }
}

// TODO - this seems to be a duplication. Can we improve?
impl AsThree for Vec<FrameBox> {

    /// Convert a Vector of Framebox to BBThree
    fn frames_to_3d(&self) -> BBThree {
        let mut bb  = BBThree {
            min_x: i32::MAX,
            min_y: i32::MAX,
            min_z: i32::MAX,
            max_x: 0,
            max_y: 0,
            max_z: 0,
        };
    
        for frame in self {
            if frame.bbox.x_min < bb.min_x { bb.min_x = frame.bbox.x_min; }
            if frame.bbox.y_min < bb.min_y { bb.min_y = frame.bbox.y_min; }
            if (frame.frame as i32) < bb.min_z { bb.min_z = frame.frame as i32; }
    
            if frame.bbox.x_max > bb.max_x { bb.max_x = frame.bbox.x_max; }
            if frame.bbox.y_max > bb.max_y { bb.max_y = frame.bbox.y_max; }
            if (frame.frame as i32) > bb.max_z { bb.min_z = frame.frame as i32; }
        }
    
        bb
    }
}


/// Given a track with missing frames, interpolate the track up to the num_frames amount
/// or the maximum available frames - whichever is smaller. Each frame is expanded by esize.
/// 
/// * `frames` - a vector of FrameBoxRaw to interpolate.
/// * `img_size` - the size of image we are working within.
pub fn interpolate_track_raw(frames: &Vec<FrameBoxRaw>, img_size: &ImageSize) -> Vec<FrameBoxRaw> {
   
    let mut interped : Vec<FrameBoxRaw> = vec![];
    let _threedee = frames.frames_to_3d();
    
    /*let mut expd = XYBox{
        x_min: threedee.min_x as i32,
        y_min: threedee.min_y as i32,
        x_max: threedee.max_x as i32,
        y_max: threedee.max_y as i32
    };
    
    expd.expand(img_size, esize);
    threedee.min_x = expd.x_min as i32;
    threedee.min_y = expd.y_min as i32;
    threedee.max_x = expd.x_max as i32;
    threedee.max_y = expd.y_max as i32;*/

    let mut minf = u32::MAX;
    let mut maxf = 0;

    // Find all the min and max values, place in vectors
    // so we can easily interpolate between them
    let mut times: Vec<f32> = vec![];
    let mut tl_x: Vec<f32> = vec![];
    let mut tl_y: Vec<f32> = vec![];
    let mut tr_x: Vec<f32> = vec![];
    let mut tr_y: Vec<f32> = vec![];
    let mut bl_x: Vec<f32> = vec![];
    let mut bl_y: Vec<f32> = vec![];
    let mut br_x: Vec<f32> = vec![];
    let mut br_y: Vec<f32> = vec![];

    // Now go through the original frames and create the new ones
    for frame in frames {
        let nbb = frame.bbox.clone();
        // nbb.expand(img_size, esize);
        times.push(frame.frame as f32);
        bl_x.push(nbb.x_min as f32);
        bl_y.push(nbb.y_min as f32);
        br_x.push(nbb.x_max as f32);
        br_y.push(nbb.y_min as f32);
        tl_x.push(nbb.x_min as f32);
        tl_y.push(nbb.y_max as f32);
        tr_x.push(nbb.x_max as f32);
        tr_y.push(nbb.y_max as f32);

        if frame.frame > maxf {
            maxf = frame.frame
        }

        if frame.frame < minf {
            minf = frame.frame
        }
    }

    let lin_bl_x = Linear::builder().elements(bl_x.clone()).knots(times.clone()).build().unwrap();
    let lin_bl_y = Linear::builder().elements(bl_y).knots(times.clone()).build().unwrap();
    let lin_br_x = Linear::builder().elements(br_x).knots(times.clone()).build().unwrap();
    let lin_br_y = Linear::builder().elements(br_y).knots(times.clone()).build().unwrap();
    let lin_tl_x = Linear::builder().elements(tl_x).knots(times.clone()).build().unwrap();
    let lin_tl_y = Linear::builder().elements(tl_y).knots(times.clone()).build().unwrap();
    let lin_tr_x = Linear::builder().elements(tr_x).knots(times.clone()).build().unwrap();
    let lin_tr_y = Linear::builder().elements(tr_y).knots(times.clone()).build().unwrap();

    // We've setup our vectors, so let's do the interpolation
    for i in minf..maxf+1 {

        let nbl_x = lin_bl_x.sample([i as f32]).last().unwrap();
        let nbl_y = lin_bl_y.sample([i as f32]).last().unwrap();
        let nbr_x = lin_br_x.sample([i as f32]).last().unwrap();
        let nbr_y = lin_br_y.sample([i as f32]).last().unwrap();

        let ntl_x = lin_tl_x.sample([i as f32]).last().unwrap();
        let ntl_y = lin_tl_y.sample([i as f32]).last().unwrap();

        let ntr_x = lin_tr_x.sample([i as f32]).last().unwrap();
        let ntr_y = lin_tr_y.sample([i as f32]).last().unwrap();

        let mut nxmin = ((nbl_x + ntl_x) / 2.0).round() as i32;
        let mut nymin = ((nbl_y + nbr_y) / 2.0).round() as i32;
        let mut nxmax = ((nbr_x + ntr_x) / 2.0).round() as i32;
        let mut nymax = ((ntl_y + ntr_y) / 2.0).round() as i32;

        // Checks to make sure the interp hasn't swapped
        if nxmin > nxmax {
            let t = nxmin;
            nxmin = nxmax;
            nxmax = t;
        }

        if nymin > nymax {
            let t = nymin;
            nymin = nymax;
            nymax = t;
        }

        if nxmin < 0 { nxmin = 0; }
        if nymin < 0 { nymin = 0; }
        if nxmax >= img_size.width as i32 { nxmax = img_size.width as i32 - 1; }
        if nymax >= img_size.height as i32 { nymax = img_size.height as i32 -1; }

        let nbb = RawBox{x_min: nxmin , y_min: nymin, x_max: nxmax, y_max: nymax};
        
        let nframe = FrameBoxRaw {
            frame: i as u32, // We stick with the original index
            bbox: nbb
        };

        interped.push(nframe);
    }
    
    interped

}


/// Given a track with missing frames, interpolate the track up to the num_frames amount
/// or the maximum available frames - whichever is smaller. Each frame is expanded by esize.
/// 
/// * `frames` - a vector of FrameBoxRaw to interpolate.
/// * `img_size` - the size of image we are working within.
pub fn interpolate_track(frames: &Vec<FrameBox>, img_size: &ImageSize, esize: i32) -> Vec<FrameBox> {
    let mut interped : Vec<FrameBox> = vec![];
    let mut threedee = interped.frames_to_3d();
    let mut expd = XYBox{
        x_min: threedee.min_x as i32,
        y_min: threedee.min_y as i32,
        x_max: threedee.max_x as i32,
        y_max: threedee.max_y as i32
    };
    
    expd.expand_equal(img_size, esize);
    threedee.min_x = expd.x_min as i32;
    threedee.min_y = expd.y_min as i32;
    threedee.max_x = expd.x_max as i32;
    threedee.max_y = expd.y_max as i32;

    let mut minf = u32::MAX;
    let mut maxf = 0;

    // Find all the min and max values, place in vectors
    // so we can easily interpolate between them

    let mut times: Vec<f32> = vec![];
    let mut tl_x: Vec<f32> = vec![];
    let mut tl_y: Vec<f32> = vec![];
    let mut tr_x: Vec<f32> = vec![];
    let mut tr_y: Vec<f32> = vec![];
    let mut bl_x: Vec<f32> = vec![];
    let mut bl_y: Vec<f32> = vec![];
    let mut br_x: Vec<f32> = vec![];
    let mut br_y: Vec<f32> = vec![];

    // Now go through the original frames and create the new ones
    for frame in frames {
        let mut nbb = frame.bbox.clone();
        nbb.expand_equal(img_size, esize);
        times.push(frame.frame as f32);
        bl_x.push(nbb.x_min as f32);
        bl_y.push(nbb.y_min as f32);
        br_x.push(nbb.x_max as f32);
        br_y.push(nbb.y_min as f32);
        tl_x.push(nbb.x_min as f32);
        tl_y.push(nbb.y_max as f32);
        tr_x.push(nbb.x_max as f32);
        tr_y.push(nbb.y_max as f32);

        if frame.frame > maxf {
            maxf = frame.frame
        }

        if frame.frame < minf {
            minf = frame.frame
        }
    }

    let lin_bl_x = Linear::builder().elements(bl_x.clone()).knots(times.clone()).build().unwrap();
    let lin_bl_y = Linear::builder().elements(bl_y).knots(times.clone()).build().unwrap();
    let lin_br_x = Linear::builder().elements(br_x).knots(times.clone()).build().unwrap();
    let lin_br_y = Linear::builder().elements(br_y).knots(times.clone()).build().unwrap();
    let lin_tl_x = Linear::builder().elements(tl_x).knots(times.clone()).build().unwrap();
    let lin_tl_y = Linear::builder().elements(tl_y).knots(times.clone()).build().unwrap();
    let lin_tr_x = Linear::builder().elements(tr_x).knots(times.clone()).build().unwrap();
    let lin_tr_y = Linear::builder().elements(tr_y).knots(times.clone()).build().unwrap();

    // We've setup our vectors, so let's do the interpolation
    for i in minf..maxf+1 {

        let nbl_x = lin_bl_x.sample([i as f32]).last().unwrap();
        let nbl_y = lin_bl_y.sample([i as f32]).last().unwrap();
        let nbr_x = lin_br_x.sample([i as f32]).last().unwrap();
        let nbr_y = lin_br_y.sample([i as f32]).last().unwrap();

        let ntl_x = lin_tl_x.sample([i as f32]).last().unwrap();
        let ntl_y = lin_tl_y.sample([i as f32]).last().unwrap();

        let ntr_x = lin_tr_x.sample([i as f32]).last().unwrap();
        let ntr_y = lin_tr_y.sample([i as f32]).last().unwrap();

        let mut nxmin = ((nbl_x + ntl_x) / 2.0).round() as i32;
        let mut nymin = ((nbl_y + nbr_y) / 2.0).round() as i32;
        let mut nxmax = ((nbr_x + ntr_x) / 2.0).round() as i32;
        let mut nymax = ((ntl_y + ntr_y) / 2.0).round() as i32;

        // Checks to make sure the interp hasn't swapped
        if nxmin > nxmax {
            let t = nxmin;
            nxmin = nxmax;
            nxmax = t;
        }

        if nymin > nymax {
            let t = nymin;
            nymin = nymax;
            nymax = t;
        }

        if nxmin < 0 { nxmin = 0; }
        if nymin < 0 { nymin = 0; }
        if nxmax >= img_size.width as i32 { nxmax = img_size.width as i32 - 1; }
        if nymax >= img_size.height as i32 { nymax = img_size.height as i32 -1; }

        let nbb = XYBox{x_min: nxmin , y_min: nymin, x_max: nxmax, y_max: nymax};
        
        let nframe = FrameBox {
            frame: i as u32, // We stick with the original index
            bbox: nbb
        };

        interped.push(nframe);
    }
    
    interped
}


/// Make sure there is only one box per frame. Expand if need be.
/// 
/// * `frames` - a vector of FrameBoxRaw to modify.
/// * `img_size` - the size of image we are working within.
fn one_frame_one_box(frames: &Vec<FrameBoxRaw>, _img_size: &ImageSize) -> (Vec<Option<RawBox>>, Vec<u32>){
    let mut box_by_frame: Vec<Vec<RawBox>> = vec![];
    let mut box_by_frame_final: Vec<Option<RawBox>> = vec![];
    let mut frame_numbers: Vec<u32> = vec![];
    
    for frame in frames {
        if !frame_numbers.contains(&(frame.frame)) {
            frame_numbers.push(frame.frame);
        } 
    }

    frame_numbers.sort();

    // Now we have the start and end frames, lets add a vector to hold each box
    for i in &frame_numbers {
        let mut nv: Vec<RawBox> = vec![];
        
        for frame in frames {
            if frame.frame == *i {
                nv.push(frame.bbox.clone());
            }
        }

        box_by_frame.push(nv);
    }

    // So we now have the boxes in the right spot. Lets start by expanding any boxes on the same frame
    // We use a cheap method - find the min max and set. Should already be the case but check anyway
    for idx in 0..box_by_frame.len() {
        let boxes = &box_by_frame[idx];

        if boxes.len() > 1 {
            let mut nbox: RawBox = boxes[0].clone();

            for bbox in boxes {
                if bbox.x_min < nbox.x_min { nbox.x_min = bbox.x_min;}
                if bbox.x_max > nbox.x_max { nbox.x_max = bbox.x_max;}
                if bbox.y_min < nbox.y_min { nbox.y_min = bbox.y_min;}
                if bbox.y_max > nbox.y_max { nbox.y_max = bbox.y_max;}
            } 

            box_by_frame_final.push(Some(nbox));
        
        } else if boxes.len() == 1 {
            box_by_frame_final.push(Some(boxes[0]));
        } else {
            box_by_frame_final.push(None);
        }
    } 

    (box_by_frame_final, frame_numbers)
}


/// Neighbouring frames must overlap. Assumes there is a neighbour. Best run after interpolation
/// We take the smallest of the two and extend it, until it overlaps with the previous.
/// For now we assume all boxes on a single frame are part of the same track and they should
/// overlap too!
/// 
/// * `frames` - a vector of FrameBoxRaw to modify.
/// * `img_size` - the size of image we are working within.
pub fn overlap_track_raw(frames: &Vec<FrameBoxRaw>, img_size: &ImageSize) -> Vec<FrameBoxRaw> {
    let (mut box_by_frame_final, frame_numbers) = one_frame_one_box(frames, img_size);
    
    // There should now be a single box per frame. So now lets move through
    // and make sure we have an overlap. We ignore any gaps (interpolate should
    // be called before this if gaps are meant to be avoided). This is acceptable
    // behaviour.

    for idx in 0..box_by_frame_final.len()-1 {
        let tcbox = box_by_frame_final[idx];
        let tnbox = box_by_frame_final[idx+1];

        match tcbox {
            Some(mut cbox) => {
                match tnbox {
                    Some(mut nbox) => {
                        if !overlap_rawbox(&cbox, &nbox) {
                            let (xd, yd) = distance_rawbox(&cbox, &nbox);
                            cbox.expand(img_size, (xd / 2 + 1).max(0) as i32, (yd as i32 / 2 + 1).max(0));
                            let _ = box_by_frame_final[idx].insert(cbox);
                        
                            nbox.expand(img_size, (xd as i32 / 2 + 1).max(0), (yd as i32 / 2 + 1).max(0) );
                            let _ = box_by_frame_final[idx+1].insert(nbox);
                        
                        }
                    },
                    None =>  {}
                }
            },
            None => {}
        }
    }

    let mut new_track:  Vec<FrameBoxRaw> = vec![];

    // Final boxes we now convert back to new_track
    for idx in 0..box_by_frame_final.len() {
        let obbox = box_by_frame_final[idx];
        if obbox.is_some() {
            let nbox = obbox.unwrap(); 
            new_track.push(FrameBoxRaw {
                frame: frame_numbers[0] + idx as u32,
                bbox: nbox
            })
        }
    }
    new_track
}


/// Firstly, we must make sure there is only ONE bbox per frame.
/// This will be a problem later I think when we want to track multiple tracks
/// as the db will be fine but in a single image we may have multiple tracks.
/// overlap too!
/// 
/// * `frames` - a vector of FrameBoxRaw to modify.
/// * `img_size` - the size of image we are working within.
pub fn smooth_track(frames: &Vec<FrameBoxRaw>, img_size: &ImageSize) -> Vec<FrameBoxRaw> {
    let (mut box_by_frame, frame_numbers) = one_frame_one_box(frames, img_size);

    let f = Universal2DBoxKalmanFilter::default();
    let mut start: usize = 0;
    let mut left = 0;
    let mut top = 0;
    let mut width = 0;
    let mut height = 0;

    // TODO - need an assert / check or return if the frames are all empty of boxes. Shouldn't happen but you never know.

    for idx in 0..box_by_frame.len() {
        let pbox = box_by_frame[idx];

        if pbox.is_some() {
            let ubox = pbox.unwrap();
            left = ubox.x_min;
            top = ubox.y_min;
            width = (ubox.x_max - left).max(3);
            height = (ubox.y_max - top).max(3);
            start = idx;
            break;
        }
    }

    // Check to make sure start frame is legit.
    if width < 3 || height < 3 {
        // Failed, so return the basic
        return frames.clone()
    }

    start = start + 1; // Don't recount the first box we've found.
    let mut bbox_kal = BoundingBox::new(left as f32, top as f32, width as f32, height as f32);
    let mut state = f.initiate(&bbox_kal.into());

    for idx in start..box_by_frame.len() {
        // Run a Kalman filter over a track in order to smooth it out.
        // We also predict regardless of whether there is a box around at all.
        let pbox = box_by_frame[idx];

        if pbox.is_some() {
            // Create a new kalman library box
            let ubox: RawBox = pbox.unwrap();
            left = ubox.x_min.max(0);
            top = ubox.y_min.max(0);
            width = (ubox.x_max - left).max(0);
            height = (ubox.y_max - top).max(0);

            bbox_kal = BoundingBox::new(left as f32, top as f32, width as f32, height as f32);
            state = f.update(&state, &bbox_kal.into());
        }

        state = f.predict(&state);
        let pred_uni_box = Universal2DBox::try_from(state).unwrap();
        
        let h = pred_uni_box.height;
        let w: f32 = h * pred_uni_box.aspect;
        let new_box = RawBox {
            x_min: (pred_uni_box.xc as i32 - ((w/2.0) as i32)).max(0),
            x_max: (pred_uni_box.xc as i32 + ((w/2.0) as i32)).max(0).min(img_size.width as i32 - 1),
            y_min: (pred_uni_box.yc as i32 - ((h/2.0) as i32)).max(0),
            y_max: (pred_uni_box.yc as i32 + ((h/2.0) as i32)).max(0).min(img_size.height as i32 - 1),
        };

        // println!("NEW Box {},{} {},{}", new_box.x_min, new_box.x_max, new_box.y_min, new_box.y_max);
        let _ = box_by_frame[idx].insert(new_box);
    }

    let mut new_track:  Vec<FrameBoxRaw> = vec![];

    // Final boxes we now convert back to new_track
    for idx in 0..box_by_frame.len() {
        let obbox = box_by_frame[idx];
        if obbox.is_some() {
            let nbox = obbox.unwrap(); 
            new_track.push(FrameBoxRaw {
                frame: frame_numbers[0] + idx as u32,
                bbox: nbox
            })
        }
    }
    new_track

}


// *** TESTS ***
#[cfg(test)]
mod tests {

    use super::*;
    use crate::bbs::{XYBox, FrameBox};

    #[test]
    fn test_interp_track() {
        // Setup some boxes then interp
        let b0 = XYBox {
            x_min: 0,
            y_min: 0,
            x_max: 10,
            y_max: 10,
        };

        let b1 = XYBox {
            x_min: 30,
            y_min: 20,
            x_max: 50,
            y_max: 30,
        };

        let b2 = XYBox {
            x_min: 100,
            y_min: 100,
            x_max: 150,
            y_max: 110,
        };

        let f0 = FrameBox {
            frame: 2,
            bbox: b0,
        };

        let f1 = FrameBox {
            frame: 4,
            bbox: b1,
        };

        let f2 = FrameBox {
            frame: 10,
            bbox: b2,
        };

        let frames = vec![f0, f1, f2];
        let img_size = &ImageSize { width: 700, height: 400 };
        let interped = interpolate_track(&frames, img_size, 0);

        assert!(interped[0].bbox.x_min == 0);
        assert!(interped[7].bbox.x_min > 30 && interped[7].bbox.x_min <= 120);
        assert!(interped[4].bbox.x_min > 30 && interped[4].bbox.x_min <= 80);
    }

    #[test]
    fn test_overlap_track() {
        let img_size = ImageSize {
            width: 400,
            height: 400
        };

        let b0 = RawBox {
            x_min: 0,
            y_min: 0,
            x_max: 10,
            y_max: 10,
        };

        let b1 = RawBox {
            x_min: 5,
            y_min: 5,
            x_max: 50,
            y_max: 30,
        };

        let b2 = RawBox {
            x_min: 30,
            y_min: 40,
            x_max: 50,
            y_max: 45,
        };

        let f0 = FrameBoxRaw {
            frame: 2,
            bbox: b0,
        };

        let f1 = FrameBoxRaw {
            frame: 2,
            bbox: b1,
        };

        let f2 = FrameBoxRaw {
            frame: 3,
            bbox: b2,
        };

        let frames = vec![f0, f1, f2];
        let new_frames = overlap_track_raw(&frames, &img_size);
        assert!(overlap_rawbox(&b0, &b1));
        assert!(new_frames.len() == 2);
        assert!(new_frames[0].frame == 2);
        assert!(new_frames[1].frame == 3);
        assert!(new_frames[0].bbox.x_max > 30);
        println!("B0 {},{} {},{}", new_frames[0].bbox.x_min, new_frames[0].bbox.y_min, new_frames[0].bbox.x_max, new_frames[0].bbox.y_max);
        println!("B1 {},{} {},{}", new_frames[1].bbox.x_min, new_frames[1].bbox.y_min, new_frames[1].bbox.x_max, new_frames[1].bbox.y_max);
        assert!(overlap_rawbox(&new_frames[0].bbox, &new_frames[1].bbox))

    }
}
