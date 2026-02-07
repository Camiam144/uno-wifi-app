use core::cell::RefCell;

use crate::{
    hal::gpio::{Input, Pin, PinExt, Pull},
    interrupts::{Binding, Handler},
};
use cortex_m::interrupt::{Mutex, free};
use embedded_hal::i2c::{self, Operation, SevenBitAddress};
// use enum_dispatch::enum_dispatch;
//
// use ringbuf::StaticRb;

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

// Some global state Gemini thinks is needed. Can always refactor later
/// Internal buffer size in bytes.
const BUFF_SIZE: usize = 32;

#[derive(Clone, Copy, PartialEq, defmt::Format)]
pub enum BusState {
    Idle,
    // Address, Buffer idx, Total length
    Writing(u8, usize, usize),
    // Address, Buffer idx, Total length
    Reading(u8, usize, usize),
    // What error did we get?
    Error(I2cError),
    // Stop has been requested but isn't yet complete
    Stopping,
    Complete,
}

/// Not a huge fan of this global mutable, the other method impls a different
/// &'static mutable on each i2c bus. Let's try that once I'm done here.
#[derive(defmt::Format)]
pub struct Transaction {
    pub state: BusState,
    pub buffer: [u8; BUFF_SIZE],
    pub is_first_trans: bool,
}

#[allow(clippy::new_without_default)]
impl Transaction {
    pub const fn new() -> Self {
        Self {
            state: BusState::Idle,
            buffer: [0; BUFF_SIZE],
            is_first_trans: true,
        }
    }

    pub fn reset(&mut self) {
        self.state = BusState::Idle;
        self.buffer = [0; BUFF_SIZE];
        self.is_first_trans = true;
    }
}

static I2C_GLOBAL: Mutex<RefCell<Transaction>> = Mutex::new(RefCell::new(Transaction::new()));

/// An instance of an IIC bus, will have to make the registers generic
pub trait I2cInstance {
    fn reg_block() -> *const ra4m1::iic0::RegisterBlock;
    // fn buffers() -> &'static IICBuffers;
    fn interrupt_event_start() -> u8;
}

// #[enum_dispatch]
// pub enum I2cRegBlock {
//     I2c0Block(*const ra4m1::iic0::RegisterBlock),
//     I2c1Block(*const ra4m1::iic1::RegisterBlock),
// }
//
// // #[enum_dispatch(I2cRegBlock)]
// trait I2cRegAccess {
//     fn clear_iccr1_ice(&self);
//     fn set_iccr1_ice(&self);
//     fn clear_iccr1_iicrst(&self);
//     fn set_iccr1_iicrst(&self);
//     // Have a 400 kHz or some adjustability here
//     fn set_clock_100khz(&self);
//     fn set_nak_rie_teie_tie(&self);
//     fn set_icsr2_stop_req(&self);
// }
//
// impl I2cRegAccess for I2cRegBlock {
//     fn clear_iccr1_ice(&self) {
//         match *self {
//             I2cRegBlock::I2c0Block(ptr) => {
//                 unsafe { (*ptr).iccr1.modify(|_, w| w.ice().clear_bit()) };
//             }
//             I2cRegBlock::I2c1Block(ptr) => {
//                 unsafe { (*ptr).iccr1.modify(|_, w| w.ice().clear_bit()) };
//             }
//         }
//     }
// }

impl I2cInstance for ra4m1::IIC0 {
    fn reg_block() -> *const ra4m1::iic0::RegisterBlock {
        ra4m1::IIC0::PTR
    }
    fn interrupt_event_start() -> u8 {
        0x035
    }
}

// impl I2cInstance for ra4m1::IIC1 {
//     fn reg_block() -> *const ra4m1::iic1::RegisterBlock {
//         ra4m1::IIC1::PTR
//     }
//     fn interrupt_event_start() -> u8 {
//         0x03A
//     }
// }

#[allow(non_camel_case_types)]
pub struct TEI_Handler<I: I2cInstance> {
    _phantom: core::marker::PhantomData<I>,
}

