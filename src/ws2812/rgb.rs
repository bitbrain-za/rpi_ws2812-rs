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

    fn to_array(&self) -> [u8; 3] {
        [self.green, self.red, self.blue]
    }

    fn bit_to_spi(input: bool) -> [u8; 3] {
        if input {
            [0xFF, 0xFF, 0x00]
        } else {
            [0xFE, 0x00, 0x00]
        }
    }

    fn byte_to_spi(input: u8) -> [u8; 24] {
        let mut out: Vec<u8> = Vec::new();
        for i in 0..8 {
            let bit = (input >> (7 - i)) & 0x01;
            let spi = Self::bit_to_spi(bit == 1);
            out.extend_from_slice(&spi);
        }
        let mut output = [0u8; 24];
        output.copy_from_slice(&out);
        output
    }

    pub fn to_spi_data(&self) -> [u8; 72] {
        let start = self.to_array();

        let mut out = [0u8; 72];

        let test = start
            .iter()
            .flat_map(|x| Self::byte_to_spi(*x))
            .collect::<Vec<u8>>();

        out.copy_from_slice(&test);
        out
    }
}
