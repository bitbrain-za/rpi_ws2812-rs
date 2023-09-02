mod ws2812;
use crate::ws2812::{Rgb, Strip};
use rppal::spi::{Bus, SlaveSelect};

fn main() {
    let mut led = Strip::new(Bus::Spi0, SlaveSelect::Ss0, 55);
    led.clear(0);
    // led.clear(1);
    led.fill(0, Rgb::red());

    // loop {
    led.refresh(0).expect("Error displaying LED");
    // std::thread::sleep(std::time::Duration::from_millis(2000));
    // }

    // led.test();
}
