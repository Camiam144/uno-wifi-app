#![no_main]
#![no_std]

use ra4m1::interrupt;
use uno_wifi_app::hal::gpio::{GpioExt, unlock_pmnpfs_register};
use uno_wifi_app::hal::i2c::{I2cBus, enable_qwiic_bus};
use uno_wifi_app::hal::timer::{TimerExt, enable_agtimers, enable_gptimers};
use uno_wifi_app::led_matrix::{self, LEDMatrix};
use uno_wifi_app::millis_timer::{self, MillisTimer, millis};
use uno_wifi_app::modulinos::{ModulinoThermo, TemperatureUnits};
// use uno_wifi_app::modulinos::ModulinoPixels;
use uno_wifi_app::{bind_interrupts, hal};
// use uno_wifi_app::time::SYSCLK_FREQ;
use uno_wifi_app::{self as _}; // global logger + panicking-behavior + memory layout

// =================================
//      Interrupts assigned here
// =================================

bind_interrupts!(struct MillisIrq {
    IEL8 => crate::millis_timer::MillisHandler<crate::hal::timer::AGTimer0>;
});

bind_interrupts!(struct LedIrq {
    IEL9 => led_matrix::LEDHandler<crate::hal::timer::Gpt2>;
});

bind_interrupts!(struct QwiicIrq {
    IEL10 => hal::i2c::TXI_Handler<ra4m1::IIC0>;
    IEL11 => hal::i2c::TEI_Handler<ra4m1::IIC0>;
    IEL12 => hal::i2c::RXI_Handler<ra4m1::IIC0>;
    IEL13 => hal::i2c::NAK_Handler<ra4m1::IIC0>;
});

#[cortex_m_rt::entry]
fn main() -> ! {
    // This is effectively the "setup" block of the app
    defmt::println!("Launching application");
    let perph = ra4m1::Peripherals::take().unwrap();

    // let core_periph = cortex_m::Peripherals::take().unwrap();

    // uncomment this line to get a dump of a bunch of registers for inspection
    // or debugging purposes.
    // show_info(&perph);

    // Eventually I should have some `init(perph)` function that sets up the board
    // as I expect
    unsafe {
        unlock_pmnpfs_register();
        enable_gptimers();
        enable_agtimers();
        enable_qwiic_bus();
    }

    unsafe {
        cortex_m::interrupt::enable();
    }

    // We are using AGT0 to drive the `millis()` function. This timer is driven
    let agt0 = perph.AGT0.into_timer();
    // Since I'm going to use agt0 here, I need to explicitly declare AGT0 in the
    // interrupt preamble.
    let mut millis_timer = MillisTimer::new(agt0, MillisIrq);
    millis_timer.start();

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
    let ledtimer = perph.GPT162.into_timer();

    let mut display_matrix = LEDMatrix::new(
        p003, p004, p011, p012, p013, p015, p204, p205, p206, p212, p213, ledtimer, LedIrq,
    );

    let mut flat_bitmap: [u8; 96] = [0; 96];
    const SNAKE_LEN: usize = 5;
    let snake_tick_dur = 250;
    let mut last_snake = millis();
    let mut snake_tail = 0;
    display_matrix.render_flatmap(&flat_bitmap);

    // Get some stuff for the qwiic? Should be on iic bus 0
    let p4_pins = perph.PORT4.split();
    let p400 = p4_pins.p400;
    let p401 = p4_pins.p401;
    let iic0_bus = perph.IIC0;

    let qwiic_bus = I2cBus::new(iic0_bus, p401, p400, QwiicIrq);

    let mut thermo_modulino = ModulinoThermo::new(qwiic_bus);

    let update_dir = 1500;
    let mut last_update = millis();
    defmt::println!("Entering main loop");
    loop {
        if millis() - last_snake >= snake_tick_dur {
            last_snake = millis();
            snake_tail += 1;
            if snake_tail >= 96 {
                snake_tail = 0;
            }
            flat_bitmap.fill(0);
            light_snake(snake_tail, SNAKE_LEN, &mut flat_bitmap);
            display_matrix.render_flatmap(&flat_bitmap);
        }

        if millis() - last_update >= update_dir {
            last_update = millis();

            let (stale, pct_h, temperature) =
                match thermo_modulino.read_data(TemperatureUnits::Fahrenheit) {
                    Ok(val) => val,
                    Err(err) => {
                        defmt::println!("Thermo read error {}", err);
                        uno_wifi_app::exit();
                    }
                };

            defmt::println!(
                "Stale {}, Pct H {}%, Temp {} deg",
                stale,
                pct_h,
                temperature
            );
        }

        cortex_m::asm::wfi();
    }
    // uno_wifi_app::exit()
}

fn light_snake(snake_tail: usize, snake_len: usize, map: &mut [u8; 96]) {
    let snake_head = snake_tail + snake_len;
    for mut i in snake_tail..=snake_head {
        if i >= 96 {
            i %= 96;
        }
        map[i] = 1;
    }
}