impl<I: I2cInstance> Handler for TEI_Handler<I> {
    unsafe fn on_interrupt(interrupt: ra4m1::Interrupt) {
        // Clear ICU flag
        let p = unsafe { ra4m1::Peripherals::steal() };
        p.ICU.ielsr[interrupt as usize].modify(|_, w| w.ir().clear_bit());
        // defmt::println!("TEI fired");

        // Check to make sure we were in transmit mode
        free(|cs| {
            let mut globals = I2C_GLOBAL.borrow(cs).borrow_mut();
            let bus_regs = unsafe { &*I::reg_block() };
            // clear the tend flag
            bus_regs.icsr2.modify(|_, w| w.tend().clear_bit());
            // I should really only hit this interrupt during transmit mode
            // TEND shouldn't ever fire outside of that.
            // lol j/k it fires if I get a Nack
            if let BusState::Error(_) = globals.state {
                return;
            }

            // Ignore clippy for now, maybe we want other stuff here later?
            match globals.state {
                BusState::Writing(_addr, idx, len) => {
                    if idx >= len {
                        // We wrote all of our data and we're done, stop the bus.
                        // blocking write will take care of the stopping.
                        globals.state = BusState::Complete;
                    }
                }
                _ => {}
            }
        })
    }
}

#[allow(non_camel_case_types)]
pub struct TXI_Handler<I: I2cInstance> {
    _phantom: core::marker::PhantomData<I>,
}

impl<I: I2cInstance> Handler for TXI_Handler<I> {
    unsafe fn on_interrupt(interrupt: ra4m1::Interrupt) {
        // Clear ICU flag
        let p = unsafe { ra4m1::Peripherals::steal() };
        p.ICU.ielsr[interrupt as usize].modify(|_, w| w.ir().clear_bit());
        // defmt::println!("TXI fired");
        // TXI is empty so we need to load it with appropriate data
        free(|cs| {
            let mut globals = I2C_GLOBAL.borrow(cs).borrow_mut();
            let bus_regs = unsafe { &*I::reg_block() };

            // use the global state to figure out where we are and what we need
            // to do. TXI can fire even after a NACK so we need to handle that.
            if let BusState::Error(_) = globals.state {
                return;
            }
            match globals.state {
                BusState::Writing(address, idx, len) => {
                    if globals.is_first_trans {
                        // Initial write, need to send address and write bit
                        // defmt::println!("First transaction launching address");
                        let addr = address << 1;
                        // Put the address and the 0 write bit into the register
                        bus_regs.icdrt.write(|w| unsafe { w.icdrt().bits(addr) });
                        // update global state
                        globals.is_first_trans = false;
                        globals.state = BusState::Writing(address, idx, len);
                    } else if idx < len {
                        // More data to transmit
                        // defmt::println!("Writing more data");
                        bus_regs
                            .icdrt
                            .write(|w| unsafe { w.bits(globals.buffer[idx]) });
                        globals.state = BusState::Writing(address, idx + 1, len);
                    }
                }
                BusState::Reading(address, _idx, _len) => {
                    // Reading is handled by the rxi interrupt but we still need
                    // the first transaction to set the read bits correctly.
                    defmt::println!("initiating read command");
                    if globals.is_first_trans {
                        let addr = address << 1;
                        bus_regs
                            .icdrt
                            .write(|w| unsafe { w.icdrt().bits(addr | 1) });
                        globals.is_first_trans = false;
                    }
                }
                _ => {}
            }
            // defmt::println!("Leaving TXI");
            // let iccr2 = bus_regs.iccr2.read().bits();
            // defmt::println!("iccr2 0b{:08b}", iccr2);
            // let icsr2 = bus_regs.icsr2.read().bits();
            // defmt::println!("icsr2 0b{:08b}", icsr2);
        });
    }
}

#[allow(non_camel_case_types)]
pub struct RXI_Handler<I: I2cInstance> {
    _phantom: core::marker::PhantomData<I>,
}

