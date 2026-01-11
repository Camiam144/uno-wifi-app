/// This is all pretty bad and not how the Embedded Hal docs say stuff should work.
/// I shouldn't be writing to raw registers here, there's gotta be some way
/// I pass around an instance of the peripherals or something and move ownership
/// of the approprate ones into the appropriate modules.
// use core::convert::Infallible;
// use core::marker::PhantomData;
use embedded_hal::digital::{InputPin, OutputPin, PinState};
use ra4m1::port0;
use ra4m1::port1;

#[derive(Copy, Clone, PartialEq)]
pub enum PinMode {
    Input,
    Output,
}

// impl PinMode {
//     fn val(&self) -> u16 {
//         match self {
//             Self::Input => 0b0,
//             Self::Output => 0b1,
//         }
//     }
// }

#[derive(Clone, Copy, PartialEq)]
pub enum Port {
    PORT0,
    PORT1,
    PORT2,
    PORT3,
    PORT4,
    PORT5,
    PORT6,
    PORT7,
    PORT8,
    PORT9,
}

pub enum RegPtr {
    Port0Ptr(*const port0::RegisterBlock),
    Port1Ptr(*const port1::RegisterBlock),
}

#[derive(Clone)]
/// Represents a single GPIO pin. Allows for setting and changing state
/// TODO: I might need something to deal with locked pins (like the InternalLED)
/// but I can do that later
pub struct Pin {
    /// Port number 0-9
    pub port: Port,
    /// Pin number 0-15
    pub pin: u8,
}

impl Pin {
    fn regs(&self) -> RegPtr {
        match self.port {
            Port::PORT0 => RegPtr::Port0Ptr(ra4m1::PORT0::PTR),
            Port::PORT1 => RegPtr::Port1Ptr(ra4m1::PORT1::PTR),
            Port::PORT2 => RegPtr::Port1Ptr(ra4m1::PORT2::PTR),
            Port::PORT3 => RegPtr::Port1Ptr(ra4m1::PORT3::PTR),
            Port::PORT4 => RegPtr::Port1Ptr(ra4m1::PORT4::PTR),
            Port::PORT5 => RegPtr::Port0Ptr(ra4m1::PORT5::PTR),
            Port::PORT6 => RegPtr::Port0Ptr(ra4m1::PORT6::PTR),
            Port::PORT7 => RegPtr::Port0Ptr(ra4m1::PORT7::PTR),
            Port::PORT8 => RegPtr::Port0Ptr(ra4m1::PORT8::PTR),
            Port::PORT9 => RegPtr::Port0Ptr(ra4m1::PORT9::PTR),
        }
    }
    // fn pin_mask(&self) -> u16 {
    //     1 << self.pin
    // }
    pub fn new(port: Port, pin: u8, mode: PinMode) -> Self {
        assert!(pin <= 15, "Pin must be 0-15");
        let mut this_pin = Self { port, pin };
        this_pin.set_mode(mode);
        this_pin
    }

