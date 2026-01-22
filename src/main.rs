#![no_main]
#![no_std]

// use cortex_m::asm::wfi;
// use uno_wifi_app::display_info::show_info;
use uno_wifi_app::hal::gpio::{GpioExt, unlock_pmnpfs_register};
use uno_wifi_app::hal::timer::{TimerExt, enable_agtimers, enable_gptimers};
use uno_wifi_app::led_matrix::LEDMatrix;
use uno_wifi_app::millis_timer::{millis, set_up_millis};
// use uno_wifi_app::time::SYSCLK_FREQ;
use uno_wifi_app::{self as _}; // global logger + panicking-behavior + memory layout
mod animation;

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
//     unsafe { <led_matrix::DisplayHandler as interrupts::Handler>::on_interrupt(ra4m1::Interrupt:IEL9) };
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

    unsafe {
        unlock_pmnpfs_register();
        enable_gptimers();
        enable_agtimers();
    }

    unsafe {
        cortex_m::interrupt::enable();
    }
    // we're using IEL8 and IEL9 right now, need some way to track and pass them around

    // We are using AGT0 to drive the `millis()` function. This timer is driven
    // by a divider of PCLKB, which can be 1, 2, or 8, or a selectable divider
    // of AGTLCLK or AGTSCLK/d (d = 1, 2, 4, 8, 16, 32, 64, or 128)
    // By default, PCLKB is running at 1/2 main speed, so 24 MHz
    // Also option is the AGTLCLK which runs off the LOCO and can run in low power
    // or snooze mode, this runs up to 32.768 kHz
    let agt0 = perph.AGT0.into_timer();
    set_up_millis(agt0);

    let p0_pins = perph.PORT0.split();
    let p2_pins = perph.PORT2.split();

    // Pins for the LED screen driver
    let p003 = p0_pins.p003.into_input();
    let p004 = p0_pins.p004.into_input();
    let p011 = p0_pins.p011.into_input();
    let p012 = p0_pins.p012.into_input();
    let p013 = p0_pins.p013.into_input();
    let p015 = p0_pins.p015.into_input();
    let p204 = p2_pins.p204.into_input();
    let p205 = p2_pins.p205.into_input();
    let p206 = p2_pins.p206.into_input();
    let p212 = p2_pins.p212.into_input();
    let p213 = p2_pins.p213.into_input();

    // Timer to drive LED display
    let ledtimer = perph.GPT165.into_timer();

    let mut display_matrix = LEDMatrix::new(
        p003, p004, p011, p012, p013, p015, p204, p205, p206, p212, p213, ledtimer,
    );
    static heart: [u32; 4] = [
        0b00110001100001001010010001000100,
        0b01000010000010000001000100000000,
        0b10100000000001000000000000000000,
        500,
    ];

    let on: [u32; 3] = [
        0b00110001100001001010010001000100,
        0b01000010000010000001000100000000,
        0b10100000000001000000000000000000,
    ];

    static smile: [u32; 4] = [0x19819, 0x80000001, 0x81f8000, 500];

    // let mut count_overflow: u32 = 0;
    // let mut num_seconds: u32 = 0;
    // let mut current_frame: usize = 0;

    static frames: &[[u32; 4]; 2] = &[heart, smile];
    display_matrix.load_sequence(&animation::animation, true);
    // display_matrix.load_frame(on);

    let mut last_millis = millis();
    let ms_per_frame = 1000;

    defmt::println!("Entering main loop");
    loop {
        // We should overflow roughly 9600 times per second
        // timer running at approx 9600 hz

        let current_millis = millis();
        //
        if current_millis - last_millis > ms_per_frame {
            last_millis = current_millis;
            //     current_frame += 1;
            //     current_frame %= frames.len();
            defmt::println!("Seconds passed {}", last_millis);
            //     display_matrix.load_frame(&frames[current_frame]);
        }
    }
    // uno_wifi_app::exit()
}
