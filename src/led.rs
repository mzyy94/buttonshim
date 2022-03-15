use rppal::i2c::I2c;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct Led {
    i2c: Arc<Mutex<I2c>>,
    buffer: Vec<u8>,
}

impl Led {
    const REG_OUTPUT: u8 = 0x01;

    const PIN_LED_DATA: u8 = 7;
    const PIN_LED_CLOCK: u8 = 6;

    pub fn new(i2c: Arc<Mutex<I2c>>) -> Self {
        Led {
            i2c,
            buffer: vec![],
        }
    }

    pub fn apply(&self) -> Result<usize, rppal::i2c::Error> {
        let i2c = &self.i2c;

        i2c.lock().unwrap().write(&self.buffer)
    }

    pub fn set_color(&mut self, r: u8, g: u8, b: u8) {
        self.buffer = vec![Self::REG_OUTPUT, 0u8];
        self.write_byte(0);
        self.write_byte(0);
        self.write_byte(0xef);
        self.write_byte(b);
        self.write_byte(g);
        self.write_byte(r);
        self.write_byte(0);
        self.write_byte(0);
    }

    pub fn set_pixel(&mut self, r: u8, g: u8, b: u8) -> Result<usize, rppal::i2c::Error> {
        self.set_color(r, g, b);
        self.apply()
    }

    fn next(&mut self) {
        if self.buffer.len() == 0 {
            self.buffer = vec![0u8]
        } else {
            self.buffer.push(*self.buffer.last().unwrap())
        }
    }

    fn set_bit(&mut self, pin: u8, value: u8) {
        let len = self.buffer.len();
        if value != 0 {
            self.buffer[len - 1] |= 1 << pin;
        } else {
            self.buffer[len - 1] &= 0xff ^ (1 << pin);
        }
    }

    fn write_byte(&mut self, b: u8) {
        let mut b = b;
        for _ in 0..8 {
            self.next();
            self.set_bit(Self::PIN_LED_CLOCK, 0);
            self.set_bit(Self::PIN_LED_DATA, b & 0x80);
            self.next();
            self.set_bit(Self::PIN_LED_CLOCK, 1);
            b <<= 1;
        }
    }
}
