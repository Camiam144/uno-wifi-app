use core::cell::Cell;
use cortex_m::interrupt::Mutex;
use cortex_m::interrupt::free;
use ra4m1::interrupt;

use crate::hal::timer::{AGTCfg, AGTPrescaler, AGTimer, AGTimerInstance, Unconfigured};

// We are using AGT0 to drive the `millis()` function. This timer is driven
// by a divider of PCLKB, which can be 1, 2, or 8, or a selectable divider
// of AGTLCLK or AGTSCLK/d (d = 1, 2, 4, 8, 16, 32, 64, or 128)
// By default, PCLKB is running at 1/2 main speed, so 24 MHz
// Also option is the AGTLCLK which runs off the LOCO and can run in low power
// or snooze mode, this runs up to 32.768 kHz

// If we ever want to change to a slower timer, we can adjust the increment
// per cycle.
const MILLIS_INCREMENT: u32 = 1;
// Get a "global mutable"
static MILLIS_COUNTER: Mutex<Cell<u32>> = Mutex::new(Cell::new(0));

// Using an AGTimer for creating the `millis()` function equivalent for timing.
pub fn millis() -> u32 {
    free(|cs| MILLIS_COUNTER.borrow(cs).get())
}

pub fn set_up_millis<T: AGTimerInstance>(agt: AGTimer<T, Unconfigured>) {
    let base_agtcfg = AGTCfg {
        counts: 3000,
        prescaler: AGTPrescaler::PCLKD_8,
    };
    let agt0 = agt.configure(base_agtcfg);
    // Interrupts are available starting on ICU8
    unsafe {
        // Use IEL8 for the millisecond counter
        ra4m1::NVIC::unmask(ra4m1::Interrupt::IEL8);
    };
    // Enable the interrupt for AGT0 underflow (Should be once per ms)
    let p = unsafe { ra4m1::Peripherals::steal() };
    p.ICU.ielsr[8].write(|w| unsafe { w.iels().bits(0x01E) });
    // Start the millis timer
    agt0.start();
}

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
        cell.set(ctr.wrapping_add(MILLIS_INCREMENT));
    })
}
