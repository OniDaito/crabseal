//! A file containing all the things we need to deal with bounding boxes
//! of various kinds, such as 2D, 3D, bearing based, pixel based, etc.
//!

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
 *   bbs.rs - bounding box functions
 *   Author - bjb8@st-andrews.ac.uk
 */

use crate::models::Points;
use crate::constants::{MIN_ANGLE, MAX_ANGLE};
use crate::image::ImageSize;

/// A BearBox is defined by min/max bearings and distances. All angles are held as radians.
/// The BearBox represents the original track taken from PAMGuard but includes the range of
/// the sonar at the time this track was made, in order to permit proper conversion to a
/// pixel based XYBox or similar.
#[derive(Copy, Clone)]
pub struct BearBox {
    /// The minimum bearing, in radians.
    pub bearing_min: f32, 
    /// The maxium bearing, in radians. 
    pub bearing_max: f32, 
    /// The minimum distance, in metres.
    pub distance_min: f32,
    /// The maximum distance, in metres.
    pub distance_max: f32,
    /// The range of the sonar in metres.
    pub sonar_range: f32
}

impl BearBox {
    fn new(bearing_min: f32, bearing_max: f32, distance_min: f32, distance_max: f32, sonar_range: f32) -> Option<BearBox> {
        if bearing_min >= -60.0f32.to_radians() && bearing_min <= 60.0f32.to_radians() &&
            bearing_max >= -60.0f32.to_radians() && bearing_max <= 60.0f32.to_radians() {
            Some(BearBox{
                bearing_min: bearing_min,
                bearing_max: bearing_max,
                distance_min: distance_min,
                distance_max: distance_max,
                sonar_range: sonar_range
            })
        } else {
            None
        }
    }
}

/// An XYBox is a 2D bounding box with min/max x and y pixel positions. The origin (0,0) is the top left of the image.
/// This box is typically used in Polar Transformed images.
#[derive(Copy, Clone)]
pub struct XYBox {
    /// Minimum x position in pixels.
    pub x_min: i32,
    /// Minimum y position in pixels.
    pub y_min: i32,
    /// Maximum x position in pixels.
    pub x_max: i32,
    /// Maximum y position in pixels.
    pub y_max: i32, 
}

/// The Raw Box is used with the non-polar, raw rectangle image from the sonar. x refers to the beam
/// (the bearing effectively) and y is the distance. All the values are in pixels.
#[derive(Copy, Clone)]
pub struct RawBox {
    /// Minimum X Pixel coordinate
    pub x_min: i32,
    /// Minimum Y Pixel coordinate
    pub y_min: i32,
    /// Maximum X Pixel coordinate
    pub x_max: i32,
    /// Maximum Y Pixel coordinate
    pub y_max: i32
}

/// FrameBox combines the 2D box with a frame number. This frame number is the frame index within a set of frames. When we
/// combine frames over time, we use this struct.
#[derive(Copy, Clone)]
pub struct FrameBox {
    /// The Frame number
    pub frame: u32,
    /// A bounding box
    pub bbox: XYBox,
}

/// FrameBoxRaw combines the 2D RAW box with a frame number. This frame number is the frame index within a set of frames. When we
/// combine frames over time, we use this struct.
#[derive(Copy, Clone)]
pub struct FrameBoxRaw {
    /// Frame number
    pub frame: u32,
    /// A RAW XYBox
    pub bbox: RawBox,
}

pub trait RefChange {
    fn centre(self) -> (u32, u32);
    fn width(self) -> u32;
    fn height(self) -> u32;
}


/// Convert a distance and bearing to X/Y in the image. Images have a top-left origin, therefore
/// the fan image comes down from the top. Negative bearings are clockwise and therefore values closer 
/// to 0, with anticlockwise, positive bearings having larger X values.
/// Use the function before any image flips, rotations etc.
/// 
/// * `bearing` - the bearing in radians.
/// * `distance` - the distance in metres.
/// * `max_range` - the maximum range in metres (sonar setting).
/// * `image_size` - the size of the image we are dealing with.
pub fn dist_bearing_to_xy(bearing: f32, distance: f32, max_range:f32, image_size: &ImageSize) -> (u32, u32) {
    let d0 = distance / max_range * image_size.height as f32;
    let mut x0 = bearing.abs().sin() * d0;
    let y0 = bearing.abs().cos() * d0;

    let hl = image_size.width as f32 / 2.0;
    
    if bearing < 0.0 { // Depends on whether or not the fan is at the top or rotated.
        x0 = hl - x0;
    } else {
        x0 = hl + x0;
    }

    (x0.round() as u32, y0.round() as u32)
}


