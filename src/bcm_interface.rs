extern crate dlopen;
use dlopen::wrapper::{Container, WrapperApi};
use std::time::Instant;
use std::ffi::CString;
use std::os::raw::c_char;
use x11cap::Image;
extern crate custom_error;
use custom_error::custom_error;



#[derive(WrapperApi)]
struct BCMApi {
#[allow(non_snake_case)]
bcm2835_init: unsafe extern "C" fn() -> u8,
bcm2835_spi_begin: unsafe extern "C" fn(),
bcm2835_spi_setBitOrder: unsafe extern "C" fn(order: u8),
bcm2835_spi_setDataMode: unsafe extern "C" fn(mode: u8),
bcm2835_spi_setClockDivider: unsafe extern "C" fn(divider: u16),
bcm2835_gpio_fsel: unsafe extern "C" fn(pin: u8, mode: u8),
bcm2835_gpio_write: unsafe extern "C" fn(pin: u8, on: u8),
bcm2835_gpio_lev: unsafe extern "C" fn(pin: u8) -> u8,
bcm2835_delay: unsafe extern "C" fn(millis: u16),
bcm2835_spi_end: unsafe extern "C" fn(),
bcm2835_close: unsafe extern "C" fn() -> u8,
bcm2835_spi_transfer: unsafe extern "C" fn(value: u8) ->u8 ,
}


custom_error!{pub BCMError 
    InitFailed = "BCM init failed."
}
pub struct BCM {
    api: Container<BCMApi>,
}

impl BCM {
    pub fn new() -> BCM {
        BCM {
            api: BCM::init(),
        }
    }

    fn init() -> Container<BCMApi> {
        let bcm: Container<BCMApi> = unsafe { Container::load("/usr/local/lib/libbcm2835.so") }
            .expect("Could not open library or load symbols");
        unsafe {
            bcm.bcm2835_init();
        }
        bcm
    }

    pub fn bcm2835_init(&self) -> Result<u8, BCMError>{
        let result: u8 = unsafe {
           self.api.bcm2835_init()
        };
        if result == 0 {

            return Err(BCMError::InitFailed);
        }

        Ok(result)
    }

    pub fn bcm2835_spi_begin(&self) {
        unsafe {
            self.api.bcm2835_spi_begin();
        }
    }

    pub fn bcm2835_spi_setBitOrder(&self, order: u8) {
        unsafe {
            self.api.bcm2835_spi_setBitOrder(order);
        }
    }
    pub fn bcm2835_spi_setDataMode(&self, mode: u8) {
        unsafe {
            self.api.bcm2835_spi_setDataMode(mode);
        }
    }
    pub fn bcm2835_spi_setClockDivider(&self, divider: u16) {
        unsafe {
            self.api.bcm2835_spi_setClockDivider(divider);
        }
    }

    pub fn bcm2835_gpio_fsel(&self, pin: u8, mode: u8) {
        unsafe {
            self.api.bcm2835_gpio_fsel(pin, mode);
        }
    }
    pub fn bcm2835_gpio_write(&self, pin: u8, on: u8) {
        unsafe {
            self.api.bcm2835_gpio_write(pin, on);
        }
    }
        pub fn bcm2835_delay(&self, millis: u16) {
        unsafe {
            self.api.bcm2835_delay(millis);
        }
    }

    pub fn bcm2835_spi_end(&self) {
        unsafe {
            self.api.bcm2835_spi_end();
        }
    }

    pub fn bcm2835_close(&self) -> u8 {
        unsafe {
            self.api.bcm2835_close()
        }
    }

    pub fn bcm2835_spi_transfer(&self, value: u8) -> u8{
        
        
        unsafe {
            self.api.bcm2835_spi_transfer(value)
        }

    }
    pub fn bcm2835_gpio_lev(&self, pin: u8) -> u8{
        
        
        unsafe {
            self.api.bcm2835_gpio_lev(pin)
        }

    }

        

}