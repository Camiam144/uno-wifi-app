#![no_main]
#![no_std]

use ra4m1::interrupt;
use uno_wifi_app::hal::gpio::{GpioExt, PinExt, Pull, unlock_pmnpfs_register};
use uno_wifi_app::hal::timer::{TimerExt, enable_agtimers, enable_gptimers};
use uno_wifi_app::led_matrix::{self, LEDMatrix};
use uno_wifi_app::millis_timer::{self, MillisTimer, millis};
use uno_wifi_app::modulinos::enable_qwiic_bus;
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
    let qwiic_sda = p4_pins.p401.into_pullup_input().internal_resistor(Pull::Up);
    let qwiic_scl = p4_pins.p400.into_pullup_input().internal_resistor(Pull::Up);
    let iic0_bus = perph.IIC0;

    // This is all very raw for now, move this into modulinos.rs asap.
    // Set the pins to the right setting (IIC0 and also as peripheral functions)
    qwiic_sda
        .pmnpfs_reg()
        .modify(|_, w| w.pmr().set_bit().ncodr().set_bit());
    qwiic_sda
        .pmnpfs_reg()
        .modify(|_, w| unsafe { w.psel().bits(0b00111) });

    qwiic_scl
        .pmnpfs_reg()
        .modify(|_, w| w.pmr().set_bit().ncodr().set_bit());
    qwiic_scl
        .pmnpfs_reg()
        .modify(|_, w| unsafe { w.psel().bits(0b00111) });

    let sda = qwiic_sda.pmnpfs_reg().read().bits();
    let scl = qwiic_scl.pmnpfs_reg().read().bits();

    defmt::println!("sda: 0b{:032b}", sda);
    defmt::println!("scl: 0b{:032b}", scl);

    // I think the address for the 8 pixel modulino is 0x6C or 0x36
    // thermo is 0x44? Buttons 0x7C or maybe 0x3E?
    // depends on Docs for "Modulino Library" vs hardware address
    // Some of the modules are software-addressable though.
    //
    // Follow the steps outlined in figure 29.5 in manual:
    // SCL0, SDA0 pins not driven
    iic0_bus.iccr1.modify(|_, w| w.ice().clear_bit());
    // IIC reset
    iic0_bus.iccr1.modify(|_, w| w.iicrst().set_bit());
    // Internal reset, SCL0, SDA0 pins in active state
    iic0_bus.iccr1.modify(|_, w| w.ice().set_bit());
    // set transfer bit rate in ICMR1 and ICBRL/ICBRH
    // for now we will leave the icmr1 clock as the default PCLKB clock,
    // which is running at 24 MHz. Standard slow mode is 100 kHz. I have these
    // precalculated
    iic0_bus.icmr1.modify(|_, w| w.cks()._011());
    iic0_bus.icbrh.modify(|_, w| unsafe { w.brh().bits(0xA) });
    iic0_bus.icbrl.modify(|_, w| unsafe { w.brl().bits(0xC) });
    // I don't know how many interrupts to set. Maybe for now we use the four
    // noacknowledge, recieve full, transmit end, and transmit empty.
    // Use polling for now fix this later
    // iic0_bus.icier.modify(|_, w| {
    //     w.nakie()
    //         .set_bit()
    //         .rie()
    //         .set_bit()
    //         .teie()
    //         .set_bit()
    //         .tie()
    //         .set_bit()
    // });
    // Should be done now? Release the reset
    iic0_bus.iccr1.modify(|_, w| w.iicrst().clear_bit());
    // Check some stuff I guess
    let iccr1 = iic0_bus.iccr1.read().bits();
    defmt::println!("iccr1 0b{:08b}", iccr1);
    let icmr1 = iic0_bus.icmr1.read().bits();
    defmt::println!("icmr1 0b{:08b}", icmr1);
    let icbrh = iic0_bus.icbrh.read().bits();
    defmt::println!("icbrh 0b{:08b}", icbrh);
    let icbrl = iic0_bus.icbrl.read().bits();
    defmt::println!("icbrl 0b{:08b}", icbrl);

    // First we have to broadcast the address of the temp probe with "WRITE"
    // Read the BBSY flag in ICCR2, then set ST in ICCR2 to 1
    if iic0_bus.iccr2.read().bbsy().bit_is_set() {
        defmt::println!("Bus isn't clear?");
    }
    // Issue start condition request
    iic0_bus.iccr2.modify(|_, w| w.st().set_bit());
    // Now we're in master transmit mode
    // We should check the TDRE flag in ICSR2
    let mut iccr2 = iic0_bus.iccr2.read().bits();
    defmt::println!("iccr2 0b{:08b}", iccr2);
    // At this point the registe is 1110000 which means
    // bus busy, master mode, transmit mode

    if iic0_bus.icsr2.read().tdre().bit_is_clear() {
        defmt::println!("Bus not in transmit mode");
        let icsr2 = iic0_bus.icsr2.read().bits();
        defmt::println!("icsr2 0b{:08b}", icsr2);
    }
    // let addr = 0x44;
    let addr = 0x36; // Arduino says this is 6C in software but 0x36 in hardware?
    let mut payload: u8 = addr << 1;
    // payload += 1; // read mode?
    iic0_bus.icdrt.write(|w| unsafe { w.icdrt().bits(payload) });
    defmt::println!("payload {:08b}", payload);
    // Check if response
    iccr2 = iic0_bus.iccr2.read().bits();
    defmt::println!("iccr2 0b{:08b}", iccr2);
    let icsr2 = iic0_bus.icsr2.read().bits();
    defmt::println!("icsr2 0b{:08b}", icsr2);

    if iic0_bus.icsr2.read().nackf().bit_is_set() {
        let icsr2 = iic0_bus.icsr2.read().bits();
        defmt::println!("icsr2 0b{:08b}", icsr2);
        defmt::println!("No module responded on {:02x}", addr);
        iic0_bus.iccr2.modify(|_, w| w.sp().set_bit());
        // wait for bus to stop
        let now = millis();
        while millis() - now <= 5 {
            cortex_m::asm::nop();
        }
        uno_wifi_app::exit();
    }

    let pixel_data: [u8; 32] = [
        0x0E | 25,
        255,
        0,
        255,
        0x0E | 25,
        255,
        0,
        255,
        0x0E | 25,
        255,
        0,
        255,
        0x0E | 25,
        255,
        0,
        255,
        0x0E | 25,
        255,
        0,
        255,
        0x0E | 25,
        255,
        0,
        255,
        0x0E | 25,
        255,
        0,
        255,
        0x0E | 25,
        255,
        0,
        255,
    ];

    for val in pixel_data.iter() {
        //Check for nack
        if !iic0_bus.icsr2.read().nackf().bit_is_clear() {
            defmt::println!("NACK from slave on write");
            iic0_bus.iccr2.modify(|_, w| w.sp().set_bit());
            // wait for bus to stop
            let now = millis();
            while millis() - now <= 5 {
                cortex_m::asm::nop();
            }
            uno_wifi_app::exit();
        }
        // Wait for tdre flag to set, indicating write buffer is empty
        while iic0_bus.icsr2.read().tdre().bit_is_clear() {
            cortex_m::asm::nop();
        }

        // Bit is 1, we can write:
        iic0_bus.icdrt.write(|w| unsafe { w.icdrt().bits(*val) });
    }

    while iic0_bus.icsr2.read().tend().bit_is_clear() {
        cortex_m::asm::nop();
    }

    // Issue a stop condition so the sensor takes a measurement
    iic0_bus.icsr2.modify(|_, w| w.stop().clear_bit());
    iic0_bus.iccr2.modify(|_, w| w.sp().set_bit());
    //
    // Wait for the thing to stop
    while iic0_bus.icsr2.read().stop().bit_is_clear() {
        cortex_m::asm::nop();
    }

    // Clear STOP and NACKF flags
    iic0_bus
        .icsr2
        .modify(|_, w| w.stop().clear_bit().nackf().clear_bit());

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
