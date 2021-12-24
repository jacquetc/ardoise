use std::error::Error;
use std::ops::Drop;
use std::time;
extern crate custom_error;
use custom_error::custom_error;
use std::time::Instant;

mod bcm_interface;
use bcm_interface::BCM;

static CS: u8 = 8;
static HRDY: u8 = 24;
static RESET: u8 = 17;
static VCOM: u16 = 1610; //e.g. -1.53 = 1530 = 0x5FA

// copied from bcm2835.h
static BCM2835_SPI_BIT_ORDER_MSBFIRST: u8 = 1;
static BCM2835_SPI_MODE0: u8 = 0;
static BCM2835_SPI_CLOCK_DIVIDER_32: u16 = 32;
static BCM2835_GPIO_FSEL_OUTP: u8 = 0x01;
static BCM2835_GPIO_FSEL_INPT: u8 = 0x00;
static HIGH: u8 = 0x1;
static LOW: u8 = 0x0;

//Built in I80 Command Code
static IT8951_TCON_SYS_RUN: u16 = 0x0001;
static IT8951_TCON_STANDBY: u16 = 0x0002;
static IT8951_TCON_SLEEP: u16 = 0x0003;
static IT8951_TCON_REG_RD: u16 = 0x0010;
static IT8951_TCON_REG_WR: u16 = 0x0011;
static IT8951_TCON_MEM_BST_RD_T: u16 = 0x0012;
static IT8951_TCON_MEM_BST_RD_S: u16 = 0x0013;
static IT8951_TCON_MEM_BST_WR: u16 = 0x0014;
static IT8951_TCON_MEM_BST_END: u16 = 0x0015;
static IT8951_TCON_LD_IMG: u16 = 0x0020;
static IT8951_TCON_LD_IMG_AREA: u16 = 0x0021;
static IT8951_TCON_LD_IMG_END: u16 = 0x0022;

//I80 User defined command code
static USDEF_I80_CMD_DPY_AREA: u16 = 0x0034;
static USDEF_I80_CMD_GET_DEV_INFO: u16 = 0x0302;
static USDEF_I80_CMD_DPY_BUF_AREA: u16 = 0x0037;
static USDEF_I80_CMD_VCOM: u16 = 0x0039;

//Panel
static IT8951_PANEL_WIDTH: u16 = 1024; //it Get Device information
static IT8951_PANEL_HEIGHT: u16 = 758;

//Rotate mode
static IT8951_ROTATE_0: u16 = 0;
static IT8951_ROTATE_90: u16 = 1;
static IT8951_ROTATE_180: u16 = 2;
static IT8951_ROTATE_270: u16 = 3;

//Pixel mode , BPP - Bit per Pixel
static IT8951_2BPP: u16 = 0;
static IT8951_3BPP: u16 = 1;
static IT8951_4BPP: u16 = 2;
static IT8951_8BPP: u16 = 3;

//Waveform Mode
static IT8951_MODE_0: u16 = 0;
static IT8951_MODE_1: u16 = 1;
static IT8951_MODE_2: u16 = 2;
static IT8951_MODE_3: u16 = 3;
static IT8951_MODE_4: u16 = 4;
//Endian Type
static IT8951_LDIMG_L_ENDIAN: u16 = 0;
static IT8951_LDIMG_B_ENDIAN: u16 = 1;
//Auto LUT
static IT8951_DIS_AUTO_LUT: u16 = 0;
static IT8951_EN_AUTO_LUT: u16 = 1;
//LUT Engine Status
static IT8951_ALL_LUTE_BUSY: u16 = 0xFFFF;

//-----------------------------------------------------------------------
// IT8951 TCon Registers defines
//-----------------------------------------------------------------------
//Register Base Address
static DISPLAY_REG_BASE: u16 = 0x1000; //Register RW access for I80 only
                                       //Base Address of Basic LUT Registers
static LUT0EWHR: u16 = (DISPLAY_REG_BASE + 0x00); //LUT0 Engine Width Height Reg
static LUT0XYR: u16 = (DISPLAY_REG_BASE + 0x40); //LUT0 XY Reg
static LUT0BADDR: u16 = (DISPLAY_REG_BASE + 0x80); //LUT0 Base Address Reg
static LUT0MFN: u16 = (DISPLAY_REG_BASE + 0xC0); //LUT0 Mode and Frame number Reg
static LUT01AF: u16 = (DISPLAY_REG_BASE + 0x114); //LUT0 and LUT1 Active Flag Reg
                                                  //Update Parameter Setting Register
