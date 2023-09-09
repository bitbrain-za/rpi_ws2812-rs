use crate::config;
use crate::homeassistant::mqtt::{LightStripMqtt, StripMode};
use crate::ws2812::{Rgb, Strip};
use lazy_static::lazy_static;
use palette::FromColor;
use rumqttc::{AsyncClient, MqttOptions, QoS};
use smart_led_effects::strip::Effect;
use smart_led_effects::{strip, Srgb};
use std::default::Default;
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
    ha: LightStripMqtt,
    strip: Strip,
    state: StripMode,
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
            ha,
            strip,
            state: StripMode::Off,
        }
    }

    pub async fn run(&mut self) {
        let (mut client, mut connection) = AsyncClient::new(self.mqtt_options.clone(), 1);

        log::info!("Subscribing to {}", self.ha.command_topic);
        client
            .subscribe(&self.ha.command_topic, QoS::AtMostOnce)
            .await
            .expect("Error subscribing");

        let online_message = self.ha.set_online();
        let online_client = client.clone();
        task::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));

            loop {
                interval.tick().await;
                interval.tick().await;
                let (topic, payload) = online_message.clone();
                LightStrip::publish(&online_client, &topic, &payload).await;
            }
        });

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
                        if let Some(state) = LightStrip::handle_state_change(&state) {
                            let resp = state.to_state_message(&ha2);
                            LightStrip::publish(&state_updater, &resp.0, &resp.1).await;
                        }
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

    fn handle_state_change(state: &StripMode) -> Option<StripMode> {
        log::debug!("State change: {}", state);

        let mut mode = MODE.lock().unwrap();
        let mut last_mode = LAST_MODE.lock().unwrap();

        if let StripMode::Brightness(brightness) = state {
            if let StripMode::Effect(_) = *mode {
                return None;
            }

            if let StripMode::Colour(r, g, b) = *mode {
                log::warn!("Modifying MODE");
                let mut hsv = palette::Hsv::from_color(Srgb::<u8>::new(r, g, b).into_format());
                let new_v: f32 = *brightness as f32 / 255.0;
                hsv.value = new_v;
                let srgb = Srgb::from_color(hsv).into_format::<u8>();

                *mode = StripMode::Colour(srgb.red, srgb.green, srgb.blue);
            } else if let StripMode::Colour(r, g, b) = *last_mode {
                log::warn!("Using LAST_MODE");
                let mut hsv = palette::Hsv::from_color(Srgb::<u8>::new(r, g, b).into_format());
                let new_v: f32 = hsv.value * (*brightness as f32 / 255.0);
                hsv.value = new_v;
                let srgb = Srgb::from_color(hsv).into_format::<u8>();
                *mode = StripMode::Colour(srgb.red, srgb.green, srgb.blue);
            }
        } else if let StripMode::On = state {
            *mode = last_mode.clone();
        } else {
            *mode = state.clone();
        }

        if *mode != StripMode::Off {
            *last_mode = mode.clone();
        }

        return Some(mode.clone());
    }

    async fn implement_state(&mut self) {
        let mode = MODE.lock().unwrap().clone();
        match mode {
            StripMode::On => {
                let _ = self.strip.fill(0, &Rgb::new(0, 0, 255));
            }
            StripMode::Off => {
                let _ = self.strip.clear(0);
            }
            StripMode::Effect(effect) => {
                if let Ok(effect) = EffectsEnum::from_str(&effect) {
                    let pixels = LightStrip::get_effect(&effect)
                        .iter_mut()
                        .map(|x| Rgb::new(x.red, x.green, x.blue))
                        .collect::<Vec<Rgb>>();
                    self.strip.set_page(0, pixels).expect("Error setting page");
                }
            }
            StripMode::Colour(r, g, b) => {
                let _ = self.strip.fill(0, &Rgb::new(r, g, b));
            }
            StripMode::Brightness(brightness) => {}
        }
        self.strip.refresh(0).expect("Error displaying LED");
    }

    fn get_effect(index: &EffectsEnum) -> Vec<Srgb<u8>> {
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
    static ref LAST_MODE: Mutex<StripMode> = Mutex::new(StripMode::Effect("rainbow".to_string()));
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
