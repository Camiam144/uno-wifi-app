/// Trying to do this in a way that makes sense with the embedded_hal types.
/// This is so hard omg I have no idea what I'm doing.
use core::marker::PhantomData;

use embedded_hal::digital::PinState;

pub mod erased;
pub use erased::AnyPin;

use crate::hal::gpio::erased::DynamicPinErased;
mod ehal_1;
pub mod unor4;

// Some stuff that comes out of PACs for other boards
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Modes {
    Input = 0,
    Output = 1,
    // TODO: Encode these into the proper variants for my board
    // They will need to live somewhere else, these are a different setup
    // Alternate = 2,
    // Analog = 3,
}
impl From<Modes> for bool {
    #[inline(always)]
    fn from(value: Modes) -> Self {
        match value {
            Modes::Input => false,
            Modes::Output => true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputType {
    PushPull = 0,
    OpenDrain = 1,
}
impl From<OutputType> for bool {
    #[inline(always)]
    fn from(value: OutputType) -> Self {
        match value {
            OutputType::PushPull => false,
            OutputType::OpenDrain => true,
        }
    }
}

pub trait GpioExt {
    type Parts;
    fn split(self) -> Self::Parts;
}

/// Id, port, and MODE for AnyPin so we can recover the typestate pin later
/// TODO: Implement MODE to string
pub trait PinExt {
    type Mode;
    fn pin_id(&self) -> u8;
    fn port_id(&self) -> u8;
    fn pmnpfs_reg(&self) -> &'static UniversalPfsReg;
}

/// Unsafe function to unlock the pin register.
///
/// # Safety
///
/// Not sure, preferably use this when nothing is actively using the register.
pub unsafe fn unlock_pmnpfs_register() {
    let ptr = ra4m1::PMISC::PTR;
    unsafe {
        (*ptr).pwpr.modify(|_, w| w.b0wi().clear_bit());
        (*ptr).pwpr.modify(|_, w| w.pfswe().set_bit());
    }
}

/// Unsafe function to lock the pin register.
///
/// # Safety
///
/// Not sure, don't lock the register while trying to set pin peripherals or states
pub unsafe fn lock_pmnpfs_register() {
    let ptr = ra4m1::PMISC::PTR;
    unsafe {
        (*ptr).pwpr.modify(|_, w| w.pfswe().clear_bit());
        (*ptr).pwpr.modify(|_, w| w.b0wi().set_bit());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Drain {
    /// CMOS
    None = 0,
    /// NMOS
    Open = 1,
}
impl From<Drain> for bool {
    fn from(value: Drain) -> Self {
        match value {
            Drain::None => false,
            Drain::Open => true,
        }
    }
}

// We are not going to worry about alternate modes (like analog or Peripherals)
// for now. That will come later.
// TODO: Add alternate modes like analog and pmnpfs peripheral

/// Generic input mode typestate
#[derive(Debug, Default, defmt::Format)]
pub struct Input;

/// Pullup resistor settings (no pulldown resistors I can find for RA4M1)
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum Pull {
    /// Floating
    None = 0,
    /// Pullup
    Up = 1,
}
impl From<Pull> for bool {
    fn from(value: Pull) -> Self {
        match value {
            Pull::None => false,
            Pull::Up => true,
        }
    }
}

pub trait PinPull: Sized {
    fn set_internal_resistor(&mut self, resistor: Pull);

    #[inline(always)]
    fn internal_resistor(mut self, resistor: Pull) -> Self {
        self.set_internal_resistor(resistor);
        self
    }
}

impl PinPull for Input {
    fn set_internal_resistor(&mut self, _resistor: Pull) {}
    fn internal_resistor(self, _resistor: Pull) -> Self {
        self
    }
}

/// Generic Output type (default to output PushPull)
#[derive(Debug, Default, defmt::Format)]
pub struct Output<Otype = PushPull> {
    _mode: PhantomData<Otype>,
}

/// Push pull output typestate
#[derive(Debug, Default, defmt::Format)]
pub struct PushPull;

#[derive(Debug, Default, defmt::Format)]
pub struct OutputOpenDrain;

// Kinda stealing this from the STM32f4 hal crate, not sure if it's best practice
// It looks like this is just a way to have some generics for the implementation
// of the `into` code

pub trait PinMode {
    const PMODE: Option<Modes> = None;
    const OUTTYPE: Option<OutputType> = None;
    const AFR: Option<u8> = None;
}
impl PinMode for Input {
    const PMODE: Option<Modes> = Some(Modes::Input);
}
impl PinMode for Output<OutputOpenDrain> {
    const PMODE: Option<Modes> = Some(Modes::Output);
    const OUTTYPE: Option<OutputType> = Some(OutputType::OpenDrain);
}
impl PinMode for Output<PushPull> {
    const PMODE: Option<Modes> = Some(Modes::Output);
    const OUTTYPE: Option<OutputType> = Some(OutputType::PushPull);
}
// const PMNPFS_BLOCK_BASE: *const crate::pac::pfs::RegisterBlock = crate::pac::PFS::PTR;
// const PMNPFS_REG_PORT_OFFSET: usize = 0x0040;
// const PMNPFS_REG_PIN_OFFSET: usize = 0x0004;

// Some pins are input only, these have to be either treated differently or
// have a different struct or something
// P200, P214, P215, are specifically called out as input only.
// P408 has a different DSCR1 reg, too
// P914/915 I think are used for the USBFS

// Generic type alias
type UniversalPfsSpec = ra4m1::pfs::p408pfs::P408PFS_SPEC;
type UniversalPfsReg = ra4m1::generic::Reg<UniversalPfsSpec>;

/// Generic Pin type
///
/// MODE is a pin mode
/// - `P` is a port number as a u8
/// - `N` is a pin number as a u8 from `0` to `15`
///
/// On this chip, pins are Input by default after a reset
/// This current method means I cannot use pins 108, 109, 110, 201, 300, 408, or 914
/// as they implement different register specs
/// We're going to try a yolo that Gemini suggested with some prompting
/// Going to alias some types, *technically* we could hit an issue where we
/// use a field on a pin that isn't implemented, but, I mean, take your life into
/// your own hands? Read the datasheet? As far as I can tell, p408pfs implements
/// the most trait/fields/whatever.
/// READ THE DATASHEET
pub struct Pin<const P: u8, const N: u8, MODE = Input> {
    _mode: PhantomData<MODE>,
}

impl<const P: u8, const N: u8, MODE> defmt::Format for Pin<P, N, MODE> {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(
            fmt,
            "P{}{:02}<{}>",
            P,
            N,
            crate::stripped_type_name::<MODE>()
        );
    }
}
impl<const P: u8, const N: u8, MODE> PinExt for Pin<P, N, MODE> {
    type Mode = MODE;
    #[inline(always)]
    fn pin_id(&self) -> u8 {
        N
    }
    #[inline(always)]
    fn port_id(&self) -> u8 {
        P
    }
    fn pmnpfs_reg(&self) -> &'static UniversalPfsReg {
        Self::pfsreg()
    }
}

#[allow(clippy::new_without_default)]
impl<const P: u8, const N: u8, MODE> Pin<P, N, MODE> {
    pub fn new() -> Self {
        Self { _mode: PhantomData }
    }

    // Little bit of math to get the right pin
    const PFS_BASE_ADDR: usize = 0x4004_0800;
    const PIN_PFS_OFFSET: usize = 0x04;
    const PORT_PFS_OFFSET: usize = 0x40;
    const fn register_addr() -> usize {
        // calc is base + (64 * port) + (4 * pin)
        Self::PFS_BASE_ADDR
            + (Self::PORT_PFS_OFFSET * P as usize)
            + (Self::PIN_PFS_OFFSET * N as usize)
    }

    #[inline(always)]
    pub fn pfsreg() -> &'static UniversalPfsReg {
        let addr = Self::register_addr();
        unsafe { &*(addr as *const UniversalPfsReg) }
    }
    /// Sets the output of the pin regardless of mode
    /// This can help avoid a short spike of the wrong value when changing pin
    /// mode into output.
    #[inline(always)]
    fn _set_state(&mut self, state: PinState) {
        match state {
            PinState::High => self._set_high(),
            PinState::Low => self._set_low(),
        }
    }
    #[inline(always)]
    fn _set_high(&mut self) {
        Self::pfsreg().modify(|_, w| w.podr().set_bit());
    }
    #[inline(always)]
    fn _set_low(&mut self) {
        Self::pfsreg().modify(|_, w| w.podr().clear_bit());
    }
    #[inline(always)]
    fn _is_set_high(&self) -> bool {
        Self::pfsreg().read().pdr().bit_is_set()
    }
    #[inline(always)]
    fn _is_set_low(&self) -> bool {
        Self::pfsreg().read().pdr().bit_is_clear()
    }
    #[inline(always)]
    fn _is_high(&self) -> bool {
        Self::pfsreg().read().pidr().bit_is_set()
    }
    #[inline(always)]
    fn _is_low(&self) -> bool {
        Self::pfsreg().read().pidr().bit_is_clear()
    }
    #[inline(always)]
    pub fn is_low(&self) -> bool {
        self._is_low()
    }
    #[inline(always)]
    pub fn is_high(&self) -> bool {
        !self.is_low()
    }
}

impl<const P: u8, const N: u8, MODE> Pin<P, N, Output<MODE>> {
    /// Implementation of traits for generic output ports
    #[inline(always)]
    pub fn set_high(&mut self) {
        self._set_high()
    }
    #[inline(always)]
    pub fn set_low(&mut self) {
        self._set_low()
    }
    #[inline(always)]
    pub fn get_state(&self) -> PinState {
        if self.is_set_low() {
            PinState::Low
        } else {
            PinState::High
        }
    }
    #[inline(always)]
    pub fn is_set_low(&self) -> bool {
        self._is_set_low()
    }
    #[inline(always)]
    pub fn is_set_high(&self) -> bool {
        !self.is_set_low()
    }
    #[inline(always)]
    pub fn set_state(&mut self, state: PinState) {
        match state {
            PinState::High => self.set_high(),
            PinState::Low => self.set_low(),
        }
    }
    #[inline(always)]
    pub fn toggle(&mut self) {
        if self.is_set_low() {
            self.set_high()
        } else {
            self.set_low()
        }
    }
}

impl<const P: u8, const N: u8, MODE> Pin<P, N, MODE>
where
    MODE: PinPull,
{
    /// Set the internal resistor in-place
    pub fn set_internal_resistor(&mut self, resistor: Pull) {
        Self::pfsreg().modify(|_, w| w.pcr().bit(resistor.into()));
    }
    /// Set the internal resistor and create a new instance of Self
    pub fn internal_resistor(mut self, resistor: Pull) -> Self {
        self.set_internal_resistor(resistor);
        self
    }
}

// All of the `into` code should live here
// TODO: implement the temporary "with" code that takes a closure
impl Input {
    pub fn new<const P: u8, const N: u8, MODE: PinMode>(
        pin: Pin<P, N, MODE>,
        pull: Pull,
    ) -> Pin<P, N, Self> {
        pin.into_mode().internal_resistor(pull)
    }
}

impl<const P: u8, const N: u8, MODE: PinMode> Pin<P, N, MODE> {
    /// Configure the pin as an input pin.
    pub fn into_input(self) -> Pin<P, N, Input> {
        self.into_mode()
    }
    /// Configure as floating input
    pub fn into_floating_input(self) -> Pin<P, N, Input> {
        self.into_mode().internal_resistor(Pull::None)
    }
    /// Configure as input pull up
    pub fn into_pullup_input(self) -> Pin<P, N, Input> {
        self.into_mode().internal_resistor(Pull::Up)
    }
    /// Configure as push pull output, initial state will be low
    pub fn into_push_pull_output(mut self) -> Pin<P, N, Output<PushPull>> {
        self._set_low();
        self.into_mode()
    }
    /// Configure as push pull output with provided initial state
    pub fn into_push_pull_output_in_state(
        mut self,
        state: PinState,
    ) -> Pin<P, N, Output<PushPull>> {
        self._set_state(state);
        self.into_mode()
    }

    pub fn into_open_drain_output(mut self) -> Pin<P, N, Output<OutputOpenDrain>> {
        self._set_low();
        self.into_mode()
    }
    /// Puts `self` into the provided mode `M` in place.
    ///
    /// In order to do this we have to violate type safety, so callers
    /// must not cause havoc when calling this.
    #[inline(always)]
    pub fn mode<M: PinMode>(&mut self) {
        if MODE::OUTTYPE != M::OUTTYPE
            && let Some(outputtype) = M::OUTTYPE
        {
            Self::pfsreg().modify(|_, w| w.ncodr().bit(outputtype.into()));
        }

        if MODE::PMODE != M::PMODE
            && let Some(mode) = M::PMODE
        {
            Self::pfsreg().modify(|_, w| w.pdr().bit(mode.into()));
        }
    }
    /// Consume the pin and get a new one with the specified mode.
    #[inline(always)]
    pub fn into_mode<M: PinMode>(mut self) -> Pin<P, N, M> {
        self.mode::<M>();
        Pin::new()
    }

    /// Into a fully erased dynamic pin. Dynamic pin starts as floating input
    #[inline]
    pub fn into_fully_erased_dynamic(self) -> DynamicPinErased {
        DynamicPinErased::new(P, N, erased::Dynamic::InputFloating, Self::pfsreg())
    }
}
impl<const P: u8, const N: u8, MODE> Pin<P, N, MODE> {
    /// Erases the pin number and port from the type.
    /// Useful when you need an array of pins with the same type
    pub fn erase(self) -> AnyPin<MODE> {
        AnyPin::new(P, N, Self::pfsreg())
    }
}
impl<const P: u8, const N: u8, MODE> From<Pin<P, N, MODE>> for AnyPin<MODE> {
    /// Pin-to-AnyPin conversion
    fn from(value: Pin<P, N, MODE>) -> Self {
        value.erase()
    }
}

// Macro for each port. Right now I just ignore pins I don't have set up or
// are reserved for something
#[macro_export]
macro_rules! gpio_port {
    ($PortN: ident, $PORTUPPER: ident, $portlower: ident, $port_num: expr, [$(($Pinupper: ident, $Pinlower:ident, $pin_num: expr),)+]) => {
        pub mod $portlower {
        use $crate::hal::gpio::*;
            pub struct $PortN;

            pub struct Parts {
        $(
        pub $Pinlower: $Pinupper<Input>,
    )+
        }
        impl GpioExt for $crate::pac::$PORTUPPER {
        type Parts = Parts;
        fn split(self) -> Parts {
        Parts {
        $(
        $Pinlower: $Pinupper::new(),
        )+
        }
        }
        }
        $(
        pub type $Pinupper<Input> = Pin<$port_num, $pin_num, Input>;
        )+
        }
    };
}

// These can all be in separate .rs files
