use std::fmt::Display;

#[derive(Clone)]
pub struct Rgb {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl Display for Rgb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(R: {}, G: {}, B: {})", self.red, self.green, self.blue)
    }
}

impl Rgb {
    pub fn new(red: u8, green: u8, blue: u8) -> Rgb {
        Rgb { red, green, blue }
    }

    pub fn red() -> Rgb {
        Rgb::new(255, 0, 0)
    }

    pub fn green() -> Rgb {
        Rgb::new(0, 255, 0)
    }

    pub fn blue() -> Rgb {
        Rgb::new(0, 0, 255)
    }

    fn bit_to_spi(input: bool) -> [u8; 3] {
        if input {
            [0xFF, 0xFE, 0x00]
        } else {
            [0xFE, 0x00, 0x00]
        }
    }

    fn byte_to_spi(input: u8) -> [u8; 24] {
        let mut out: Vec<u8> = Vec::new();
        for i in 0..8 {
            let bit = (input >> i) & 0x01;
            let spi = Self::bit_to_spi(bit == 1);
            out.extend_from_slice(&spi);
        }
        let mut output = [0u8; 24];
        output.copy_from_slice(&out);
        output
    }

    pub fn to_spi_data(&self) -> [u8; 72] {
        let mut out = [0u8; 72];
        let mut output: Vec<u8> = Vec::new();
        let vec = [self.green, self.red, self.blue];
        for i in 0..3 {
            let spi = Self::byte_to_spi(vec[i]);
            output.extend_from_slice(&spi);
        }
        out.copy_from_slice(&output);
        out
    }

    fn byte_to_spi_data(input: &u8) -> [u8; 3] {
        let b0_offset = input & 0x07;
        let b1_offset = (input & 0x18) >> 3;
        let b2_offset = (input & 0xe0) >> 5;

        let opt0 = [0x24, 0x26, 0x34, 0x36, 0xa4, 0xa6, 0xb4, 0xb6];
        let opt1 = [0x49, 0x4d, 0x69, 0x6d];
        let opt2 = [0x92, 0x93, 0x9a, 0x9b, 0xd2, 0xd3, 0xda, 0xdb];

        [
            opt0[b0_offset as usize],
            opt1[b1_offset as usize],
            opt2[b2_offset as usize],
        ]
    }
}
