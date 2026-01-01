#![no_main]
#![no_std]

use uno_wifi_app as _; // global logger + panicking-behavior + memory layout

use ra4m1::Peripherals;

#[cortex_m_rt::entry]
fn main() -> ! {
    defmt::println!("Launching application");

    let periph = unsafe { Peripherals::steal() };

    defmt::println!("Attempting to configure pin102 to output");

    periph
        .PORT1
        .pdr()
        .modify(|r, w| unsafe { w.bits(r.bits() | 1 << 2) });

    defmt::println!("Port 1 pin 02 should be set to output in pdr now");
    // I guess we should wait some time before reading?
    for _ in 0..20 {
        cortex_m::asm::nop();
    }

    let pdr_reader = periph.PORT1.pdr().read();
    let pdr_val = pdr_reader.bits();
    // should be at 0x40040020
    defmt::println!("Port 1 pdr val 0x{:08X}", pdr_val);

    let mut podr_reader = periph.PORT1.podr().read();
    defmt::println!("Port 1 podr val 0x{:08X}", podr_reader.bits());

    let mut finished_iterations = 0;
    for _ in 0..20 {
        // This should set pin102 to high
        periph
            .PORT1
            .podr()
            .modify(|r, w| unsafe { w.bits(r.bits() | 1 << 2) });

        for _ in 0..20 {
            cortex_m::asm::nop();
        }
        // Then read?
        podr_reader = periph.PORT1.podr().read();
        defmt::println!("Val 0x{:08X}", podr_reader.bits());

        // Then we wait for a bit of time I guess
        for _ in 0..1000000 {
            cortex_m::asm::nop();
        }
        // then we toggle the bit off
        periph
            .PORT1
            .podr()
            .modify(|r, w| unsafe { w.bits(r.bits() ^ 1 << 2) });

        // Read again? Did it change?
        for _ in 0..20 {
            cortex_m::asm::nop();
        }
        podr_reader = periph.PORT1.podr().read();
        defmt::println!("Val 0x{:08X}", podr_reader.bits());

        // Then we wait for a bit of time I need a timer
        for _ in 0..1000000 {
            cortex_m::asm::nop();
        }

        finished_iterations += 1;
    }

    defmt::println!("Completed iterations {}", finished_iterations);
    uno_wifi_app::exit()
}
