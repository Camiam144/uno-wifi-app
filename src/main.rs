#![no_main]
#![no_std]

use cortex_m::asm;
use cortex_m::delay::Delay;
use embedded_hal::digital::PinState;
use uno_wifi_app::hal::gpio::{Pin, PinMode, Port};
use uno_wifi_app::hal::simple_timer::get_timer;
use uno_wifi_app::{self as _, time::SYSCLK_FREQ}; // global logger + panicking-behavior + memory layout

// mod delay;

// use delay::time;

#[cortex_m_rt::entry]
fn main() -> ! {
    // This is effectively the "setup" block of the app
    defmt::println!("Launching application");
    let perph = unsafe { ra4m1::Peripherals::steal() };

    let core_periph = cortex_m::Peripherals::take().unwrap();

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

    defmt::println!("GTWP write protection for on ch 4 (should be all 0s)");
    let gtwp_val = perph.GPT164.gtwp.read().bits();
    defmt::println!("{:032b}", gtwp_val);

    let counter_ref = &perph.GPT164.gtcnt;
    let mut count_read = counter_ref.read().bits();

    // Read a bunch of stuff to see if we set things up as expected
    let sckdivcr_val = perph.SYSTEM.sckdivcr.read().bits();
    defmt::println!("SCKDIVCR register b0-2 are pckld divider");
    defmt::println!("0b{:032b}", sckdivcr_val);

    let mut p205 = Pin::new(Port::PORT2, 5, PinMode::Output);
    let mut p012 = Pin::new(Port::PORT0, 12, PinMode::Output);
    p205.set_high();
    p012.set_low();
    // let p115_ref = Pin::new(Port::PORT1, 15, PinMode::Input);
    // let mut prev_state = p115_ref.read_state();
    // if prev_state == PinState::High {
    //     defmt::println!("p115_ref set high");
    // }
    // const NUM_LOOPS: usize = 50;
    // let mut numarray: [u32; NUM_LOOPS] = [0; NUM_LOOPS];
    // let mut i = 0;

    let gtst_reader = &perph.GPT164.gtst;
    defmt::println!("Initial gtst reg values");
    let gtst_val = gtst_reader.read().bits();
    defmt::println!("0b{:032b}", gtst_val);

    get_timer();
    defmt::println!("Entering main loop");
    loop {
        // for i in 0..NUM_LOOPS * 20 {
        // defmt::println!("Current count: {:032b}", count_read);
        // count_read = counter_ref.read().bits();
        // numarray[i % NUM_LOOPS] = count_read;

        let overflow_flag = gtst_reader.read().tcfpo().bit();
        if overflow_flag {
            defmt::println!("Counter Overflow");
            gtst_reader.write(|w| w.tcfpo().clear_bit());

            if p205.is_high() {
                p205.set_low();
            } else {
                p205.set_high();
            }
        }

        // let curr_state = p115_ref.read_state();
        // if curr_state != prev_state {
        //     defmt::println!("P115 toggled");
        //     prev_state = curr_state;
        // }
    }
    // for cnt in numarray {
    //     defmt::println!("{}", cnt);
    // }
    //
    // uno_wifi_app::exit()
}
