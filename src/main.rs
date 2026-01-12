#![no_main]
#![no_std]

use cortex_m::delay::{self, Delay};
use uno_wifi_app::arduino_led_matrix::ArduinoLEDMatrix;
use uno_wifi_app::hal::gpio::{AnyPin, GpioExt, unlock_pmnpfs_register};
use uno_wifi_app::hal::simple_timer::get_timer;
use uno_wifi_app::time::SYSCLK_FREQ;
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

    let p0_pins = perph.PORT0.split();
    let p2_pins = perph.PORT2.split();

    // let p012 = p0_pins.p012;
    // let p205 = p2_pins.p205;
    // let test012 = p012.into_input();
    // let test205 = p205.into_input();

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

    let mut delay = Delay::new(core_periph.SYST, SYSCLK_FREQ.raw());
    let mut led_matrix = ArduinoLEDMatrix::new(
        p003, p004, p011, p012, p013, p015, p204, p205, p206, p212, p213,
    );
    let heart: [u32; 3] = [
        0b00110001100001001010010001000100,
        0b01000010000010000001000100000000,
        0b10100000000001000000000000000000,
    ];

    let smile: [u32; 3] = [0x19819, 0x80000001, 0x81f8000];

    let mut count_overflow: u32 = 0;
    // let mut num_seconds: u32 = 0;
    let mut current_frame: usize = 0;

    let frames = [heart, smile];
    led_matrix.load_frame(frames[0]);
    // let mut overflow_flag = perph.GPT164.gtst.read().tcfpo().bit();

    get_timer();
    defmt::println!("Entering main loop");
    loop {
        // We should overflow roughly 9600 times per second
        // timer running at approx 9600 hz
        let overflow_flag = perph.GPT164.gtst.read().tcfpo().bit();
        if overflow_flag {
            // defmt::println!("Counter Overflow");
            perph.GPT164.gtst.write(|w| w.tcfpo().clear_bit());
            count_overflow += 1;
            led_matrix.render_frame();
        }

        if (count_overflow % 9600) == 0 {
            // num_seconds += 1;
            // defmt::println!("{}s", num_seconds);
            current_frame += 1;
            // defmt::println!("loading frame {}", current_frame);
            led_matrix.load_frame(frames[current_frame % 2]);
        }
    }
    // uno_wifi_app::exit()
}