/// Convert a bearing bounding box into an XY box.
/// 
/// * `bb` - the bearing box to convert.
/// * `image_size` - the size of the image we are dealing with.
pub fn bear_to_xy(bb: &BearBox, image_size: &ImageSize) -> XYBox {
    let txy : [(u32, u32); 4] = [
        dist_bearing_to_xy(bb.bearing_min, bb.distance_min, bb.sonar_range, image_size),
        dist_bearing_to_xy(bb.bearing_max, bb.distance_max, bb.sonar_range, image_size),
        dist_bearing_to_xy(bb.bearing_min, bb.distance_max, bb.sonar_range, image_size),
        dist_bearing_to_xy(bb.bearing_max, bb.distance_min, bb.sonar_range, image_size),
    ];

    let mut min_x = txy[0].0 as i32;
    let mut min_y = txy[0].1 as i32;
    let mut max_x = txy[0].0 as i32;
    let mut max_y = txy[0].1 as i32;

    for i in 1..4 {
        if (txy[i].0 as i32) < min_x {
            min_x = txy[i].0 as i32;
        }
        if (txy[i].0 as i32) > max_x {
            max_x = txy[i].0 as i32;
        }
        if (txy[i].1 as i32) < min_y {
            min_y = txy[i].1 as i32;
        }
        if (txy[i].1 as i32) > max_y {
            max_y = txy[i].1 as i32;
        }
     }

    XYBox { x_min: min_x, y_min: min_y, x_max: max_x, y_max: max_y }
}


/// Use the function before any image flips, rotations etc.
/// 
/// * `points` - the points to be contained in a bounding box
/// * `sonar_range` - the maximum range in metres (sonar setting).
pub fn points_to_bb(points: &Vec<Points>, sonar_range: f32) -> BearBox {
    //! Find a bearing bounding box that contains all the points.
    let bearing_limits = (MIN_ANGLE.to_radians(), MAX_ANGLE.to_radians());
    let distance_limits = (0.0, sonar_range);
    let mut min_b = bearing_limits.1;
    let mut max_b = bearing_limits.0;
    let mut min_d = distance_limits.1;
    let mut max_d = distance_limits.0;

    // TODO - Minbearing and maxbearing have been swapped here! It works but clearly
    // there is an issue somehere in what min and max actually mean in the PAMGuard
    // database
    for point in points {
        if point.maxbearing < min_b && point.maxbearing >= bearing_limits.0 {
            min_b = point.maxbearing;
        }
        if point.minbearing > max_b && point.minbearing < bearing_limits.1 {
            max_b = point.minbearing;
        }
        if point.minrange < min_d && point.minrange >= distance_limits.0 {
            min_d = point.minrange;
        }
        if point.maxrange > max_d && point.maxrange < distance_limits.1 {
            max_d = point.maxrange;
        }
    }

    BearBox { bearing_min: min_b, bearing_max: max_b, distance_min: min_d, distance_max: max_d, sonar_range}
}

pub trait Expand {
    fn expand_equal(&mut self, img_size: &ImageSize, esize: i32);
    fn expand(&mut self, img_size: &ImageSize, wsize: i32, hsize: i32);
}


impl Expand for XYBox {

    /// Expand this XYBox by an equal amount in X and Y, both sides.
    /// 
    /// * `img_size` - the size of the image we are dealing with.
    /// * `esize` - expansion amount in pixels.
    fn expand_equal(&mut self, img_size: &ImageSize, esize: i32) {
        self.expand(img_size, esize, esize)
    }

