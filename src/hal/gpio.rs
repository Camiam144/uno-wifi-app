/// Trying to do this in a way that makes sense with the embedded_hal types.
/// This is so hard omg I have no idea what I'm doing.
use core::{convert::Infallible, marker::PhantomData};

use embedded_hal::digital::{ErrorType, InputPin, PinState};

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
    fn pmnpfs_reg(&self) -> &'static ra4m1::generic::Reg<ra4m1::pfs::p000pfs::P000PFS_SPEC>;
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
    fn internal_resistor(mut self, _resistor: Pull) -> Self {
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
const PMNPFS_BLOCK_BASE: *const crate::pac::pfs::RegisterBlock = crate::pac::PFS::PTR;
// const PMNPFS_REG_PORT_OFFSET: usize = 0x0040;
// const PMNPFS_REG_PIN_OFFSET: usize = 0x0004;

// Some pins are input only, these have to be either treated differently or
// have a different struct or something
// P200, P214, P215, are specifically called out as input only.
// P408 has a different DSCR1 reg, too
// P914/915 I think are used for the USBFS

/// Generic Pin type
///
/// MODE is a pin mode
/// - `P` is a port number as a u8
/// - `N` is a pin number as a u8 from `0` to `15`
///
/// On this chip, pins are Input by default after a reset
/// This current method means I cannot use pins 108, 109, 110, 201, 300, 408, or 914
/// as they implement different register specs
/// TODO: how do I do this nicely without needing to create a ton of different pins?
/// this PAC has 4 extra types: p108pfs, p109pfs, p201pfs, and p408pfs to hold the
/// different reset values. Probably have to make special pins for these. You can
/// dereference the const ptr to the right pmnpfs but the spec type is different
pub struct Pin<const P: u8, const N: u8, MODE = Input> {
    _mode: PhantomData<MODE>,
    pfsreg: &'static ra4m1::generic::Reg<ra4m1::pfs::p000pfs::P000PFS_SPEC>,
}
impl<const P: u8, const N: u8, MODE> Pin<P, N, MODE> {
    pub fn new(pfsreg: &'static ra4m1::generic::Reg<ra4m1::pfs::p000pfs::P000PFS_SPEC>) -> Self {
        Self {
            _mode: PhantomData,
            pfsreg,
        }
    }
    // #[inline(always)]
    // fn pfs(&self) -> &crate::pac::pfs::RegisterBlock {
    //     unsafe { &*pfs_ptr::<P, N>() }
    // }
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
    fn pmnpfs_reg(&self) -> &'static ra4m1::generic::Reg<ra4m1::pfs::p000pfs::P000PFS_SPEC> {
        self.pfsreg
    }
}

