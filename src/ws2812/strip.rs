use crate::ws2812::rgb::Rgb;
use crate::ws2812::Ws2812Error;
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};

pub struct Strip {
    pub spi: Spi,
    pub count: usize,
    pub pages: Vec<Vec<Rgb>>,
}

type Result<T> = std::result::Result<T, Ws2812Error>;
impl Strip {
    pub fn new(bus: Bus, ss: SlaveSelect, count: usize) -> Strip {
        Strip {
            count,
            spi: Spi::new(bus, ss, 32_000_000, Mode::Mode0).unwrap(),
            pages: vec![Vec::with_capacity(count), Vec::with_capacity(count)],
        }
    }

    pub fn test(&mut self) {
        let buffer = [0x55, 0x55];
        loop {
            self.spi.write(&buffer);
            std::thread::sleep(std::time::Duration::from_millis(1000));
        }
    }

    pub fn fill(&mut self, page: usize, rgb: Rgb) {
        self.pages[page] = Vec::new();
        self.pages[page].extend_from_slice(&vec![rgb; self.count]);
    }

    pub fn set_led(&mut self, page: usize, led: usize, rgb: Rgb) -> Result<()> {
        if led > self.count {
            return Err(Ws2812Error::LedOutOfRange(format!(
                "Led {} is out of range",
                led
            )));
        }

        self.pages[page][led] = rgb;
        Ok(())
    }

    pub fn refresh(&mut self, page: usize) -> Result<()> {
        let mut buffer = Vec::new();
        self.pages[page]
            .iter()
            .for_each(|led| buffer.extend_from_slice(&led.to_spi_data()));

        // println!("Buffer: {:?}", buffer);
        if let Err(e) = self.spi.write(&buffer) {
            Err(Ws2812Error::SpiError(format!(
                "Error writing to SPI: {}",
                e
            )))
        } else {
            Ok(())
        }
    }

    pub fn clear(&mut self, page: usize) {
        self.fill(
            page,
            Rgb {
                red: 0,
                green: 0,
                blue: 0,
            },
        )
    }
}
