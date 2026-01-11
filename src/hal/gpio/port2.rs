use crate::hal::gpio::*;

// pub struct Port2;
//
// pub struct P200<MODE> {
//     _mode: PhantomData<MODE>,
// } // Input only
// pub struct P201<MODE> {
//     _mode: PhantomData<MODE>,
// }
// pub struct P204<MODE> {
//     _mode: PhantomData<MODE>,
// }
// pub struct P205<MODE> {
//     _mode: PhantomData<MODE>,
// }
// pub struct P206<MODE> {
//     _mode: PhantomData<MODE>,
// }
// pub struct P212<MODE> {
//     _mode: PhantomData<MODE>,
// }
// pub struct P213<MODE> {
//     _mode: PhantomData<MODE>,
// }
// pub struct P214<MODE> {
//     _mode: PhantomData<MODE>,
// } // Input only
// pub struct P215<MODE> {
//     _mode: PhantomData<MODE>,
// } // Input only
pub type P200<Input> = Pin<2, 0, Input>;
// pub type P201<Input> = Pin<2, 1, Input>;
pub type P204<Input> = Pin<2, 4, Input>;
pub type P205<Input> = Pin<2, 5, Input>;
pub type P206<Input> = Pin<2, 6, Input>;
pub type P212<Input> = Pin<2, 12, Input>;
pub type P213<Input> = Pin<2, 13, Input>;
pub type P214<Input> = Pin<2, 14, Input>;
pub type P215<Input> = Pin<2, 15, Input>;

pub struct Port2Pins {
    pub p200: P200<Input>,
    // pub p201: P201<Input>,
    pub p204: P204<Input>,
    pub p205: P205<Input>,
    pub p206: P206<Input>,
    pub p212: P212<Input>,
    pub p213: P213<Input>,
    pub p214: P214<Input>,
    pub p215: P215<Input>,
}

impl GpioExt for crate::pac::PORT2 {
    type Parts = Port2Pins;
    fn split(self) -> Port2Pins {
        Port2Pins {
            p200: P200::new(unsafe { (*PMNPFS_BLOCK_BASE).p200pfs() }),
            // p201: P201::new(unsafe { (*PMNPFS_BLOCK_BASE).p201pfs() }),
            p204: P204::new(unsafe { (*PMNPFS_BLOCK_BASE).p204pfs() }),
            p205: P205::new(unsafe { (*PMNPFS_BLOCK_BASE).p205pfs() }),
            p206: P206::new(unsafe { (*PMNPFS_BLOCK_BASE).p206pfs() }),
            p212: P212::new(unsafe { (*PMNPFS_BLOCK_BASE).p212pfs() }),
            p213: P213::new(unsafe { (*PMNPFS_BLOCK_BASE).p213pfs() }),
            p214: P214::new(unsafe { (*PMNPFS_BLOCK_BASE).p214pfs() }),
            p215: P215::new(unsafe { (*PMNPFS_BLOCK_BASE).p215pfs() }),
            // p200: P200 { _mode: PhantomData },
            // p201: P201 { _mode: PhantomData },
            // p204: P204 { _mode: PhantomData },
            // p205: P205 { _mode: PhantomData },
            // p206: P206 { _mode: PhantomData },
            // p212: P212 { _mode: PhantomData },
            // p213: P213 { _mode: PhantomData },
            // p214: P214 { _mode: PhantomData },
            // p215: P215 { _mode: PhantomData },
        }
    }
}

// gpio_pin!(P201, p201pfs);
// gpio_pin!(P204, p204pfs);
// gpio_pin!(P205, p205pfs);
// gpio_pin!(P206, p206pfs);
// gpio_pin!(P212, p212pfs);
// gpio_pin!(P213, p213pfs);
//
// // custom macro for 200, 214, and 215
// macro_rules! gpio_pin_no_output {
//     ($Pin:ident, $pfs:ident) => {
//         impl $Pin<Input> {
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
//         }
//         impl<MODE> ErrorType for $Pin<MODE> {
//             type Error = Infallible;
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
//     };
// }
//
// gpio_pin_no_output!(P200, p200pfs);
// gpio_pin_no_output!(P214, p214pfs);
// gpio_pin_no_output!(P215, p215pfs);
