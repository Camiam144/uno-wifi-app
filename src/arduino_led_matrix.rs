use core::ops::Deref;

use embedded_hal::digital::PinState;
use ra4m1::PORT0;

use crate::hal::gpio::{Pin, PinMode, Port};

// TODO: Figure out the graphics library

pub const NUM_LEDS: u8 = 96;

/// These are all of the pin combinations to turn on and off each LED. The indices
/// start at 0 in the top left. Each index is given as [HI, LO] to turn on that
/// specific LED.
const PINS: [[u8; 2]; 96] = [
    [7, 3], // 0
    [3, 7],
    [7, 4],
    [4, 7],
    [3, 4],
    [4, 3],
    [7, 8],
    [8, 7],
    [3, 8],
    [8, 3],
    [4, 8], // 10
    [8, 4],
    [7, 0],
    [0, 7],
    [3, 0],
    [0, 3],
    [4, 0],
    [0, 4],
    [8, 0],
    [0, 8],
    [7, 6], // 20
    [6, 7],
    [3, 6],
    [6, 3],
    [4, 6],
    [6, 4],
    [8, 6],
    [6, 8],
    [0, 6],
    [6, 0],
    [7, 5], // 30
    [5, 7],
    [3, 5],
    [5, 3],
    [4, 5],
    [5, 4],
    [8, 5],
    [5, 8],
    [0, 5],
    [5, 0],
    [6, 5], // 40
    [5, 6],
    [7, 1],
    [1, 7],
    [3, 1],
    [1, 3],
    [4, 1],
    [1, 4],
    [8, 1],
    [1, 8],
    [0, 1], // 50
    [1, 0],
    [6, 1],
    [1, 6],
    [5, 1],
    [1, 5],
    [7, 2],
    [2, 7],
    [3, 2],
    [2, 3],
    [4, 2],
    [2, 4],
    [8, 2],
    [2, 8],
    [0, 2],
    [2, 0],
    [6, 2],
    [2, 6],
    [5, 2],
    [2, 5],
    [1, 2],
    [2, 1],
    [7, 10],
    [10, 7],
    [3, 10],
    [10, 3],
    [4, 10],
    [10, 4],
    [8, 10],
    [10, 8],
    [0, 10],
    [10, 0],
    [6, 10],
    [10, 6],
    [5, 10],
    [10, 5],
    [1, 10],
    [10, 1],
    [2, 10],
    [10, 2],
    [7, 9],
    [9, 7],
    [3, 9],
    [9, 3],
    [4, 9],
    [9, 4],
];

/// The 12x8 LED matrix on the board. This struct will hold all of the logic
/// to get the thing to work. I'm going to use the system timer for now, not
/// sure if that's the best call but eh.
pub struct ArduinoLEDMatrix {
    pub rows: [Pin; 11],
}

const LEDPORT0BITMASK: u16 = (1 << 3) | (1 << 4) | (1 << 11) | (1 << 12) | (1 << 13) | (1 << 15);
const LEDPORT2BITMASK: u16 = (1 << 4) | (1 << 5) | (1 << 6) | (1 << 12) | (1 << 13);

impl ArduinoLEDMatrix {
    /// Initialize the matrix with the proper set of pins, all set to low.
    pub fn new() -> Self {
        let mut pins = [
            Pin::new(Port::PORT0, 3, PinMode::Output),
            Pin::new(Port::PORT0, 4, PinMode::Output),
            Pin::new(Port::PORT0, 11, PinMode::Output),
            Pin::new(Port::PORT0, 12, PinMode::Output),
            Pin::new(Port::PORT0, 13, PinMode::Output),
            Pin::new(Port::PORT0, 15, PinMode::Output),
            Pin::new(Port::PORT2, 4, PinMode::Output),
            Pin::new(Port::PORT2, 5, PinMode::Output),
            Pin::new(Port::PORT2, 6, PinMode::Output),
            Pin::new(Port::PORT2, 12, PinMode::Output),
            Pin::new(Port::PORT2, 13, PinMode::Output),
        ];
        for p in pins.iter_mut() {
            p.set_low();
        }
        ArduinoLEDMatrix { rows: pins }
    }

    /// Turn the whole grid off, then turn on the specific index if called for
    fn turn_led(&mut self, idx: usize, on: bool) {
        // Unsafe write to set the whole LED screen to low

        unsafe {
            let p1 = ra4m1::PORT0::PTR;
            (*p1)
                .pcntr1()
                .modify(|r, w| w.pdr().bits(r.pdr().bits() & !LEDPORT0BITMASK));
        }
        unsafe {
            let p2 = ra4m1::PORT2::PTR;
            (*p2)
                .pcntr1()
                .modify(|r, w| w.pdr().bits(r.pdr().bits() & !LEDPORT2BITMASK));
        }

        let [hi, lo] = PINS.get(idx).unwrap();
        if on {
            let high_pin = self.rows.get_mut(*hi as usize).unwrap();
            high_pin.set_mode(PinMode::Output);
            high_pin.set_high();
            let low_pin = self.rows.get_mut(*lo as usize).unwrap();
            low_pin.set_mode(PinMode::Output);
            low_pin.set_low();
        }
    }

    pub fn on(&mut self, led_idx: usize) {
        self.turn_led(led_idx, true);
    }
    pub fn off(&mut self, led_idx: usize) {
        self.turn_led(led_idx, false);
    }
}

impl Default for ArduinoLEDMatrix {
    fn default() -> Self {
        Self::new()
    }
}
