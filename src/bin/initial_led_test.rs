#![no_main]
#![no_std]

use cortex_m::delay::Delay;
use uno_wifi_app::hal::gpio::{Pin, PinMode, Port};
use uno_wifi_app::led_matrix;
use uno_wifi_app::{self as _, time::SYSCLK_FREQ}; // global logger + panicking-behavior + memory layout

// mod delay;

// use delay::time;

#[cortex_m_rt::entry]
fn main() -> ! {
    // This is effectively the "setup" block of the app
    defmt::println!("Launching application");

    let core_periph = cortex_m::Peripherals::take().unwrap();
    // let periph = unsafe { ra4m1::Peripherals::steal() };

    // let p102_pfs = periph.PFS.p100pfs().get(2).unwrap();
    // // unlock the register, make the writes, and relock the register?
    // // defmt::println!("PWPR vals 0b{:032b}", periph.PMISC.pwpr.read().bits());
    // periph.PMISC.pwpr.write(|w| w.b0wi().clear_bit());
    // periph.PMISC.pwpr.write(|w| w.pfswe().set_bit());
    //
    // p102_pfs.modify(|_, w| unsafe { w.psel().bits(0b00000) });
    // p102_pfs.write(|w| w.pmr().clear_bit());
    //
    // periph.PMISC.pwpr.write(|w| w.pfswe().clear_bit());
    // periph.PMISC.pwpr.write(|w| w.b0wi().set_bit());
    //

    // periph.PMISC.pwpr.write(|w| w.b0wi().clear_bit());
    // periph.PMISC.pwpr.write(|w| w.pfswe().set_bit());

    // periph.PMISC.pwpr.write(|w| w.pfswe().clear_bit());
    // periph.PMISC.pwpr.write(|w| w.b0wi().set_bit());
    // let p012_pfs = periph.PFS.p012pfs();
    // let p205_pfs = periph.PFS.p205pfs();
    // defmt::println!("Pin 012: 0xb{:032b}", p012_pfs.read().bits());
    // defmt::println!("Pin 205: 0xb{:032b}", p205_pfs.read().bits());

    let mut delay = Delay::new(core_periph.SYST, SYSCLK_FREQ.raw());
    let mut p012 = Pin::new(Port::PORT0, 12, PinMode::Output);
    let mut p205 = Pin::new(Port::PORT2, 5, PinMode::Output);

    defmt::println!("Entering main loop");
    loop {
        p012.set_low();
        p205.set_high();

        delay.delay_ms(5);

        p205.set_low();
        p012.set_high();

        delay.delay_ms(5);
    }

    // uno_wifi_app::exit()
}
