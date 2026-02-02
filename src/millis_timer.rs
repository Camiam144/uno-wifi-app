use core::cell::Cell;
use core::marker::PhantomData;
use cortex_m::interrupt::Mutex;
use cortex_m::interrupt::free;
use ra4m1::interrupt;

use crate::hal::timer::Configured;
use crate::hal::timer::{AGTCfg, AGTPrescaler, AGTimer, AGTimerInstance, Unconfigured};
use crate::interrupts::Binding;
use crate::interrupts::Handler;

// If we ever want to change to a slower timer, we can adjust the increment
// per cycle.
const MILLIS_INCREMENT: u32 = 1;

// Get a "global mutable"
static MILLIS_COUNTER: Mutex<Cell<u32>> = Mutex::new(Cell::new(0));

// Using an AGTimer for creating the `millis()` function equivalent for timing.
pub fn millis() -> u32 {
    free(|cs| MILLIS_COUNTER.borrow(cs).get())
}

/// We are using an AGT to drive the `millis()` function. This timer is driven
/// by a divider of PCLKB, which can be 1, 2, or 8, or a selectable divider
/// of AGTLCLK or AGTSCLK/d (d = 1, 2, 4, 8, 16, 32, 64, or 128)
/// By default, PCLKB is running at 1/2 main speed, so 24 MHz
/// Also option is the AGTLCLK which runs off the LOCO and can run in low power
/// or snooze mode, this runs up to 32.768 kHz
pub struct MillisTimer<T: AGTimerInstance> {
    _phantom: PhantomData<T>,
    timer: AGTimer<T, Configured>,
}

impl<T: AGTimerInstance> MillisTimer<T> {
    pub fn new<IRQ>(agt: AGTimer<T, Unconfigured>, _irq: IRQ) -> Self
    where
        IRQ: Binding<MillisHandler<T>>,
    {
        let base_agtcfg = AGTCfg {
            counts: 3000,
            prescaler: AGTPrescaler::PCLKD_8,
        };
        let agt_cfg = agt.configure(base_agtcfg);

        let millis_interrupt = <IRQ as Binding<MillisHandler<T>>>::interrupt();
        unsafe {
            ra4m1::NVIC::unmask(millis_interrupt);
        };

        // Just hardcode these in as there are only 2 AGT options.
        let offset = match T::CHANNEL {
            0 => 0x01E,
            1 => 0x021,
            _ => unreachable!(),
        };

        // Enable the interrupt for AGT0 underflow (Should be once per ms)
        let p = unsafe { ra4m1::Peripherals::steal() };
        p.ICU.ielsr[millis_interrupt as usize].write(|w| unsafe { w.iels().bits(offset) });

        Self {
            _phantom: PhantomData,
            timer: agt_cfg,
        }
    }

    pub fn start(&mut self) {
        self.timer.start();
    }
    pub fn stop(&mut self) {
        self.timer.stop();
    }
    pub fn clear(&mut self) {
        self.timer.clear();
    }
}

pub struct MillisHandler<T: AGTimerInstance> {
    _phantom: PhantomData<T>,
}

// This is the function that should run when the interrupt is triggered, so
// I think this is the impl Handler for MillisHandler or whatever
impl<T: AGTimerInstance> Handler for MillisHandler<T> {
    unsafe fn on_interrupt(interrupt: interrupt) {
        // clear the flag
        let p = unsafe { ra4m1::Peripherals::steal() };
        p.ICU.ielsr[interrupt as usize].modify(|_, w| w.ir()._0());

        // Handle interrupt by clearing TUNDF
        unsafe {
            (*T::BLOCK).agtcr.modify(|_, w| w.tundf().clear_bit());
        }

        // Update current MILLIS_COUNTER
        free(|cs| {
            let cell = MILLIS_COUNTER.borrow(cs);
            let ctr = cell.get();
            cell.set(ctr.wrapping_add(MILLIS_INCREMENT));
        })
    }
}
