#![no_std]
#![no_main]

// Imports
use core::fmt::Write;
use cortex_m::delay;
use cortex_m_rt::entry;
use panic_halt as _;
use stm32f4xx_hal::{
    i2c::Mode,
    pac::{self},
    prelude::*,
    serial::config::Config,
};

use defmt_rtt;


#[entry]
fn main() -> ! {
    // Setup handler for device peripherals
    let dp = pac::Peripherals::take().unwrap();

    // I2C Config steps:
    // 1) Need to configure the system clocks
    // - Promote RCC structure to HAL to be able to configure clocks
    let rcc = dp.RCC.constrain();
    // - Configure system clocks
    // 8 MHz must be used for the Nucleo-F401RE board according to manual
    let clocks = rcc.cfgr.use_hse(8.MHz()).freeze();
    // 2) Configure/Define SCL and SDA pins
    let gpiob = dp.GPIOB.split();
    let scl = gpiob.pb8;
    let sda = gpiob.pb9;
    // 3) Configure I2C perihperal channel
    // We're going to use I2C1 since its pins are the ones connected to the I2C interface we're using
    // To configure/instantiate serial peripheral channel we have two options:
    // Use the i2c device peripheral handle and instantiate a transmitter instance using extension trait
    let mut i2c = dp.I2C1.i2c(
        (scl, sda),
        Mode::Standard {
            frequency: 100.kHz(),
        },
        &clocks,
    );



   let mut delay = dp.TIM1.delay_ms(&clocks);

    struct Coeffs {
        ac5: i16,
        ac6: i16,
        mc: i16,
        md: i16,
    }

    let mut calib_coeffs = Coeffs {
        ac5: 0,
        ac6: 0,
        mc: 0,
        md: 0,
    };

    const BMP180_ADDR: u8 = 0x77;
    const REG_ID_ADDR: u8 = 0xD0;
    const AC5_MSB_ADDR: u8 = 0xB2;
    const AC6_MSB_ADDR: u8 = 0xB4;
    const MC_MSB_ADDR: u8 = 0xBC;
    const MD_MSB_ADDR: u8 = 0xBE;
    const CTRL_MEAS_ADDR: u8 = 0xF4;
    const MEAS_OUT_LSB_ADDR: u8 = 0xF7;
    const MEAS_OUT_MSB_ADDR: u8 = 0xF6;

    let mut rx_buffer: [u8; 2] = [0; 2];
    let mut rx_word: i16;

    // Read Device ID as Sanity Check
    i2c.write(BMP180_ADDR, &[REG_ID_ADDR]).unwrap();
    i2c.read(BMP180_ADDR, &mut rx_buffer).unwrap();



    // defmt::println!("hi");
    // OR 
    // i2c.write_read(BMP180_ADDR, &[REG_ID_ADDR], &mut rx_buffer)
    //     .unwrap();
    // if rx_buffer[0] == 0x55 {
    //     writeln!(tx, "Device ID is {}\r", rx_buffer[0]).unwrap();
    // } else {
    //     writeln!(tx, "Device ID Cannot be Detected \r").unwrap();
    // }

    // Read Calibration Coefficients
    // Read AC5
    i2c.write_read(BMP180_ADDR, &[AC5_MSB_ADDR], &mut rx_buffer)
        .unwrap();

    rx_word = ((rx_buffer[0] as i16) << 8) | rx_buffer[1] as i16;




    defmt::println!("Read Calibration Coefficients->  rx_word {:?}", rx_word);


    calib_coeffs.ac5 = rx_word;

    // Read AC6
    i2c.write_read(BMP180_ADDR, &[AC6_MSB_ADDR], &mut rx_buffer)
        .unwrap();
    rx_word = ((rx_buffer[0] as i16) << 8) | rx_buffer[1] as i16;

    defmt::println!("Read AC6->  rx_word {:?}", rx_word);

    calib_coeffs.ac6 = rx_word;

    // Read MC
    i2c.write_read(BMP180_ADDR, &[MC_MSB_ADDR], &mut rx_buffer)
        .unwrap();
    rx_word = ((rx_buffer[0] as i16) << 8) | rx_buffer[1] as i16;

    defmt::println!("Read Mc {:?}", rx_word);

    calib_coeffs.mc = rx_word;

    // Read MD
    i2c.write_read(BMP180_ADDR, &[MD_MSB_ADDR], &mut rx_buffer)
        .unwrap();
    rx_word = ((rx_buffer[0] as i16) << 8) | rx_buffer[1] as i16;

    defmt::println!("Read Md{:?}", rx_word);

    calib_coeffs.md = rx_word;

    // Application Loop
    loop {
        // Kick off Temperature Measurement by writing 0x2E in register 0xF4
        i2c.write(BMP180_ADDR, &[CTRL_MEAS_ADDR, 0x2E]).unwrap();
        // Wait 4.5 ms for measurment to complete as specified by the datasheet
        delay.delay_ms(5_u32);

        // Collect Temperature Measurment
        // Read Measurement MSB
        // Achieving same as above using an alternate method syntax here to do a write followed by read
        i2c.write(BMP180_ADDR, &[MEAS_OUT_MSB_ADDR]).unwrap();
        i2c.read(BMP180_ADDR, &mut rx_buffer).unwrap();
        rx_word = (rx_buffer[0] as i16) << 8;

        defmt::println!("loop msb rx_word {:?}", rx_word);

        // Read Measurement LSB
        i2c.write(BMP180_ADDR, &[MEAS_OUT_LSB_ADDR]).unwrap();
        i2c.read(BMP180_ADDR, &mut rx_buffer).unwrap();
        rx_word |= rx_buffer[0] as i16;


        defmt::println!("loop lsb rx_word {:?}", rx_word);



        // Calculate Temperature According to Datasheet Formulas
        let x1 = (rx_word as i32 - calib_coeffs.ac6 as i32) * (calib_coeffs.ac5 as i32) >> 15;
        let x2 = ((calib_coeffs.mc as i32) << 11) / (x1 + calib_coeffs.md as i32);
        let b5 = x1 + x2;
        let t = ((b5 + 8) >> 4) / 10;


        defmt::println!("temperature:-> {:?}", t);
        delay.delay_ms(1000_u32);

    }
}
