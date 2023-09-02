mod ws2812;
use crate::ws2812::{Rgb, Strip};
use rppal::spi::{Bus, SlaveSelect};

use palette::{FromColor, Hsl, ShiftHueAssign, Srgb};

fn main() {
    let mut led = Strip::new(Bus::Spi0, SlaveSelect::Ss0, 55, 2).expect("Error creating strip");
    let _ = led.clear(0);
    let _ = led.clear(1);

    let start_red = Srgb::new(1.0, 0.0, 0.0);
    let mut hsv = Hsl::from_color(start_red);
    loop {
        for _ in 0..360 {
            let srgb = (Srgb::from_color(hsv)).into_format::<u8>();
            let rgb = Rgb::new(srgb.red, srgb.green, srgb.blue);
            hsv.shift_hue_assign(1.0);
            led.fill(0, &rgb).expect("Error filling");
            led.refresh(0).expect("Error displaying LED");

            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}
