#![no_main]
#![no_std]

use uno_wifi_app as _; // global logger + panicking-behavior + memory layout

#[cortex_m_rt::entry]
fn main() -> ! {
    defmt::println!("Hello, world!");

    uno_wifi_app::exit()
}