impl<I: I2cInstance> Handler for RXI_Handler<I> {
    unsafe fn on_interrupt(interrupt: ra4m1::Interrupt) {
        todo!();
    }
}

#[allow(non_camel_case_types)]
pub struct NAK_Handler<I: I2cInstance> {
    _phantom: core::marker::PhantomData<I>,
}

impl<I: I2cInstance> Handler for NAK_Handler<I> {
    unsafe fn on_interrupt(interrupt: ra4m1::Interrupt) {
        // Clear ICU interrupt
        let p = unsafe { ra4m1::Peripherals::steal() };
        p.ICU.ielsr[interrupt as usize].modify(|_, w| w.ir().clear_bit());

        // Error the bus, the error state will cause the main loop to stop
        // the bus.
        free(|cs| {
            let mut globals = I2C_GLOBAL.borrow(cs).borrow_mut();

            let bus_regs = unsafe { &*I::reg_block() };
            // Clear NACK flag
            bus_regs.icsr2.modify(|_, w| w.nackf().clear_bit());
            // defmt::println!("Nack fired");

            match globals.state {
                BusState::Writing(_, 0, _) | BusState::Reading(_, 0, _) => {
                    globals.state =
                        BusState::Error(I2cError::NoAcknowledge(i2c::NoAcknowledgeSource::Address));
                }
                _ => {
                    globals.state =
                        BusState::Error(I2cError::NoAcknowledge(i2c::NoAcknowledgeSource::Data));
                }
            }
        });
    }
}
/// Qwiic connector I2C bus on the board, IIC0. This is Wire1 in the Arduino code.
/// It is driven by pin 401 for SDA and pin 400 for SCL. Pass in the unconfigured
/// pins and this struct will take care of configuring them.
pub struct I2cBus<I: I2cInstance> {
    _phantom: core::marker::PhantomData<I>,
}

impl<I: I2cInstance> I2cBus<I> {
    /// The bus is driven by pin 401 for SDA and pin 400 for SCL. Pass in unconfigured
    /// pins and this struct will take care of configuring them and the owned IICO instance.
    pub fn new<IRQ>(
        _bus: I,
        sda_pin: Pin<4, 1, Input>,
        scl_pin: Pin<4, 0, Input>,
        _irq: IRQ,
    ) -> Self
    where
        IRQ: Binding<TEI_Handler<I>>
            + Binding<TXI_Handler<I>>
            + Binding<RXI_Handler<I>>
            + Binding<NAK_Handler<I>>,
    {
        // Init the bus
        let sda_pin = sda_pin.into_pullup_input().internal_resistor(Pull::Up);
        let scl_pin = scl_pin.into_pullup_input().internal_resistor(Pull::Up);

        let iic0_bus = unsafe { &*I::reg_block() };

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

        iic0_bus.icier.modify(|_, w| {
            w.nakie()
                .set_bit()
                .rie()
                .set_bit()
                .teie()
                .set_bit()
                .tie()
                .set_bit()
        });
        // Should be done now? Release the reset
        iic0_bus.iccr1.modify(|_, w| w.iicrst().clear_bit());

        let tei = <IRQ as Binding<TEI_Handler<I>>>::interrupt();
        let txi = <IRQ as Binding<TXI_Handler<I>>>::interrupt();
        let rxi = <IRQ as Binding<RXI_Handler<I>>>::interrupt();
        let nak = <IRQ as Binding<NAK_Handler<I>>>::interrupt();

        // Unmask
        unsafe {
            ra4m1::NVIC::unmask(tei);
            ra4m1::NVIC::unmask(txi);
            ra4m1::NVIC::unmask(rxi);
            ra4m1::NVIC::unmask(nak);
        }

        // Map events to interrupts
        let p = unsafe { ra4m1::Peripherals::steal() };

        p.ICU.ielsr[rxi as usize].write(|w| unsafe { w.iels().bits(I::interrupt_event_start()) });
        p.ICU.ielsr[txi as usize]
            .write(|w| unsafe { w.iels().bits(I::interrupt_event_start() + 1) });
        p.ICU.ielsr[tei as usize]
            .write(|w| unsafe { w.iels().bits(I::interrupt_event_start() + 2) });
        // This last interrupt is actually EEI which can be arbitration lost, nack,
        // timeout, start, or stop. I'm only turning on the nack interrupt so we only
        // should get this when we nack.
        p.ICU.ielsr[nak as usize]
            .write(|w| unsafe { w.iels().bits(I::interrupt_event_start() + 3) });

        I2cBus {
            _phantom: core::marker::PhantomData,
        }
    }

