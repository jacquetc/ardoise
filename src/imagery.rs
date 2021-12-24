use rayon::prelude::*;
use x11cap::{Bgr8, Image};
use captrs::Capturer;

#[path = "eink_interface.rs"]
mod eink_interface;

pub struct Imagery {
    pub eink_width: u16,
    pub eink_height: u16,
    capture_width: u16,
    capture_height: u16,
    rotation: u16,
}

impl Imagery {
    pub fn new(
        eink_width: u16,
        eink_height: u16,
        capture_width: u16,
        capture_height: u16,
        rotation: u16,
    ) -> Imagery {
        let mut imagery = Imagery {
            eink_width: eink_width,
            eink_height: eink_height,
            capture_width: capture_width,
            capture_height: capture_height,
            rotation: rotation,
        };

        imagery
    }

    pub fn get_slice_adapted_to_eink_size(&self, image: &Option<Image>) -> Vec<Bgr8> {
        let bgr_slice = image.as_ref().unwrap().as_slice();
        let screen_width: u32 = image.as_ref().unwrap().get_dimensions().0 as u32;

        let whole_screen_length: usize = self.eink_height as usize * self.eink_width as usize;

        let mut bgr8_vec: Vec<Bgr8> = Vec::new();
        bgr8_vec.reserve(whole_screen_length);
        let mut index: u32 = 0;
        let mut line: u32 = 0;
        let eink_width: u32 = self.eink_width as u32;
        for brg in bgr_slice.iter() {
            if line >= self.eink_height as u32 {
                break;
            }

            if (line * screen_width) <= index && index < (line * screen_width + eink_width) {
                bgr8_vec.push(brg.clone());
            }
            index += 1;
            line = index.div_euclid(screen_width);
        }

        // fill with white pixel if height is smaller than eink height
        let mut white: Bgr8 = bgr_slice.last().unwrap().clone();
        white.b = 255;
        white.r = 255;
        white.g = 255;

        // extend to fill the screen :
        if bgr8_vec.len() < whole_screen_length {
            bgr8_vec.extend(vec![white; whole_screen_length - bgr8_vec.len()])
        }

        bgr8_vec
    }

    pub fn compare_image_slices(&self, old_slice: &[Bgr8], new_slice: &[Bgr8]) -> Option<Area> {
        //let first_bgr = new_slice.get(0);

        let line_col_min_max: Option<[Option<u16>; 4]> =
            self.determine_changed_zone(old_slice, new_slice);
        line_col_min_max?;
        let min_changed_line_number = line_col_min_max.unwrap()[0].unwrap();
        let max_changed_line_number = line_col_min_max.unwrap()[1].unwrap();
        let min_changed_col_number = line_col_min_max.unwrap()[2].unwrap();
        let max_changed_col_number = line_col_min_max.unwrap()[3].unwrap();

        let changed_area_x: u16 = min_changed_col_number;
        let changed_area_y: u16 = min_changed_line_number;

        let changed_area_width: u16 = max_changed_col_number - min_changed_col_number + 1;
        let changed_area_height: u16 = max_changed_line_number - min_changed_line_number + 1;

        let geometry: [u16; 4] = [
            changed_area_x,
            changed_area_y,
            changed_area_width,
            changed_area_height,
        ];

        self.create_pixel_area(geometry, new_slice)
    }

    pub fn compare(&self, old_image: &Option<Image>, new_image: &Option<Image>) -> Option<Area> {
        if new_image.is_none() {
            return None;
        }

          let new_slice: &[Bgr8] = new_image.as_ref().unwrap().as_slice();


      let mut old_slice: &[Bgr8] = &[];
      let mut white_vec: Vec<Bgr8> = Vec::new();

        if old_image.as_ref().is_none() {
            // first image, fill it white
            let mut white: Bgr8 = new_slice.first().expect("problem").clone();
    white.b = 255;
    white.r = 255;
    white.g = 255;
            
            white_vec = vec![white ; self.capture_height as usize * self.capture_width as usize];
            old_slice = white_vec.as_slice();
            //println!("old_slice: {}", old_slice.len());
        }
        else {
            old_slice = old_image.as_ref().unwrap().as_slice();
        }

         

        //let width: u16 = new_image.as_ref().unwrap().get_dimensions().0 as u16;
        self.compare_image_slices(old_slice, new_slice)
    }

