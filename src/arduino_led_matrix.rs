use crate::hal::gpio::erased::{AnyPin, DynamicPinErased};
use crate::hal::gpio::{Output, Pin, PinExt};
use crate::hal::{
    gpio::Input,
    timer::{CountDir, GPTSourceT, GPTimer, TimerCfg, TimerError, TimerModeT, TimerT, claim_timer},
};

use core::ptr;

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
    /// These are the dynamic erased pins for internal use. Once the pins go in
    /// to this driver, they never come out.
    dynpins: [DynamicPinErased; 11],
    // The smallest way to store the matrix is an array of 3 u32 numbers, using
    // the bits of the numbers for the true/false of the leds. Here we use the
    // first number for LEDs 1-32, the 2nd for 33-64, and the 3rd for 65-96
    framebuffer: [u32; 3],
    // The timer shouldn't be optional, this struct should own a timer.
    // Either it gets one during init or the programmer gets one and passes it in.
    led_timer: Option<GPTimer>,
}

const LEDPORT0BITMASK: u16 = (1 << 3) | (1 << 4) | (1 << 11) | (1 << 12) | (1 << 13) | (1 << 15);
const LEDPORT2BITMASK: u16 = (1 << 4) | (1 << 5) | (1 << 6) | (1 << 12) | (1 << 13);
// This will be used in the interrupt driven display
static mut I_ISR: usize = 0;
// Arduio code also inits a framebuffer in memory and then memcpys the current
// frame into it, I can do this later maybe, probably something to do with
// handling interrupts

/// Unsafe function to read the I_ISR val
unsafe fn read_i_isr() -> usize {
    unsafe { ptr::read_volatile(&raw const I_ISR) }
}

/// Unsafe function to write the I_ISR val
/// Not sure why I did this besides to copy the C++ paradigm in the arduino code.
unsafe fn write_i_isr(val: usize) {
    unsafe {
        ptr::write_volatile(&raw mut I_ISR, val);
    }
}
impl ArduinoLEDMatrix {
    /// Initialize the matrix with the proper set of pins, all set to low.
    pub fn new(
        p003: Pin<0, 3, Input>,
        p004: Pin<0, 4, Input>,
        p011: Pin<0, 11, Input>,
        p012: Pin<0, 12, Input>,
        p013: Pin<0, 13, Input>,
        p015: Pin<0, 15, Input>,
        p204: Pin<2, 4, Input>,
        p205: Pin<2, 5, Input>,
        p206: Pin<2, 6, Input>,
        p212: Pin<2, 12, Input>,
        p213: Pin<2, 13, Input>,
    ) -> Self {
        let dynpins: [DynamicPinErased; 11] = [
            p003.into_fully_erased_dynamic(),
            p004.into_fully_erased_dynamic(),
            p011.into_fully_erased_dynamic(),
            p012.into_fully_erased_dynamic(),
            p013.into_fully_erased_dynamic(),
            p015.into_fully_erased_dynamic(),
            p204.into_fully_erased_dynamic(),
            p205.into_fully_erased_dynamic(),
            p206.into_fully_erased_dynamic(),
            p212.into_fully_erased_dynamic(),
            p213.into_fully_erased_dynamic(),
        ];
        Self {
            dynpins,
            framebuffer: [0, 0, 0],
            led_timer: None,
        }
    }

    /// Handles the logic to set up a timer and the ISR callback
    pub fn begin(&mut self) -> Result<(), TimerError> {
        // TODO: Write the interrupt and callback
        let timer_cfg = TimerCfg {
            timer_type: TimerT::GPT_16_Timer,
            count_direction: CountDir::Up,
            gtssr: GPTSourceT::SOFTWARE,
            gtpsr: GPTSourceT::SOFTWARE,
            gtcsr: GPTSourceT::SOFTWARE,
            mode: TimerModeT::PERIODIC,
            freq: 10000,
        };
        let channel = claim_timer(&timer_cfg.timer_type)?;
        let led_timer = GPTimer::new_from_config(timer_cfg, channel);
        self.led_timer = Some(led_timer);
        // TODO: Hook up the interrupt
        self.led_timer.as_mut().unwrap().set_frequency(10000.0)?;
        self.led_timer.as_mut().unwrap().start()?;
        Ok(())
    }

    pub fn on(&mut self, led_idx: usize) {
        self.turn_led(led_idx, true);
    }
    pub fn off(&mut self, led_idx: usize) {
        self.turn_led(led_idx, false);
    }
    pub fn clear(&mut self) {
        let frame: [u32; 3] = [0, 0, 0];
        self.framebuffer = frame;
        self.draw_grid();
    }

