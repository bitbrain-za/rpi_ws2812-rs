use std::default::Default;
use std::fmt;
use std::fs::File;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub id: String,
    pub friendly_name: String,
    pub mqtt_config: MqttConfig,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct MqttConfig {
    pub broker: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub topic: String,
}

pub fn load(path: &str) -> Result<Config, String> {
    let file = File::open(path).map_err(|e| format!("Failed to open config file: {}", e))?;
    let config =
        serde_json::from_reader(file).map_err(|e| format!("Failed to parse config file: {}", e))?;
    Ok(config)
}

impl Config {
    pub fn save(&self, path: &str) -> Result<(), String> {
        let file =
            File::create(path).map_err(|e| format!("Failed to create config file: {}", e))?;
        serde_json::to_writer_pretty(file, self)
            .map_err(|e| format!("Failed to write config file: {}", e))?;
        Ok(())
    }

    pub fn base_topic(&self) -> String {
        format!("{}/{}", self.mqtt_config.topic, self.id)
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Config {{ MQTT: {{ broker: {}, port: {}, username: {}, password: {} }}",
            self.mqtt_config.broker,
            self.mqtt_config.port,
            self.mqtt_config.username,
            self.mqtt_config.password,
        )
    }
}

impl fmt::Display for MqttConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MqttConfig {{ broker: {}, port: {}, username: {}, password: {} }}",
            self.broker, self.port, self.username, self.password
        )
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            id: format!("card-monitor_{}", Uuid::new_v4()),
            friendly_name: "Card Monitor".to_string(),
            mqtt_config: MqttConfig::default(),
        }
    }
}

impl Default for MqttConfig {
    fn default() -> Self {
        MqttConfig {
            broker: "localhost".to_string(),
            port: 1883,
            username: "username".to_string(),
            password: "password".to_string(),
            topic: "bitbrain/light_strip".to_string(),
        }
    }
}