static UP0SR: u16 = (DISPLAY_REG_BASE + 0x134); //Update Parameter0 Setting Reg

static UP1SR: u16 = (DISPLAY_REG_BASE + 0x138); //Update Parameter1 Setting Reg
static LUT0ABFRV: u16 = (DISPLAY_REG_BASE + 0x13C); //LUT0 Alpha blend and Fill rectangle Value
static UPBBADDR: u16 = (DISPLAY_REG_BASE + 0x17C); //Update Buffer Base Address
static LUT0IMXY: u16 = (DISPLAY_REG_BASE + 0x180); //LUT0 Image buffer X/Y offset Reg
static LUTAFSR: u16 = (DISPLAY_REG_BASE + 0x224); //LUT Status Reg (status of All LUT Engines)
static BGVR: u16 = (DISPLAY_REG_BASE + 0x250); //Bitmap (1bpp) image color table
                                               //-------System Registers----------------
static SYS_REG_BASE: u16 = 0x0000;

//Address of System Registers
static I80CPCR: u16 = (SYS_REG_BASE + 0x04);
//-------Memory Converter Registers----------------
static MCSR_BASE_ADDR: u16 = 0x0200;
static MCSR: u16 = (MCSR_BASE_ADDR + 0x0000);
static LISAR: u16 = (MCSR_BASE_ADDR + 0x0008);

custom_error! {ITError
    InitFailed = "IT init failed.",
    DowncastFailed = "Downcast failed"
}

pub struct IT {
    _bcm_interface: BCM,
    _dev_info: DevInfo,
    _frame_buffer: Vec<u8>,
}

impl Drop for IT {
    fn drop(&mut self) {
        /*
                 self._bcm_interface.bcm2835_spi_end();
        self._bcm_interface.bcm2835_close();  */
    }
}

impl IT {
    pub fn new() -> Result<IT, Box<Error>> {
        let mut it: IT = Self::init()?;

        //Get Device Info
        let dev_info = it.get_system_info()?;

        it._dev_info = dev_info;
        // initialize frame buffer
        let size: usize = ((it._dev_info.panel_height) as usize
            * (it._dev_info.panel_width) as usize
            / 2) as usize;
        let frame_buffer: Vec<u8> = vec![0; size];
        it._frame_buffer = frame_buffer;

        /*

           gpFrameBuf = malloc(dev_info.panel_width * dev_info.panel_height);
           if !gpFrameBuf
           {
               return Err("malloc error!");
           }

            gulImgBufAddr = gstI80DevInfo.usImgBufAddrL | (gstI80DevInfo.usImgBufAddrH << 16);

        */

        //Set to Enable I80 Packed mode
        it.write_reg(I80CPCR, 0x0001);

        if VCOM != it.get_VCOM() {
            it.set_VCOM(VCOM);
            println!("VCOM = {}", it.get_VCOM());
        }

        Ok(it)
    }

    fn init() -> Result<IT, Box<Error>> {
        let bcm_interface: BCM = BCM::new();

        bcm_interface.bcm2835_init()?;

        bcm_interface.bcm2835_spi_begin();
        bcm_interface.bcm2835_spi_setBitOrder(BCM2835_SPI_BIT_ORDER_MSBFIRST); //default
        bcm_interface.bcm2835_spi_setDataMode(BCM2835_SPI_MODE0); //default
        bcm_interface.bcm2835_spi_setClockDivider(BCM2835_SPI_CLOCK_DIVIDER_32); //default

        bcm_interface.bcm2835_gpio_fsel(CS, BCM2835_GPIO_FSEL_OUTP);
        bcm_interface.bcm2835_gpio_fsel(HRDY, BCM2835_GPIO_FSEL_INPT);
        bcm_interface.bcm2835_gpio_fsel(RESET, BCM2835_GPIO_FSEL_OUTP);
        bcm_interface.bcm2835_gpio_write(CS, HIGH);

        bcm_interface.bcm2835_gpio_write(RESET, LOW);
        bcm_interface.bcm2835_delay(100);
        bcm_interface.bcm2835_gpio_write(RESET, HIGH);

        // dummy DevInfo :
        let dev_info: DevInfo = DevInfo {
            panel_width: 0,
            panel_height: 0,
            image_buffer_base_address_l: 0,
            image_buffer_base_address_h: 0,
            firmware_version: [0; 8],
            lut_version: [0; 8],
        };

        Ok(IT {
            _bcm_interface: bcm_interface,
            _dev_info: dev_info,
            _frame_buffer: Vec::new(),
        })
    }

