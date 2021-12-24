extern crate dlopen;

use dlopen::wrapper::{Container, WrapperApi};
use std::time::Instant;
use std::ffi::CString;
use std::os::raw::c_char;
use x11cap::Image;

#[path = "it8951.rs"]
mod it8951;


#[derive(WrapperApi)]
pub struct IT8952Api {
#[allow(non_snake_case)]
    IT8951_Init: unsafe extern "C" fn() -> u8,
    IT8951_GUI_Example: unsafe extern "C" fn(),
    IT8951Display1bppExample: unsafe extern "C" fn(),
    IT8951_GUI_Example2:
        unsafe extern "C" fn(x: u16, y: u16, rect_width: u16, rect_height: u16, mode: u16),
    IT8951DisplayExample2: unsafe extern "C" fn(),
    IT8951_BMP_Example: unsafe extern "C" fn(x: u32, y: u32, path: *const c_char),
    drawBufferPixel: unsafe extern "C" fn(x: u16, y: u16, width: u16, color: u8),
    display: unsafe extern "C" fn(x: u16, y: u16, rect_width: u16, rect_height: u16),
}

pub struct Interface {
    api: Container<IT8952Api>,
}

impl Interface {
    pub fn new() -> Interface {
        Interface {
            api: Interface::init(),
        }
        
    }

    fn init() -> Container<IT8952Api> {
        let it: Container<IT8952Api> = unsafe { Container::load("/usr/local/lib/libIT8951.so") }
            .expect("Could not open library or load symbols");
        unsafe {
            it.IT8951_Init();
        }
        it
    }

    pub fn display(&self, x: u16, y: u16, rect_width: u16, rect_height: u16) {
        unsafe {
            self.api.display(x, y, rect_width, rect_height);
        }
    }

    pub fn draw_buffer_pixel(&self, x: u16, y: u16, width: u16, color: u8){
        unsafe {
            self.api.drawBufferPixel(x, y, width, color);
        }
    }

    pub fn load_image(&self) {

        unsafe {
            self.api.IT8951_GUI_Example();
            self.api.IT8951Display1bppExample();
        }
    }
}
