use crate::hal::gpio::{Input, Pin, PinExt, Pull};
use embedded_hal::i2c::{self, Operation, SevenBitAddress};

/// Unlock the qwiic bus
///
/// # Safety
///
/// Don't do this when something else is writing to the mstpb8 reg
pub unsafe fn enable_qwiic_bus() {
    let p = unsafe { ra4m1::Peripherals::steal() };
    p.MSTP.mstpcrb.modify(|_, w| w.mstpb9().clear_bit());
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum I2cError {
    Bus,
    ArbitrationLoss,
    NoAcknowledge(i2c::NoAcknowledgeSource),
    Overrun,
    Other,
}

impl i2c::Error for I2cError {
    fn kind(&self) -> i2c::ErrorKind {
        match *self {
            Self::Bus => i2c::ErrorKind::Bus,
            Self::ArbitrationLoss => i2c::ErrorKind::ArbitrationLoss,
            Self::NoAcknowledge(nack) => i2c::ErrorKind::NoAcknowledge(nack),
            Self::Overrun => i2c::ErrorKind::Overrun,
            Self::Other => i2c::ErrorKind::Other,
        }
    }
}

/// Qwiic connector I2C bus on the board, IIC0. This is Wire1 in the Arduino code.
/// It is driven by pin 401 for SDA and pin 400 for SCL. Pass in the unconfigured
/// pins and this struct will take care of configuring them.
pub struct Iic0 {
    bus: ra4m1::IIC0,
}

impl Iic0 {
    /// The bus is driven by pin 401 for SDA and pin 400 for SCL. Pass in unconfigured
    /// pins and this struct will take care of configuring them and the owned IICO instance.
    pub fn new(
        iic0_bus: ra4m1::IIC0,
        sda_pin: Pin<4, 1, Input>,
        scl_pin: Pin<4, 0, Input>,
    ) -> Self {
        // Init the bus
        let sda_pin = sda_pin.into_pullup_input().internal_resistor(Pull::Up);
        let scl_pin = scl_pin.into_pullup_input().internal_resistor(Pull::Up);

        // Set the pins to the right setting (IIC0 and also as peripheral functions)
        // I should be able to set OpenDrain on the pin itself.
        sda_pin
            .pmnpfs_reg()
            .modify(|_, w| w.pmr().set_bit().ncodr().set_bit());
        sda_pin
            .pmnpfs_reg()
            .modify(|_, w| unsafe { w.psel().bits(0b00111) });

        scl_pin
            .pmnpfs_reg()
            .modify(|_, w| w.pmr().set_bit().ncodr().set_bit());
        scl_pin
            .pmnpfs_reg()
            .modify(|_, w| unsafe { w.psel().bits(0b00111) });

        // Follow the steps outlined in figure 29.5 in manual:
        // SCL0, SDA0 pins not driven
        iic0_bus.iccr1.modify(|_, w| w.ice().clear_bit());
        // IIC reset
        iic0_bus.iccr1.modify(|_, w| w.iicrst().set_bit());
        // Internal reset, SCL0, SDA0 pins in active state
        iic0_bus.iccr1.modify(|_, w| w.ice().set_bit());
        // set transfer bit rate in ICMR1 and ICBRL/ICBRH
        // for now we will leave the icmr1 clock as the default PCLKB clock,
        // which is running at 24 MHz. Standard slow mode is 100 kHz. I have these
        // precalculated and hardcoded for now, eventually will want 400 kHz too.
        iic0_bus.icmr1.modify(|_, w| w.cks()._011());
        iic0_bus.icbrh.modify(|_, w| unsafe { w.brh().bits(0xA) });
        iic0_bus.icbrl.modify(|_, w| unsafe { w.brl().bits(0xC) });
        // I don't know how many interrupts to set. Maybe for now we use the four
        // noacknowledge, receive full, transmit end, and transmit empty.
        // TODO: Implement interrupt driven behavior

        // iic0_bus.icier.modify(|_, w| {
        //     w.nakie()
        //         .set_bit()
        //         .rie()
        //         .set_bit()
        //         .teie()
        //         .set_bit()
        //         .tie()
        //         .set_bit()
        // });
        // Should be done now? Release the reset
        iic0_bus.iccr1.modify(|_, w| w.iicrst().clear_bit());
        // let iccr1 = iic0_bus.iccr1.read().bits();
        // defmt::println!("iccr1 0b{:08b}", iccr1);
        // let icmr1 = iic0_bus.icmr1.read().bits();
        // defmt::println!("icmr1 0b{:08b}", icmr1);
        // let icbrh = iic0_bus.icbrh.read().bits();
        // defmt::println!("icbrh 0b{:08b}", icbrh);
        // let icbrl = iic0_bus.icbrl.read().bits();
        // defmt::println!("icbrl 0b{:08b}", icbrl);
        Iic0 { bus: iic0_bus }
    }

    fn stop_bus(&self) {
        self.bus.icsr2.modify(|_, w| w.stop().clear_bit());
        self.bus.iccr2.modify(|_, w| w.sp().set_bit());
        // Spin wait for the bus to stop
        while self.bus.icsr2.read().stop().bit_is_clear() {
            cortex_m::asm::nop();
        }
        // Clear STOP and NACKF flags so we're ready for another transaction
        self.bus
            .icsr2
            .modify(|_, w| w.stop().clear_bit().nackf().clear_bit());
    }

    fn start_request(&self) {
        self.bus.iccr2.modify(|_, w| w.st().set_bit());
    }

    /// Sets up the I2C peripheral for a write operation
    /// Only 7 bit addresses for now.
    /// start indicates if this is the start of an operation
    /// stop indicatees if we need to send a stop condition
    pub fn write_blocking(
        &self,
        address: SevenBitAddress,
        buffer: &[u8],
        start: bool,
        stop: bool,
    ) -> Result<(), I2cError> {
        // If we have a "start", we need to prep the registers
        // Read the BBSY flag in ICCR2, then set ST in ICCR2 to 1
        if start {
            if self.bus.iccr2.read().bbsy().bit_is_set() {
                // Return an error, bus is busy
                defmt::println!("bus busy");
                return Err(I2cError::Bus);
            }
            // Issue start condition request
            self.start_request();

            // Wait for bus to move to transmit mode
            while self.bus.icsr2.read().tdre().bit_is_clear() {
                cortex_m::asm::nop();
                // defmt::println!("Not in transmit mode");
                // let iccr2 = self.bus.iccr2.read().bits();
                // defmt::println!("iccr2 0b{:08b}", iccr2);
                // let icsr2 = self.bus.icsr2.read().bits();
                // defmt::println!("icsr2 0b{:08b}", icsr2);
                // self.stop_bus();
                // return Err(I2cError::Bus);
            }
        }
        // Set up the write transaction
        let addr = address << 1;
        // Put the address and the 0 write bit into the register
        self.bus.icdrt.write(|w| unsafe { w.icdrt().bits(addr) });

        for chunk in buffer {
            // Wait for data to be transmitted
            // and acknowledged
            // Check for nack from the target
            if self.bus.icsr2.read().nackf().bit_is_set() {
                // Stop the bus and return an error
                self.stop_bus();
                return Err(I2cError::NoAcknowledge(i2c::NoAcknowledgeSource::Address));
            }
            // Spin wait until ready, eventually this should probably be wfi?
            while self.bus.icsr2.read().tdre().bit_is_clear() {
                cortex_m::asm::nop();
            }
            // Everything is good, we can write
            self.bus.icdrt.write(|w| unsafe { w.icdrt().bits(*chunk) });
        }

        // Wait for last byte to transmit
        while self.bus.icsr2.read().tend().bit_is_clear() {
            cortex_m::asm::nop();
        }

        // Stop bus if appropriate
        if stop {
            self.bus.icsr2.modify(|_, w| w.stop().clear_bit());
            self.stop_bus();
        }

        Ok(())
    }
}

impl i2c::ErrorType for Iic0 {
    type Error = I2cError;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum OpKind {
    Read,
    Write,
}

pub trait Kind {
    fn kind(&self) -> OpKind;
}

impl Kind for i2c::Operation<'_> {
    fn kind(&self) -> OpKind {
        match self {
            Operation::Read(_) => OpKind::Read,
            Operation::Write(_) => OpKind::Write,
        }
    }
}

// impl I2c<SevenBitAddress> for Iic0 {
//     fn transaction(
//         &mut self,
//         address: SevenBitAddress,
//         operations: &mut [embedded_hal::i2c::Operation<'_>],
//     ) -> Result<(), Self::Error> {
//         // Read the BBSY flag in ICCR2, then set ST in ICCR2 to 1
//         if self.bus.iccr2.read().bbsy().bit_is_set() {
//             // Return an error, bus is busy
//             return Err(Error::Bus);
//         }
//
//         let mut last_op: Option<OpKind> = None;
//
//         // I need to know if I'm on the 2nd to last or last op for reading
//         let mut ops_remaining = operations.len();
//
//         for op in operations {
//             ops_remaining -= 1;
//             let kind = op.kind();
//
//             match op {
//                 Operation::Write(buffer) => {
//                     // Execute a write operation. Issue a start/restart if op is
//                     // different from previous, issue a stop if this is last op
//
//                 }
//             }
//
//         }
//
//         // Issue start condition request
//         self.bus.iccr2.modify(|_, w| w.st().set_bit());
//         // Now we're in master transmit mode
//         // We should check the TDRE flag in ICSR2
//
//         let first_op = &operations[0];
//
//         // let addr = 0x44;
//         let addr = match first_op {
//             Operation::Write(_) => address << 1,
//             Operation::Read(_) => (address << 1) + 1,
//         };
//
//         self.bus.icdrt.write(|w| unsafe { w.icdrt().bits(addr) });
//         // Check if response
//
//         if self.bus.icsr2.read().nackf().bit_is_set() {
//             // Return an error in here
//             // also stop the bus?
//             self.bus.iccr2.modify(|_, w| w.sp().set_bit());
//             return Err(Error::NoAcknowledge(i2c::NoAcknowledgeSource::Data));
//
//         }
//
//         if !iic0_bus.icsr2.read().nackf().bit_is_clear() {
//             defmt::println!("NACK from slave on write");
//             iic0_bus.iccr2.modify(|_, w| w.sp().set_bit());
//             // wait for bus to stop
//             let now = millis();
//             while millis() - now <= 5 {
//                 cortex_m::asm::nop();
//             }
//             uno_wifi_app::exit();
//         }
//         // Wait for tdre flag to set, indicating write buffer is empty
//         while iic0_bus.icsr2.read().tdre().bit_is_clear() {
//             cortex_m::asm::nop();
//         }
//
//         // Bit is 1, we can write:
//         iic0_bus.icdrt.write(|w| unsafe { w.icdrt().bits(*val) });
//     }
//
//     while iic0_bus.icsr2.read().tend().bit_is_clear() {
//         cortex_m::asm::nop();
//     }
//
//     // Issue a stop condition so the sensor takes a measurement
//     iic0_bus.icsr2.modify(|_, w| w.stop().clear_bit());
//     iic0_bus.iccr2.modify(|_, w| w.sp().set_bit());
//     //
//     // Wait for the thing to stop
//     while iic0_bus.icsr2.read().stop().bit_is_clear() {
//         cortex_m::asm::nop();
//     }
//
//     // Clear STOP and NACKF flags
//     iic0_bus
//         .icsr2
//         .modify(|_, w| w.stop().clear_bit().nackf().clear_bit());
//
//         Ok(())
// }
