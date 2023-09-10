use crate::ws2812::{Rgb, Strip};
use palette::{FromColor, Hsv};
use smart_led_effects::strip::EffectIterator;
use smart_led_effects::{strip, Srgb};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum RunMode {
    Static(f32, f32),
    Dynamic(String),
    Off,
}

pub struct MyStrip {
    pub mode: RunMode,
    pub brightness: f32,

    effects_map: HashMap<String, Box<dyn EffectIterator>>,
    previous_mode: RunMode,
    previous_brightness: f32,

    strip: Strip,
}

impl MyStrip {
    pub fn new(count: usize, strip: Strip) -> Self {
        let mut effects_map: HashMap<String, Box<dyn EffectIterator>> = HashMap::new();
        effects_map.insert(
            "rainbow".to_string(),
            Box::new(strip::Rainbow::new(count, None)),
        );
        effects_map.insert(
            "bounce".to_string(),
            Box::new(strip::Bounce::new(count, None, None, None, None, None)),
        );
        effects_map.insert(
            "breathe".to_string(),
            Box::new(strip::Breathe::new(count, None, None)),
        );
        effects_map.insert(
            "cycle".to_string(),
            Box::new(strip::Cycle::new(count, None)),
        );
        effects_map.insert(
            "fire".to_string(),
            Box::new(strip::Fire::new(count, None, None)),
        );
        effects_map.insert(
            "meteor".to_string(),
            Box::new(strip::Meteor::new(count, None, None, None)),
        );
        effects_map.insert(
            "running_lights".to_string(),
            Box::new(strip::RunningLights::new(count, None, false)),
        );
        effects_map.insert(
            "cylon".to_string(),
            Box::new(strip::Cylon::new(
                count,
                Srgb::<u8>::new(255, 0, 0),
                None,
                None,
            )),
        );
        effects_map.insert(
            "timer".to_string(),
            Box::new(strip::Timer::new(
                count,
                std::time::Duration::from_millis(5000),
                None,
                None,
                None,
                true,
            )),
        );
        effects_map.insert(
            "twinkle".to_string(),
            Box::new(strip::Twinkle::new(count, None, None, None, None)),
        );
        effects_map.insert(
            "sparkle".to_string(),
            Box::new(strip::Twinkle::sparkle(count, None)),
        );
        effects_map.insert(
            "snow".to_string(),
            Box::new(strip::SnowSparkle::new(count, None, None, None, None)),
        );
        effects_map.insert(
            "wipe".to_string(),
            Box::new(strip::Wipe::colour_wipe(
                count,
                Srgb::<u8>::new(0, 255, 0),
                false,
            )),
        );

        Self {
            mode: RunMode::Off,
            brightness: 1.0,
            effects_map,
            previous_mode: RunMode::Off,
            previous_brightness: 1.0,
            strip,
        }
    }

    pub fn turn_off(&mut self) {
        if self.mode != RunMode::Off {
            self.previous_mode = self.mode.clone();
            self.previous_brightness = self.brightness;
        }
        self.mode = RunMode::Off;
    }

    pub fn turn_on(&mut self) {
        if self.mode == RunMode::Off {
            self.mode = self.previous_mode.clone();
            self.brightness = self.previous_brightness;
        }
    }

    pub fn set_brightness(&mut self, brightness: f32) {
        self.brightness = brightness;
    }

    pub fn _set_hs(&mut self, h: f32, s: f32) {
        self.mode = RunMode::Static(h, s);
    }

    pub fn set_rgb(&mut self, r: u8, g: u8, b: u8) {
        let srgb: Srgb<u8> = Srgb::<u8>::new(r, g, b);
        let hsv = Hsv::from_color(srgb.into_format());
        self.brightness = hsv.value;
        self.mode = RunMode::Static(hsv.hue.into_inner(), hsv.saturation);
    }

    pub fn _get_hsv(&self) -> Option<Hsv<u8>> {
        match self.mode {
            RunMode::Static(h, s) => {
                let hsv = Hsv::new(h, s, self.brightness);
                Some(hsv)
            }
            RunMode::Dynamic(_) => None,
            RunMode::Off => None,
        }
    }

    pub fn set_temperature(&mut self, mired: u64) {
        let kelvin = 1000000 / mired as i64;
        let rgb: colortemp::RGB = colortemp::temp_to_rgb(kelvin);
        self.set_rgb(rgb.r as u8, rgb.g as u8, rgb.b as u8);
    }

    pub fn get_rgb(&self) -> Option<(u8, u8, u8)> {
        match self.mode {
            RunMode::Static(h, s) => {
                let hsv = Hsv::new(h, s, self.brightness);
                let srgb = Srgb::from_color(hsv).into_format::<u8>();
                Some((srgb.red, srgb.green, srgb.blue))
            }
            RunMode::Dynamic(_) => None,
            RunMode::Off => None,
        }
    }

    pub fn get_effect_pixels(&mut self) -> Option<Vec<Srgb<u8>>> {
        match &self.mode {
            RunMode::Dynamic(e) => match self.effects_map.get_mut(e) {
                Some(effect) => effect.next(),
                None => None,
            },
            _ => None,
        }
    }

    pub fn set_effect(&mut self, effect: &str) {
        log::debug!("Setting effect: {}", effect);
        self.mode = RunMode::Dynamic(effect.to_string());
    }

    pub fn list_effects(&self) -> Vec<String> {
        self.effects_map.keys().cloned().collect()
    }

    pub fn state_message(&self) -> String {
        let brightness = (self.brightness * 255.0) as u8;
        match &self.mode {
            RunMode::Static(_h, _s) => {
                let rgb = self.get_rgb().unwrap();
                let payload = format!(
                    "{{\"state\": \"ON\", \"brightness\": {}, \"color\": {{\"r\": {}, \"g\": {}, \"b\": {}}}}}",
                    brightness, rgb.0, rgb.1, rgb.2
                );
                payload
            }
            RunMode::Dynamic(e) => {
                let payload = format!(
                    "{{\"state\": \"ON\", \"brightness\": {}, \"effect\": \"{}\"}}",
                    brightness, e
                );
                payload
            }
            RunMode::Off => "{\"state\": \"OFF\"}".to_string(),
        }
    }

    pub fn update(&mut self) {
        match &self.mode {
            RunMode::Static(h, s) => {
                let hsv = Hsv::new(*h, *s, self.brightness);
                let srgb = Srgb::from_color(hsv).into_format::<u8>();
                let rgb = Rgb::new(srgb.red, srgb.green, srgb.blue);
                let _ = self.strip.clear(0);
                let _ = self.strip.fill(0, &rgb);
            }
            RunMode::Dynamic(e) => {
                let effect_name = e.to_lowercase();
                if let Some(effect) = self.effects_map.get_mut(&effect_name) {
                    let pixels = effect.next();

                    if let Some(pixels) = pixels {
                        let pixels = pixels
                            .iter()
                            .map(|x| Rgb::new(x.red, x.green, x.blue))
                            .collect::<Vec<Rgb>>();
                        self.strip.set_page(0, pixels).expect("Error setting page");
                    }
                }
            }
            RunMode::Off => {
                let _ = self.strip.clear(0);
            }
        }
        self.strip.refresh(0).expect("Error displaying LED");
    }
}
