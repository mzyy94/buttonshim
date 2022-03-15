use rppal::i2c::I2c;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct Led {
    i2c: Arc<Mutex<I2c>>,
    color: ColorBuffer,
}

#[derive(Clone, Debug, Default)]
struct ColorBuffer(Vec<u8>);

impl Led {
    pub fn new(i2c: Arc<Mutex<I2c>>) -> Self {
        Led {
            i2c,
            color: ColorBuffer::default(),
        }
    }

    pub fn apply(&self) -> Result<usize, rppal::i2c::Error> {
        let i2c = &self.i2c;

        i2c.lock().unwrap().write(self.color.buffer())
    }

    pub fn set_pixel(&mut self, r: u8, g: u8, b: u8) -> Result<usize, rppal::i2c::Error> {
        self.color.set_color(r, g, b);
        self.apply()
    }
}

impl ColorBuffer {
    const REG_OUTPUT: u8 = 0x01;

    const PIN_LED_DATA: u8 = 7;
    const PIN_LED_CLOCK: u8 = 6;

    fn buffer(&self) -> &Vec<u8> {
        &self.0
    }

    fn set_color(&mut self, r: u8, g: u8, b: u8) {
        self.0 = vec![Self::REG_OUTPUT, 0u8];
        self.write_byte(0);
        self.write_byte(0);
        self.write_byte(0xef);
        self.write_byte(b);
        self.write_byte(g);
        self.write_byte(r);
        self.write_byte(0);
        self.write_byte(0);
    }

    fn next(&mut self) {
        self.0.push(*self.0.last().unwrap())
    }

    fn set_bit(&mut self, pin: u8, value: u8) {
        let len = self.0.len();
        if value != 0 {
            self.0[len - 1] |= 1 << pin;
        } else {
            self.0[len - 1] &= 0xff ^ (1 << pin);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_buffer() {
        let mut color = ColorBuffer::default();
        color.set_color(0x12, 0x34, 0x56);

        assert_eq!(
            color.buffer(),
            &vec![
                0x01, 0x00, 0x00, 0x40, 0x00, 0x40, 0x00, 0x40, 0x00, 0x40, 0x00, 0x40, 0x00, 0x40,
                0x00, 0x40, 0x00, 0x40, 0x00, 0x40, 0x00, 0x40, 0x00, 0x40, 0x00, 0x40, 0x00, 0x40,
                0x00, 0x40, 0x00, 0x40, 0x00, 0x40, 0x80, 0xc0, 0x80, 0xc0, 0x80, 0xc0, 0x00, 0x40,
                0x80, 0xc0, 0x80, 0xc0, 0x80, 0xc0, 0x80, 0xc0, 0x00, 0x40, 0x80, 0xc0, 0x00, 0x40,
                0x80, 0xc0, 0x00, 0x40, 0x80, 0xc0, 0x80, 0xc0, 0x00, 0x40, 0x00, 0x40, 0x00, 0x40,
                0x80, 0xc0, 0x80, 0xc0, 0x00, 0x40, 0x80, 0xc0, 0x00, 0x40, 0x00, 0x40, 0x00, 0x40,
                0x00, 0x40, 0x00, 0x40, 0x80, 0xc0, 0x00, 0x40, 0x00, 0x40, 0x80, 0xc0, 0x00, 0x40,
                0x00, 0x40, 0x00, 0x40, 0x00, 0x40, 0x00, 0x40, 0x00, 0x40, 0x00, 0x40, 0x00, 0x40,
                0x00, 0x40, 0x00, 0x40, 0x00, 0x40, 0x00, 0x40, 0x00, 0x40, 0x00, 0x40, 0x00, 0x40,
                0x00, 0x40, 0x00, 0x40
            ]
        );
    }
}