    /// Expand this XYBox by an amount in X and another amount in Y, both sides.
    /// 
    /// * `img_size` - the size of the image we are dealing with.
    /// * `wsize` - expansion amount in pixels in X.
    /// * `hsize` - expansion amount in pixels in Y.
    fn expand(&mut self, img_size: &ImageSize, wsize: i32, hsize: i32) {

        if hsize == 0 && wsize == 0 { return; }

        self.x_min = self.x_min - wsize;
        self.y_min = self.y_min - hsize;
        

        if self.x_max + wsize >= img_size.width as i32 {
            self.x_max = img_size.width as i32 - 1;
        } else {
            self.x_max = self.x_max + wsize;
        }

        if self.y_max + hsize >= img_size.height as i32 {
            self.y_max = img_size.height as i32 - 1;
        } else {
            self.y_max = self.y_max + hsize;
        }
    }
}

// TODO - seems like excess code again here
impl Expand for RawBox {
    
    /// Expand this RawBox by an equal amount in X and Y, both sides.
    /// 
    /// * `img_size` - the size of the image we are dealing with.
    /// * `esize` - expansion amount in pixels.
    fn expand_equal(&mut self, img_size: &ImageSize, esize: i32) {
        self.expand(img_size, esize, esize)
    }

    /// Expand this RawBox by an amount in X and another amount in Y, both sides.
    /// 
    /// * `img_size` - the size of the image we are dealing with.
    /// * `wsize` - expansion amount in pixels in X.
    /// * `hsize` - expansion amount in pixels in Y.
    fn expand(&mut self, img_size: &ImageSize, wsize: i32, hsize: i32) {
        if hsize == 0 && wsize == 0 { return; }

        self.x_min = self.x_min - wsize;
        self.y_min = self.y_min - hsize;

        if self.x_min < 0 { self.x_min = 0;}
        if self.y_min < 0 { self.y_min = 0;}

        if self.x_max + wsize >= img_size.width as i32 {
            self.x_max = img_size.width as i32 - 1;
        } else {
            self.x_max = self.x_max + wsize;
        }

        if self.y_max + hsize >= img_size.height as i32 {
            self.y_max = img_size.height as i32 - 1;
        } else {
            self.y_max = self.y_max + hsize;
        }
    }
}

impl RefChange for RawBox {
    /// Return the centre point of this RawBox, in pixels. X then Y.
    fn centre(self) -> (u32, u32) {
        ((self.x_max + self.x_min / 2) as u32, (self.y_max + self.y_min / 2) as u32)
    }

    /// Return the height of this RawBox in pixels.
    fn height(self) -> u32 {
        (self.y_max - self.y_min).max(0) as u32
    }

    /// Return the width of this RawBox in pixels.
    fn width(self) -> u32 {
        (self.x_max - self.x_min).max(0) as u32
    }
}

pub trait FixedSize {
    fn shift(&mut self, x: i32, y: i32);
    fn fixed_size(&mut self, img_size: &ImageSize, width: u32, height: u32);
}

impl FixedSize for XYBox {
    /// Move this XYBox by X,Y pixels.
    /// 
    /// * `x` - move amount in pixels in X.
    /// * `y` - move amount in pixels in Y.
    fn shift(&mut self, x: i32, y: i32) {   
        let nx_min = self.x_min as i32 + x;
        let nx_max = self.x_max as i32 + x;
        let ny_min = self.y_min as i32 + y;
        let ny_max = self.y_max as i32 + y;

        assert!(nx_min >= 0);
        assert!(ny_min >= 0);

        // TODO - should check against image size

        self.x_min = nx_min;
        self.y_min = ny_min;
        self.x_max = nx_max;
        self.y_max = ny_max;
    }