    pub fn size(&self) -> (u16, u16) {

        println!("panel size : {}, {}", self._dev_info.panel_width, self._dev_info.panel_height);

        (self._dev_info.panel_width, self._dev_info.panel_height)
    }

    fn get_system_info(&self) -> Result<DevInfo, Box<Error>> {
        //Send I80 CMD
        self.lcd_write_cmd_code(USDEF_I80_CMD_GET_DEV_INFO);
        //Burst Read Request for SPI interface only
        let boxed_data: Box<[u16]> =
            self.lcd_read_n_data(((std::mem::size_of::<DevInfo>()) / 2) as u32); //Polling HRDY for each words(2-bytes) if possible
        let dev_info_ptr = Box::into_raw(boxed_data) as *mut DevInfo;
        let boxed_dev_info: Box<DevInfo> = unsafe { Box::from_raw(dev_info_ptr) };

        let dev_info = *boxed_dev_info;

        println!(
            "Panel(W,H) = ({},{})",
            dev_info.panel_width, dev_info.panel_height
        );

        let base_address: u32 = (dev_info.image_buffer_base_address_l as u32)
            | ((dev_info.image_buffer_base_address_h as u32) << 16);
        println!("Image Buffer Address = {}", base_address);
        //Show Firmware and LUT Version
        println!("FW Version = {:?}", dev_info.firmware_version);
        println!("LUT Version = {:?}", dev_info.lut_version);

        Ok(dev_info)
    }

    fn read_reg(&self, reg_address: u16) -> u16 {
        //Send Cmd , Register Address and Write Value
        self.lcd_write_cmd_code(IT8951_TCON_REG_RD);
        self.lcd_write_data(reg_address);
        self.lcd_read_data()
    }

    fn write_reg(&self, reg_address: u16, value: u16) {
        //Send Cmd , Register Address and Write Value
        self.lcd_write_cmd_code(IT8951_TCON_REG_WR);
        self.lcd_write_data(reg_address);
        self.lcd_write_data(value);
    }

    fn get_VCOM(&self) -> u16 {
        self.lcd_write_cmd_code(USDEF_I80_CMD_VCOM);
        self.lcd_write_data(0);
        //Read data from Host Data bus
        let vcom: u16 = self.lcd_read_data();
        vcom
    }
    fn set_VCOM(&self, vcom: u16) {
        self.lcd_write_cmd_code(USDEF_I80_CMD_VCOM);
        self.lcd_write_data(1);
        //Read data from Host Data bus
        self.lcd_write_data(vcom);
    }

    fn lcd_write_cmd_code(&self, cmd_code: u16) {
        //Set Preamble for Write Command
        let w_preamble: u16 = 0x6000;

        self.lcd_wait_for_ready();

        self._bcm_interface.bcm2835_gpio_write(CS, LOW);

        self._bcm_interface
            .bcm2835_spi_transfer((w_preamble >> 8) as u8);
        self._bcm_interface.bcm2835_spi_transfer(w_preamble as u8);

        //LCDWaitForReady();

        self._bcm_interface
            .bcm2835_spi_transfer((cmd_code >> 8) as u8);
        self._bcm_interface.bcm2835_spi_transfer(cmd_code as u8);

        self._bcm_interface.bcm2835_gpio_write(CS, HIGH);
    }
    fn lcd_send_cmd_arg(&self, cmd_code: u16, arg: [u16; 5], arg_number: u16) {
        self.lcd_write_cmd_code(cmd_code);

        for n in 0..arg_number {
            self.lcd_write_data(arg[n as usize]);
        }
    }

    fn lcd_write_data(&self, data: u16) {
        //Set Preamble for Write Data
        let w_preamble: u16 = 0x0000;

        self.lcd_wait_for_ready();

        self._bcm_interface.bcm2835_gpio_write(CS, LOW);

        self._bcm_interface
            .bcm2835_spi_transfer((w_preamble >> 8) as u8);
        self._bcm_interface.bcm2835_spi_transfer(w_preamble as u8);

        //LCDWaitForReady();

        self._bcm_interface.bcm2835_spi_transfer((data >> 8) as u8);
        self._bcm_interface.bcm2835_spi_transfer(data as u8);

        self._bcm_interface.bcm2835_gpio_write(CS, HIGH);
    }

