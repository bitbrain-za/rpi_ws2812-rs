use super::availability::Availability;
use super::device::Device;
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
    effect_list: Vec<String>,
    schema: String,
    optimistic: bool,
    icon: String,
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
            effect_list: vec!["Rainbow".to_string(), "Pulse".to_string()],
            schema: "json".to_string(),
            optimistic: false,
            icon: "mdi:lightbulb".to_string(),
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

    pub fn set_offline(&self) -> (String, String) {
        self.availability.set_offline()
    }

    pub fn set_color(&self, r: u8, g: u8, b: u8) -> (String, String) {
        let payload = format!(
            "{{\"state\": \"ON\", \"color\": {{\"r\": {}, \"g\": {}, \"b\": {}}}}}",
            r, g, b
        );
        let topic = self.state_topic.clone();

        (topic, payload)
    }

    pub fn set_brightness(&self, brightness: u8) -> (String, String) {
        let payload = format!("{{\"state\": \"ON\", \"brightness\": {}}}", brightness);
        let topic = self.state_topic.clone();

        (topic, payload)
    }

    pub fn set_effect(&self, effect: &str) -> (String, String) {
        let payload = format!("{{\"state\": \"ON\", \"effect\": \"{}\"}}", effect);
        let topic = self.state_topic.clone();

        (topic, payload)
    }

    pub fn set_off(&self) -> (String, String) {
        let payload = "{{\"state\": \"OFF\"}}".to_string();
        let topic = self.state_topic.clone();

        (topic, payload)
    }

    pub fn set_color_temp(&self, color_temp: u16) -> (String, String) {
        let payload = format!("{{\"state\": \"ON\", \"color_temp\": {}}}", color_temp);
        let topic = self.state_topic.clone();

        (topic, payload)
    }

    pub fn parse_state_message(&self, message: &str) -> Option<LightStripState> {
        let message = match serde_json::from_str::<serde_json::Value>(message) {
            Ok(m) => m,
            Err(e) => {
                log::error!("Error parsing message: {:?}", e);
                return None;
            }
        };

        let state = message["state"].as_str().unwrap();
        let brightness = message["brightness"].as_u64();
        let color = message["color"].as_object();
        let effect = message["effect"].as_str();
        let color_temp = message["color_temp"].as_u64();

        Some(LightStripState {
            state: state.to_string(),
            brightness: brightness.map(|b| b as u8),
            color: color.map(|c| {
                (
                    c["r"].as_u64().unwrap() as u8,
                    c["g"].as_u64().unwrap() as u8,
                    c["b"].as_u64().unwrap() as u8,
                )
            }),
            effect: effect.map(|e| e.to_string()),
            color_temp: color_temp.map(|c| c as u16),
        })
    }
}

pub struct LightStripState {
    pub state: String,
    pub brightness: Option<u8>,
    pub color: Option<(u8, u8, u8)>,
    pub effect: Option<String>,
    pub color_temp: Option<u16>,
}

impl LightStripState {
    pub fn to_state_message(&self, mqtt: &LightStripMqtt) -> (String, String) {
        match self.state.as_str() {
            "ON" => {
                let mut payload = "{\"state\": \"ON\"".to_string();
                if let Some(brightness) = self.brightness {
                    payload.push_str(&format!(", \"brightness\": {}", brightness));
                }
                if let Some(color) = self.color {
                    payload.push_str(&format!(
                        ", \"color\": {{\"r\": {}, \"g\": {}, \"b\": {}}}",
                        color.0, color.1, color.2
                    ));
                }
                if let Some(effect) = &self.effect {
                    payload.push_str(&format!(", \"effect\": \"{}\"", effect));
                }
                if let Some(color_temp) = self.color_temp {
                    payload.push_str(&format!(", \"color_temp\": {}", color_temp));
                }
                payload.push_str("}");
                let topic = mqtt.state_topic.clone();

                (topic, payload)
            }
            _ => {
                let payload = "{\"state\": \"OFF\"}".to_string();
                let topic = mqtt.state_topic.clone();

                (topic, payload)
            }
        }
    }
}

impl FromStr for LightStripState {
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

        Ok(LightStripState {
            state: state.to_string(),
            brightness: brightness.map(|b| b as u8),
            color: color.map(|c| {
                (
                    c["r"].as_u64().unwrap() as u8,
                    c["g"].as_u64().unwrap() as u8,
                    c["b"].as_u64().unwrap() as u8,
                )
            }),
            effect: effect.map(|e| e.to_string()),
            color_temp: color_temp.map(|c| c as u16),
        })
    }
}

impl fmt::Display for LightStripState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.state.to_uppercase().as_str() {
            "OFF" => write!(f, "OFF"),
            "ON" => {
                let mut s = "ON".to_string();
                if let Some(brightness) = self.brightness {
                    s.push_str(&format!(" brightness: {}", brightness));
                }
                if let Some(color) = self.color {
                    s.push_str(&format!(" color: {:?}", color));
                }
                if let Some(effect) = &self.effect {
                    s.push_str(&format!(" effect: {}", effect));
                }
                if let Some(color_temp) = self.color_temp {
                    s.push_str(&format!(" color_temp: {}", color_temp));
                }
                write!(f, "{}", s)
            }
            _ => write!(f, "Unknown state"),
        }
    }
}
