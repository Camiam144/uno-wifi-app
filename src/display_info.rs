pub fn show_info(perph: &ra4m1::Peripherals) {
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
    // Print out stop registers to make sure everything is okay
    defmt::println!("MSTPCRA");
    let mstpcra_val = perph.SYSTEM.mstpcra.read().bits();
    defmt::println!("0b{:032b}", mstpcra_val);

    let mstpcrb_val = perph.MSTP.mstpcrb.read().bits();
    defmt::println!("MSTPCRB");
    defmt::println!("0b{:032b}", mstpcrb_val);

    let mstpcrc_val = perph.MSTP.mstpcrc.read().bits();
    defmt::println!("MSTPCRC");
    defmt::println!("0b{:032b}", mstpcrc_val);

    let mstpcrd_val = perph.MSTP.mstpcrd.read().bits();
    defmt::println!("MSTPCRD");
    defmt::println!("0b{:032b}", mstpcrd_val);

    // Clock speeds
    let sckdivcr_val = perph.SYSTEM.sckdivcr.read().bits();
    defmt::println!("SCKDIVCR clock reg");
    defmt::println!("0b{:032b}", sckdivcr_val);

    // Used ICU interrupts (in general 0-7 are used by the bootloader)
    let base_reg: u32 = 0x40006300;
    let offset: u32 = 0x04;
    for i in 0..=31 {
        let ielsrn_val = unsafe {
            let ptr: *const u32 = (base_reg + offset * i) as *const u32;
            core::ptr::read(ptr)
        };
        defmt::println!("IELSR{0} register", i);
        defmt::println!("0b{:032b}", ielsrn_val);
        defmt::println!("event code 0x{:02x}", ielsrn_val & 0xFF);
    }
}