    fn lcd_write_n_data(&self, word_count: u32) {
        //Set Preamble for Write Data
        let w_preamble: u16 = 0x0000;

        //self.lcd_wait_for_ready();

        self._bcm_interface.bcm2835_gpio_write(CS, LOW);

        self._bcm_interface
            .bcm2835_spi_transfer((w_preamble >> 8) as u8);
        self._bcm_interface.bcm2835_spi_transfer(w_preamble as u8);

        //LCDWaitForReady();
        let data_vec = self._frame_buffer.as_slice();
        //println!(" data_vec.len() {} wc: {}",  data_vec.len(), word_count);
        if data_vec.len() < (word_count as usize) {
            println!("len: {} , word_count: {}", data_vec.len(), word_count );
            panic!("data_vec too short");
        }
        let mut n: usize = 0;
        while n < (word_count as usize) {
            self._bcm_interface.bcm2835_spi_transfer(data_vec[n + 1]);
            self._bcm_interface.bcm2835_spi_transfer(data_vec[n]);
            n += 2;
        }

        self._bcm_interface.bcm2835_gpio_write(CS, HIGH);
        //println!("gg: {}", now.elapsed().as_millis());
    }

    fn lcd_read_data(&self) -> u16 {
        let w_preamble: u16 = 0x1000;
        self.lcd_wait_for_ready();

        self._bcm_interface.bcm2835_gpio_write(CS, LOW);

        self._bcm_interface
            .bcm2835_spi_transfer((w_preamble >> 8) as u8);
        self._bcm_interface.bcm2835_spi_transfer(w_preamble as u8);
        self.lcd_wait_for_ready();

        let mut read_data: u16 = self._bcm_interface.bcm2835_spi_transfer(0x00) as u16; //dummy
        read_data = self._bcm_interface.bcm2835_spi_transfer(0x00) as u16; //dummy
        self.lcd_wait_for_ready();

        read_data = (self._bcm_interface.bcm2835_spi_transfer(0x00) as u16) << 8;
        read_data |= self._bcm_interface.bcm2835_spi_transfer(0x00) as u16;

        self._bcm_interface.bcm2835_gpio_write(CS, HIGH);
        read_data
    }

    fn lcd_read_n_data(&self, word_count: u32) -> Box<[u16]> {
        //Set Preamble for Write Data
        let w_preamble: u16 = 0x1000;
        self.lcd_wait_for_ready();

        self._bcm_interface.bcm2835_gpio_write(CS, LOW);

        self._bcm_interface
            .bcm2835_spi_transfer((w_preamble >> 8) as u8);
        self._bcm_interface.bcm2835_spi_transfer(w_preamble as u8);
        self.lcd_wait_for_ready();

        // initialise:
        let mut data_vec: Vec<u16> = vec![0; word_count as usize];

        data_vec[0] = self._bcm_interface.bcm2835_spi_transfer(0x00).into(); //dummy
        data_vec[0] = self._bcm_interface.bcm2835_spi_transfer(0x00).into(); //dummy

        for n in 0..word_count {
            let mut word: u16 = (self._bcm_interface.bcm2835_spi_transfer(0x00) as u16) << 8;
            word = word ^ self._bcm_interface.bcm2835_spi_transfer(0x00) as u16;
            data_vec[n as usize] = word;
        }

        self._bcm_interface.bcm2835_gpio_write(CS, HIGH);

        let boxed: Box<[u16]> = data_vec.into_boxed_slice();
        boxed
    }

    fn lcd_wait_for_ready(&self) {
        let mut data: u8 = self._bcm_interface.bcm2835_gpio_lev(HRDY);
        while data == 0 {
            data = self._bcm_interface.bcm2835_gpio_lev(HRDY);
        }
    }

    fn load_image_area_start(&self, load_image_info: &LdImgInfo, area_image_info: &AreaImgInfo) {
        let mut arg: [u16; 5] = [0; 5];
        //Setting Argument for Load image start
        arg[0] = (load_image_info.endian_type << 8)
            | (load_image_info.pixel_format << 4)
            | (load_image_info.rotate);
        arg[1] = area_image_info.x;
        arg[2] = area_image_info.y;
        arg[3] = area_image_info.width;
        arg[4] = area_image_info.height;
        //Send Cmd and Args
        self.lcd_send_cmd_arg(IT8951_TCON_LD_IMG_AREA, arg, 5);
    }

    fn load_image_end(&self) {
        self.lcd_write_cmd_code(IT8951_TCON_LD_IMG_END);
    }