    pub fn rotate(&self, base_area: Area, rotation_angle: u16) -> Area {
        match rotation_angle {
            90 => {
                let rotated_x = self.eink_width - base_area.y - base_area.height;
                let rotated_y = base_area.x;
                let rotated_width = base_area.height;
                let rotated_height = base_area.width;

/*                 if rotated_y + rotated_height >= self.eink_height {
                    rotated_height = self.eink_height - rotated_y;
                } */

                let base_bgr_vec: Vec<Bgr8> = base_area.bgr_vec;

                let rotated_bgr_vec_vec: Vec<Vec<Bgr8>> = (0..rotated_height)
                    .into_par_iter()
                    .map(|line_number| {
                        let mut sub_bgr_vec: Vec<Bgr8> = Vec::new();

                        for index in (0..rotated_width).into_iter() {
                            let calculated_index =
                                index as usize * rotated_height as usize + line_number as usize;
                            sub_bgr_vec.push(base_bgr_vec[calculated_index]);
                        }

                        sub_bgr_vec.reverse();
                        sub_bgr_vec
                    })
                    .collect();

                let rotated_bgr_vector = rotated_bgr_vec_vec.concat();

                Area {
                    x: rotated_x,
                    y: rotated_y,
                    width: rotated_width,
                    height: rotated_height,
                    bgr_vec: rotated_bgr_vector,
                }
            }

            _ => base_area,
        }
    }

    fn transform_to_grey(bgr_vec: &Vec<Bgr8>) -> Vec<u8> {
        let mut grey_vector: Vec<u8> = Vec::new();

        for bgr in bgr_vec.iter() {
            let r: f32 = bgr.r.into();
            let g: f32 = bgr.g.into();
            let b: f32 = bgr.b.into();
            let grey: f32 = r * 0.2125 + g * 0.7154 + b * 0.0721;
            let grey_u8: u8 = grey as u8;
            //let grey_u8: u8 = u8::try_from(grey).expect("problem with grey_u8");

            grey_vector.push(grey_u8);
        }
        grey_vector
    }

    pub fn transform_to_grey_4bpp(bgr_vec: &Vec<Bgr8>) -> Vec<u8> {
        let bgr_vec_len = bgr_vec.len();
        let mut grey_vector: Vec<u8> = Vec::new();
        grey_vector.reserve(bgr_vec_len / 2);

        assert_eq!(bgr_vec_len.rem_euclid(4), 0);
        let mut index: u32 = 0;
        let grey_par_iter = bgr_vec.par_iter().map(|bgr| {
            let r: f32 = bgr.r.into();
            let g: f32 = bgr.g.into();
            let b: f32 = bgr.b.into();
            let grey: f32 = (r * 0.2125 + g * 0.7154 + b * 0.0721).div_euclid(16.0);
            let result: u8 = grey as u8;
            result
        });

        for grey_u8 in grey_par_iter.collect::<Vec<u8>>().iter() {
            if index.rem_euclid(2) > 0 {
                // means it's an odd number

                let last_grey: u8 = grey_vector.pop().unwrap();
                let bitwise_gray = grey_u8 << 4;
                let two_greys: u8 = bitwise_gray + last_grey;

                grey_vector.push(two_greys);
            } else {
                grey_vector.push(grey_u8.clone());
            }

            index += 1;
        }

        // invert  : byte1, byte2 -> byte2, byte1

        for i in 0..(grey_vector.len()) {
            if index.rem_euclid(2) > 0 {
                // means it's an odd number
                grey_vector.swap(i, i - 1);
            }
        }
        assert_eq!(grey_vector.len(), bgr_vec_len / 2);
        //println!("grey_vector.len() : {}",grey_vector.len());

        grey_vector
    }

    fn draw_buffer_4bpp(
        &self,
        area: &Area,
        grey_vec: &Vec<u8>,
        mut interface: eink_interface::Interface,
    ) -> eink_interface::Interface {
        let mut y: u16 = 0;
        let mut width = area.width;
        if width >= self.eink_width {
            width = self.eink_width;
        }
        let mut height = area.height;
        if height >= self.eink_height {
            height = self.eink_height;
        }
        //println!("len: {}", grey_vec.len());
        for grey_chunks in grey_vec.chunks_exact((area.width / 2) as usize) {
            if y >= height {
                break;
            }
            let mut x: u16 = 0;
            for grey in grey_chunks.iter() {
                if x >= width {
                    break;
                }
                interface.draw_buffer_pixel(x, y, width / 2, grey.clone());
                x += 1;
            }
            y += 1;
        }
        //println!("yy: {}", y);
        interface
    }

