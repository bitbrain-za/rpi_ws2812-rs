use crate::config;
use crate::homeassistant::mqtt::{LightStripMqtt, StripMode};
use crate::ws2812::{Rgb, Strip};
use lazy_static::lazy_static;
use rumqttc::{AsyncClient, MqttOptions, QoS};
use serde::{Deserialize, Serialize};
use smart_led_effects::strip::Effect;
use smart_led_effects::{strip, Srgb};
use std::default::Default;
use std::fmt;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tokio::task;
use tokio::time::sleep;

const COUNT: usize = 55;
const UPDATE_INTERVAL: Duration = Duration::from_millis(100);

pub struct LightStrip {
    mqtt_options: MqttOptions,
    stop: AtomicBool,
    command_topic: String,
    state: LightState,
    ha: LightStripMqtt,
    strip: Strip,
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
    pub fn new(config: &config::Config, ha: Option<LightStripMqtt>, strip: Strip) -> LightStrip {
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

        let mut ha = match ha {
            Some(ha) => ha,
            None => LightStripMqtt::default(),
        };

        ha.effect_list = strip::list();

        LightStrip {
            mqtt_options,
            stop: AtomicBool::new(false),
            command_topic: format!("{}/set", config.base_topic()),
            state: LightState::default(),
            ha,
            strip,
        }
    }

    pub async fn run(&mut self) {
        let (mut client, mut connection) = AsyncClient::new(self.mqtt_options.clone(), 1);

        log::info!("Subscribing to {}", self.command_topic);
        client
            .subscribe(&self.command_topic, QoS::AtMostOnce)
            .await
            .expect("Error subscribing");

        let ha2 = self.ha.clone();
        let state_updater = client.clone();
        task::spawn(async move {
            while let Ok(notification) = connection.poll().await {
                let message = match notification {
                    rumqttc::Event::Incoming(rumqttc::Packet::Publish(p)) => p,
                    _ => continue,
                };
                let message = String::from_utf8(message.payload.to_vec()).unwrap();
                log::debug!("Received message: {}", message);

                match StripMode::from_str(&message) {
                    Ok(state) => {
                        LightStrip::handle_state_change(&state).await;
                        let resp = state.to_state_message(&ha2);
                        LightStrip::publish(&state_updater, &resp.0, &resp.1).await;
                    }
                    Err(e) => {
                        log::error!("Error parsing message: {:?}", e);
                    }
                }
            }
        });

        let (disco_topic, disco_payload) = self.ha.discovery_message();
        LightStrip::publish(&client, &disco_topic, &disco_payload).await;

        let (topic, payload) = self.ha.set_online();
        LightStrip::publish(&client, &topic, &payload).await;

        let state = MODE.lock().unwrap().clone();
        let resp = state.to_state_message(&self.ha);
        LightStrip::publish(&client, &resp.0, &resp.1).await;

        while !self.stop.load(Ordering::Relaxed) {
            self.implement_state().await;
            sleep(UPDATE_INTERVAL).await;
        }
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

    async fn handle_state_change(state: &StripMode) {
        log::debug!("State change: {}", state);
        let mut mode = MODE.lock().unwrap();
        *mode = state.clone();
    }

    async fn implement_state(&mut self) {
        let mode = MODE.lock().unwrap().clone();
        match mode {
            StripMode::Off => {
                let _ = self.strip.clear(0);
            }
            StripMode::Effect(effect) => {}
            StripMode::Colour(r, g, b) => {
                let _ = self.strip.fill(0, &Rgb::new(r, g, b));
            }
            StripMode::Brightness(brightness) => {}
            StripMode::ColourTemp(color_temp) => {}
        }
        self.strip.refresh(0).expect("Error displaying LED");
    }

    fn get_effect(index: EffectsEnum) -> Vec<Srgb<u8>> {
        match index {
            EffectsEnum::Bounce => BOUNCE.lock().unwrap().next().unwrap(),
            EffectsEnum::Rainbow => RAINBOW.lock().unwrap().next().unwrap(),
            EffectsEnum::Breathe => BREATHE.lock().unwrap().next().unwrap(),
            EffectsEnum::Cycle => CYCLE.lock().unwrap().next().unwrap(),
            EffectsEnum::Fire => FIRE.lock().unwrap().next().unwrap(),
            EffectsEnum::Meteor => METEOR.lock().unwrap().next().unwrap(),
            EffectsEnum::RunningLights => RUNNING_LIGHTS.lock().unwrap().next().unwrap(),
            EffectsEnum::Cylon => CYLON.lock().unwrap().next().unwrap(),
            EffectsEnum::Timer => TIMER.lock().unwrap().next().unwrap(),
            EffectsEnum::Twinkle => TWINKLE.lock().unwrap().next().unwrap(),
            EffectsEnum::Sparkle => SPARKLE.lock().unwrap().next().unwrap(),
            EffectsEnum::Snow => SNOW.lock().unwrap().next().unwrap(),
            EffectsEnum::Wipe => WIPE.lock().unwrap().next().unwrap(),
        }
    }
}

enum EffectsEnum {
    Bounce,
    Rainbow,
    Breathe,
    Cycle,
    Fire,
    Meteor,
    RunningLights,
    Cylon,
    Timer,
    Twinkle,
    Sparkle,
    Snow,
    Wipe,
}

impl FromStr for EffectsEnum {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "bounce" => Ok(EffectsEnum::Bounce),
            "rainbow" => Ok(EffectsEnum::Rainbow),
            "breathe" => Ok(EffectsEnum::Breathe),
            "cycle" => Ok(EffectsEnum::Cycle),
            "fire" => Ok(EffectsEnum::Fire),
            "meteor" => Ok(EffectsEnum::Meteor),
            "runninglights" => Ok(EffectsEnum::RunningLights),
            "cylon" => Ok(EffectsEnum::Cylon),
            "timer" => Ok(EffectsEnum::Timer),
            "twinkle" => Ok(EffectsEnum::Twinkle),
            "sparkle" => Ok(EffectsEnum::Sparkle),
            "snow" => Ok(EffectsEnum::Snow),
            "wipe" => Ok(EffectsEnum::Wipe),
            _ => Err(format!("Unknown effect: {}", s)),
        }
    }
}

