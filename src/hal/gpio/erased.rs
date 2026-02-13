//! Erase information from pins so they can be more easily used.

use embedded_hal::digital::{ErrorKind, PinState};

use crate::hal::gpio::{Input, Output, PinMode, PushPull, UniversalPfsReg};

use super::PinExt;
use core::marker::PhantomData;

/// Fully erased pin
/// `MODE` is any pin mode
pub struct AnyPin<MODE> {
    // Bits 0-3 Pin: bits 4-7 port (only 9 ports total)
    pin_port: u8,
    _mode: PhantomData<MODE>,
    pfsreg: &'static UniversalPfsReg,
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
    fn pmnpfs_reg(&self) -> &'static UniversalPfsReg {
        self.pfsreg
    }
}

impl<MODE> defmt::Format for AnyPin<MODE> {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(
            fmt,
            "P({}{})<{}>",
            self.port_id(),
            self.pin_id(),
            crate::stripped_type_name::<MODE>()
        )
    }
}

// TODO: Write restore function to go from AnyPin -> Concrete Pin
impl<MODE> AnyPin<MODE> {
    pub fn from_pin_port(pin_port: u8, pfsreg: &'static UniversalPfsReg) -> Self {
        Self {
            pin_port,
            _mode: PhantomData,
            pfsreg,
        }
    }
    pub fn into_pin_port(self) -> u8 {
        self.pin_port
    }
    pub fn new(port: u8, pin: u8, pfsreg: &'static UniversalPfsReg) -> Self {
        Self {
            pin_port: port << 4 | pin,
            _mode: PhantomData,
            pfsreg,
        }
    }

    /// Convert the fully erased pin into a fully erased dynamic pin.
    /// This can be helpful for drivers who need access to an array of erased
    /// pins that can have the input/output state changed at runtime while
    /// still staying in the same array. Default mode is floating input
    pub fn into_dynamic(self) -> DynamicPinErased {
        let port = self.port_id();
        let pin = self.pin_id();
        let pfsreg = self.pfsreg;
        DynamicPinErased::new(port, pin, Dynamic::InputFloating, pfsreg)
    }
}

impl<MODE: PinMode> AnyPin<MODE> {
    /// This function mutates the pin in place
    /// It is the caller's responsibility to not screw up your system.
    /// You probably want to use the `into_mode()` method instead.
    #[inline(always)]
    pub fn mode<M: PinMode>(&mut self) {
        if MODE::OUTTYPE != M::OUTTYPE
            && let Some(outputtype) = M::OUTTYPE
        {
            self.pfsreg.modify(|_, w| w.ncodr().bit(outputtype.into()));
        }

        if MODE::PMODE != M::PMODE
            && let Some(mode) = M::PMODE
        {
            self.pfsreg.modify(|_, w| w.pdr().bit(mode.into()));
        }
    }

    #[inline(always)]
    pub fn into_mode<M: PinMode>(mut self) -> AnyPin<M> {
        self.mode::<M>();
        let pinport = &self.pin_port;
        AnyPin::from_pin_port(*pinport, self.pfsreg)
    }
}
impl<MODE> AnyPin<MODE> {
    // TODO: need to split between output and input MODE trait
    #[inline(always)]
    pub fn set_high(&mut self) {
        // Write to the register, need some way to get register
        self.pfsreg.modify(|_, w| w.podr().set_bit());
    }
    #[inline(always)]
    pub fn set_low(&mut self) {
        // Write to the register, need some way to get register
        self.pfsreg.modify(|_, w| w.podr().clear_bit());
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

/// Tracks the current mode for dynamic pins
#[derive(Debug, PartialEq, Eq, defmt::Format)]
pub enum Dynamic {
    InputFloating,
    InputPullUp,
    OutputPushPull,
    OutputOpenDrain,
}

/// Error type for DynamicPin
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinModeError {
    IncorrectMode,
}
impl embedded_hal::digital::Error for PinModeError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Other
    }
}

