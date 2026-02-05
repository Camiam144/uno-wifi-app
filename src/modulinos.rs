// Arudino Modulino code

use embedded_hal::i2c::SevenBitAddress;

use crate::scale;

#[derive(Copy, Clone)]
struct Pixel {
    brightness: u8,
    b: u8,
    g: u8,
    r: u8,
}

impl Pixel {
    /// Brightness from 0 to 100, autoscaled to 0-31.
    pub fn set(&mut self, r: u8, b: u8, g: u8, brightness: u8) {
        let scaled_bright = scale(brightness.into(), 0, 100, 0, 0x1F) as u8;
        self.r = r;
        self.g = g;
        self.b = b;
        self.brightness = 0xE0 | scaled_bright;
    }
}

pub struct ModulinoPixels {
    pixels: [Pixel; 8],
}

impl ModulinoPixels {
    pub const ADDR: SevenBitAddress = 0x36;
    pub const NUMLEDS: usize = 8;

    pub fn new() -> Self {
        ModulinoPixels {
            pixels: [Pixel {
                brightness: 0,
                b: 0,
                g: 0,
                r: 0,
            }; Self::NUMLEDS],
        }
    }

    pub fn address(&self) -> SevenBitAddress {
        Self::ADDR
    }

    /// r, g, b from 0-255
    /// brightness from 0-100 (autoscaled to 0-31)
    pub fn set(&mut self, idx: usize, r: u8, g: u8, b: u8, brightness: u8) {
        if idx >= Self::NUMLEDS {
            return;
        }
        self.pixels[idx].set(r, b, g, brightness);
    }

    pub fn clear_pixel(&mut self, idx: usize) {
        self.set(idx, 0, 0, 0, 0);
    }

    pub fn clear_all(&mut self) {
        for pxl in self.pixels.iter_mut() {
            pxl.set(0, 0, 0, 0);
        }
    }

    /// Represents the pixels as a byte array
    /// to the best of my knowledge, from most to least significant, the pixel
    /// color is transmitted as [0xE0 | brightness, b, g, r]
    /// so this provides that format for all 32 bits of pixel information
    pub fn as_buffer(&self) -> [u8; Self::NUMLEDS * 4] {
        let mut outbuffer: [u8; Self::NUMLEDS * 4] = [0; Self::NUMLEDS * 4];
        for (i, px) in self.pixels.iter().enumerate() {
            outbuffer[i * 4] = px.brightness;
            outbuffer[i * 4 + 1] = px.b;
            outbuffer[i * 4 + 2] = px.g;
            outbuffer[i * 4 + 3] = px.r;
        }
        outbuffer
    }
}

impl Default for ModulinoPixels {
    fn default() -> Self {
        ModulinoPixels {
            pixels: [Pixel {
                brightness: 0,
                b: 0,
                g: 0,
                r: 0,
            }; Self::NUMLEDS],
        }
    }
}
