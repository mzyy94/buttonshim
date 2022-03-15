use buttonshim::{Button, ButtonShim, State};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

fn main() {
    println!(
        r#"
    Button SHIM: rainbow.rs

    Light up the LED a different colour of the rainbow with each button pressed.

    Press Ctrl+C to exit.

    "#
    );

    let mut buttonshim = ButtonShim::new();
    let (tx, rx) = mpsc::channel();
    buttonshim.buttons.set_sender(tx);

    let mut led = buttonshim.led.clone();
    thread::spawn(move || loop {
        let item = rx.recv().unwrap();
        match item {
            Button::A(State::Pressed(_)) => led.set_pixel(0x94, 0x00, 0xd3).unwrap(),
            Button::B(State::Pressed(_)) => led.set_pixel(0x00, 0x00, 0xff).unwrap(),
            Button::C(State::Pressed(_)) => led.set_pixel(0x00, 0xff, 0x00).unwrap(),
            Button::D(State::Pressed(_)) => led.set_pixel(0xff, 0xff, 0x00).unwrap(),
            Button::E(State::Pressed(_)) => led.set_pixel(0xff, 0x00, 0x00).unwrap(),
            _ => 0,
        };
    });
    buttonshim.buttons.start_polling(Duration::from_millis(100));

    buttonshim.led.set_pixel(0, 0, 0).unwrap();
    thread::park();
}