/// Fully Erased Dyanmic pin
/// used for things like charlieplexed LEDs or multiplexed communication where you
/// need a collection of pins that all may have different states.
/// These are way less safe to use than the other type state pins, so be careful.
/// General idea is to ensure the pin is in the proper form and has proper ownership
/// using the type state pins and then let the driver handle the erasure.
pub struct DynamicPinErased {
    pin_port: u8,
    mode: Dynamic,
    pfsreg: &'static UniversalPfsReg,
}

impl defmt::Format for DynamicPinErased {
    // TODO: Add stripped_type_name like from stm32f4xx hal
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(
            fmt,
            "P{}{:02}<{}>",
            self.port_id(),
            self.pin_id(),
            self.mode
        )
    }
}
impl Dynamic {
    pub fn is_input(&self) -> bool {
        match self {
            Dynamic::InputFloating | Dynamic::InputPullUp | Dynamic::OutputOpenDrain => true,
            Dynamic::OutputPushPull => false,
        }
    }

    pub fn is_output(&self) -> bool {
        match self {
            Dynamic::InputFloating | Dynamic::InputPullUp => false,
            Dynamic::OutputPushPull | Dynamic::OutputOpenDrain => true,
        }
    }
}

// If we go to convert back for some reason
pub struct Unknown;
impl PinMode for Unknown {}

impl PinExt for DynamicPinErased {
    type Mode = Unknown;
    #[inline(always)]
    fn pin_id(&self) -> u8 {
        self.pin_port & 0x0f
    }
    #[inline(always)]
    fn port_id(&self) -> u8 {
        self.pin_port >> 4
    }

    #[inline(always)]
    fn pmnpfs_reg(&self) -> &'static UniversalPfsReg {
        self.pfsreg
    }
}
impl DynamicPinErased {
    pub fn new(port: u8, pin: u8, mode: Dynamic, pfsreg: &'static UniversalPfsReg) -> Self {
        Self {
            pin_port: port << 4 | pin,
            mode,
            pfsreg,
        }
    }
    pub fn into_pin_port(self) -> u8 {
        self.pin_port
    }
    /// Switch to floating input
    #[inline]
    pub fn make_floating_input(&mut self) {
        // Note (unsafe) mutable reference to current pin
        let port_id: u8 = self.port_id();
        let pin_id: u8 = self.pin_id();
        AnyPin::<Unknown>::new(port_id, pin_id, self.pfsreg).into_mode::<Input>();
        self.mode = Dynamic::InputFloating;
    }
    /// Switch to output
    #[inline]
    pub fn make_push_pull_output(&mut self) {
        let port_id: u8 = self.port_id();
        let pin_id: u8 = self.pin_id();
        AnyPin::<Unknown>::new(port_id, pin_id, self.pfsreg).into_mode::<Output<PushPull>>();
        self.mode = Dynamic::OutputPushPull;
    }

    /// Drive the pin high if it's in the correct state
    pub fn set_high(&mut self) -> Result<(), PinModeError> {
        if self.mode.is_output() {
            let port_id: u8 = self.port_id();
            let pin_id: u8 = self.pin_id();
            AnyPin::<Unknown>::new(port_id, pin_id, self.pfsreg)
                // .into_mode::<Output>()
                .set_high();
            Ok(())
        } else {
            Err(PinModeError::IncorrectMode)
        }
    }
    /// Drive the pin low if it's in the correct state
    pub fn set_low(&mut self) -> Result<(), PinModeError> {
        if self.mode.is_output() {
            let port_id: u8 = self.port_id();
            let pin_id: u8 = self.pin_id();
            AnyPin::<Unknown>::new(port_id, pin_id, self.pfsreg)
                // .into_mode::<Output>()
                .set_low();
            Ok(())
        } else {
            Err(PinModeError::IncorrectMode)
        }
    }
}