lazy_static! {
    static ref MODE: Mutex<StripMode> = Mutex::new(StripMode::Off);
    static ref BOUNCE: Mutex<strip::Bounce> =
        Mutex::new(strip::Bounce::new(COUNT, None, None, None, None, None));
    static ref RAINBOW: Mutex<strip::Rainbow> = Mutex::new(strip::Rainbow::new(COUNT, None));
    static ref BREATHE: Mutex<strip::Breathe> = Mutex::new(strip::Breathe::new(COUNT, None, None));
    static ref CYCLE: Mutex<strip::Cycle> = Mutex::new(strip::Cycle::new(COUNT, None));
    static ref FIRE: Mutex<strip::Fire> = Mutex::new(strip::Fire::new(COUNT, None, None));
    static ref METEOR: Mutex<strip::Meteor> =
        Mutex::new(strip::Meteor::new(COUNT, None, None, None));
    static ref RUNNING_LIGHTS: Mutex<strip::RunningLights> =
        Mutex::new(strip::RunningLights::new(COUNT, None, false));
    static ref CYLON: Mutex<strip::Cylon> = Mutex::new(strip::Cylon::new(
        COUNT,
        Srgb::<u8>::new(255, 0, 0),
        None,
        None
    ));
    static ref TIMER: Mutex<strip::Timer> = Mutex::new(strip::Timer::new(
        COUNT,
        std::time::Duration::from_millis(5000),
        None,
        None,
        None,
        true
    ));
    static ref TWINKLE: Mutex<strip::Twinkle> =
        Mutex::new(strip::Twinkle::new(COUNT, None, None, None, None));
    static ref SPARKLE: Mutex<strip::Twinkle> = Mutex::new(strip::Twinkle::sparkle(COUNT, None));
    static ref SNOW: Mutex<strip::SnowSparkle> =
        Mutex::new(strip::SnowSparkle::new(COUNT, None, None, None, None));
    static ref WIPE: Mutex<strip::Wipe> = Mutex::new(strip::Wipe::colour_wipe(
        COUNT,
        Srgb::<u8>::new(0, 255, 0),
        false
    ));
}
