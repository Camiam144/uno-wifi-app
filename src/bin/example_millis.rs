#![no_main]
#![no_std]

use core::cell::Cell;

use cortex_m::interrupt::{Mutex, free};
use ra4m1::interrupt;
use uno_wifi_app::arduino_led_matrix::ArduinoLEDMatrix;
use uno_wifi_app::hal::gpio::{GpioExt, unlock_pmnpfs_register};
use uno_wifi_app::hal::timer::{
    AGTCfg, AGTPrescaler, CountDir, GPTSourceT, PeriodicCfg, TimerExt, enable_agtimers,
    enable_gptimers,
};
// use uno_wifi_app::time::SYSCLK_FREQ;
use uno_wifi_app::{self as _}; // global logger + panicking-behavior + memory layout

// mod delay;

// use delay::time;

#[cortex_m_rt::entry]
fn main() -> ! {
    // This is effectively the "setup" block of the app
    defmt::println!("Launching application");
    let perph = ra4m1::Peripherals::take().unwrap();

    let core_periph = cortex_m::Peripherals::take().unwrap();

    unsafe { unlock_pmnpfs_register() };
    unsafe { enable_gptimers() };

    defmt::println!("OFS0 values");
    let ofs_addr: u32 = 0x00000400;
    let ofs0_val = unsafe {
        let ptr: *const u32 = ofs_addr as *const u32;
        core::ptr::read(ptr)
    };
    defmt::println!("0b{:032b}", ofs0_val);

    defmt::println!("OFS1 values");
    let ofs1_addr: u32 = 0x00000404;
    let ofs1_val = unsafe {
        let ptr: *const u32 = ofs1_addr as *const u32;
        core::ptr::read(ptr)
    };
    defmt::println!("0b{:032b}", ofs1_val);
    // Print out stop registers to make sure everything is okay
    defmt::println!("MSTPCRA");
    let mstpcra_val = perph.SYSTEM.mstpcra.read().bits();
    defmt::println!("0b{:032b}", mstpcra_val);

    let mstpcrb_val = &perph.MSTP.mstpcrb.read().bits();
    defmt::println!("MSTPCRB");
    defmt::println!("0b{:032b}", mstpcrb_val);

    let mstpcrc_val = &perph.MSTP.mstpcrc.read().bits();
    defmt::println!("MSTPCRC");
    defmt::println!("0b{:032b}", mstpcrc_val);

    let mstpcrd_val = &perph.MSTP.mstpcrd.read().bits();
    defmt::println!("MSTPCRD");
    defmt::println!("0b{:032b}", mstpcrd_val);

    // Clock speeds
    let sckdivcr_val = &perph.SYSTEM.sckdivcr.read().bits();
    defmt::println!("SCKDIVCR clock reg");
    defmt::println!("0b{:032b}", sckdivcr_val);

    // Interrupts are available starting on ICU8
    // Use IEL8/ICU8/whatever for the millisecond counter
    unsafe {
        ra4m1::NVIC::unmask(ra4m1::Interrupt::IEL8);
    };
    unsafe {
        cortex_m::interrupt::enable();
    }

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

    let timregval = unsafe {
        let agt0mr1addr = 0x40084009;
        let ptr: *const u32 = agt0mr1addr as *const u32;
        core::ptr::read(ptr)
    };
    defmt::println!("AGT0 AGTMR1");
    defmt::println!("0b{:08b}", timregval);

    const MILLIS_INCREMENT: u32 = 1;
    // Get a "global mutable"
    static MILLIS_COUNTER: Mutex<Cell<u32>> = Mutex::new(Cell::new(0));

    // Using an AGTimer for creating the `millis()` function equivalent for timing.
    fn millis() -> u32 {
        free(|cs| MILLIS_COUNTER.borrow(cs).get())
    }

    // Enable the interrupt for AGT0 underflow (Should be once per ms)
    perph.ICU.ielsr[8].write(|w| unsafe { w.iels().bits(0x01E) });

    // Write the interrupt function
    #[interrupt]
    unsafe fn IEL8() {
        // clear the flag
        // can I steal from within a function?
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

    // let mut delay = Delay::new(core_periph.SYST, SYSCLK_FREQ.raw());
    let mut led_matrix = ArduinoLEDMatrix::new(
        p003, p004, p011, p012, p013, p015, p204, p205, p206, p212, p213,
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
    led_matrix.load_frame(frames[0]);

    let ledtimer = perph.GPT162.into_timer().into_periodic();
    let ledtimercfg: PeriodicCfg = PeriodicCfg {
        gtssr: GPTSourceT::SOFTWARE,
        gtpsr: GPTSourceT::SOFTWARE,
        gtcsr: GPTSourceT::SOFTWARE,
        count_dir: CountDir::Up,
        freq_hz: 9600.0,
    };

    let ledtimer = ledtimer.configure(ledtimercfg);

    let frametimer = perph.GPT163.into_timer().into_periodic();
    let frametimercfg: PeriodicCfg = PeriodicCfg {
        gtssr: GPTSourceT::SOFTWARE,
        gtpsr: GPTSourceT::SOFTWARE,
        gtcsr: GPTSourceT::SOFTWARE,
        count_dir: CountDir::Up,
        freq_hz: 1.0,
    };
    let frametimer = frametimer.configure(frametimercfg);

    ledtimer.start();
    frametimer.start();
    agt0.start();

    defmt::println!("Entering main loop");
    loop {
        // We should overflow roughly 9600 times per second
        // timer running at approx 9600 hz
        let has_overflowed = ledtimer.has_overflowed();
        if has_overflowed {
            ledtimer.clear_overflow_flag();
            // count_overflow += 1;
            led_matrix.render_frame();
        }

        let frame_overflow = frametimer.has_overflowed();
        if frame_overflow {
            frametimer.clear_overflow_flag();
            // count_overflow = 0;
            // num_seconds += 1;
            // defmt::println!("{}s", num_seconds);
            current_frame += 1;
            current_frame %= 2;
            // defmt::println!("loading frame {}", current_frame);
            led_matrix.load_frame(frames[current_frame]);
        }
    }
    // uno_wifi_app::exit()
}
