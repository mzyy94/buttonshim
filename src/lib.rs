use rppal::i2c::I2c;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

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

    pub fn set_pixel(&mut self, r: u8, g: u8, b: u8) -> Result<usize, rppal::i2c::Error> {
        self.led.set_color(r, g, b);
        self.led.apply()
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

#[derive(Copy, Clone, Debug)]
pub enum Button {
    A(State),
    B(State),
    C(State),
    D(State),
    E(State),
}

impl Button {
    fn from_index(i: usize, state: State) -> Option<Self> {
        match i {
            0 => Some(Button::A(state)),
            1 => Some(Button::B(state)),
            2 => Some(Button::C(state)),
            3 => Some(Button::D(state)),
            4 => Some(Button::E(state)),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum State {
    Released,
    Pressed(Instant),
    Hold,
}

#[derive(Debug)]
pub struct Buttons {
    i2c: Arc<Mutex<I2c>>,
    buttons: Arc<Mutex<Vec<State>>>,
    hold_threshold: Duration,
    sender: Option<Sender<Button>>,
}

impl Buttons {
    const REG_INPUT: u8 = 0x00;

    fn new(i2c: Arc<Mutex<I2c>>) -> Self {
        Buttons {
            i2c,
            buttons: Arc::new(Mutex::new(vec![State::Released; 5])),
            hold_threshold: Duration::from_secs(2),
            sender: None,
        }
    }

    pub fn a(&self) -> State {
        self.buttons.lock().unwrap()[0]
    }
    pub fn b(&self) -> State {
        self.buttons.lock().unwrap()[1]
    }
    pub fn c(&self) -> State {
        self.buttons.lock().unwrap()[2]
    }
    pub fn d(&self) -> State {
        self.buttons.lock().unwrap()[3]
    }
    pub fn e(&self) -> State {
        self.buttons.lock().unwrap()[4]
    }

    fn get_state(
        i2c: &Arc<Mutex<I2c>>,
        current: Vec<State>,
        hold_threshold: Duration,
    ) -> Vec<State> {
        let mut buttons = vec![State::Released; 5];

        let state = i2c
            .lock()
            .unwrap()
            .smbus_read_byte(Self::REG_INPUT)
            .unwrap();

        for i in 0..5 {
            if state & (0b00001 << i) == 0 {
                buttons[i] = match current[i] {
                    State::Released => State::Pressed(Instant::now()),
                    State::Pressed(pressed) => {
                        if pressed.elapsed() > hold_threshold {
                            State::Hold
                        } else {
                            State::Pressed(pressed)
                        }
                    }
                    State::Hold => State::Hold,
                }
            } else {
                buttons[i] = State::Released
            }
        }
        buttons
    }

    pub fn update(&mut self) -> () {
        let i2c = &self.i2c;
        let current: Vec<_> = self.buttons.lock().unwrap().clone();
        *self.buttons.lock().unwrap() = Self::get_state(i2c, current, self.hold_threshold);
    }

    pub fn start_polling(&self, interval: Duration) {
        let i2c = Arc::clone(&self.i2c);
        let buttons = Arc::clone(&self.buttons);
        let hold_threshold = self.hold_threshold;
        let sender = self.sender.as_ref().map(Sender::clone);

        thread::spawn(move || loop {
            let current = buttons.lock().unwrap().clone();
            let now = Self::get_state(&i2c, current.clone(), hold_threshold);
            *buttons.lock().unwrap() = now.clone();
            if let Some(ref sender) = sender {
                for i in 0..5 {
                    if current[i] != now[i] {
                        if let Some(button) = Button::from_index(i, now[i]) {
                            sender.send(button).unwrap();
                        }
                    }
                }
            }

            thread::sleep(interval)
        });
    }

    pub fn set_sender(&mut self, sender: Sender<Button>) {
        self.sender = Some(sender);
    }
}
