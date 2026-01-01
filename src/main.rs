#![no_main]
#![no_std]

use uno_wifi_app as _; // global logger + panicking-behavior + memory layout

use ra4m1::Peripherals;

#[cortex_m_rt::entry]
fn main() -> ! {
    defmt::println!("Launching application");

    let periph = unsafe { Peripherals::steal() };

    defmt::println!("Reading the PmnPFS reg for pin 102");
    let p102_pfs = periph.PFS.p100pfs().get(2).unwrap();

    defmt::println!("PmnPFS 0b{:b}", p102_pfs.read().bits());

    // unlock the register, make the writes, and relock the register?
    defmt::println!("PWPR vals 0b{:b}", periph.PMISC.pwpr.read().bits());
    periph.PMISC.pwpr.write(|w| w.b0wi().clear_bit());
    periph.PMISC.pwpr.write(|w| w.pfswe().set_bit());

    p102_pfs.modify(|_, w| unsafe { w.psel().bits(0b00000) });
    p102_pfs.write(|w| w.pmr().clear_bit());

    periph.PMISC.pwpr.write(|w| w.pfswe().clear_bit());
    periph.PMISC.pwpr.write(|w| w.b0wi().set_bit());
    for _ in 0..20 {
        cortex_m::asm::nop();
    }
    // .modify(|r, w| unsafe { w.pmr().clear_bit() });
    defmt::println!("PmnPFS 0b{:b}", p102_pfs.read().bits());

    defmt::println!("Attempting to configure pin102 to output");
    // periph
    //     .PORT1
    //     .pdr()
    //     .modify(|r, w| unsafe { w.pdr().bits(r.bits() | 1 << 2) });
    periph
        .PORT1
        .pcntr1()
        .modify(|r, w| unsafe { w.pdr().bits(r.pdr().bits() | 1 << 2) });

    defmt::println!("Port 1 pin 02 should be set to output in pdr now");
    // I guess we should wait some time before reading?
    for _ in 0..20 {
        cortex_m::asm::nop();
    }

    let pdr_reader = periph.PORT1.pdr().read();
    let pdr_val = pdr_reader.bits();
    // should be at 0x40040020
    defmt::println!("P102 PFS val 0b{:b}", p102_pfs.read().bits());
    defmt::println!("Port 1 pdr val 0x{:08X}", pdr_val);

    let mut pcntrl1_reader = periph.PORT1.pcntr1().read();
    defmt::println!("Port 1 pcntrl val 0x{:08X}", pcntrl1_reader.bits());

    let mut finished_iterations = 0;
    for _ in 0..10 {
        // This should set pin102 to high
        periph
            .PORT1
            .pcntr1()
            .modify(|r, w| unsafe { w.podr().bits(r.podr().bits() | 1 << 2) });

        for _ in 0..10 {
            cortex_m::asm::nop();
        }
        // Then read?
        pcntrl1_reader = periph.PORT1.pcntr1().read();
        defmt::println!("Val 0x{:08X}", pcntrl1_reader.bits());

        // Then we wait for a bit of time I guess
        for _ in 0..1000000 {
            cortex_m::asm::nop();
        }
        // then we toggle the bit off
        periph
            .PORT1
            .pcntr1()
            .modify(|r, w| unsafe { w.podr().bits(r.podr().bits() ^ 1 << 2) });

        // Read again? Did it change?
        for _ in 0..20 {
            cortex_m::asm::nop();
        }
        pcntrl1_reader = periph.PORT1.pcntr1().read();
        defmt::println!("Val 0x{:08X}", pcntrl1_reader.bits());

        // Then we wait for a bit of time I need a timer
        for _ in 0..1000000 {
            cortex_m::asm::nop();
        }

        finished_iterations += 1;
    }

    defmt::println!("Completed iterations {}", finished_iterations);
    uno_wifi_app::exit()
}