    fn stop_bus(&self) {
        let bus = unsafe { &*I::reg_block() };
        bus.icsr2.modify(|_, w| w.stop().clear_bit());
        bus.iccr2.modify(|_, w| w.sp().set_bit());
        // Spin wait for the bus to stop
        while bus.icsr2.read().stop().bit_is_clear() {
            cortex_m::asm::nop();
        }
        // Clear STOP and NACKF flags so we're ready for another transaction
        bus.icsr2
            .modify(|_, w| w.stop().clear_bit().nackf().clear_bit());
    }

    fn start_request(&self) {
        let bus = unsafe { &*I::reg_block() };
        bus.iccr2.modify(|_, w| w.st().set_bit());
    }

    /// Executes a blocking write operation for as many bytes are in the buffer
    /// Only 7 bit addresses for now.
    /// start indicates if this is the start of an operation
    /// stop indicatees if we need to send a stop condition
    pub fn write_blocking(
        &self,
        address: SevenBitAddress,
        buffer: &[u8],
        start: bool,
        _stop: bool,
    ) -> Result<(), I2cError> {
        let bus = unsafe { &*I::reg_block() };

        // Push the data into the global buffer so we can get it from the interrupt
        free(|cs| {
            let mut tx = I2C_GLOBAL.borrow(cs).borrow_mut();

            if buffer.len() > BUFF_SIZE {
                // this is why we need a circular buffer, then we can have
                // unlimited sized writes
                self.stop_bus();
                defmt::panic!("Write is too long");
            }

            // put the data into the buffer
            // if we had a ringbuffer this would also work.
            // defmt::println!("Pushing data");
            tx.buffer[0..buffer.len()].copy_from_slice(buffer);
            tx.state = BusState::Writing(address, 0, buffer.len());
        });

        // If we have a "start", we need to prep the registers
        // Read the BBSY flag in ICCR2, then set ST in ICCR2 to 1
        // Maybe this should go in the handler too?
        if start {
            // defmt::println!("Starting Bus");
            if bus.iccr2.read().bbsy().bit_is_set() {
                // Return an error, bus is busy
                // opt is spin-wait until unbusy, but that could hang if there is an issue
                defmt::println!("bus busy");
                return Err(I2cError::Bus);
            }
            // Issue start condition request
            self.start_request();

            // Wait for bus to move to transmit mode
            while bus.icsr2.read().tdre().bit_is_clear() {
                cortex_m::asm::nop();
                // self.stop_bus();
                // return Err(I2cError::Bus);
            }
            // defmt::println!("In transmit mode");
        }

        // defmt::println!("Moving to txi loop");
        // let iccr2 = bus.iccr2.read().bits();
        // defmt::println!("iccr2 0b{:08b}", iccr2);
        // let icsr2 = bus.icsr2.read().bits();
        // defmt::println!("icsr2 0b{:08b}", icsr2);
        // below this is the interrupt driven part
        loop {
            // wait for an interrupt and then check if we're done
            cortex_m::asm::wfi();

            let state = free(|cs| I2C_GLOBAL.borrow(cs).borrow().state);

            match state {
                BusState::Complete | BusState::Stopping | BusState::Error(_) => {
                    break;
                }
                _ => {
                    continue;
                }
            }
        }

        free(|cs| {
            let mut globals = I2C_GLOBAL.borrow(cs).borrow_mut();
            self.stop_bus();

            if let BusState::Error(err) = globals.state {
                globals.reset();
                return Err(err);
            };

            globals.reset();
            Ok(())
        })
    }

