use rppal::i2c::I2c;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

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
    Clicked,
}

impl State {
    fn pressing(self, hold_threshold: Duration) -> Self {
        match self {
            State::Released | State::Clicked => State::Pressed(Instant::now()),
            State::Pressed(pressed) => {
                if pressed.elapsed() > hold_threshold {
                    State::Hold
                } else {
                    State::Pressed(pressed)
                }
            }
            State::Hold => State::Hold,
        }
    }

    fn releasing(self, hold_threshold: Duration) -> Self {
        match self {
            State::Pressed(pressed) => {
                if pressed.elapsed() < hold_threshold {
                    State::Clicked
                } else {
                    State::Released
                }
            }
            _ => State::Released,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Buttons {
    i2c: Arc<Mutex<I2c>>,
    buttons: Arc<Mutex<Vec<State>>>,
    hold_threshold: Duration,
    sender: Option<Sender<Button>>,
}

impl Buttons {
    const REG_INPUT: u8 = 0x00;

    pub fn new(i2c: Arc<Mutex<I2c>>) -> Self {
        Buttons {
            i2c,
            buttons: Arc::new(Mutex::new(vec![State::Released; 5])),
            hold_threshold: Duration::from_secs(2),
            sender: None,
        }
    }

    pub fn set_hold_threshold(&mut self, hold_threshold: Duration) {
        self.hold_threshold = hold_threshold;
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
                buttons[i] = current[i].pressing(hold_threshold)
            } else {
                buttons[i] = current[i].releasing(hold_threshold)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn change_state() {
        let state = State::Released;

        let state = state.pressing(Duration::from_millis(2000));
        assert!(match state {
            State::Pressed(_) => true,
            _ => false,
        });

        thread::sleep(Duration::from_millis(10));

        let state = state.pressing(Duration::from_millis(1));
        assert_eq!(state, State::Hold);

        let state = state.releasing(Duration::from_millis(1));
        assert_eq!(state, State::Released);

        let state = state.pressing(Duration::from_millis(100));
        thread::sleep(Duration::from_millis(10));
        let state = state.releasing(Duration::from_millis(100));
        assert_eq!(state, State::Clicked);
    }
}
