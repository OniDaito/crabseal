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
 *   pipes.rs - the types used in our pipeline setup
 *   Author - bjb8@st-andrews.ac.uk
 *   
 */
use crate::image::ImageSize;
use crate::models::{Groups, Images, Points};
use crate::bbs::{FrameBox, FrameBoxRaw};
use crate::image::ImageVolume;
use chrono::{DateTime, Utc};
use image::{ImageBuffer, Luma};
use std::path::PathBuf;


// Start with the basic types ...

/// Data that represents the group and sonar this type originated from.
/// This origin is carried by all subsequent objects as they pass through the pipeline.
#[derive(Clone)]
pub struct OriginT {
    /// The Groups object from the database.
    pub group: Groups,
    /// The Sonar ID of this particular image/group/volume.
    pub sonar_id: i32,
    /// The classid - refers to the code in the original DB.
    pub classid: u8,
    /// The original image size.
    pub img_size: ImageSize,
    /// Actual size after the initial crop.
    pub crop_size: ImageSize,
}

/// A group and its associated images, points and sonarid. Generated from a DB Node. 
#[derive(Clone)]
pub struct GroupT {
    pub origin: OriginT,
    pub images: Vec<Images>,
    pub points: Vec<Vec<Points>>,
}

/// A Volume - a stack of 2D images that represent the sonar over time.
pub struct VolumeT {
    /// A stack of images
    pub volume: ImageVolume,
    /// Does this volume cover all or some of the origin image size? (left, top, width, height).
    pub extents: (u32, u32, u32, u32),
    /// The origin of this object.
    pub origin:Option<OriginT>,
}

/// A Single image - often used for background removal
pub struct ImageT {
    pub image: ImageBuffer<Luma<u8>, Vec<u8>>,
    pub extents: (u32, u32, u32, u32),  
    pub origin:Option<OriginT>,
}

impl VolumeT {
    
    /// Create a new VolumeT
    /// 
    /// * `vol` - the ImageVolume that forms the VolumeT.
    /// * `origin` - an optional OriginT.
    pub fn new (vol: ImageVolume, origin:Option<OriginT>) -> VolumeT {
        let mut extents = (0, 0, 0, 0);

        if origin.is_some() {
            let torigin = origin.clone().unwrap();
            extents.2 = torigin.img_size.width;
            extents.3 = torigin.img_size.height;
        }
    
        VolumeT {
            volume: vol,
            extents: extents, // Assumes there is at least one in the vol!
            origin: origin,
        }
    }
}

pub trait Dimensions {
    fn width(&self) -> usize;
    fn height(&self) -> usize;
    fn depth(&self) -> usize;
}

impl Dimensions for VolumeT {
    /// Return the width of this volume in pixels.
    fn width(&self) -> usize {
        if self.volume.0.len() > 0 {
            return self.volume.0[0].width() as usize
        }
        0
    }

    /// Return the height of this volume in pixels.
    fn height(&self) -> usize {
        if self.volume.0.len() > 0 {
            return self.volume.0[0].height() as usize
        }
        0
    }

    /// Return the depth of this volume in pixels / number of frames.
    fn depth(&self) -> usize {
        self.volume.0.len()
    }
}

pub struct TrackRawT {
    /// A Track in the XY space of an original 'raw' image, with the indices into the volumes
    pub boxes : Vec<FrameBoxRaw>,
    pub origin: Option<OriginT>
  
}

impl TrackRawT {

    /// Create a new TrackRawT object
    /// 
    /// * `boxes` - List of FrameBoxRaw objects
    /// * `origin` - an optional OriginT.
    pub fn new (boxes: Vec<FrameBoxRaw>, origin:Option<OriginT> ) -> TrackRawT{
        TrackRawT {
            boxes: boxes,
            origin: origin,
        }
    }
}

/// A track in the XY space of a polar transformed image,  with the indices into the volumes
pub struct TrackPolarT {
    pub boxes: Vec<FrameBox>,
    pub origin: Option<OriginT>
  
}

/// A blank group produced by looking at a bunch of GLFS and the DB
pub struct BlankGroupT {
    pub glf_file: PathBuf,
    pub time_start: DateTime<Utc>,
    pub time_end: DateTime<Utc>,
}

/// The final result that gets sent to one of the various sets.
#[derive(Clone)]
pub struct DatumT {
    pub raw: ImageVolume,
    pub mask: ImageVolume,
    pub origin: Option<OriginT>,
    pub extents: (u32, u32, u32, u32)
}


impl DatumT {
    
    /// Create a new DatumT object.
    /// 
    /// * `raw` - The raw/base/image VolumeT.
    /// * `mask` - The mask/track VolumeT.
    pub fn new (raw: &VolumeT, mask: &VolumeT ) -> DatumT{
        if raw.origin.is_some() && mask.origin.is_some() {
            let raw_o = raw.origin.clone().unwrap();
            let mask_o = raw.origin.clone().unwrap();

            assert!(raw_o.group.huid == mask_o.group.huid);
            assert!(raw_o.sonar_id == mask_o.sonar_id);
        }

        // Removed for now as we can have asymetric volumes
        /*assert!(raw.extents.0 == mask.extents.0);
        assert!(raw.extents.1 == mask.extents.1);
        assert!(raw.extents.2 == mask.extents.2);
        assert!(raw.extents.3 == mask.extents.3);*/
       
        DatumT {
            raw: raw.volume.clone(),
            mask: mask.volume.clone(),
            origin: raw.origin.clone(),
            extents: mask.extents.clone()
        }
    }
}

/// A Datum that has been sliced into bits.
#[derive(Clone)]
pub struct SlicedDatumT {
    pub slices : Vec<DatumT>
}
