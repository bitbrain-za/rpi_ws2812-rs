use super::availability::Availability;
use super::device::Device;
use palette::{FromColor, Hsv, Srgb};
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

const MANUFACTURER: &str = "bitbrain";
const MODEL: &str = "lightstrip";
const SW_VERSION: &str = "3.2.0";
const NAME: &str = "Light";
const HW_VERSION: &str = "1.0.0";
const UNIQUE_ID: &str = "light-strip-1234";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LightStripMqtt {
    name: String,
    unique_id: String,
    device: Device,
    availability: Availability,
    pub state_topic: String,
    pub command_topic: String,
    brightness: bool,
    rgb: bool,
    color_temp: bool,
    effect: bool,
    pub effect_list: Vec<String>,
    schema: String,
    optimistic: bool,
    icon: String,
    retain: bool,
}

impl Default for LightStripMqtt {
    fn default() -> Self {
        let base_topic = Self::static_base_topic();
        LightStripMqtt {
            name: NAME.to_string(),
            unique_id: UNIQUE_ID.to_string(),
            device: Device {
                identifiers: vec![UNIQUE_ID.to_string()],
                manufacturer: MANUFACTURER.to_string(),
                model: MODEL.to_string(),
                name: NAME.to_string(),
                sw_version: SW_VERSION.to_string(),
                hw_version: HW_VERSION.to_string(),
            },
            availability: Availability::new(&base_topic),
            state_topic: format!("{}/state", &base_topic),
            command_topic: format!("{}/set", &base_topic),
            brightness: true,
            rgb: true,
            color_temp: true,
            effect: true,
            effect_list: vec!["test".to_string(), "test2".to_string()],
            schema: "json".to_string(),
            optimistic: false,
            icon: "mdi:lightbulb".to_string(),
            retain: true,
        }
    }
}

impl LightStripMqtt {
    fn _base_topic(&self) -> String {
        format!("{}/lightstrip/{}", MANUFACTURER, self.unique_id)
    }
    fn static_base_topic() -> String {
        format!("{}/lightstrip/{}", MANUFACTURER, UNIQUE_ID)
    }
    fn ha_discovery_topic(&self) -> String {
        format!("homeassistant/light/{}/config", self.unique_id)
    }

    pub fn discovery_message(&self) -> (String, String) {
        (
            self.ha_discovery_topic(),
            serde_json::to_string_pretty(&self).unwrap(),
        )
    }

    pub fn set_online(&self) -> (String, String) {
        self.availability.set_online()
    }

    pub fn _set_offline(&self) -> (String, String) {
        self.availability._set_offline()
    }

    pub fn _set_color(&self, r: u8, g: u8, b: u8) -> (String, String) {
        let payload = format!(
            "{{\"state\": \"ON\", \"color\": {{\"r\": {}, \"g\": {}, \"b\": {}}}}}",
            r, g, b
        );
        let topic = self.state_topic.clone();

        (topic, payload)
    }

    pub fn _set_brightness(&self, brightness: u8) -> (String, String) {
        let payload = format!("{{\"state\": \"ON\", \"brightness\": {}}}", brightness);
        let topic = self.state_topic.clone();

        (topic, payload)
    }

    pub fn _set_effect(&self, effect: &str) -> (String, String) {
        let payload = format!("{{\"state\": \"ON\", \"effect\": \"{}\"}}", effect);
        let topic = self.state_topic.clone();

        (topic, payload)
    }

    pub fn _set_off(&self) -> (String, String) {
        let payload = "{{\"state\": \"OFF\"}}".to_string();
        let topic = self.state_topic.clone();

        (topic, payload)
    }

    pub fn _set_color_temp(&self, color_temp: u16) -> (String, String) {
        let payload = format!("{{\"state\": \"ON\", \"color_temp\": {}}}", color_temp);
        let topic = self.state_topic.clone();

        (topic, payload)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StripMode {
    On,
    Colour(u8, u8, u8),
    Effect(String),
    Brightness(u8),
    Off,
}

impl StripMode {
    pub fn to_state_message(&self, mqtt: &LightStripMqtt) -> (String, String) {
        match self {
            StripMode::On => {
                let payload = "{\"state\": \"OFF\"}".to_string();
                let topic = mqtt.state_topic.clone();

                (topic, payload)
            }
            StripMode::Off => {
                let payload = "{\"state\": \"OFF\"}".to_string();
                let topic = mqtt.state_topic.clone();

                (topic, payload)
            }
            StripMode::Colour(r, g, b) => {
                let srgb: Srgb<u8> = Srgb::<u8>::new(*r, *g, *b);
                let hsv = Hsv::from_color(srgb.into_format());
                let value: f32 = hsv.value;
                let brightness: u8 = (value * 255.0) as u8;
                let payload = format!(
                    "{{\"state\": \"ON\", \"brightness\": {},  \"color\": {{\"r\": {}, \"g\": {}, \"b\": {}}}}}", 
                    brightness,
                    r, g, b
                );
                let topic = mqtt.state_topic.clone();

                (topic, payload)
            }
            StripMode::Effect(effect) => {
                let payload = format!(
                    "{{\"state\": \"ON\", \"brightness\": 255, \"effect\": \"{}\"}}",
                    effect
                );
                let topic = mqtt.state_topic.clone();

                (topic, payload)
            }
            StripMode::Brightness(brightness) => {
                let payload = format!("{{\"state\": \"ON\", \"brightness\": {}}}", brightness);
                let topic = mqtt.state_topic.clone();

                (topic, payload)
            }
        }
    }
}

impl FromStr for StripMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let message = serde_json::from_str::<serde_json::Value>(s).map_err(|e| e.to_string())?;

        let state = message["state"]
            .as_str()
            .ok_or("No state found".to_string())?;
        let brightness = message["brightness"].as_u64();
        let color = message["color"].as_object();
        let effect = message["effect"].as_str();
        let color_temp = message["color_temp"].as_u64();

        if "OFF" == state.to_uppercase().as_str() {
            return Ok(StripMode::Off);
        }
        if "ON" != state.to_uppercase().as_str() {
            return Err("Unknown state".to_string());
        }

        if let Some(color) = color {
            let r = color["r"].as_u64().unwrap() as u8;
            let g = color["g"].as_u64().unwrap() as u8;
            let b = color["b"].as_u64().unwrap() as u8;
            return Ok(StripMode::Colour(r, g, b));
        }

        if let Some(effect) = effect {
            return Ok(StripMode::Effect(effect.to_string()));
        }

        if let Some(brightness) = brightness {
            return Ok(StripMode::Brightness(brightness as u8));
        }

        if let Some(mired) = color_temp {
            let kelvin = 1000000 / mired as i64;
            let rgb: colortemp::RGB = colortemp::temp_to_rgb(kelvin);
            let (r, g, b) = (rgb.r as u8, rgb.g as u8, rgb.b as u8);
            return Ok(StripMode::Colour(r, g, b));
        }

        Ok(StripMode::On)
    }
}

impl fmt::Display for StripMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StripMode::On => write!(f, "ON"),
            StripMode::Off => write!(f, "OFF"),
            StripMode::Colour(r, g, b) => {
                write!(f, "ON: Colour: r: {}, g: {}, b: {}", r, g, b)
            }
            StripMode::Effect(effect) => write!(f, "ON: Effect: {}", effect),
            StripMode::Brightness(brightness) => write!(f, "ON: Brightness: {}", brightness),
        }
    }
}
