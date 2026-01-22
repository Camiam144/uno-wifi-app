#![no_main]
#![no_std]

use cortex_m::delay::Delay;
use uno_wifi_app::hal::gpio::{GpioExt, Pin, PinMode, unlock_pmnpfs_register};
use uno_wifi_app::led_matrix;

// mod delay;

// use delay::time;

#[cortex_m_rt::entry]
fn main() -> ! {
    // This is effectively the "setup" block of the app
    defmt::println!("Launching application");

    let core_periph = cortex_m::Peripherals::take().unwrap();
    let periph = ra4m1::Peripherals::take().unwrap();

    unsafe {
        unlock_pmnpfs_register();
    }
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

    let p0_pins = periph.PORT0.split();
    let p2_pins = periph.PORT2.split();

    let mut delay = Delay::new(core_periph.SYST, 48000000);
    let mut p012 = p0_pins.p012.into_push_pull_output();
    let mut p205 = p2_pins.p205.into_push_pull_output();

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
