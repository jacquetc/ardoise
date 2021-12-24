extern crate captrs;
extern crate clap;
extern crate x11cap;
use clap::App;

use captrs::Capturer;
use std::error::Error;
use std::time::Instant;
//use x11_screenshot::Screen;
use x11cap::{Bgr8, Image};
#[macro_use]
extern crate dlopen_derive;

mod imagery;
use imagery::{Area, Imagery};
//use imagery::

#[path = "it8951.rs"]
mod it8951;

fn main() {
    let matches = App::new("Ardoise")
        .version("1.0")
        .author("Cyril Jacquet <cyril.jacquet.libre@mailfence.com>")
        .about("E-Ink")
        .args_from_usage(
            "-r, --rotate=[ROTATION] 'rotate'
                              -d, --display=[DISPLAY] 'select display'",
        )
        .get_matches();

    let rotation_arg: u16 = match matches.value_of("rotate").unwrap_or("0").parse() {
        Ok(n) => n,
        Err(_) => {
            println!("write a number");
            0
        }
    };

    let display_number_arg: usize = match matches.value_of("display").unwrap_or("0").parse() {
        Ok(n) => n,
        Err(_) => {
            println!("write a number");
            0
        }
    };



//------------------------------------------

    let now = Instant::now();

    let mut image_array: [Option<Image>; 2] = [None, None];

    let interface: Result<it8951::IT, Box<Error>> = it8951::IT::new();

    if interface.is_err() {
        panic!("no interface");
    }
    let mut interface = interface.unwrap();

    let capt = Capturer::new(display_number_arg).unwrap();
    println!("geom : {}, {}", capt.geometry().0, capt.geometry().1);

    let imagery = Imagery::new(
        interface.size().0,
        interface.size().1,
        capt.geometry().0 as u16,
        capt.geometry().1 as u16,
        rotation_arg,
    );
    //let mut it_c_interface: eink_interface::Interface = eink_interface::Interface::new();

    loop {
        let now2 = Instant::now();

        let capt_result = Capturer::new(display_number_arg);
        let mut capt = capt_result.unwrap();

        let _result = capt.capture_store_frame();
        let image = capt.image.unwrap();
        image_array.reverse();
        image_array[1] = Some(image);

        let a = now2.elapsed().as_millis();
        //println!("a: {}", &a);

        let old_image = &image_array[0];
        let new_image = &image_array[1];

        let area_choice;

        if new_image.is_none() {
            println!("new_image is none");
            continue;
        }
/*         if old_image.is_none() {
            let bgr8_vec = imagery.get_slice_adapted_to_eink_size(&new_image);

            assert_eq!(
                imagery.eink_width as usize * imagery.eink_height as usize,
                bgr8_vec.len()
            );
            area_choice = Some(Area {
                x: 0,
                y: 0,
                width: imagery.eink_width,
                height: imagery.eink_height,
                bgr_vec: bgr8_vec,
            });
        } else { */
            //let a1 = now2.elapsed().as_millis();
            area_choice = imagery.compare(old_image, new_image);
            //let a2 = now2.elapsed().as_millis();
            //println!("compare: {}", { a2 - a1 });
        //}
        if area_choice.is_none() {
            continue;
        }
        if area_choice.is_some() {
            let mut area = area_choice.unwrap();

            area = imagery.rotate(area, rotation_arg);
            let grey_vec = Imagery::transform_to_grey_4bpp(&area.bgr_vec);
            /*
                                      let c1 = now2.elapsed().as_millis();
             it_c_interface = draw_buffer_4bpp(&area, &grey_vec, it_c_interface);
            it_c_interface.display(area.x, area.y, area.width, area.height);
            let c2 = now2.elapsed().as_millis();
            println!("c: {}", { c2 - c1 });   */

            let r1 = now2.elapsed().as_millis();
            interface = send_to_buffer_4bpp(grey_vec, interface);
            interface.display(area.x, area.y, area.width, area.height);
            let r2 = now2.elapsed().as_millis();
            //println!("r: {}", { r2 - r1 });

            let b = now2.elapsed().as_millis();
            //println!("b: {}", &b);
            //println!("{} = b - a ", { b - a });
            /*
            let duration = time::Duration::from_secs(10);
            thread::sleep(duration); */
        }
    }

    //println!("total : {}", now.elapsed().as_millis());
}

fn draw_buffer(area: &Area, grey_vec: &Vec<u8>, mut interface: it8951::IT) -> it8951::IT {
    let mut y = 0;

    let mut width = area.width;
    if width > 1872 {
        width = 1872;
    }

    let mut height = area.height;
    if height > 1404 {
        height = 1404;
    }

    for grey_chunks in grey_vec.chunks_exact(usize::from(area.width)) {
        if y > height {
            break;
        }

        let mut x = 0;
        for grey in grey_chunks.iter() {
            if x > width {
                break;
            }
            interface.draw_buffer_pixel(x, y, width, grey.clone());
            x += 1;
        }
        y += 1;
    }

    interface
}

pub fn send_to_buffer_4bpp(grey_vec: Vec<u8>, mut interface: it8951::IT) -> it8951::IT {
    interface.load_buffer_from_vec(grey_vec);
    interface
}

#[test]
fn test_compare_slices_and_send_black_cross() {
    let interface: Result<it8951::IT, Box<Error>> = it8951::IT::new();

    if interface.is_err() {
        panic!("no interface");
    }
    let mut interface = interface.unwrap();

    let mut capt = Capturer::new(0).unwrap();

    let imagery = Imagery::new(1872, 1404, 400, capt.geometry().1 as u16, 0);

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

    let old_vec: Vec<Bgr8> = vec![white; 160000];
    let mut new_vec: Vec<Bgr8> = vec![white; 160000];

    let mut line: u32 = 0;
    for n in 0..160000 {
        if n == (400 * line + 200) {
            new_vec[n as usize] = black;
        }

        if (400 * 200) <= n && n < (400 * 201) {
            new_vec[n as usize] = black;
        }
        line = (n as u32).div_euclid(400);
    }

    let area_option: Option<Area> =
        imagery.compare_image_slices(old_vec.as_slice(), new_vec.as_slice());
    let area = area_option.unwrap();

    let grey_vec = Imagery::transform_to_grey_4bpp(&area.bgr_vec);

    interface = send_to_buffer_4bpp(grey_vec, interface);

    interface.display(1100, 500, area.width, area.height);

    interface.display(100, 500, 400, 400);
}
