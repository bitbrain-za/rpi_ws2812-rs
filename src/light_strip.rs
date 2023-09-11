use crate::config;
use crate::homeassistant::mqtt::{LightStripMqtt, StripMode};
use crate::ws2812::{MyStrip, Strip};
use rumqttc::{AsyncClient, MqttOptions, QoS};
use smart_led_effects::strip;
use std::default::Default;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::task;
use tokio::time::sleep;

const COUNT: usize = 55;
const UPDATE_INTERVAL: Duration = Duration::from_millis(10);

pub struct LightStrip {
    mqtt_options: MqttOptions,
    stop: AtomicBool,
    ha: LightStripMqtt,
    strip: MyStrip,
}

impl LightStrip {
    pub fn new(config: &config::Config, ha: Option<LightStripMqtt>, strip: Strip) -> LightStrip {
        let mut mqtt_options = MqttOptions::new(
            &config.id,
            &config.mqtt_config.broker,
            config.mqtt_config.port,
        );
        mqtt_options.set_credentials(&config.mqtt_config.username, &config.mqtt_config.password);
        mqtt_options.set_keep_alive(Duration::from_secs(60));

        log::info!(
            "Connecting to broker: {}:{}",
            config.mqtt_config.broker,
            config.mqtt_config.port
        );

        let mut ha = match ha {
            Some(ha) => ha,
            None => LightStripMqtt::default(),
        };

        ha.effect_list = strip::list();

        LightStrip {
            mqtt_options,
            stop: AtomicBool::new(false),
            ha,
            strip: MyStrip::new(COUNT, strip),
        }
    }

    pub async fn run(&mut self) {
        let (client, mut connection) = AsyncClient::new(self.mqtt_options.clone(), 1);

        let (tx, mut rx) = tokio::sync::mpsc::channel(10);

        log::info!("Subscribing to {}", self.ha.command_topic);
        client
            .subscribe(&self.ha.command_topic, QoS::AtMostOnce)
            .await
            .expect("Error subscribing");

        log::info!("Starting Online thread");
        let online_message = self.ha.set_online();
        let online_client = client.clone();
        task::spawn(async move {
            loop {
                let (topic, payload) = online_message.clone();
                LightStrip::publish(&online_client, &topic, &payload, false).await;
                sleep(Duration::from_secs(60)).await;
            }
        });

        log::info!("Starting State thread");
        task::spawn(async move {
            while let Ok(notification) = connection.poll().await {
                let message = match notification {
                    rumqttc::Event::Incoming(rumqttc::Packet::Publish(p)) => p,
                    _ => continue,
                };
                let message = String::from_utf8(message.payload.to_vec()).unwrap();

                match StripMode::from_str(&message) {
                    Ok(state) => {
                        if let Err(e) = tx.send(state).await {
                            log::error!("Error sending state: {:?}", e);
                        }
                    }
                    Err(e) => {
                        log::error!("Error parsing message: {:?}", e);
                    }
                }
            }
        });

        log::info!("Sending discovery message");
        let (disco_topic, disco_payload) = self.ha.discovery_message();
        LightStrip::publish(&client, &disco_topic, &disco_payload, true).await;

        while !self.stop.load(Ordering::Relaxed) {
            if let Ok(state) = rx.try_recv() {
                self.handle_state_change(&state);
                let payload = self.strip.state_message();
                let topic = self.ha.state_topic.clone();
                LightStrip::publish(&client, &topic, &payload, true).await;
            }

            self.strip.update();
            sleep(UPDATE_INTERVAL).await;
        }
    }

    async fn publish(client: &AsyncClient, topic: &String, message: &String, retain: bool) {
        log::info!("Publishing message: {} to {}", message, topic);

        if let Err(e) = client
            .publish(topic, QoS::AtLeastOnce, retain, message.as_bytes().to_vec())
            .await
        {
            log::error!("Error publishing message: {:?}", e);
        }
    }

    fn handle_state_change(&mut self, state: &StripMode) {
        log::info!("State change: {}", state);

        match state {
            StripMode::Brightness(brightness) => {
                self.strip.set_brightness(*brightness as f32 / 255.0);
            }
            StripMode::On => {
                self.strip.turn_on();
            }
            StripMode::Off => {
                self.strip.turn_off();
            }
            StripMode::Colour(r, g, b) => {
                self.strip.set_rgb(*r, *g, *b);
            }
            StripMode::Effect(e) => {
                self.strip.set_effect(e);
            }
        }
    }
}