    /// Change the size of this bounding box about it's centre.
    /// 
    /// * `img_size` - the size of the image we are dealing with.
    /// * `width` - new width in pixels.
    /// * `height` - new height in pixels.
    fn fixed_size(&mut self, img_size: &ImageSize, width: u32, height: u32) {
        // It's a bit overkill this function. We might be able to simplify it
        let tw = (self.x_max - self.x_min) as i32;
        let w = (width as i32 - tw) / 2;
        let n = (width as i32 - tw) % 2;
        let nx_min = self.x_min as i32 - w;
        let mut nx_max = self.x_max as i32 + w;
        nx_max += n;

        let th = (self.y_max - self.y_min) as i32;
        let h = (height as i32 - th) / 2;
        let m = (height as i32 - th) % 2;
        let ny_min = self.y_min as i32 - h;
        let mut ny_max = self.y_max as i32 + h;
        ny_max += m;

        assert!(nx_max - nx_min == width as i32);
        assert!(ny_max - ny_min == height as i32);
 
        self.x_min = nx_min;
        self.x_max = nx_max;
        self.y_min = ny_min;
        self.y_max = ny_max;

        // Now do a shift if we exceed the bounds
        let mut shift_x = 0;
        let mut shift_y = 0;
        
        if nx_min < 0 {
            shift_x = 0 - nx_min;
        }
        if nx_max >= img_size.width as i32 {
            shift_x = (img_size.width as i32) - nx_max - 1;
        }

        if ny_min < 0 {
            shift_y = 0 - ny_min;
        }

        if ny_max >= img_size.height as i32 {
            shift_y = img_size.height as i32 - ny_max - 1;
        }

        self.shift(shift_x, shift_y);

    }
}

pub trait RawCoords {
    fn to_raw(&self, image_size: &ImageSize, btable: &Vec<f32>) -> RawBox;
}


impl RawCoords for BearBox {
    /// Convert a BearBox into a RawBox.
    /// 
    /// * `img_size` - the size of the image we are dealing with.
    /// * `btable` - the bearing table.
    fn to_raw(&self, image_size: &ImageSize, btable: &Vec<f32>) -> RawBox {
        // Convert to the RAW coords from the default, bearbox polar ones.
        // We assume the sonar basics of -60 to +60 fans. The range
        // is also passed in.

        fn _find_idx(c: f32, btable: &Vec<f32>) -> usize {
            for i in 0..btable.len() - 1 {
                let a = btable[i];
                let b = btable[i+1];

                if a >= c && b < c {
                    return i
                }
            }
            0
        }
           
        //let xmin = (-self.bearing_max - MIN_ANGLE.to_radians()) / (MAX_ANGLE.to_radians() - MIN_ANGLE.to_radians()) * image_size.width as f32;
        //let xmax = (-self.bearing_min - MIN_ANGLE.to_radians()) / (MAX_ANGLE.to_radians() - MIN_ANGLE.to_radians()) * image_size.width as f32;
        let xmin_pixel = (_find_idx(self.bearing_max, btable) as f32 / btable.len() as f32) * image_size.width as f32;
        let xmax_pixel = (_find_idx(self.bearing_min, btable) as f32 / btable.len() as f32) * image_size.width as f32;

        let ymin_pixel = self.distance_min / self.sonar_range * (image_size.height as f32);
        let ymax_pixel = self.distance_max / self.sonar_range * (image_size.height as f32);

        let rbox = RawBox{ x_min: xmin_pixel as i32, y_min: ymin_pixel as i32, x_max: xmax_pixel as i32, y_max: ymax_pixel as i32};
        rbox
    }
}

pub trait Area {
    fn area(&self) -> u32;
  
}

impl Area for RawBox {
    /// Return the area of a RawBox
    fn area(&self) -> u32 {
        ((self.x_max - self.x_min) * (self.y_max - self.y_min)) as u32
    }
}

/// Return true if the bounding boxes overlap
/// 
/// * `a` - first RawBox to compare.
/// * `b` - RawBox to compare twith the first.
pub fn overlap_rawbox(a: &RawBox, b: &RawBox) -> bool {
    if a.x_max > b.x_min && a.x_min < b.x_max {
        if a.y_max > b.y_min && a.y_min < b.y_max {
            return true;
        } 
    }
    false
}

/// Return the X and Y distances between two raw boxes.
/// 
/// * `a` - first RawBox to compare.
/// * `b` - RawBox to compare twith the first.
pub fn distance_rawbox(a: &RawBox, b: &RawBox) -> (i32, i32) {
    let mut x_dist: i32 = 0;
    if !(a.x_max > b.x_min && a.x_min < b.x_max) {
        if a.x_max < b.x_min {
            x_dist = b.x_min - a.x_max;
        } else {
            x_dist = a.x_min - b.x_max;
        }
    }

    let mut y_dist: i32 = 0;

    if !(a.y_max > b.y_min && a.y_min < b.y_max) {
        if a.y_max < b.y_min {
            y_dist = b.y_min - a.y_max;
        } else {
            y_dist = a.y_min - b.y_max;
        }
    }

    (x_dist.max(0), y_dist.max(0))
}