    /// Sets the pin I/O mode. Sets the PDR bit in the PCNTR1 register.
    /// Will silently fail if the PSEL bits or PMR bit in the PmnPFS register are set.
    /// I should probably have a check for that, or have some sort of return value
    /// if setting this fails.
    pub fn set_mode(&mut self, pin_mode: PinMode) {
        let reg = self.regs();

        // We need this because the different registers are different types
        // in the PAC. This is true for any ra4m1 pacs as these port0/1 registers
        // have different accesible fields I guess?
        match reg {
            RegPtr::Port0Ptr(val) => unsafe {
                let pcntrl1 = (*val).pcntr1();
                match pin_mode {
                    PinMode::Input => {
                        pcntrl1.modify(|r, w| w.pdr().bits(r.pdr().bits() & !(1 << self.pin)));
                    }
                    PinMode::Output => {
                        pcntrl1.modify(|r, w| w.pdr().bits(r.pdr().bits() | 1 << self.pin));
                    }
                }
            },
            RegPtr::Port1Ptr(val) => unsafe {
                let pcntrl1 = (*val).pcntr1();
                match pin_mode {
                    PinMode::Input => {
                        pcntrl1.modify(|r, w| w.pdr().bits(r.pdr().bits() & !(1 << self.pin)));
                    }
                    PinMode::Output => {
                        pcntrl1.modify(|r, w| w.pdr().bits(r.pdr().bits() | 1 << self.pin));
                    }
                }
            },
        }
    }
    /// Handles locking and unlocking the register to write to the PMR bit in
    /// the pfs register. 0 = general i/o pin 1 = peripherial I/O pin.
    pub fn set_pmr_in_pfs(&mut self, state: PinState) {
        let pmisc_ptr = ra4m1::PMISC::PTR;
        let pin_pfs = ra4m1::PFS::PTR;
        let port_offset: u32 = match self.port {
            Port::PORT0 => 0,
            Port::PORT1 => 0x40,
            Port::PORT2 => 2 * 0x40,
            Port::PORT3 => 3 * 0x40,
            Port::PORT4 => 4 * 0x40,
            Port::PORT5 => 5 * 0x40,
            Port::PORT6 => 6 * 0x40,
            Port::PORT7 => 7 * 0x40,
            Port::PORT8 => 8 * 0x40,
            Port::PORT9 => 9 * 0x40,
        };

        unsafe {
            (*pmisc_ptr).pwpr.write(|w| w.b0wi().clear_bit());
            (*pmisc_ptr).pwpr.write(|w| w.pfswe().set_bit());

            // (*pin_pfs)

            (*pmisc_ptr).pwpr.write(|w| w.pfswe().clear_bit());
            (*pmisc_ptr).pwpr.write(|w| w.b0wi().set_bit());
        }
    }
    /// Set the pin state to high or low. See also .set_high() and .set_low()
    /// Uses the PODR bit in the PCNTR1 register.
    pub fn set_state(&mut self, state: PinState) {
        // TODO: This is gross, should probably be a trait implementation somewhere
        match self.regs() {
            RegPtr::Port0Ptr(val) => unsafe {
                let pcntrl1 = (*val).pcntr1();
                match state {
                    PinState::Low => {
                        pcntrl1.modify(|r, w| w.podr().bits(r.podr().bits() & !(1 << self.pin)));
                    }
                    PinState::High => {
                        pcntrl1.modify(|r, w| w.podr().bits(r.podr().bits() | 1 << self.pin));
                    }
                }
            },
            RegPtr::Port1Ptr(val) => unsafe {
                let pcntrl1 = (*val).pcntr1();
                match state {
                    PinState::Low => {
                        pcntrl1.modify(|r, w| w.podr().bits(r.podr().bits() & !(1 << self.pin)));
                    }
                    PinState::High => {
                        pcntrl1.modify(|r, w| w.podr().bits(r.podr().bits() | 1 << self.pin));
                    }
                }
            },
        }
    }
    pub fn set_high(&mut self) {
        self.set_state(PinState::High);
    }
    pub fn set_low(&mut self) {
        self.set_state(PinState::Low);
    }

    pub fn read_mode(&self) -> PinMode {
        let pdr_bits = match self.regs() {
            RegPtr::Port0Ptr(val) => unsafe { (*val).pcntr1().read().pdr().bits() },
            RegPtr::Port1Ptr(val) => unsafe { (*val).pcntr1().read().pdr().bits() },
        };
        if pdr_bits & (1 << self.pin) != 0 {
            PinMode::Output
        } else {
            PinMode::Input
        }
    }

    pub fn read_state(&self) -> PinState {
        let podr_bits = match self.regs() {
            RegPtr::Port0Ptr(val) => unsafe { (*val).pcntr1().read().podr().bits() },
            RegPtr::Port1Ptr(val) => unsafe { (*val).pcntr1().read().podr().bits() },
        };
        if podr_bits & (1 << self.pin) != 0 {
            PinState::High
        } else {
            PinState::Low
        }
    }

    pub fn is_high(&self) -> bool {
        self.read_state() == PinState::High
    }
    pub fn is_low(&self) -> bool {
        !self.is_high()
    }
}