    fn determine_changed_zone(
        &self,
        old_slice: &[Bgr8],
        new_slice: &[Bgr8],
    ) -> Option<[Option<u16>; 4]> {
        //let mut line_number = 0;

        let old_iter = old_slice.par_chunks((self.capture_width) as usize);
        let new_iter = new_slice.par_chunks((self.capture_width) as usize);

        let old_new_iter = old_iter.zip(new_iter.enumerate());

        let result_iter = old_new_iter.map(|(old_line, (line_n, new_line))| {
            if line_n >= self.eink_height as usize && (self.rotation == 0 || self.rotation == 180) {
                return (None, None, None);
            }
            if line_n >= self.eink_width as usize && (self.rotation == 90 || self.rotation == 270){
                return (None, None, None);
            }
            let mut line_number: Option<u16> = None;
            let mut min_col: Option<u16> = None;
            let mut max_col: Option<u16> = None;

            if old_line != new_line {
                line_number = Some(line_n as u16);

                let mut col_number = 0;
                let old_new_col_iter = old_line.iter().zip(new_line.iter());
                for (old_bgr, new_bgr) in old_new_col_iter {

                    if col_number >= self.eink_width && (self.rotation == 0 || self.rotation == 180) {
                        break;
                    }
                    if col_number >= self.eink_height && (self.rotation == 90 || self.rotation == 270) {
                        break;
                    }

                    if old_bgr != new_bgr {
                        // find column min
                        if min_col.is_none() {
                            min_col = Some(col_number);
                        }
                        max_col = Some(col_number);
                    }

                    col_number += 1;
                } 
 
                if min_col >= Some(self.eink_width)  && (self.rotation == 0 || self.rotation == 180) {
                    return (None, None, None);
                } 
          
                if min_col >= Some(self.eink_height)  && (self.rotation == 90 || self.rotation == 270) {
                    return (None, None, None);
                } 
            }
            if min_col.is_none() {
                return (None, None, None);
            }

            (line_number, min_col, max_col)
        });

        let change_limits: Vec<(Option<u16>, Option<u16>, Option<u16>)> = result_iter.collect();

        let mut min_changed_line_number: Option<u16> = None;
        let mut max_changed_line_number: Option<u16> = None;

        let mut min_changed_col_number: Option<u16> = None;
        let mut max_changed_col_number: Option<u16> = None;

        for (line_number, min_col, max_col) in change_limits.iter() {
            if line_number.is_none() {
                continue;
            }
            assert_ne!(min_col, &None);
            assert_ne!(max_col, &None);

            if min_changed_line_number.is_none() || line_number < &min_changed_line_number {
                min_changed_line_number = line_number.clone();
            }

            max_changed_line_number = line_number.clone();

            if min_changed_col_number.is_none() || min_col < &min_changed_col_number {
                min_changed_col_number = min_col.clone();
            }

            if max_changed_col_number.is_none() || max_col > &max_changed_col_number {
                max_changed_col_number = max_col.clone()
            }
        }

        min_changed_line_number?;
        max_changed_line_number?;
        min_changed_col_number?;
        max_changed_col_number?;

        Some([
            min_changed_line_number,
            max_changed_line_number,
            min_changed_col_number,
            max_changed_col_number,
        ])
    }