#[cfg(test)]
mod tests {
    use crate::{files::read_bearing_table, image::width_from_height};
    use super::*;

    #[test]
    fn test_bearing_to_xy() {
        // Test the bearing to xy
        let (x0, y0) = dist_bearing_to_xy(0.0, 50.0, 55.0, &ImageSize { width: width_from_height(400), height: 400});
        assert_eq!(x0, 346);
        assert_eq!(y0, 364);
    
        let (x1, y1) = dist_bearing_to_xy(30.0_f32.to_radians(), 50.0, 55.0, &ImageSize { width: width_from_height(400), height: 400});
        assert!(x1 > 400);
        assert!(y1 < 350);
    }
    
    #[test]
    fn test_bb_to_fix() {
        let mut bb = XYBox {
            x_min: 20,
            y_min: 20,
            x_max: 40,
            y_max: 40
        };
    
        let image_size = ImageSize { width: 692, height: 400 };
        bb.fixed_size(&image_size, 32, 32);
    
        assert_eq!(bb.x_min, 14);
        assert_eq!(bb.y_min, 14);
        assert_eq!(bb.x_max, 46);
        assert_eq!(bb.y_max, 46);
    
        let mut bb2 = XYBox {
            x_min: 20,
            y_min: 20,
            x_max: 40,
            y_max: 40
        };
    
        bb2.fixed_size(&image_size, 15, 15);
    
        assert_eq!(bb2.x_min, 22);
        assert_eq!(bb2.y_min, 22);
        assert_eq!(bb2.x_max, 37);
        assert_eq!(bb2.y_max, 37);
    
        let mut bb3 = XYBox {
            x_min: 680,
            y_min: 0,
            x_max: 690,
            y_max: 20
        };
    
        bb3.fixed_size(&image_size, 30, 30);
    
        assert_eq!(bb3.x_min, 661);
        assert_eq!(bb3.y_min, 0);
        assert_eq!(bb3.x_max, 691);
        assert_eq!(bb3.y_max, 30);
    
    }
    
    #[test]
    fn test_bb_expand() {
    
        let mut bb = XYBox {
            x_min: 20,
            y_min: 20,
            x_max: 40,
            y_max: 40
        };
    
        let image_size = ImageSize { width: 400, height: 692 };
        bb.expand(&image_size, 10, 10);
    
        assert_eq!(bb.x_min, 10);
        assert_eq!(bb.y_min, 10);
        assert_eq!(bb.x_max, 50);
        assert_eq!(bb.y_max, 50);
    }

    #[test]
    fn test_bb_to_raw() {
        let bb = BearBox::new(-10.0f32.to_radians(), 10.0f32.to_radians(), 40.0, 42.0, 55.0).unwrap();
        let image_size = ImageSize { width: 512, height: 1658 };
        let btable = read_bearing_table().unwrap();
        let rb = bb.to_raw(&image_size, &btable);

        assert_eq!(rb.x_min, 204);
        assert_eq!(rb.y_min, 1205);
        assert_eq!(rb.x_max, 306);
        assert_eq!(rb.y_max, 1266);
    }

    #[test]
    fn test_overlap() {
        let bb0 = RawBox {
            x_min: 20,
            y_min: 20,
            x_max: 40,
            y_max: 40
        };

        let bb1 = RawBox {
            x_min: 25,
            y_min: 25,
            x_max: 45,
            y_max: 45
        };

        assert!(overlap_rawbox(&bb0, &bb1));

    }

    #[test]
    fn test_distance() {
        let bb0 = RawBox {
            x_min: 20,
            y_min: 20,
            x_max: 40,
            y_max: 40
        };

        let bb1 = RawBox {
            x_min: 50,
            y_min: 50,
            x_max: 60,
            y_max: 60
        };


        let bb2 = RawBox {
            x_min: 25,
            y_min: 25,
            x_max: 35,
            y_max: 35
        };

        assert!(distance_rawbox(&bb0, &bb1) == (10, 10));
        assert!(distance_rawbox(&bb0, &bb2) == (0, 0));
    }
    
}