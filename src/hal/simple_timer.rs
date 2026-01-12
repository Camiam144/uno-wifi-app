use crate::hal::timer::{self};
// All we do here is set up one simple GPTimer on channel 2

/// Enable the GPT 32 and 16 timers
pub fn enable_gptimers() {
    let gpt_stopreg = ra4m1::MSTP::PTR;
    defmt::println!("enabling mstpcrd in the MSTP");
    // Enable gpt_16 & gpt_32
    unsafe {
        (*gpt_stopreg)
            .mstpcrd
            .modify(|r, w| w.bits(r.bits() & !(0b11 << 5)));
    }
    let reg_val = unsafe { (*gpt_stopreg).mstpcrd.read().bits() };
    defmt::println!("mstpcrd value, bits 5 & 6 should be 0");
    defmt::println!("0b{:032b}", reg_val);
}

// You can clearly write more than 1 bit at a time if needed in one operation
// but this is just for simplicity for now
pub fn start_timer(ch: u8) {
    defmt::assert!(ch <= 7, "Channel must be between 0 and 7, got {}", ch);
    // All timers share the same GTSTR GTSTP and GTCLR registers?
    let gtstr = ra4m1::GPT164::PTR;
    unsafe {
        (*gtstr).gtstr.modify(|r, w| w.bits(r.bits() | 1 << ch));
    }
    let regval = unsafe { (*gtstr).gtstr.read().bits() };
    defmt::println!(
        "Starting timer on ch {} (bit {}), all reg should be the same",
        ch,
        ch
    );
    defmt::println!("0b{:032b}", regval);
}

pub fn stop_timer(ch: u8) {
    defmt::assert!(ch <= 7, "Channel must be between 0 and 7, got {}", ch);
    // All timers share the same GTSTR and GTSTP registers
    let gtstp = ra4m1::GPT164::PTR;
    unsafe {
        (*gtstp).gtstp.modify(|r, w| w.bits(r.bits() | 1 << ch));
    }
    let regval = unsafe { (*gtstp).gtstp.read().bits() };
    defmt::println!(
        "Stopping timer on ch {} (bit {}), all reg should be the same",
        ch,
        ch
    );
    defmt::println!("0b{:032b}", regval);
}

pub fn get_reg_block_for_gpt16(ch: u8) -> *const ra4m1::gpt162::RegisterBlock {
    defmt::assert!(
        (2..=7).contains(&ch),
        "gpt 16 timer channel must be between 2 and 7"
    );

    match ch {
        2 => ra4m1::GPT162::PTR,
        3 => ra4m1::GPT163::PTR,
        4 => ra4m1::GPT164::PTR,
        5 => ra4m1::GPT165::PTR,
        6 => ra4m1::GPT166::PTR,
        7 => ra4m1::GPT167::PTR,
        _ => unreachable!(),
    }
}