    fn create_pixel_area(&self, geometry: [u16; 4], gbr_slice: &[Bgr8]) -> Option<Area> {
        let mut changed_area_x: u16 = geometry[0];
        let mut changed_area_y: u16 = geometry[1];

        let mut changed_area_width: u16 = geometry[2];
        let mut changed_area_height: u16 = geometry[3];

/*          print!(
             " before shifts : x {}, y {}, width {}, height {}\n",
             changed_area_x, changed_area_y, changed_area_width, changed_area_height
         );  */
        // shift x :

        if self.rotation == 0 || self.rotation == 180 {
            if changed_area_x.rem_euclid(4) > 0 {
                let x_shift: u16 = changed_area_x.rem_euclid(4);
                changed_area_x -= x_shift;
                changed_area_width += x_shift;
            }

            // shift width :
            let shift_width = 4 - changed_area_width.rem_euclid(4);
            if changed_area_width.rem_euclid(4) > 0 {
                //println!("--- shift used");
                changed_area_width += shift_width;
            }
            // if superior to screen width, move x to the left
            if (changed_area_x + changed_area_width) > self.eink_width {
                //println!("+++ end shift used");
                changed_area_x -= 4;
            }
            assert_eq!(changed_area_width.rem_euclid(4), 0);
        }

        if self.rotation == 90 || self.rotation == 270 {
            if changed_area_y.rem_euclid(4) > 0 {
                let y_shift: u16 = changed_area_y.rem_euclid(4);
                changed_area_y -= y_shift;
                changed_area_height += y_shift;
            }

            // shift width :
            let shift_height = 4 - changed_area_height.rem_euclid(4);
            if changed_area_height.rem_euclid(4) > 0 {
                //println!("--- shift used");
                changed_area_height += shift_height;
            }
            // if superior to screen width, move x to the left
            if (changed_area_y + changed_area_height) > self.eink_width {
                //println!("+++ end shift used");
                changed_area_y -= 4;
            }
            assert_eq!(changed_area_height.rem_euclid(4), 0);
        }

        /*     print!(
            "                x {}, y {}, width {}, height {}\n",
            changed_area_x, changed_area_y, changed_area_width, changed_area_height
        ); */

        if changed_area_width == 0 || changed_area_height == 0 {
            return None;
        }
        // compose new image


        let bgr_vec_vec: Vec<Vec<Bgr8>> = gbr_slice
            .par_chunks((self.capture_width) as usize)
            .enumerate()
            .filter(|(line_number, _new_line)| {
                if line_number >= &(changed_area_y as usize)
                    && line_number < &((changed_area_y + changed_area_height) as usize)
                {
                    return true;
                }
                return false;
            })
            .map(|(_line_number, new_line)| {
                let mut bgr_vector: Vec<Bgr8> = Vec::new();
                let mut col_number = 0;
                for bgr in new_line.iter() {
                    if col_number >= changed_area_x
                        && col_number < (changed_area_x + changed_area_width)
                    {
                        // insert Bgr into vec :
                        bgr_vector.push(bgr.clone());
                    }
                    col_number += 1;
                }

                bgr_vector
            })
            .collect();

        let bgr_vec: Vec<Bgr8> = bgr_vec_vec.concat();

        //println!("vec len :  {}", bgr_vec.;len());

        assert_eq!(
            bgr_vec.len(),
            changed_area_width as usize * changed_area_height as usize
        );

        Some(Area {
            x: changed_area_x,
            y: changed_area_y,
            width: changed_area_width,
            height: changed_area_height,
            bgr_vec: bgr_vec,
        })
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Area {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub bgr_vec: Vec<Bgr8>,
}

#[test]
fn test_determine_changed_zone() {
    let mut capt = Capturer::new(0).unwrap();

    let imagery = Imagery::new(1872, 1404, 5, capt.geometry().1 as u16, 0);

    let _result = capt.capture_store_frame();
    let image = capt.image.unwrap();
    let mut black: Bgr8 = image.as_slice().first().expect("problem").clone();
    black.b = 0;
    black.r = 0;
    black.g = 0;

    let mut white = black.clone();
    white.b = 255;
    white.r = 255;
    white.g = 255;

    let old_slice = &[
        white, white, black, black, white, /**/
        white, white, black, black, white,
    ];
    let new_slice = &[
        white, white, white, white, white, /**/
        white, white, white, white, white,
    ];

    let geometry: Option<[Option<u16>; 4]> = imagery.determine_changed_zone(old_slice, new_slice);
    assert_ne!(geometry, None);

    let min_changed_line_number: u16 = geometry.unwrap()[0].unwrap();
    let max_changed_line_number: u16 = geometry.unwrap()[1].unwrap();
    let min_changed_col_number: u16 = geometry.unwrap()[2].unwrap();
    let max_changed_col_number: u16 = geometry.unwrap()[3].unwrap();

    assert_eq!(min_changed_line_number, 0);
    assert_eq!(max_changed_line_number, 1);
    assert_eq!(min_changed_col_number, 2);
    assert_eq!(max_changed_col_number, 3);
}

#[test]
fn test_determine_changed_zone_2() {
    let mut capt = Capturer::new(0).unwrap();

    let imagery = Imagery::new(1872, 1404, 5, capt.geometry().1 as u16, 0);

    let _result = capt.capture_store_frame();
    let image = capt.image.unwrap();
    let mut black: Bgr8 = image.as_slice().first().expect("problem").clone();
    black.b = 0;
    black.r = 0;
    black.g = 0;

    let mut white = black.clone();
    white.b = 255;
    white.r = 255;
    white.g = 255;

    let old_slice = &[
        white, white, white, black, white, /**/
        white, white, white, black, white,
    ];
    let new_slice = &[
        white, white, white, white, white, /**/
        white, white, white, white, white,
    ];

    let geometry: Option<[Option<u16>; 4]> = imagery.determine_changed_zone(old_slice, new_slice);
    assert_ne!(geometry, None);

    let min_changed_line_number: u16 = geometry.unwrap()[0].unwrap();
    let max_changed_line_number: u16 = geometry.unwrap()[1].unwrap();
    let min_changed_col_number: u16 = geometry.unwrap()[2].unwrap();
    let max_changed_col_number: u16 = geometry.unwrap()[3].unwrap();

    assert_eq!(min_changed_line_number, 0);
    assert_eq!(max_changed_line_number, 1);
    assert_eq!(min_changed_col_number, 3);
    assert_eq!(max_changed_col_number, 3);
}

#[test]
fn test_determine_changed_zone_3() {
    let mut capt = Capturer::new(0).unwrap();

    let imagery = Imagery::new(1872, 1404, 8, capt.geometry().1 as u16, 0);

    let _result = capt.capture_store_frame();
    let image = capt.image.unwrap();
    let mut black: Bgr8 = image.as_slice().first().expect("problem").clone();
    black.b = 0;
    black.r = 0;
    black.g = 0;

    let mut white = black.clone();
    white.b = 255;
    white.r = 255;
    white.g = 255;

    let old_slice = &[
        white, white, white, white, white, white, white, white, /**/
        white, white, white, white, black, white, white, white,
    ];
    let new_slice = &[
        white, white, white, white, white, white, white, white, /**/
        white, white, white, white, white, white, white, white,
    ];

    let geometry: Option<[Option<u16>; 4]> = imagery.determine_changed_zone(old_slice, new_slice);
    assert_ne!(geometry, None);

    let min_changed_line_number: u16 = geometry.unwrap()[0].unwrap();
    let max_changed_line_number: u16 = geometry.unwrap()[1].unwrap();
    let min_changed_col_number: u16 = geometry.unwrap()[2].unwrap();
    let max_changed_col_number: u16 = geometry.unwrap()[3].unwrap();

    assert_eq!(min_changed_line_number, 1);
    assert_eq!(max_changed_line_number, 1);
    assert_eq!(min_changed_col_number, 4);
    assert_eq!(max_changed_col_number, 4);
}

#[test]
fn test_determine_changed_zone_whole() {
    let mut capt = Capturer::new(0).unwrap();

    let imagery = Imagery::new(1872, 1404, 1920, capt.geometry().1 as u16, 0);

    let _result = capt.capture_store_frame();
    let image = capt.image.unwrap();
    let mut black: Bgr8 = image.as_slice().first().expect("problem").clone();
    black.b = 0;
    black.r = 0;
    black.g = 0;

    let mut white = black.clone();
    white.b = 255;
    white.r = 255;
    white.g = 255;

    let old_slice = &[white; 2 * 1920];
    let new_slice = &[black; 2 * 1920];

    let geometry: Option<[Option<u16>; 4]> = imagery.determine_changed_zone(old_slice, new_slice);
    assert_ne!(geometry, None);

    let min_changed_line_number: u16 = geometry.unwrap()[0].unwrap();
    let max_changed_line_number: u16 = geometry.unwrap()[1].unwrap();
    let min_changed_col_number: u16 = geometry.unwrap()[2].unwrap();
    let max_changed_col_number: u16 = geometry.unwrap()[3].unwrap();

    assert_eq!(min_changed_line_number, 0);
    assert_eq!(max_changed_line_number, 1);
    assert_eq!(min_changed_col_number, 0);
    assert_eq!(max_changed_col_number, 1871);
}

#[test]
fn test_determine_changed_zone_nothing() {
    let mut capt = Capturer::new(0).unwrap();
    let _result = capt.capture_store_frame();

    let imagery = Imagery::new(1872, 1404, 5, capt.geometry().1 as u16, 0);

    let image = capt.image.unwrap();
    let mut black: Bgr8 = image.as_slice().first().expect("problem").clone();
    black.b = 0;
    black.r = 0;
    black.g = 0;

    let mut white = black.clone();
    white.b = 255;
    white.r = 255;
    white.g = 255;

    let old_slice = &[
        white, white, white, white, white, /**/
        white, white, white, white, white,
    ];
    let new_slice = &[
        white, white, white, white, white, /**/
        white, white, white, white, white,
    ];

    let geometry: Option<[Option<u16>; 4]> = imagery.determine_changed_zone(old_slice, new_slice);
    assert_eq!(geometry, None);
}

#[test]
fn test_area() {
    let mut capt = Capturer::new(0).unwrap();

    let imagery = Imagery::new(1872, 1404, 5, capt.geometry().1 as u16, 0);

    let _result = capt.capture_store_frame();
    let image = capt.image.unwrap();
    let mut black: Bgr8 = image.as_slice().first().expect("problem").clone();
    black.b = 0;
    black.r = 0;
    black.g = 0;

    let mut white = black.clone();
    white.b = 255;
    white.r = 255;
    white.g = 255;

    let old_slice = &[
        white, white, white, white, white, /**/
        white, white, white, white, white,
    ];
    let new_slice = &[
        white, black, white, white, white, /**/
        white, black, white, white, white,
    ];

    let geometry: Option<[Option<u16>; 4]> = imagery.determine_changed_zone(old_slice, new_slice);
    assert_ne!(geometry, None);

    let min_changed_line_number: u16 = geometry.unwrap()[0].unwrap();
    let max_changed_line_number: u16 = geometry.unwrap()[1].unwrap();
    let min_changed_col_number: u16 = geometry.unwrap()[2].unwrap();
    let max_changed_col_number: u16 = geometry.unwrap()[3].unwrap();

    assert_eq!(min_changed_line_number, 0);
    assert_eq!(max_changed_line_number, 1);
    assert_eq!(min_changed_col_number, 1);
    assert_eq!(max_changed_col_number, 1);

    let changed_area_x: u16 = min_changed_col_number;
    let changed_area_y: u16 = min_changed_line_number;

    let changed_area_width: u16 = max_changed_col_number - min_changed_col_number + 1;
    let changed_area_height: u16 = max_changed_line_number - min_changed_line_number + 1;

    let geometry: [u16; 4] = [
        changed_area_x,
        changed_area_y,
        changed_area_width,
        changed_area_height,
    ];

    let area = imagery.create_pixel_area(geometry, new_slice);
    assert_ne!(area, None);

    let area = area.unwrap();

    assert_eq!(area.x, 0);
    assert_eq!(area.y, 0);
    assert_eq!(area.width, 4);
    assert_eq!(area.height, 2);

    assert_eq!(
        area.bgr_vec,
        vec![white, black, white, white, white, black, white, white]
    );

    let mut y: u16 = 0;
    for grey_chunks in area.bgr_vec.chunks_exact((area.width / 2) as usize) {
        let mut x = 0;
        for grey in grey_chunks.iter() {
            //interface.draw_buffer_pixel(x, y, width  / 2, grey.clone());
            x += 1;
        }
        assert_eq!(2, x);

        y += 1;
    }
    assert_eq!(4, y);
}

#[test]
fn test_transform_to_grey_4bpp() {
    let mut capt = Capturer::new(0).unwrap();

    let _result = capt.capture_store_frame();
    let image = capt.image.unwrap();
    let mut black: Bgr8 = image.as_slice().first().expect("problem").clone();
    black.b = 0;
    black.r = 0;
    black.g = 0;

    let mut white = black.clone();
    white.b = 255;
    white.r = 255;
    white.g = 255;

    let old_slice = &[
        white, black, white, white, /**/
        white, black, white, white,
    ];

    let mut brg8_vec: Vec<Bgr8> = Vec::new();
    for brg in old_slice.iter() {
        brg8_vec.push(brg.clone());
    }

    let grey_vec: Vec<u8> = Imagery::transform_to_grey_4bpp(&brg8_vec);

    assert_eq!(
        grey_vec,
        vec![0b0000_1111, 0b1111_1111 /**/, 0b0000_1111, 0b1111_1111]
    );
}

#[test]
fn test_compare_slices() {
    let mut capt = Capturer::new(0).unwrap();

    let imagery = Imagery::new(1872, 1404, 12, capt.geometry().1 as u16, 0);

    let _result = capt.capture_store_frame();
    let image = capt.image.unwrap();
    let mut black: Bgr8 = image.as_slice().first().expect("problem").clone();
    black.b = 0;
    black.r = 0;
    black.g = 0;

    let mut white = black.clone();
    white.b = 255;
    white.r = 255;
    white.g = 255;

    let old_slice = &[
        white, white, white, white, white, white, white, white, white, white, white,
        white, /**/
        white, white, white, white, white, white, white, white, white, white, white,
        white, /**/
        white, white, white, white, white, white, white, white, white, white, white,
        white, /**/
        white, white, white, white, white, white, white, white, white, white, white,
        white, /**/
    ];

    let new_slice = &[
        white, white, white, white, white, white, white, white, white, white, white,
        white, /**/
        white, white, white, white, black, black, white, white, black, white, white,
        white, /**/
        white, white, white, white, white, white, black, white, white, white, white,
        white, /**/
        white, white, white, white, white, white, white, white, white, white, white,
        white, /**/
    ];

    let area_option: Option<Area> = imagery.compare_image_slices(old_slice, new_slice);
    assert!(area_option.is_some());

    let area = area_option.unwrap();

    assert_eq!(area.x, 4);
    assert_eq!(area.y, 1);
    assert_eq!(area.width, 8);
    assert_eq!(area.height, 2);

    assert_eq!(area.bgr_vec.len(), 16);
    assert_eq!(
        area.bgr_vec,
        vec![
            black, black, white, white, black, white, white, white, /**/
            white, white, black, white, white, white, white, white
        ]
    );
}

#[test]
fn test_compare_slices_whole() {
    let mut capt = Capturer::new(0).unwrap();

    let imagery = Imagery::new(1872, 1404, 1920, capt.geometry().1 as u16, 0);

    let _result = capt.capture_store_frame();
    let image = capt.image.unwrap();
    let mut black: Bgr8 = image.as_slice().first().expect("problem").clone();
    black.b = 0;
    black.r = 0;
    black.g = 0;

    let mut white = black.clone();
    white.b = 255;
    white.r = 255;
    white.g = 255;

    let old_slice = &[white; 1920];

    let new_slice = &[black; 1920];

    let area_option: Option<Area> = imagery.compare_image_slices(old_slice, new_slice);
    assert!(area_option.is_some());

    let area = area_option.unwrap();

    assert_eq!(area.x, 0);
    assert_eq!(area.y, 0);
    assert_eq!(area.width, 1872);
    assert_eq!(area.height, 1);

    assert_eq!(area.bgr_vec.len(), 1872);
    assert_eq!(area.bgr_vec, vec![black; 1872]);
}

#[test]
fn test_compare_slices_whole_minus_one() {
    let mut capt = Capturer::new(0).unwrap();

    let imagery = Imagery::new(
        1872,
        1404,
        capt.geometry().0 as u16,
        capt.geometry().1 as u16,
        0,
    );

    let _result = capt.capture_store_frame();
    let image = capt.image.unwrap();
    let mut black: Bgr8 = image.as_slice().first().expect("problem").clone();
    black.b = 0;
    black.r = 0;
    black.g = 0;

    let mut white = black.clone();
    white.b = 255;
    white.r = 255;
    white.g = 255;

    let old_slice = &[white; (1872 * 2)];

    let mut new_vec: Vec<Bgr8> = vec![white];
    new_vec.extend_from_slice(&[black; (1871)]);
    let result_vec = new_vec.clone();
    new_vec.extend_from_slice(&[white; (1872)]);
    let new_slice = new_vec.as_slice();

    let area_option: Option<Area> = imagery.compare_image_slices(old_slice, new_slice);
    assert!(area_option.is_some());

    let area = area_option.unwrap();

    assert_eq!(area.x, 0);
    assert_eq!(area.y, 0);
    assert_eq!(area.width, 1872);
    assert_eq!(area.height, 1);

    assert_eq!(area.bgr_vec.len(), 1872);
    assert_eq!(area.bgr_vec, result_vec);
}

#[test]
fn test_compare_slices_whole_minus_one_2() {
    let mut capt = Capturer::new(0).unwrap();

    let imagery = Imagery::new(1872, 1404, 1872, capt.geometry().1 as u16, 0);

    let _result = capt.capture_store_frame();
    let image = capt.image.unwrap();
    let mut black: Bgr8 = image.as_slice().first().expect("problem").clone();
    black.b = 0;
    black.r = 0;
    black.g = 0;

    let mut white = black.clone();
    white.b = 255;
    white.r = 255;
    white.g = 255;

    let old_slice = &[white; (1872 * 2)];

    let mut new_vec: Vec<Bgr8> = vec![black; 1871];
    new_vec.extend_from_slice(&[white]);
    let result_vec = new_vec.clone();
    new_vec.extend_from_slice(&[white; 1872]);
    let new_slice = new_vec.as_slice();

    let area_option: Option<Area> = imagery.compare_image_slices(old_slice, new_slice);
    assert!(area_option.is_some());

    let area = area_option.unwrap();

    assert_eq!(area.x, 0);
    assert_eq!(area.y, 0);
    assert_eq!(area.width, 1872);
    assert_eq!(area.height, 1);

    assert_eq!(area.bgr_vec.len(), 1872);
    assert_eq!(area.bgr_vec, result_vec);
}

#[test]
fn test_compare_slices_whole_height_and_more() {
    let mut capt = Capturer::new(0).unwrap();

    let imagery = Imagery::new(1872, 1404, 2, capt.geometry().1 as u16, 0);

    let _result = capt.capture_store_frame();
    let image = capt.image.unwrap();
    let mut black: Bgr8 = image.as_slice().first().expect("problem").clone();
    black.b = 0;
    black.r = 0;
    black.g = 0;

    let mut white = black.clone();
    white.b = 255;
    white.r = 255;
    white.g = 255;

    let old_slice = &[white; (2900)];

    let new_vec: Vec<Bgr8> = vec![black; 2900];
    let result_vec: Vec<Bgr8> = vec![black; 1404 * 2];
    let new_slice = new_vec.as_slice();

    let area_option: Option<Area> = imagery.compare_image_slices(old_slice, new_slice);
    assert!(area_option.is_some());

    let area = area_option.unwrap();

    assert_eq!(area.x, 0);
    assert_eq!(area.y, 0);
    assert_eq!(area.width, 4);
    assert_eq!(area.height, 1404);

    assert_eq!(area.bgr_vec.len(), 1404 * 2);
    assert_eq!(area.bgr_vec, result_vec);
}

#[ignore]
#[test]
fn draw_whole_white_in_black_box() {
    /*    let mut capt = Capturer::new(0).unwrap();

        let imagery = Imagery {
        eink_width: capt.geometry().0 as u16,
        eink_height: capt.geometry().1 as u16
    };

    let _result = capt.capture_store_frame();
    let image = capt.image.unwrap();
    let mut black: Bgr8 = image.as_slice().first().expect("problem").clone();
    black.b = 0;
    black.r = 0;
    black.g = 0;

    let mut white = black.clone();
    white.b = 255;
    white.r = 255;
    white.g = 255;

    let old_vec = &[white; (2900)];

    let new_vec: Vec<Bgr8> = vec![black; 2900];
    let result_vec: Vec<Bgr8> = vec![black; 1404 * 2];
    let new_slice = new_vec.as_slice();

    let area_option: Option<Area> = imagery.compare_image_slices(old_slice, new_slice, 2);
    assert!(area_option.is_some());

    let area = area_option.unwrap();

    assert_eq!(area.x, 0);
    assert_eq!(area.y, 0);
    assert_eq!(area.width, 4);
    assert_eq!(area.height, 1404);

    assert_eq!(area.bgr_vec.len(), 1404 * 2);
    assert_eq!(area.bgr_vec, result_vec); */
    unimplemented!();
} /*

  #[test]
  fn test_adapt_slice_to_eink_size() {
      let mut capt = Capturer::new(0).unwrap();

          let imagery = Imagery {
        eink_width: capt.geometry().0 as u16,
        eink_height: capt.geometry().1 as u16
    };

      let _result = capt.capture_store_frame();
      let image = capt.image.unwrap();
      let mut black: Bgr8 = image.as_slice().first().expect("problem").clone();
      black.b = 0;
      black.r = 0;
      black.g = 0;

      let base_vec: Vec<Bgr8> = vec![black; 1920 * 1080];
      let slice: &[Bgr8] = base_vec.as_slice();

      let sized_vec: Vec<Bgr8> = imagery.get_slice_adapted_to_eink_size(&image);

      assert_eq!(sized_vec.len(), 1872 * 1404);
  } */
#[test]
fn test_rotation_90() {
    let mut capt = Capturer::new(0).unwrap();
    let imagery = Imagery::new(1872, 1404, 4, capt.geometry().1 as u16, 90);

    let _result = capt.capture_store_frame();
    let image = capt.image.unwrap();
    let mut black: Bgr8 = image.as_slice().first().expect("problem").clone();
    black.b = 0;
    black.r = 0;
    black.g = 0;

    let mut white = black.clone();
    white.b = 255;
    white.r = 255;
    white.g = 255;

    let base_bgr_vec: Vec<Bgr8> = vec![white, black, black, white, white, black, black, white];

    let base_area = Area {
        x: 5,
        y: 10,
        width: 4,
        height: 2,
        bgr_vec: base_bgr_vec,
    };

    let rotated_area = imagery.rotate(base_area.clone(), 90);

    assert_eq!(
        rotated_area.x,
        imagery.eink_width - base_area.y - base_area.height
    );
    assert_eq!(rotated_area.y, base_area.x);
    assert_eq!(rotated_area.width, base_area.height);
    assert_eq!(rotated_area.height, base_area.width);

    let rotated_bgr_vec: Vec<Bgr8> = vec![white, white, black, black, black, black, white, white];

    assert_eq!(rotated_area.bgr_vec, rotated_bgr_vec);
}
