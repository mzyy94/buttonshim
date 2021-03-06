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

    pub fn new() -> Result<Self, rppal::i2c::Error> {
        let i2c = I2c::new()?;
        Self::with_i2c(i2c)
    }

    pub fn with_i2c(mut i2c: I2c) -> Result<Self, rppal::i2c::Error> {
        i2c.set_timeout(100)?;
        i2c.set_slave_address(Self::ADDR)?;

        i2c.smbus_write_byte(Self::REG_CONFIG, 0b00011111)?;
        i2c.smbus_write_byte(Self::REG_POLARITY, 0b00000000)?;
        i2c.smbus_write_byte(Self::REG_OUTPUT, 0b00000000)?;

        let i2c = Arc::new(Mutex::new(i2c));

        Ok(ButtonShim {
            led: Led::new(Arc::clone(&i2c)),
            buttons: Buttons::new(Arc::clone(&i2c)),
        })
    }

    pub fn set_pixel(&mut self, r: u8, g: u8, b: u8) -> Result<usize, rppal::i2c::Error> {
        self.led.set_pixel(r, g, b)
    }
}