    /// Load the single frame into the struct's framebuffer
    pub fn load_frame(&mut self, frame: [u32; 3]) {
        self.framebuffer = frame;
        // defmt::println!("Loaded buffer:");
        // for buf in self.framebuffer.iter() {
        //     defmt::println!("{:032b}", buf);
        // }
        // defmt::println!("Reversed buffer:");
        // for rbuf in self.framebuffer.iter() {
        //     let rev = self.reverse(*rbuf);
        //     defmt::println!("{:032b}", rev);
        // }
        let this_frame: [u32; 3] = [
            self.reverse(self.framebuffer[0]),
            self.reverse(self.framebuffer[1]),
            self.reverse(self.framebuffer[2]),
        ];

        self.framebuffer = this_frame;
    }

    /// Render out the framebuffer
    pub fn render_frame(&mut self) {
        self.draw_grid();
    }

    /// Turn the whole grid off, then turn on the specific index if called for
    /// idx must be 0 <= idx <= 95 (for our 96 grid LED)
    fn turn_led(&mut self, idx: usize, on: bool) {
        // Unsafe write to set the whole LED screen to low
        // *TECHNICALLY* we should be doing this pin by pin but this is probably faster
        // Should *TECHNICALLY* be safe enough because we own all of the pins
        // unsafe {
        //     let p1 = ra4m1::PORT0::PTR;
        //     (*p1)
        //         .pcntr1()
        //         .modify(|r, w| w.pdr().bits(r.pdr().bits() & !LEDPORT0BITMASK));
        //     let p2 = ra4m1::PORT2::PTR;
        //     (*p2)
        //         .pcntr1()
        //         .modify(|r, w| w.pdr().bits(r.pdr().bits() & !LEDPORT2BITMASK));
        // }
        // Waste time here to make sure we're still aligned.
        // Only other option is unsafely just doing whatever like we're in C++
        for pin in self.dynpins.iter_mut() {
            pin.make_floating_input();
        }
        let [hi, lo] = &PINS.get(idx).unwrap();
        if on {
            let high_pin = &mut self.dynpins[*hi as usize];
            high_pin.make_push_pull_output();
            // defmt::println!("Pin Hi: {}", high_pin);
            // quick unsafe read to see if it's really high
            // let reg = high_pin.pmnpfs_reg();
            // let val = reg.read().bits();
            // defmt::println!("{:032b}", val);
            high_pin.set_high().unwrap();
            // let val = reg.read().bits();
            // defmt::println!("{:032b}", val);
            let low_pin = &mut self.dynpins[*lo as usize];
            low_pin.make_push_pull_output();
            // defmt::println!("Pin lo: {}", low_pin);
            // let reg = low_pin.pmnpfs_reg();
            // let val = reg.read().bits();
            // defmt::println!("{:032b}", val);
            low_pin.set_low().unwrap();
            // let val = reg.read().bits();
            // defmt::println!("{:032b}", val);
        }
    }
    /// Shamelessly stolen from the arduino code, need to learn why this bit twiddling works
    fn reverse(&self, x: u32) -> u32 {
        let mut x = ((x >> 1) & 0x55555555_u32) | ((x & 0x55555555_u32) << 1);
        x = ((x >> 2) & 0x33333333_u32) | ((x & 0x33333333_u32) << 2);
        x = ((x >> 4) & 0x0f0f0f0f_u32) | ((x & 0x0f0f0f0f_u32) << 4);
        x = ((x >> 8) & 0x00ff00ff_u32) | ((x & 0x00ff00ff_u32) << 8);
        x = ((x >> 16) & 0x0000ffff_u32) | ((x & 0x0000ffff_u32) << 16);
        x
    }
    /// This should fire on a timer. What I should do in order to mimic the
    /// Arduino code is to write this as an ISR that steps through each of the
    /// LEDs and toggles each one based on the frame information. Then I use
    /// the ISR as a callback on a GPT timer instance. First lets just get it
    /// working then we can make it good.
    /// TODO: Rewrite this as an ISR that fires on a timer callback.
    fn draw_grid(&mut self) {
        let i_isr = unsafe { read_i_isr() };
        self.turn_led(
            i_isr,
            (self.framebuffer[i_isr >> 5] & (1 << (i_isr & 31))) != 0,
        );
        unsafe { write_i_isr((i_isr + 1) % NUM_LEDS as usize) };
    }
}