    // Executes a blocking read operation for as many bytes are in the buffer
    // Don't pass empty read buffers yet.
    // Only 7 bit addresses for now.
    // start indicates if this is the start of an operation
    // stop indicatees if we need to send a stop condition
    // pub fn read_blocking(
    //     &self,
    //     address: SevenBitAddress,
    //     buffer: &mut [u8],
    //     start: bool,
    //     stop: bool,
    // ) -> Result<(), I2cError> {
    //     if start {
    //         if self.bus.iccr2.read().bbsy().bit_is_set() {
    //             // Return an error, bus is busy
    //             defmt::println!("bus busy");
    //             return Err(I2cError::Bus);
    //         }
    //         // Issue start condition request
    //         self.start_request();
    //
    //         // Wait for bus to move to transmit mode
    //         while self.bus.icsr2.read().tdre().bit_is_clear() {
    //             cortex_m::asm::nop();
    //         }
    //     }
    //
    //     // Set up the read transaction
    //     let addr = (address << 1) | 1;
    //     // Put the address and the 1 read bit into the register
    //     self.bus.icdrt.write(|w| unsafe { w.icdrt().bits(addr) });
    //
    //     // Check for target ack:
    //     if self.bus.icsr2.read().nackf().bit_is_set() {
    //         // Stop the bus and return an error
    //         self.stop_bus();
    //         return Err(I2cError::NoAcknowledge(i2c::NoAcknowledgeSource::Address));
    //     }
    //     // Unlock the ackbt bit to use during the read
    //     self.bus.icmr3.modify(|_, w| w.ackwp().set_bit());
    //
    //     // Spin wait until there is data to dummy read
    //     while self.bus.icsr2.read().rdrf().bit_is_clear() {
    //         cortex_m::asm::nop();
    //     }
    //     // dummy read to start the clock
    //     let _ = self.bus.icdrr.read().icdrr().bits();
    //
    //     // Rread the whole buffer. I need some logic for 1 & 2 byte reads?
    //     let num_bytes = buffer.len();
    //     for (bytenum, read_buffer) in buffer.iter_mut().enumerate() {
    //         let bytes_remaining = num_bytes - bytenum;
    //         // spin wait for byte to arrive
    //         while self.bus.icsr2.read().rdrf().bit_is_clear() {
    //             cortex_m::asm::nop();
    //         }
    //
    //         // If we're on the 2nd to last byte, we need to do some magic
    //         if bytes_remaining == 2 {
    //             self.bus.icmr3.modify(|_, w| w.wait().set_bit());
    //         }
    //         if bytes_remaining == 1 {
    //             // If we're about to read the final bit, request a stop condition
    //             self.bus.iccr2.modify(|_, w| w.sp().set_bit());
    //         }
    //
    //         *read_buffer = self.bus.icdrr.read().icdrr().bits();
    //
    //         // If at least one more byte remains to be read, ack the transaction
    //         if bytes_remaining >= 2 {
    //             self.bus.icmr3.modify(|_, w| w.ackbt().clear_bit());
    //         } else {
    //             // if this is the final byte, nack the transaction
    //             self.bus.icmr3.modify(|_, w| w.ackbt().set_bit());
    //         }
    //     }
    //
    //     // Relock the ack bit
    //     self.bus.icmr3.modify(|_, w| w.ackwp().clear_bit());
    //
    //     // Stop bus if appropriate (we already requested a stop condition before
    //     // we read the final bit though?)
    //     if stop {
    //         self.bus.icsr2.modify(|_, w| w.stop().clear_bit());
    //         self.stop_bus();
    //     }
    //
    //     Ok(())
    // }
}

// Below here is for the embedded hal traits

impl<I: I2cInstance> i2c::ErrorType for I2cBus<I> {
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
