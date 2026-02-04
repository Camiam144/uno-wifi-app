// To start the IIC initialization flow, the manual says as follows:
// set ICE in ICCR1 to 0 to turn off the SCL and SDA pins
// set IICRST in ICCR1 to 1 to reset the IIC
// set ICE in ICCR1 to 1 to activate the SCLn and SDAn pins
// set SARLy and SARUy and ICSER to set the address format and slave address
// set CKS[2:0] in ICMR1 and ICBRL/ICBRH to set the transfer bit rate
// set ICMR2 and ICMR3 and ICFER if required
// set ICIER to enable interrupts
// set IICRST in ICCR1 to 0 to disable the reset state
// now the module is good to use.
//
// For the qwiic bus, pins 400 and 401 need to be set to 00111 in the PSEL

/// Unlock the qwiic bus
///
/// # Safety
///
/// Don't do this when something else is writing to the mstpb8 reg
pub unsafe fn enable_qwiic_bus() {
    let p = unsafe { ra4m1::Peripherals::steal() };
    p.MSTP.mstpcrb.modify(|_, w| w.mstpb9().clear_bit());
}