    fn write_host_area_packed_pixel(
        &self,
        load_image_info: &LdImgInfo,
        area_image_info: &AreaImgInfo,
        factor: u8,
    ) {
        //Send Load Image start Cmd
        self.load_image_area_start(load_image_info, area_image_info);

        let word_count: u32 = ((area_image_info.height as u32) * (area_image_info.width as u32))
            / (2 * factor as u32) as u32;

        self.lcd_write_n_data(word_count);

        self.load_image_end();
    }

    fn wait_for_display_ready(&self) {
        while self.read_reg(LUTAFSR) == 1 {}
    }

    fn display_area(&self, x: u16, y: u16, width: u16, height: u16, dpy_mode: u16) {
        //Send I80 Display Command (User defined command of IT8951)
        self.lcd_write_cmd_code(USDEF_I80_CMD_DPY_AREA); //0x0034
                                                         //Write arguments
        self.lcd_write_data(x);
        self.lcd_write_data(y);
        self.lcd_write_data(width);
        self.lcd_write_data(height);
        self.lcd_write_data(dpy_mode);
    }

    pub fn display(&self, x: u16, y: u16, rect_width: u16, rect_height: u16) {
        let width: u16 = self._dev_info.panel_width;
        let height: u16 = self._dev_info.panel_height;
        let mut rect_width = rect_width;
        let mut rect_height = rect_height;
        if rect_width > width {
            //println!("rectWidth > width");
            rect_width = width;
        }
        if rect_height > height {
            rect_height = height;
        }  
        //EPD_Clear(0xff);
        //EPD_FillRect(x, y, rectWidth, rectHeight, 0x00);

        //self.wait_for_display_ready();

        //println!("aa: {}", now.elapsed().as_millis());

        //Setting Load image information
        let load_image_info = LdImgInfo {
            endian_type: IT8951_LDIMG_L_ENDIAN,
            pixel_format: IT8951_4BPP,
            rotate: IT8951_ROTATE_0,
        };
        //Set Load Area
        let area_image_info = AreaImgInfo {
            x: x,
            y: y,
            width: rect_width,
            height: rect_height,
        };
        //Load Image from Host to IT8951 Image Buffer
        self.write_host_area_packed_pixel(&load_image_info, &area_image_info, 1); //Display function 2

        self.display_area(x, y, rect_width, rect_height, 2);
    }

    pub fn draw_buffer_pixel(&mut self, x: u16, y: u16, width: u16, color: u8) {
        let index: usize = y as usize * width as usize + x as usize;

        self._frame_buffer[index] = color;
    }

    pub fn load_buffer_from_vec(&mut self, grey_vec: Vec<u8>) {
        self._frame_buffer = grey_vec;
    }
}
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
struct LdImgInfo {
    endian_type: u16,  //little or Big Endian
    pixel_format: u16, //bpp
    rotate: u16,       //Rotate mode
}

//structure prototype 2
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
struct AreaImgInfo {
    x: u16,
    y: u16,
    width: u16,
    height: u16,
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
//#[repr(C)]
struct DevInfo {
    panel_width: u16,
    panel_height: u16,
    image_buffer_base_address_l: u16,
    image_buffer_base_address_h: u16,
    firmware_version: [u16; 8], //16 Bytes String
    lut_version: [u16; 8],      //16 Bytes String
}

#[ignore]
#[test]
fn test_init() {
    let it_result = IT::new();
    assert!(it_result.is_ok());

    let mut it = it_result.unwrap();
    assert_eq!(it._dev_info.panel_height, 1404);

    let mut grey_vec: Vec<u8> = vec![
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
    ];

    it.load_buffer_from_vec(grey_vec);
    it.display(0, 0, 16, 16);

    grey_vec = vec![
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b1111_1111,
        0b1111_1111,
        0b1111_1111,
        0b1111_1111,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
        0b0000_0000,
    ];

    it.load_buffer_from_vec(grey_vec);
    it.display(40, 0, 16, 16);

    // fn test_vertical_line() {

    let white: u8 = 0b1111_1111;
    let black: u8 = 0b0000_0000;
    grey_vec = vec![white; 8000];
    let mut line: u32 = 0;
    for n in 0..8000 {
        if n == (200 * line + 100) {
            grey_vec[n as usize] = black;
        }

        if (200 * 200) < n && n < (200 * 201) {
            grey_vec[n as usize] = black;
        }
        line = (n as u32).div_euclid(200);
    }

    it.load_buffer_from_vec(grey_vec);
    it.display(500, 500, 400, 400);
}
