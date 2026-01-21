#![no_main]
#![no_std]

use core::cell::Cell;

use cortex_m::interrupt::{Mutex, free};
use ra4m1::interrupt;
// use uno_wifi_app::display_info::show_info;
use uno_wifi_app::hal::gpio::{GpioExt, unlock_pmnpfs_register};
use uno_wifi_app::hal::timer::{AGTCfg, AGTPrescaler, TimerExt, enable_agtimers, enable_gptimers};
use uno_wifi_app::led_matrix::LEDMatrix;
// use uno_wifi_app::time::SYSCLK_FREQ;
use uno_wifi_app::{self as _, interrupts}; // global logger + panicking-behavior + memory layout

// =================================
//      Interrupts assigned here
// =================================

// struct LedIrq {}
// impl core::marker::Copy for LedIrq {}
// impl core::clone::Clone for LedIrq {
//     fn clone(&self) -> Self {
//         *self
//     }
// }
// #[interrupt]
// fn IEL9() {
//     unsafe { <led_matrix::DisplayHandler as interrupts::Handler>::on_interrupt(ra4m1::Interrupt) };
// }
// unsafe impl interrupts::Binding<led_matrix::DisplayHandler> for LedIrq {
//     fn interrupt() -> interrupt {
//         ra4m1::Interrupt::IEL9
//     }
// }

#[cortex_m_rt::entry]
fn main() -> ! {
    // This is effectively the "setup" block of the app
    defmt::println!("Launching application");
    let perph = ra4m1::Peripherals::take().unwrap();

    // let core_periph = cortex_m::Peripherals::take().unwrap();

    // show_info(&perph);

    unsafe { unlock_pmnpfs_register() };
    unsafe { enable_gptimers() };

    // We are using AGT0 to drive the `millis()` function. This timer is driven
    // by a divider of PCLKB, which can be 1, 2, or 8, or a selectable divider
    // of AGTLCLK or AGTSCLK/d (d = 1, 2, 4, 8, 16, 32, 64, or 128)
    // By default, PCLKB is running at 1/2 main speed, so 24 MHz
    // Also option is the AGTLCLK which runs off the LOCO and can run in low power
    // or snooze mode, this runs up to 32.768 kHz
    unsafe { enable_agtimers() };
    let agt0 = perph.AGT0.into_timer();
    let base_agtcfg = AGTCfg {
        counts: 3000,
        prescaler: AGTPrescaler::PCLKD_8,
    };
    let agt0 = agt0.configure(base_agtcfg);

    // If we ever want to change to a slower timer, we can adjust the increment
    // per cycle.
    const MILLIS_INCREMENT: u32 = 1;
    // Get a "global mutable"
    static MILLIS_COUNTER: Mutex<Cell<u32>> = Mutex::new(Cell::new(0));

    // Using an AGTimer for creating the `millis()` function equivalent for timing.
    fn millis() -> u32 {
        free(|cs| MILLIS_COUNTER.borrow(cs).get())
    }

    // Interrupts are available starting on ICU8
    unsafe {
        // Use IEL8 for the millisecond counter
        ra4m1::NVIC::unmask(ra4m1::Interrupt::IEL8);
    };
    unsafe {
        cortex_m::interrupt::enable();
    }

    // Enable the interrupt for AGT0 underflow (Should be once per ms)
    perph.ICU.ielsr[8].write(|w| unsafe { w.iels().bits(0x01E) });

    #[interrupt]
    unsafe fn IEL8() {
        // clear the flag
        let p = unsafe { ra4m1::Peripherals::steal() };
        p.ICU.ielsr[8].modify(|_, w| w.ir()._0());

        // Handle interrupt by clearing TUNDF...
        // Would be nice if I could pass in the timer struct
        p.AGT0.agtcr.modify(|_, w| w.tundf().clear_bit());

        // Update current MILLIS_COUNTER
        free(|cs| {
            let cell = MILLIS_COUNTER.borrow(cs);
            let ctr = cell.get();
            cell.set(ctr + MILLIS_INCREMENT);
        })
    }

    // Start the millis timer
    agt0.start();

    let p0_pins = perph.PORT0.split();
    let p2_pins = perph.PORT2.split();

    // Pins for the LED screen driver
    let p003 = p0_pins.p003;
    let p004 = p0_pins.p004;
    let p011 = p0_pins.p011;
    let p012 = p0_pins.p012;
    let p013 = p0_pins.p013;
    let p015 = p0_pins.p015;
    let p204 = p2_pins.p204;
    let p205 = p2_pins.p205;
    let p206 = p2_pins.p206;
    let p212 = p2_pins.p212;
    let p213 = p2_pins.p213;

    // Timer to drive LED display
    let ledtimer = perph.GPT162.into_timer();

    let mut display_matrix = LEDMatrix::new(
        p003, p004, p011, p012, p013, p015, p204, p205, p206, p212, p213, ledtimer,
    );
    let heart: [u32; 3] = [
        0b00110001100001001010010001000100,
        0b01000010000010000001000100000000,
        0b10100000000001000000000000000000,
    ];

    let smile: [u32; 3] = [0x19819, 0x80000001, 0x81f8000];

    // let mut count_overflow: u32 = 0;
    // let mut num_seconds: u32 = 0;
    let mut current_frame: usize = 0;

    let frames = [heart, smile];
    display_matrix.load_frame(frames[0]);

    let mut last_millis = 0;
    let ms_per_frame = 1000;

    defmt::println!("Entering main loop");
    loop {
        // We should overflow roughly 9600 times per second
        // timer running at approx 9600 hz

        let current_millis = millis();

        if current_millis - last_millis > ms_per_frame {
            last_millis = current_millis;
            current_frame += 1;
            current_frame %= frames.len();
            defmt::println!("loading frame {}", current_frame);
            display_matrix.load_frame(frames[current_frame]);
        }
    }
    // uno_wifi_app::exit()
}
