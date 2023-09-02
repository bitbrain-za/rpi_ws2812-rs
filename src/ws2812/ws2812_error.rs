#[derive(Debug)]
pub enum Ws2812Error {
    LedOutOfRange(String),
    SpiError(String),
    PageOutOfRange(String),
}
