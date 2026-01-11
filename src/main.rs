#![no_main]
#![no_std]

use cortex_m::delay::{self, Delay};
use uno_wifi_app::arduino_led_matrix::ArduinoLEDMatrix;
use uno_wifi_app::hal::gpio::{GpioExt, unlock_pmnpfs_register};
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

    let p012 = p0_pins.p012;
    let p205 = p2_pins.p205;

    // These are in order of the lines in the spec sheet
    // let matrix_pins = [
    //     p0_pins.p003.erase(),
    //     p0_pins.p004.erase(),
    //     p0_pins.p011.erase(),
    //     p0_pins.p012.erase(),
    //     p0_pins.p013.erase(),
    //     p0_pins.p015.erase(),
    //     p2_pins.p204.erase(),
    //     p2_pins.p205.erase(),
    //     p2_pins.p206.erase(),
    //     p2_pins.p212.erase(),
    //     p2_pins.p213.erase(),
    // ];

    // defmt::println!("OFS0 values");
    // let ofs_addr: u32 = 0x00000400;
    // let ofs0_val = unsafe {
    //     let ptr: *const u32 = ofs_addr as *const u32;
    //     core::ptr::read(ptr)
    // };
    // defmt::println!("0b{:032b}", ofs0_val);
    //
    // defmt::println!("OFS1 values");
    // let ofs1_addr: u32 = 0x00000404;
    // let ofs1_val = unsafe {
    //     let ptr: *const u32 = ofs1_addr as *const u32;
    //     core::ptr::read(ptr)
    // };
    // defmt::println!("0b{:032b}", ofs1_val);
    // defmt::println!("Some buffers");
    // let ptr = ra4m1::PFS::PTR;
    // unsafe {
    //     let p002
    // }
    // defmt::println!();

    let mut delay = Delay::new(core_periph.SYST, SYSCLK_FREQ.raw());
    // let mut led_matrix = ArduinoLEDMatrix::new(matrix_pins);
    let heart: [u32; 3] = [
        0b00110001100001001010010001000100,
        0b01000010000010000001000100000000,
        0b10100000000001000000000000000000,
    ];

    let smile: [u32; 3] = [0x19819, 0x80000001, 0x81f8000];

    let mut count_overflow = 0;
    let mut num_seconds = 0;
    let mut current_frame = 0;

    let frames = [heart, smile];
    // led_matrix.load_frame(frames[current_frame]);
    // let mut overflow_flag = perph.GPT164.gtst.read().tcfpo().bit();

    get_timer();
    defmt::println!("Entering main loop");
    let mut p012 = p012.into_push_pull_output();
    let mut p205 = p205.into_push_pull_output();
    p012.set_low();
    p205.set_low();
    loop {
        p012.set_low();
        p205.set_high();

        delay.delay_ms(10);
        p205.set_low();
        p012.set_high();
        delay.delay_ms(10);
        // let overflow_flag = perph.GPT164.gtst.read().tcfpo().bit();
        //
        // if overflow_flag {
        //     // defmt::println!("Counter Overflow");
        //     perph.GPT164.gtst.write(|w| w.tcfpo().clear_bit());
        //     count_overflow += 1;
        //     led_matrix.render_frame();
        //
        //     if count_overflow % (4800) == 0 {
        //         num_seconds += 1;
        //         // defmt::println!("{}s", num_seconds);
        //         current_frame += 1;
        //         // defmt::println!("loading frame {}", current_frame);
        //         led_matrix.load_frame(frames[current_frame % 2]);
        //     }
        // }
    }
    // uno_wifi_app::exit()
}
