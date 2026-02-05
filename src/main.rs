#![no_main]
#![no_std]

use ra4m1::interrupt;
use uno_wifi_app::hal::gpio::{GpioExt, unlock_pmnpfs_register};
use uno_wifi_app::hal::timer::{TimerExt, enable_agtimers, enable_gptimers};
use uno_wifi_app::led_matrix::{self, LEDMatrix};
use uno_wifi_app::millis_timer::{self, MillisTimer, millis};
use uno_wifi_app::modulinos::{I2cError, Iic0, enable_qwiic_bus};
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
    let tick_dur = 250;
    let mut last_millis = millis();
    let mut snake_tail = 0;
    display_matrix.render_flatmap(&flat_bitmap);

    // Get some stuff for the qwiic? Should be on iic bus 0
    let p4_pins = perph.PORT4.split();
    let p400 = p4_pins.p400;
    let p401 = p4_pins.p401;
    let iic0_bus = perph.IIC0;

    let modulino = Iic0::new(iic0_bus, p401, p400);

    // Each pixel is [brightness, B, G, R]
    let pixel_data: [u8; 32] = [
        0xE0 | 1,
        0,
        0xFF,
        0xFF,
        0xE0 | 4,
        0,
        0xFF,
        0xFF,
        0xE0 | 8,
        0,
        0xFF,
        0xFF,
        0xE0 | 16,
        0,
        0xFF,
        0xFF,
        0xE0 | 31,
        0,
        0xFF,
        0xFF,
        0xE0 | 16,
        0,
        0xFF,
        0xFF,
        0xE0 | 8,
        0,
        0xFF,
        0xFF,
        0xE0 | 4,
        0,
        0xFF,
        0xFF,
    ];

    let pixel_addr = 0x36;

    match modulino.write_blocking(pixel_addr, &pixel_data, true, true) {
        Ok(()) => {
            defmt::println!("pixels woo");
        }
        Err(err) => {
            defmt::println!("oh no: {}", err)
        }
    }

    // for val in pixel_data.iter() {
    //Check for nack

    // // For the temp sensor, we need to let it do a measurement so just
    // // busy wait for a measurement
    // let now = millis();
    // while millis() - now <= 100 {
    //     cortex_m::asm::nop();
    // }
    //
    // let iccr1 = iic0_bus.iccr1.read().bits();
    // defmt::println!("xmit iccr1 0b{:08b}", iccr1);
    //
    // // Now we should be able to read
    // // check if bus is busy
    // if iic0_bus.iccr2.read().bbsy().bit_is_set() {
    //     defmt::println!("Bus for recieve isn't clear?");
    // }
    // // issue a start command
    // iic0_bus.iccr2.modify(|_, w| w.st().set_bit());
    // payload = (addr << 1) | 0b00000001; // read mode
    // iic0_bus.icdrt.write(|w| unsafe { w.icdrt().bits(payload) });
    // defmt::println!("payload {:08b}", payload);
    //
    // iccr2 = iic0_bus.iccr2.read().bits();
    // defmt::println!("xmit iccr2 0b{:08b}", iccr2);
    // let icsr2 = iic0_bus.icsr2.read().bits();
    // defmt::println!("xmit icsr2 0b{:08b}", icsr2);
    //
    // if iic0_bus.icsr2.read().nackf().bit_is_set() {
    //     let icsr2 = iic0_bus.icsr2.read().bits();
    //     defmt::println!("icsr2 0b{:08b}", icsr2);
    //     defmt::println!("No module responded to read on {:02x}", addr);
    //     iic0_bus.iccr2.modify(|_, w| w.sp().set_bit());
    //     uno_wifi_app::exit();
    // }
    // // Wait for TDRE bit to indicate transmission done
    // while iic0_bus.icsr2.read().tdre().bit_is_set() {
    //     cortex_m::asm::nop();
    // }
    //
    // iccr2 = iic0_bus.iccr2.read().bits();
    // defmt::println!("xmit iccr2 0b{:08b}", iccr2);
    // let icsr2 = iic0_bus.icsr2.read().bits();
    // defmt::println!("xmit icsr2 0b{:08b}", icsr2);
    //
    // // Wait for RDRF bit to be set
    // defmt::println!("Waiting for dummy rdrf read");
    // while iic0_bus.icsr2.read().rdrf().bit_is_clear() {
    //     cortex_m::asm::nop();
    // }
    //
    // // Dummy read to start stuff
    // let _ = iic0_bus.icdrr.read().icdrr().bits();
    // // Now for the temp/humidity module we should get 4 bytes of data back
    // let mut recieved: [u8; 4] = [0; 4];
    // // now we should do 4 consecutive reads
    // for (i, item) in recieved.iter_mut().enumerate() {
    //     while iic0_bus.icsr2.read().rdrf().bit_is_clear() {
    //         cortex_m::asm::nop();
    //     }
    //     *item = iic0_bus.icdrr.read().icdrr().bits();
    //     if i == 2 {
    //         // Next byte is 2nd to last to se do this thing
    //         iic0_bus.icmr3.modify(|_, w| w.wait().set_bit());
    //     }
    //     if i == 3 {
    //         // Next byte is the last one so we set nack and set stop condition
    //         iic0_bus.iccr2.modify(|_, w| w.sp().set_bit());
    //         iic0_bus.icmr3.modify(|_, w| w.ackbt().set_bit());
    //     }
    //     defmt::println!("{}", item);
    // }
    //
    // // set flags for next op
    // iic0_bus
    //     .icsr2
    //     .modify(|_, w| w.nackf().clear_bit().stop().clear_bit());
    //
    // // Do some magic stuff
    // let humid: u16 = ((recieved[0] as u16 & 0b00111111) << 8_u16) + recieved[1] as u16;
    // let temperature: u16 = ((recieved[2] as u16) << 6_u16) + ((recieved[3] as u16) >> 2);
    //
    // let pct_h = pct_humid(humid);
    // let celcius = temp_c(temperature);

    // defmt::println!("Temp: {} and humidity {}", celcius, pct_h);

    defmt::println!("Entering main loop");
    loop {
        if millis() - last_millis >= tick_dur {
            last_millis = millis();
            snake_tail += 1;
            if snake_tail >= 96 {
                snake_tail = 0;
            }
            flat_bitmap.fill(0);
            light_snake(snake_tail, SNAKE_LEN, &mut flat_bitmap);
            display_matrix.render_flatmap(&flat_bitmap);
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

// Stuff for nw
fn pct_humid(humid: u16) -> u16 {
    (humid / (2_u16.pow(14) - 1)) * 100
}

fn temp_c(temp: u16) -> i32 {
    ((temp / (2_u16.pow(14) - 1)) as i32) * 165 - 40
}

fn scale(val: usize, in_min: usize, in_max: usize, out_min: usize, out_max: usize) -> usize {
    (val - in_min) * (out_max - out_min) / (in_max - in_min) + out_min
}
