use rppal::i2c::I2c;
use std::sync::{Arc, Mutex};

mod buttons;
mod led;

pub use buttons::*;
pub use led::*;

#[derive(Clone, Debug)]
pub struct ButtonShim {
    pub led: Led,
    pub buttons: Buttons,
}

impl ButtonShim {
    const ADDR: u16 = 0x3f;

    const REG_OUTPUT: u8 = 0x01;
    const REG_POLARITY: u8 = 0x02;
    const REG_CONFIG: u8 = 0x03;

    pub fn new() -> Self {
        let mut i2c = I2c::new().unwrap();
        i2c.set_timeout(100).unwrap();
        i2c.set_slave_address(Self::ADDR).unwrap();

        i2c.smbus_write_byte(Self::REG_CONFIG, 0b00011111).unwrap();
        i2c.smbus_write_byte(Self::REG_POLARITY, 0b00000000)
            .unwrap();
        i2c.smbus_write_byte(Self::REG_OUTPUT, 0b00000000).unwrap();

        let i2c = Arc::new(Mutex::new(i2c));

        ButtonShim {
            led: Led::new(Arc::clone(&i2c)),
            buttons: Buttons::new(Arc::clone(&i2c)),
        }
    }

    pub fn set_pixel(&mut self, r: u8, g: u8, b: u8) -> Result<usize, rppal::i2c::Error> {
        self.led.set_color(r, g, b);
        self.led.apply()
    }
}