impl<const P: u8, const N: u8, MODE> Pin<P, N, MODE> {
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
        self.pfsreg.modify(|_, w| w.podr().set_bit());
    }
    #[inline(always)]
    fn _set_low(&mut self) {
        self.pfsreg.modify(|_, w| w.podr().clear_bit());
    }
    #[inline(always)]
    fn _is_set_high(&self) -> bool {
        self.pfsreg.read().pdr().bit_is_set()
    }
    #[inline(always)]
    fn _is_set_low(&self) -> bool {
        self.pfsreg.read().pdr().bit_is_clear()
    }
    #[inline(always)]
    fn _is_high(&self) -> bool {
        self.pfsreg.read().pidr().bit_is_set()
    }
    #[inline(always)]
    fn _is_low(&self) -> bool {
        self.pfsreg.read().pidr().bit_is_clear()
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
        self.pfsreg.modify(|_, w| w.pcr().bit(resistor.into()));
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
    /// Puts `self` into the provided mode `M` in place.
    ///
    /// In order to do this we have to violate type safety, so callers
    /// must not cause havoc when calling this.
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
    /// Consume the pin and get a new one with the specified mode.
    #[inline(always)]
    pub fn into_mode<M: PinMode>(mut self) -> Pin<P, N, M> {
        self.mode::<M>();
        Pin::new(self.pfsreg)
    }

    /// Into a fully erased dynamic pin. Dynamic pin starts as floating input
    #[inline]
    pub fn into_fully_erased_dynamic(self) -> DynamicPinErased {
        DynamicPinErased::new(P, N, erased::Dynamic::InputFloating, self.pfsreg)
    }
}
impl<const P: u8, const N: u8, MODE> Pin<P, N, MODE> {
    /// Erases the pin number and port from the type.
    /// Useful when you need an array of pins with the same type
    pub fn erase(self) -> AnyPin<MODE> {
        AnyPin::new(P, N, self.pfsreg)
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
// TODO: Some pins come in an array of pmnpfs like port 1. idk what to do, different macro?
// Maybe I'll just implement those manually.
#[macro_export]
macro_rules! gpio_port {
    ($PortN: ident, $PORTUPPER: ident, $portlower: ident, $port_num: expr, [$(($Pinupper: ident, $Pinlower:ident, $pin_num: expr,  $pfs:ident),)+]) => {
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
        $Pinlower: $Pinupper::new(unsafe { (*PMNPFS_BLOCK_BASE).$pfs()}),
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

// Here goes nuthin...
// This is going to work pin-by-pin because I have a few special case pins I'll
// need to deal with (a few pins can't be Output pins, they can only be input pins)
// #[macro_export]
// macro_rules! gpio_pin {
//     ($Pin: ident, $pfs: ident, $port: ident, $pnum: ident, $MODE: ty) => {
//         Pin::<$port, $pin, $MODE>new($pfs);
//     };
// }
// #[macro_export]
// macro_rules! gpio_pin {
//     ($Pin:ident, $pfs:ident) => {
//         impl $Pin<Input> {
//             pub fn into_output(self) -> $Pin<Output> {
//                 let ptr = unsafe { &*$crate::pac::PFS::PTR };
//                 ptr.$pfs().modify(|_, w| w.pdr().set_bit());
//                 $Pin { _mode: PhantomData }
//             }
//             pub fn into_input_pullup(self, resistor: Pull) -> $Pin<InputPullUp> {
//                 let ptr = unsafe { &*$crate::pac::PFS::PTR };
//                 ptr.$pfs().modify(|_, w| {
//                     w.pmr()
//                         .clear_bit()
//                         .pdr()
//                         .clear_bit()
//                         .pcr()
//                         .bit(resistor.into())
//                 });
//                 $Pin { _mode: PhantomData }
//             }
//             pub fn into_output_drain(self, drain: Drain) -> $Pin<OutputOpenDrain> {
//                 let ptr = unsafe { &*$crate::pac::PFS::PTR };
//                 ptr.$pfs().modify(|_, w| {
//                     w.pmr()
//                         .clear_bit()
//                         .pdr()
//                         .clear_bit()
//                         .ncodr()
//                         .bit(drain.into())
//                 });
//                 $Pin { _mode: PhantomData }
//             }
//         }
//         impl $Pin<Input> {
//             pub fn is_high(&self) -> bool {
//                 let ptr = unsafe { &*$crate::pac::PFS::PTR };
//                 ptr.$pfs().read().pidr().bit_is_set()
//             }
//             pub fn is_low(&self) -> bool {
//                 let ptr = unsafe { &*$crate::pac::PFS::PTR };
//                 ptr.$pfs().read().pidr().bit_is_clear()
//             }
//         }
//         impl<MODE> ErrorType for $Pin<MODE> {
//             type Error = Infallible;
//         }
//         impl embedded_hal::digital::InputPin for $Pin<Input> {
//             #[inline(always)]
//             fn is_high(&mut self) -> Result<bool, Self::Error> {
//                 Ok(Self::is_high(self))
//             }
//             #[inline(always)]
//             fn is_low(&mut self) -> Result<bool, Self::Error> {
//                 Ok(Self::is_low(self))
//             }
//         }
//         impl $Pin<InputPullUp> {
//             pub fn set_internal_resistor(&mut self, resistor: Pull) {
//                 let ptr = unsafe { &*$crate::pac::PFS::PTR };
//                 ptr.$pfs().modify(|_, w| w.pcr().bit(resistor.into()));
//             }
//             pub fn is_high(&self) -> bool {
//                 let ptr = unsafe { &*$crate::pac::PFS::PTR };
//                 ptr.$pfs().read().pidr().bit_is_set()
//             }
//             pub fn is_low(&self) -> bool {
//                 let ptr = unsafe { &*$crate::pac::PFS::PTR };
//                 ptr.$pfs().read().pidr().bit_is_clear()
//             }
//         }
//         impl embedded_hal::digital::InputPin for $Pin<InputPullUp> {
//             #[inline(always)]
//             fn is_high(&mut self) -> Result<bool, Self::Error> {
//                 Ok(Self::is_high(self))
//             }
//             #[inline(always)]
//             fn is_low(&mut self) -> Result<bool, Self::Error> {
//                 Ok(Self::is_low(self))
//             }
//         }
//         impl $Pin<Output> {
//             pub fn into_input(self) -> $Pin<Input> {
//                 let ptr = unsafe { &*$crate::pac::PFS::PTR };
//                 ptr.$pfs()
//                     .modify(|_, w| w.pmr().clear_bit().pdr().clear_bit());
//                 $Pin { _mode: PhantomData }
//             }
//             pub fn into_input_pullup(self, resistor: Pull) -> $Pin<InputPullUp> {
//                 let ptr = unsafe { &*$crate::pac::PFS::PTR };
//                 ptr.$pfs().modify(|_, w| {
//                     w.pmr()
//                         .clear_bit()
//                         .pdr()
//                         .clear_bit()
//                         .pcr()
//                         .bit(resistor.into())
//                 });
//                 $Pin { _mode: PhantomData }
//             }
//             pub fn into_output_drain(self, drain: Drain) -> $Pin<OutputOpenDrain> {
//                 let ptr = unsafe { &*$crate::pac::PFS::PTR };
//                 ptr.$pfs().modify(|_, w| {
//                     w.pmr()
//                         .clear_bit()
//                         .pdr()
//                         .clear_bit()
//                         .ncodr()
//                         .bit(drain.into())
//                 });
//                 $Pin { _mode: PhantomData }
//             }
//         }
//         impl $Pin<Output> {
//             pub fn set_high(&mut self) {
//                 let ptr = unsafe { &*$crate::pac::PFS::PTR };
//                 ptr.$pfs().modify(|_, w| w.podr().set_bit());
//             }
//             pub fn set_low(&mut self) {
//                 let ptr = unsafe { &*$crate::pac::PFS::PTR };
//                 ptr.$pfs().modify(|_, w| w.podr().clear_bit());
//             }
//         }
//         impl embedded_hal::digital::OutputPin for $Pin<Output> {
//             #[inline(always)]
//             fn set_high(&mut self) -> Result<(), Self::Error> {
//                 self.set_high();
//                 Ok(())
//             }
//             #[inline(always)]
//             fn set_low(&mut self) -> Result<(), Self::Error> {
//                 self.set_low();
//                 Ok(())
//             }
//             #[inline(always)]
//             fn set_state(&mut self, state: PinState) -> Result<(), Self::Error> {
//                 match state {
//                     PinState::Low => {
//                         self.set_low();
//                         Ok(())
//                     }
//                     PinState::High => {
//                         self.set_high();
//                         Ok(())
//                     }
//                 }
//             }
//         }
//         impl $Pin<OutputOpenDrain> {
//             pub fn set_drain(&mut self, drain: Drain) {
//                 let ptr = unsafe { &*$crate::pac::PFS::PTR };
//                 ptr.$pfs().modify(|_, w| {
//                     w.pmr()
//                         .clear_bit()
//                         .pdr()
//                         .clear_bit()
//                         .ncodr()
//                         .bit(drain.into())
//                 });
//             }
//             pub fn set_high(&mut self) {
//                 let ptr = unsafe { &*$crate::pac::PFS::PTR };
//                 ptr.$pfs().modify(|_, w| w.podr().set_bit());
//             }
//             pub fn set_low(&mut self) {
//                 let ptr = unsafe { &*$crate::pac::PFS::PTR };
//                 ptr.$pfs().modify(|_, w| w.podr().clear_bit());
//             }
//         }
//         impl embedded_hal::digital::OutputPin for $Pin<OutputOpenDrain> {
//             #[inline(always)]
//             fn set_high(&mut self) -> Result<(), Self::Error> {
//                 self.set_high();
//                 Ok(())
//             }
//             #[inline(always)]
//             fn set_low(&mut self) -> Result<(), Self::Error> {
//                 self.set_low();
//                 Ok(())
//             }
//             #[inline(always)]
//             fn set_state(&mut self, state: PinState) -> Result<(), Self::Error> {
//                 match state {
//                     PinState::Low => {
//                         self.set_low();
//                         Ok(())
//                     }
//                     PinState::High => {
//                         self.set_high();
//                         Ok(())
//                     }
//                 }
//             }
//         }
//     };
// }

// pub enum PfsRegBlock<'a> {
//     PfsGeneric(&'a ra4m1::pfs::P000PFS),
//     Pfs108(&'a ra4m1::pfs::P108PFS),
//     Pfs109(&'a ra4m1::pfs::P109PFS),
//     Pfs201(&'a ra4m1::pfs::P201PFS),
//     Pfs408(&'a ra4m1::pfs::P408PFS),
// }
//
// /// This function returns the appropriate pmnpfs register block
// fn pmnpfsfn<const P: u8, const N: u8>() -> PfsRegBlock<'static> {
//     // This is super painful, this can't be the best way? Should I just do
//     // pointer math to get the block?
//     let ptr = unsafe { *ra4m1::PFS::PTR };
//     match (P, N) {
//         (0, 0) => PfsRegBlock::PfsGeneric(ptr.p000pfs()),
//         (0, 1) => PfsRegBlock::PfsGeneric(ptr.p001pfs()),
//         (1, 8) => PfsRegBlock::Pfs108(ptr.p108pfs()),
//         (_, _) => PfsRegBlock::PfsGeneric(ptr.p002pfs()),
//     }
// }

// These can all be in separate .rs files
pub struct Port1;
pub struct Port3;
pub struct Port4;
pub struct Port5;
pub struct Port9;

// match these up to the appropriate .rs files as we split up the HAL
// Due to the specific nature of this board idk if there's a better way to do it.
pub struct P100;
pub struct P101;
pub struct P102;
pub struct P103;
pub struct P104;
pub struct P105;
pub struct P106;
pub struct P107;
pub struct P108;
pub struct P109;
pub struct P110;
pub struct P111;
pub struct P112;
pub struct P113;

pub struct Port1Pins {
    pub p100: P100,
    pub p101: P101,
    pub p102: P102,
    pub p103: P103,
    pub p104: P104,
    pub p105: P105,
    pub p106: P106,
    pub p107: P107,
    pub p108: P108,
    pub p109: P109,
    pub p110: P110,
    pub p111: P111,
    pub p112: P112,
    pub p113: P113,
}

pub struct P300;
pub struct P301;
pub struct P302;
pub struct P303;
pub struct P304;

pub struct Port3Pins {
    pub p300: P300,
    pub p301: P301,
    pub p302: P302,
    pub p303: P303,
    pub p304: P304,
}

pub struct P400;
pub struct P401;
pub struct P402;
pub struct P407;
pub struct P408;
pub struct P409;
pub struct P410;
pub struct P411;

pub struct Port4Pins {
    pub p400: P400,
    pub p401: P401,
    pub p402: P402,
    pub p407: P407,
    pub p408: P408,
    pub p409: P409,
    pub p410: P410,
    pub p411: P411,
}

pub struct P500;
pub struct P501;
pub struct P502;

pub struct Port5Pins {
    pub p500: P500,
    pub p501: P501,
    pub p502: P502,
}

// Pins Cannot be output
pub struct P914;
pub struct P915;

pub struct Port9Pins {
    pub p914: P914,
    pub p915: P915,
}
