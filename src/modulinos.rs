// Arudino Modulino code

use embedded_hal::i2c::SevenBitAddress;

use crate::{
    hal::i2c::{I2cBus, I2cError, I2cInstance},
    millis_timer::millis,
    scale,
};

// Not sure if ADDR lives here, there are several modulinos that can have the
// address updated.
pub trait Modulino {
    const ADDR: SevenBitAddress;

    fn address(&self) -> SevenBitAddress {
        Self::ADDR
    }
    // If I implement the same thing on multiple modulinos, put it in here.
}

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

    // pub fn as_buffer(&self) -> [u8; 4] {
    //     [self.brightness, self.b, self.g, self.r]
    // }
}

pub struct ModulinoPixels<I: I2cInstance> {
    pub bus: I2cBus<I>,
    pixels: [Pixel; 8],
}

// Can be a custom address
impl<I: I2cInstance> Modulino for ModulinoPixels<I> {
    const ADDR: SevenBitAddress = 0x36;
}

impl<I: I2cInstance> ModulinoPixels<I> {
    pub fn new(i2cbus: I2cBus<I>) -> Self {
        ModulinoPixels {
            bus: i2cbus,
            pixels: [Pixel {
                brightness: 0,
                b: 0,
                g: 0,
                r: 0,
            }; 8],
        }
    }

    /// r, g, b from 0-255
    /// brightness from 0-100 (autoscaled to 0-31)
    pub fn set(&mut self, idx: usize, r: u8, g: u8, b: u8, brightness: u8) {
        if idx >= 8 {
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
    pub fn as_buffer(&self) -> [u8; 8 * 4] {
        let mut outbuffer: [u8; 8 * 4] = [0; 8 * 4];
        for (i, px) in self.pixels.iter().enumerate() {
            outbuffer[i * 4] = px.brightness;
            outbuffer[i * 4 + 1] = px.b;
            outbuffer[i * 4 + 2] = px.g;
            outbuffer[i * 4 + 3] = px.r;
        }
        outbuffer
    }

    /// Show the current pixel setup on the board.
    /// Not sure if I want to propogate errors here? Probably need to, if I can't
    /// show the value for some reason the caller probably should know about it.
    pub fn show(&self) -> Result<(), I2cError> {
        let data = self.as_buffer();

        self.bus.write_blocking(self.address(), &data, true, true)
    }
}

pub enum TemperatureUnits {
    Celsius,
    Fahrenheit,
}

pub struct ModulinoThermo<I: I2cInstance> {
    results: [u8; 4],
    pub bus: I2cBus<I>,
}

impl<I: I2cInstance> Modulino for ModulinoThermo<I> {
    const ADDR: SevenBitAddress = 0x44;
}

impl<I: I2cInstance> ModulinoThermo<I> {
    pub fn new(i2cbus: I2cBus<I>) -> Self {
        ModulinoThermo {
            results: [0; 4],
            bus: i2cbus,
        }
    }
    fn pct_humid(&self, humid: u16) -> f32 {
        (humid as f32 / (16384.0 - 1.0)) * 100.0
    }

    fn temp_c(&self, temp: u16) -> f32 {
        (temp as f32 / (16384.0 - 1.0)) * 165.0 - 40.0
    }

    /// Read all of the values from the probe. It takes the same amount of time
    /// to read all of the values so might as well read them all
    ///
    /// Returns ( stale, humidity %, temperature )
    ///
    /// stale is true if the data is stale (no new measurement since last read)
    /// Where the temperature is in the units specified in the function call
    pub fn read_data(&mut self, unit: TemperatureUnits) -> Result<(bool, f32, f32), I2cError> {
        self.bus.write_blocking(self.address(), &[], true, true)?;

        const HUMID_MEASUREMENT_TIME_MS: u32 = 36;

        // Need to wait at least 34 ms to take a measurement and make a read.
        let last_interval = millis();
        // This holds control of the bus still
        while millis() - last_interval < HUMID_MEASUREMENT_TIME_MS {
            cortex_m::asm::wfi();
        }

        self.bus
            .read_blocking(self.address(), &mut self.results, true, true)?;

        let stale = self.results[0] >> 6;
        let stale = match stale {
            0 => false,
            1 => true,
            _ => panic!("Corrupt stale bit: 0b{:02b}", stale),
        };
        // Mask highest 2 bits of raw humidity (those are the stale indicator bits)
        let raw_humidity = u16::from_be_bytes([self.results[0] & 0x3F, self.results[1]]);

        let temp = u16::from_be_bytes([self.results[2], self.results[3]]);
        // shift temp right 2 bits to align with manual
        let temp = temp >> 2;

        let final_humidity = self.pct_humid(raw_humidity);
        let temp_c = self.temp_c(temp);

        let final_temp = match unit {
            TemperatureUnits::Celsius => temp_c,
            TemperatureUnits::Fahrenheit => temp_c * 9.0 / 5.0 + 32.0,
        };
        Ok((stale, final_humidity, final_temp))
    }
}

pub enum Button {
    A,
    B,
    C,
}

pub struct ModulinoButtons<I: I2cInstance> {
    last_status: [u8; 3],
    pub bus: I2cBus<I>,
}

// Can be a custom address
impl<I: I2cInstance> Modulino for ModulinoButtons<I> {
    const ADDR: SevenBitAddress = 0x3E;
}

/// Button Modulino
/// Still need to implement the advanced feature library
impl<I: I2cInstance> ModulinoButtons<I> {
    // TODO: Add Debouncing
    pub fn new(i2cbus: I2cBus<I>) -> Self {
        ModulinoButtons {
            last_status: [0; 3],
            bus: i2cbus,
        }
    }

    pub fn is_pressed(&self, button: Button) -> bool {
        // Use not matches! so clippy is happy
        !matches!(self.last_status[button as usize], 0)
    }

    pub fn update(&mut self) -> Result<bool, I2cError> {
        let mut read_buffer: [u8; 4] = [0; 4];

        match self
            .bus
            .read_blocking(self.address(), &mut read_buffer, true, true)
        {
            Ok(_) => {}
            Err(err) => {
                defmt::println!("Read error {}", err);
            }
        }
        // defmt::println!("buf {:?}", read_buffer,);

        // First byte is pinstrap address from module so we can ignore it
        let was_updated: bool = read_buffer[1] != self.last_status[0]
            || read_buffer[2] != self.last_status[1]
            || read_buffer[3] != self.last_status[2];

        self.last_status[0] = read_buffer[1];
        self.last_status[1] = read_buffer[2];
        self.last_status[2] = read_buffer[3];
        Ok(was_updated)
    }

    pub fn set_leds(&self, a: bool, b: bool, c: bool) -> Result<(), I2cError> {
        let values = [a.into(), b.into(), c.into()];
        // defmt::println!("Setting LEDS {} {} {}", values[0], values[1], values[2]);
        self.bus
            .write_blocking(self.address(), &values, true, true)?;
        Ok(())
    }
}
