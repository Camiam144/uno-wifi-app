use super::{AnyPin, DynamicPinErased, Output, Pin, erased::PinModeError};
use core::convert::Infallible;
use embedded_hal::digital::{ErrorType, InputPin, OutputPin, StatefulOutputPin};

impl<const P: u8, const N: u8, MODE> ErrorType for Pin<P, N, MODE> {
    type Error = Infallible;
}

impl<const P: u8, const N: u8, MODE> InputPin for Pin<P, N, MODE> {
    #[inline(always)]
    fn is_high(&mut self) -> Result<bool, Self::Error> {
        Ok(Self::is_high(self))
    }
    #[inline(always)]
    fn is_low(&mut self) -> Result<bool, Self::Error> {
        Ok(Self::is_low(self))
    }
}

impl<const P: u8, const N: u8, MODE> OutputPin for Pin<P, N, Output<MODE>> {
    #[inline(always)]
    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.set_low();
        Ok(())
    }
    #[inline(always)]
    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.set_high();
        Ok(())
    }
}
impl<const P: u8, const N: u8, MODE> StatefulOutputPin for Pin<P, N, Output<MODE>> {
    #[inline(always)]
    fn is_set_high(&mut self) -> Result<bool, Self::Error> {
        Ok(Self::is_set_high(self))
    }
    #[inline(always)]
    fn is_set_low(&mut self) -> Result<bool, Self::Error> {
        Ok(Self::is_set_low(self))
    }
}

// Implementations for AnyPin
impl<MODE> ErrorType for AnyPin<MODE> {
    type Error = Infallible;
}

impl<MODE> OutputPin for AnyPin<Output<MODE>> {
    #[inline(always)]
    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.set_low();
        Ok(())
    }
    #[inline(always)]
    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.set_high();
        Ok(())
    }
}
impl<MODE> StatefulOutputPin for AnyPin<Output<MODE>> {
    #[inline(always)]
    fn is_set_high(&mut self) -> Result<bool, Self::Error> {
        Ok(Self::is_set_high(self))
    }
    #[inline(always)]
    fn is_set_low(&mut self) -> Result<bool, Self::Error> {
        Ok(Self::is_set_low(self))
    }
}
// TODO: Finish implementation of traits in AnyPin

// Dynamic Pin
impl ErrorType for DynamicPinErased {
    type Error = PinModeError;
}

impl OutputPin for DynamicPinErased {
    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.set_high()
    }
    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.set_low()
    }
}

impl InputPin for DynamicPinErased {
    fn is_high(&mut self) -> Result<bool, Self::Error> {
        Self::is_high(self)
    }
    fn is_low(&mut self) -> Result<bool, Self::Error> {
        Self::is_low(self)
    }
}
