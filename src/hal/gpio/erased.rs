//! Erase information from pins so they can be more easily used.

use embedded_hal::digital::PinState;

use super::PinExt;
use core::marker::PhantomData;

/// Fully erased pin
/// `MODE` is any pin mode
pub struct AnyPin<MODE> {
    // Bits 0-3 Pin: bits 4-7 port (only 9 ports total)
    pin_port: u8,
    _mode: PhantomData<MODE>,
    pfsreg: &'static ra4m1::generic::Reg<ra4m1::pfs::p000pfs::P000PFS_SPEC>,
}

impl<MODE> PinExt for AnyPin<MODE> {
    type Mode = MODE;

    #[inline(always)]
    fn pin_id(&self) -> u8 {
        self.pin_port & 0x0f
    }
    #[inline(always)]
    fn port_id(&self) -> u8 {
        self.pin_port >> 4
    }

    #[inline(always)]
    fn pmnpfs_reg(&self) -> &'static ra4m1::generic::Reg<ra4m1::pfs::p000pfs::P000PFS_SPEC> {
        self.pfsreg
    }
}

impl<MODE> defmt::Format for AnyPin<MODE> {
    // TODO: Add stripped_type_name like from stm32f4xx hal
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(fmt, "P({}{})", self.port_id(), self.pin_id(),)
    }
}

// TODO: Write restore function to go from AnyPin -> Concrete Pin
impl<MODE> AnyPin<MODE> {
    pub fn from_pin_port(
        pin_port: u8,
        pfsreg: &'static ra4m1::generic::Reg<ra4m1::pfs::p000pfs::P000PFS_SPEC>,
    ) -> Self {
        Self {
            pin_port,
            _mode: PhantomData,
            pfsreg,
        }
    }
    pub fn into_pin_port(self) -> u8 {
        self.pin_port
    }
    pub fn new(
        port: u8,
        pin: u8,
        pfsreg: &'static ra4m1::generic::Reg<ra4m1::pfs::p000pfs::P000PFS_SPEC>,
    ) -> Self {
        Self {
            pin_port: port << 4 | pin,
            _mode: PhantomData,
            pfsreg,
        }
    }
}

impl<MODE> AnyPin<MODE> {
    // TODO: need to split between output and input MODE trait
    #[inline(always)]
    pub fn set_high(&mut self) {
        // Write to the register, need some way to get register
        self.pfsreg.write(|w| w.podr().set_bit());
    }
    #[inline(always)]
    pub fn set_low(&mut self) {
        // Write to the register, need some way to get register
        self.pfsreg.write(|w| w.podr().clear_bit());
    }
    // These bits here feel kinda unsafe but idk what else to do for now
    #[inline(always)]
    pub fn into_input(&mut self) {
        self.set_low();
        self.pfsreg.write(|w| w.pdr().clear_bit());
    }
    #[inline(always)]
    pub fn into_output(&mut self) {
        self.set_low();
        self.pfsreg.write(|w| w.pdr().set_bit());
    }
    #[inline(always)]
    pub fn is_set_low(&self) -> bool {
        self.pfsreg.read().podr().bit_is_clear()
    }
    #[inline(always)]
    pub fn is_set_high(&self) -> bool {
        !self.is_set_low()
    }
    #[inline(always)]
    pub fn get_state(&self) -> PinState {
        if self.is_set_low() {
            PinState::Low
        } else {
            PinState::High
        }
    }
}
