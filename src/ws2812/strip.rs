use crate::ws2812::rgb::Rgb;
use crate::ws2812::Ws2812Error;
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};

pub struct Strip {
    pub spi: Spi,
    pub count: usize,
    pub pages: Vec<Vec<Rgb>>,
    current_page: usize,
}

type Result<T> = std::result::Result<T, Ws2812Error>;
impl Strip {
    pub fn new(bus: Bus, ss: SlaveSelect, count: usize, pages: usize) -> Result<Strip> {
        if count > 1024 {
            return Err(Ws2812Error::LedOutOfRange(format!(
                "Led count {} is out of range",
                count
            )));
        }

        Ok(Strip {
            count,
            spi: Spi::new(bus, ss, 32_000_000, Mode::Mode0).unwrap(),
            pages: vec![Vec::with_capacity(count); pages],
            current_page: 0,
        })
    }

    pub fn fill(&mut self, page: usize, rgb: &Rgb) -> Result<()> {
        if self.pages.len() <= page {
            return Err(Ws2812Error::PageOutOfRange(format!(
                "Page {} is out of range",
                page
            )));
        }
        self.pages[page] = Vec::new();
        self.pages[page].extend_from_slice(&vec![rgb.clone(); self.count]);
        Ok(())
    }

    pub fn set_led(&mut self, page: usize, led: usize, rgb: &Rgb) -> Result<()> {
        if self.pages.len() <= page {
            return Err(Ws2812Error::PageOutOfRange(format!(
                "Page {} is out of range",
                page
            )));
        }
        if led > self.count {
            return Err(Ws2812Error::LedOutOfRange(format!(
                "Led {} is out of range",
                led
            )));
        }

        self.pages[page][led] = rgb.clone();
        Ok(())
    }

    pub fn refresh(&mut self, page: usize) -> Result<()> {
        if self.pages.len() <= page {
            return Err(Ws2812Error::PageOutOfRange(format!(
                "Page {} is out of range",
                page
            )));
        }
        let mut buffer = Vec::new();
        self.pages[page]
            .iter()
            .for_each(|led| buffer.extend_from_slice(&led.to_spi_data()));

        if let Err(e) = self.spi.write(&buffer) {
            Err(Ws2812Error::SpiError(format!(
                "Error writing to SPI: {}",
                e
            )))
        } else {
            self.current_page = page;
            Ok(())
        }
    }

    pub fn clear(&mut self, page: usize) -> Result<()> {
        self.fill(page, &Rgb::new(0, 0, 0))
    }
}
