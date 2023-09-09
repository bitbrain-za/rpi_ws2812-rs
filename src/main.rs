use log::LevelFilter;
use simple_logger::SimpleLogger;
use std::env;
use systemd_journal_logger::JournalLog;
mod ws2812;
use crate::ws2812::{Rgb, Strip};
use rppal::spi::{Bus, SlaveSelect};

mod config;
mod homeassistant;
mod light_strip;

use smart_led_effects::strip as effects;
use smart_led_effects::strip::Effect;

#[tokio::main]
async fn main() {
    // JournalLog::default().install().unwrap();
    SimpleLogger::new().init().unwrap();
    log::set_max_level(LevelFilter::Debug);
    let args: Vec<String> = env::args().collect();
    log::debug!("{:?}", args);

    let config_path = match args.get(1) {
        Some(p) => p,
        None => "config.json",
    };

    let conf = match config::load(config_path) {
        Ok(conf) => conf,
        Err(e) => {
            println!("{}", e);
            let conf = config::Config::default();
            conf.save(config_path).expect("Error saving config");
            conf
        }
    };
    log::info!("Config Loaded: {}", conf);

    let mut led = Strip::new(Bus::Spi0, SlaveSelect::Ss0, 55, 2).expect("Error creating strip");
    let _ = led.clear(0);
    let _ = led.clear(1);

    let mut rainbow = effects::Rainbow::new(55, None);

    light_strip::LightStrip::new(&conf, None, led).run().await;
}
