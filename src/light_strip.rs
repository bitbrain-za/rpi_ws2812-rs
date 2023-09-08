use crate::config;
use crate::homeassistant::mqtt::{LightStripMqtt, LightStripState};
use rumqttc::{AsyncClient, MqttOptions, QoS};
use serde::{Deserialize, Serialize};
use std::default::Default;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::task;

pub struct LightStrip {
    config: config::Config,
    mqtt_options: MqttOptions,
    stop: AtomicBool,
    command_topic: String,
    state: LightState,
    ha: LightStripMqtt,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LightState {
    pub state: String,
    pub brightness: u8,
    pub rgb: (u8, u8, u8),
    pub color_temp: u16,
    pub effect: String,
}

impl Default for LightState {
    fn default() -> Self {
        LightState {
            state: "OFF".to_string(),
            brightness: 255,
            rgb: (255, 255, 255),
            color_temp: 215,
            effect: "none".to_string(),
        }
    }
}

impl fmt::Display for LightState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = serde_json::to_string_pretty(&self).unwrap();
        write!(f, "{}", s)
    }
}

impl LightStrip {
    pub fn new(config: &config::Config, ha: Option<LightStripMqtt>) -> LightStrip {
        let mut mqtt_options = MqttOptions::new(
            &config.id,
            &config.mqtt_config.broker,
            config.mqtt_config.port,
        );
        mqtt_options.set_credentials(&config.mqtt_config.username, &config.mqtt_config.password);
        mqtt_options.set_keep_alive(Duration::from_secs(5));

        log::info!(
            "Connecting to broker: {}:{}",
            config.mqtt_config.broker,
            config.mqtt_config.port
        );

        let ha = match ha {
            Some(ha) => ha,
            None => LightStripMqtt::default(),
        };

        LightStrip {
            config: config.clone(),
            mqtt_options,
            stop: AtomicBool::new(false),
            command_topic: format!("{}/set", config.base_topic()),
            state: LightState::default(),
            ha,
        }
    }

    pub async fn run(&self) {
        let (mut client, mut connection) = AsyncClient::new(self.mqtt_options.clone(), 1);

        client
            .subscribe(&self.command_topic, QoS::AtMostOnce)
            .await
            .expect("Error subscribing");

        task::spawn(async move {
            while let Ok(notification) = connection.poll().await {
                log::debug!("Received notification: {:?}", notification);

                let message = match notification {
                    rumqttc::Event::Incoming(rumqttc::Packet::Publish(p)) => p,
                    _ => continue,
                };
                let message = String::from_utf8(message.payload.to_vec()).unwrap();
                log::debug!("Received message: {}", message);
            }
        });

        let (disco_topic, disco_payload) = self.ha.discovery_message();
        LightStrip::publish(&client, &disco_topic, &disco_payload).await;

        let (topic, payload) = self.ha.set_online();
        LightStrip::publish(&client, &topic, &payload).await;

        self.update_state_topic(&client).await;

        while !self.stop.load(Ordering::Relaxed) {}
    }

    async fn publish(client: &AsyncClient, topic: &String, message: &String) {
        log::debug!("Publishing message: {} to {}", message, topic);

        if let Err(e) = client
            .publish(topic, QoS::AtLeastOnce, false, message.as_bytes().to_vec())
            .await
        {
            log::error!("Error publishing message: {:?}", e);
        }
    }

    async fn update_state_topic(&self, client: &AsyncClient) {
        let topic = format!("{}/{}/state", self.config.mqtt_config.topic, self.config.id);
        let message = format!("{}", self.state);
        LightStrip::publish(client, &topic, &message).await;
    }
}

impl Default for LightStrip {
    fn default() -> Self {
        let config = config::Config::default();
        LightStrip::new(&config, None)
    }
}

impl fmt::Display for LightStrip {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LightStrip {{ mqtt: {}}}", self.config.mqtt_config)
    }
}
