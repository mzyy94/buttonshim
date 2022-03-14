use rppal::i2c::I2c;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
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
}

#[derive(Debug)]
pub struct Led {
    i2c: Arc<Mutex<I2c>>,
    buffer: Vec<u8>,
}

impl Led {
    const REG_OUTPUT: u8 = 0x01;

    const PIN_LED_DATA: u8 = 7;
    const PIN_LED_CLOCK: u8 = 6;

    fn new(i2c: Arc<Mutex<I2c>>) -> Self {
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

#[derive(Copy, Clone, Debug)]
pub enum State {
    Released,
    Pressed,
}

#[derive(Debug)]
pub struct Buttons {
    i2c: Arc<Mutex<I2c>>,
    buttons: Vec<State>,
}

impl Buttons {
    const REG_INPUT: u8 = 0x00;

    fn new(i2c: Arc<Mutex<I2c>>) -> Self {
        Buttons {
            i2c,
            buttons: vec![State::Released; 5],
        }
    }

    pub fn a(&self) -> State {
        self.buttons[0]
    }
    pub fn b(&self) -> State {
        self.buttons[1]
    }
    pub fn c(&self) -> State {
        self.buttons[2]
    }
    pub fn d(&self) -> State {
        self.buttons[3]
    }
    pub fn e(&self) -> State {
        self.buttons[4]
    }

    pub fn update(&mut self) -> () {
        let i2c = &self.i2c;

        let state = i2c
            .lock()
            .unwrap()
            .smbus_read_byte(Self::REG_INPUT)
            .unwrap();

        for i in 0..5 {
            if state & (0b00001 << i) == 0 {
                self.buttons[i] = State::Pressed
            } else {
                self.buttons[i] = State::Released
            }
        }
    }
}