/// This is super simple for now, turn into a good struct later
pub fn get_timer() {
    defmt::println!("Running timer.");
    // Just trying stuff out
    enable_gptimers();

    let channel: u8 = 4;

    stop_timer(channel);

    let reg_ptr = get_reg_block_for_gpt16(channel);

    unsafe {
        // Set the Source Start, Stop, and Clear registers to the same source
        (*reg_ptr)
            .gtssr
            .write(|w| w.bits(timer::GPTSourceT::SOFTWARE as u32));
        (*reg_ptr)
            .gtpsr
            .write(|w| w.bits(timer::GPTSourceT::SOFTWARE as u32));
        (*reg_ptr)
            .gtcsr
            .write(|w| w.bits(timer::GPTSourceT::SOFTWARE as u32));
        // Set count direction to up
        (*reg_ptr).gtuddtyc.modify(|r, w| w.bits(r.bits() | 1));
        // Set timer to periodic sawtooth and prescaler to 1024
        (*reg_ptr).gtcr.write(|w| {
            w.md()
                .bits(timer::TimerModeT::PERIODIC as u8)
                .tpcs()
                .bits(timer::TimerPrescalerSelect::PCLKD_4 as u8)
        });
        // More magic numbers "set for overall period" on gtpbr
        (*reg_ptr).gtpr.write(|w| w.gtpr().bits(1250));
        (*reg_ptr).gtpbr.write(|w| w.gtpbr().bits(1250));
        // Clear the count, not sure which is correct
        (*reg_ptr).gtcnt.write(|w| w.gtcnt().bits(0_u32));
        // Set output on a, set to initial low, toggle on match, toggle on overflow
        // To set the duty cycle it looks like we set a compare match for the
        // registers and then modify the pin to be how we want, for example
        // have the pin initial output high, low on match, high on cycle end
        // then write the actual counts necessary into gtccrc and e (comp match)
        (*reg_ptr).gtior.modify(|_, w| {
            w.oae()
                .set_bit()
                .obe()
                .set_bit()
                .gtioa()
                .bits(0b01100)
                .gtiob()
                .bits(0b01100)
        });
        // Not sure what these magic numbers are, but the note is "for 25/75 M/S clock"
        // 0x1FF = 0b111111111
        (*reg_ptr).gtccra.write(|w| w.bits(0xFFFF));
        (*reg_ptr).gtccrb.write(|w| w.bits(0xFFFF));
        // (*reg_ptr).gtccrc.write(|w| w.bits(5000));
        // (*reg_ptr).gtccre.write(|w| w.bits(5000));
        // (*reg_ptr).gtccrd.write(|w| w.bits(0x5B8E));
        // Enable buffering I guess
        (*reg_ptr)
            .gtber
            .write(|w| w.ccra()._01().ccrb()._01().pr()._01());
    }

    // Read out some values to double check
    let gtssr_read = unsafe { (*reg_ptr).gtssr.read().bits() };
    defmt::println!("gtssr results after setting, only bit 31 set");
    defmt::println!("0b{:032b}", gtssr_read);

    let gtpsr_read = unsafe { (*reg_ptr).gtpsr.read().bits() };
    defmt::println!("gtpsr results after setting, only bit 31 set");
    defmt::println!("0b{:032b}", gtpsr_read);

    let gtcsr_read = unsafe { (*reg_ptr).gtcsr.read().bits() };
    defmt::println!("gtcsr results after setting, only bit 31 set");
    defmt::println!("0b{:032b}", gtcsr_read);

    defmt::println!("Timer gtior settings");
    let gtior_read = unsafe { (*reg_ptr).gtior.read().bits() };
    defmt::println!("0b{:032b}", gtior_read);

    defmt::println!("Compare match register");
    let gtccrb_read = unsafe { (*reg_ptr).gtccrb.read().bits() };
    defmt::println!("0b{:032b}", gtccrb_read);

    defmt::println!("Timer Cycle gtpr");
    let gtpr_read = unsafe { (*reg_ptr).gtpr.read().bits() };
    defmt::println!("0b{:032b}", gtpr_read);

    defmt::println!("Timer gtcr settings");
    let gtcr_read = unsafe { (*reg_ptr).gtcr.read().bits() };
    defmt::println!("0b{:032b}", gtcr_read);

    // Set the PSEL?
    // let mut p115 = Pin::new(super::gpio::Port::PORT1, 15, super::gpio::PinMode::Output);

    // Do the PWPR stuff on 115 to let it be used as a periph output
    // let pmisc_ptr = ra4m1::PMISC::PTR;
    // let pfs_ptr = ra4m1::PFS::PTR;
    // unsafe {
    //     (*pmisc_ptr).pwpr.write(|w| w.b0wi().clear_bit());
    //     (*pmisc_ptr).pwpr.write(|w| w.pfswe().set_bit());
    //
    //     (*pfs_ptr)
    //         .p115pfs()
    //         .write(|w| w.pmr().set_bit().psel().bits(0b00011).pdr().set_bit());
    //
    //     (*pmisc_ptr).pwpr.write(|w| w.pfswe().clear_bit());
    //     (*pmisc_ptr).pwpr.write(|w| w.b0wi().set_bit());
    // }
    //
    // let p115_pfs_val = unsafe { (*pfs_ptr).p115pfs().read().bits() };
    // defmt::println!("Final p115 pfs setting");
    // defmt::println!("0b{:032b}", p115_pfs_val);
    // Start the timer
    start_timer(channel);
    // Start the counter
    // unsafe {
    //     (*reg_ptr).gtcr.write(|w| w.bits(1));
    // }
}
