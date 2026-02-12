use core::cell::RefCell;

use crate::{
    hal::gpio::{Input, Pin, PinExt, Pull},
    interrupts::{Binding, Handler},
};
use cortex_m::interrupt::{Mutex, free};
use embedded_hal::i2c::{self, I2c, Operation, SevenBitAddress};
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

// This has to be pub so we can propogate the Error back up to the caller.
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
struct Transaction {
    state: BusState,
    buffer: [u8; BUFF_SIZE],
    is_first_trans: bool,
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

            // Ignore clippy for now, we will need to update this for 10 bit.
            // TEI will fire after the first transmit of a 10 bit address in read mode.
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
                    if len > BUFF_SIZE {
                        globals.state = BusState::Error(I2cError::Overrun);
                        return;
                    }

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
                BusState::Reading(address, 0, _len) => {
                    // Reading is handled by the rxi interrupt but we still need
                    // the first transaction to set the read bits correctly.
                    // defmt::println!("initiating read command");
                    if globals.is_first_trans {
                        let addr = address << 1;
                        bus_regs
                            .icdrt
                            .write(|w| unsafe { w.icdrt().bits(addr | 1) });
                        // Do not advance global idx state or we get the
                        // RXI out of whack. First RXI trigger needs idx = 0
                        globals.is_first_trans = false;
                    }
                }
                _ => {}
            }
        });
    }
}

#[allow(non_camel_case_types)]
pub struct RXI_Handler<I: I2cInstance> {
    _phantom: core::marker::PhantomData<I>,
}

impl<I: I2cInstance> Handler for RXI_Handler<I> {
    unsafe fn on_interrupt(interrupt: ra4m1::Interrupt) {
        // Clear ICU interrupt
        let p = unsafe { ra4m1::Peripherals::steal() };
        p.ICU.ielsr[interrupt as usize].modify(|_, w| w.ir().clear_bit());

        // defmt::println!("RXI Fired");
        free(|cs| {
            let mut globals = I2C_GLOBAL.borrow(cs).borrow_mut();
            let bus_regs = unsafe { &*I::reg_block() };

            // Make sure we didn't nack from the device
            if let BusState::Error(_) = globals.state {
                return;
            }

            // This is kind of complex depending on how many bytes we have remaining
            // In all cases ( >2 bytes, 2 bytes, 1 byte total) we have to dummy read.
            // Timing of dummy read changes depending on how many bytes we expect.
            // if we have more than 2, we dummy read immediately after address ack
            // and then read each byte until there are only two remaining bytes.
            // when we have exactly 2 remaining, we set the WAIT flag before reading,
            // then read the 2nd to last byte, then wait for 1 byte remaining, then nack
            // and clear flags and request a stop then read the final byte.
            // I am not sure if I have to stop before reading the byte or if I can
            // read and then request a stop.
            // if we only had 2 bytes total, we set wait, then dummy read, then nack.
            // if we have exactly 1, we set wait, then nack, then dummy read.

            // The docs say to issue stop with SP *before* reading the register.
            // but it seems to work just fine if I read the register first. Unsure...
            match globals.state {
                BusState::Reading(_addr, idx, 1) => {
                    // Single byte read
                    // Set Wait, Set Nack, dummy read, read, done
                    // Wait
                    bus_regs.icmr3.modify(|_, w| w.wait().set_bit());
                    // need to unlock ackbit before setting
                    bus_regs.icmr3.modify(|_, w| w.ackwp().set_bit());
                    // Nack
                    bus_regs.icmr3.modify(|_, w| w.ackbt().set_bit());
                    // Relock reg
                    bus_regs.icmr3.modify(|_, w| w.ackwp().clear_bit());
                    // Dummy read
                    let _ = bus_regs.icdrr.read().icdrr().bits();
                    // Actual read
                    // idx should always be zero for one byte reads.
                    globals.buffer[idx] = bus_regs.icdrr.read().icdrr().bits();
                    // Done
                    globals.state = BusState::Complete;
                }
                BusState::Reading(addr, idx, 2) => {
                    // Two byte read
                    // Set Wait, Dummy read, then nack, read, read, done.
                    match idx {
                        0 => {
                            // On first RXI fire, set wait, dummy read.
                            // Wait
                            bus_regs.icmr3.modify(|_, w| w.wait().set_bit());
                            // Dummy read before first byte
                            let _ = bus_regs.icdrr.read().icdrr().bits();
                            // Advance state
                            globals.state = BusState::Reading(addr, 1, 2);
                        }
                        1 => {
                            // Second RXI fire, set nack, real read, 1 byte remaning
                            // unlock reg
                            bus_regs.icmr3.modify(|_, w| w.ackwp().set_bit());
                            // Nack
                            bus_regs.icmr3.modify(|_, w| w.ackbt().set_bit());
                            // Relock reg
                            bus_regs.icmr3.modify(|_, w| w.ackwp().clear_bit());
                            // Real read
                            globals.buffer[0] = bus_regs.icdrr.read().icdrr().bits();
                            // Advance state
                            globals.state = BusState::Reading(addr, 2, 2);
                        }
                        _ => {
                            // Should only ever be idx = 2
                            if idx > 2 {
                                globals.state = BusState::Error(I2cError::Overrun);
                                return;
                            }
                            // Third and final RXI fire
                            // Real read & update globals
                            globals.buffer[1] = bus_regs.icdrr.read().icdrr().bits();
                            // clear Wait
                            bus_regs.icmr3.modify(|_, w| w.wait().clear_bit());
                            // Done, issue stop condition (done in blocking loop)
                            globals.state = BusState::Complete;
                        }
                    }
                }
                BusState::Reading(addr, idx, len) => {
                    // >= 3 byte read
                    // Check how many bytes are remaining
                    // this is how many remain *after* the current read
                    let n_bytes_remaining = len - idx;

                    // Note, this logic is presented in the OPPOSITE order of the previous
                    // match arms. Here 0 is the final byte to read, not idx = 0.
                    match n_bytes_remaining {
                        0 => {
                            // Final byte to read, set stop, read, clear wait flag.
                            // again, I set stop after reading, this might need to
                            // be changed. Also manual says to dummy read after nack? Why?
                            // Read
                            globals.buffer[idx - 1] = bus_regs.icdrr.read().icdrr().bits();
                            // Clear wait
                            bus_regs.icmr3.modify(|_, w| w.wait().clear_bit());
                            // Done
                            globals.state = BusState::Complete;
                        }
                        1 => {
                            // One byte remaining *after* current read, nack
                            // unlock reg
                            bus_regs.icmr3.modify(|_, w| w.ackwp().set_bit());
                            // Nack
                            bus_regs.icmr3.modify(|_, w| w.ackbt().set_bit());
                            // Relock reg
                            bus_regs.icmr3.modify(|_, w| w.ackwp().clear_bit());
                            // Read
                            globals.buffer[idx - 1] = bus_regs.icdrr.read().icdrr().bits();
                            // Advance state
                            globals.state = BusState::Reading(addr, idx + 1, len);
                        }
                        2 => {
                            // Two bytes remaining after current read, set wait
                            // Wait
                            bus_regs.icmr3.modify(|_, w| w.wait().set_bit());
                            // Read
                            globals.buffer[idx - 1] = bus_regs.icdrr.read().icdrr().bits();
                            // Advance state
                            globals.state = BusState::Reading(addr, idx + 1, len);
                        }
                        _ => {
                            // This special case could go outside in the match block
                            // since len <= 2 has already been caught higher up.
                            if idx == 0 {
                                // First read, do the dummy read stuff
                                // On first RXI fire, dummy read.
                                let _ = bus_regs.icdrr.read().icdrr().bits();
                                // Advance state
                                globals.state = BusState::Reading(addr, 1, len);
                            } else {
                                // Real read
                                globals.buffer[idx - 1] = bus_regs.icdrr.read().icdrr().bits();
                                // Advance state
                                globals.state = BusState::Reading(addr, idx + 1, len);
                            }
                        }
                    }
                }
                _ => {}
            }
        });
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
        // defmt::println!("Nack ERI");

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
        // Put pins in the right state for use
        let sda_pin = sda_pin
            .into_open_drain_output()
            .into_pullup_input()
            .internal_resistor(Pull::Up);
        let scl_pin = scl_pin
            .into_open_drain_output()
            .into_pullup_input()
            .internal_resistor(Pull::Up);

        let iic0_bus = unsafe { &*I::reg_block() };

        // Set the pins to the right setting (IIC0 and also as peripheral functions)
        sda_pin.pmnpfs_reg().modify(|_, w| w.pmr().set_bit());
        sda_pin
            .pmnpfs_reg()
            .modify(|_, w| unsafe { w.psel().bits(0b00111) });

        scl_pin.pmnpfs_reg().modify(|_, w| w.pmr().set_bit());
        scl_pin
            .pmnpfs_reg()
            .modify(|_, w| unsafe { w.psel().bits(0b00111) });

        // Init the bus
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
        iic0_bus.icmr1.modify(|_, w| w.cks()._010());
        iic0_bus.icbrh.modify(|_, w| unsafe { w.brh().bits(22) });
        iic0_bus.icbrl.modify(|_, w| unsafe { w.brl().bits(27) });
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
        // defmt::println!("Stop rqst");
        let bus = unsafe { &*I::reg_block() };
        bus.icsr2.modify(|_, w| w.stop().clear_bit());
        bus.iccr2.modify(|_, w| w.sp().set_bit());
        // Spin wait for the bus to stop
        while bus.icsr2.read().stop().bit_is_clear() {
            cortex_m::asm::nop();
        }

        // defmt::println!("Stopped ok");

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
        stop: bool,
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
            if !buffer.is_empty() {
                tx.buffer[0..buffer.len()].copy_from_slice(buffer);
            }
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
            // Should I have a timeout here?
            while bus.icsr2.read().tdre().bit_is_clear() {
                cortex_m::asm::nop();
            }
            // defmt::println!("Started ok");
        }

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
            if stop {
                self.stop_bus();
            }

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
    // clobbers the buffer you pass in, make sure it's long enough;
    // Only 7 bit addresses for now.
    // start indicates if this is the start of an operation
    // stop indicatees if we need to send a stop condition
    pub fn read_blocking(
        &self,
        address: SevenBitAddress,
        buffer: &mut [u8],
        start: bool,
        stop: bool,
    ) -> Result<(), I2cError> {
        let bus = unsafe { &*I::reg_block() };

        // Push the data into the global buffer so we can get it from the interrupt
        free(|cs| {
            let mut rx = I2C_GLOBAL.borrow(cs).borrow_mut();

            if buffer.len() > BUFF_SIZE {
                // Passed buffer is larger than max length read
                // if we had ring buffers we could have "infinite" length reads
                // but then I don't know how we get them out
                self.stop_bus();
                defmt::panic!("Read is too long");
            }

            rx.state = BusState::Reading(address, 0, buffer.len());
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
            // Timeout needed?
            while bus.icsr2.read().tdre().bit_is_clear() {
                cortex_m::asm::nop();
                // self.stop_bus();
                // return Err(I2cError::Bus);
            }
            // defmt::println!("In transmit mode for read more");
        }

        loop {
            // wait for an interrupt and then check if we're done
            cortex_m::asm::wfi();

            let (state, data) = free(|cs| {
                let globals = I2C_GLOBAL.borrow(cs).borrow();
                (globals.state, globals.buffer)
            });

            match state {
                BusState::Complete => {
                    // give the user back the data
                    buffer.copy_from_slice(&data[0..buffer.len()]);
                    break;
                }
                BusState::Stopping => {
                    break;
                }
                BusState::Error(_) => {
                    // Docs say to dummy read after a nack?
                    // Technically should come *after* the stop request.
                    // but before the bus actually stops?
                    // NACK -> Stop Request -> Dummy read -> bus stop.
                    // Does this break if I do it here?
                    let _ = bus.icdrr.read().icdrr().bits();
                    break;
                }
                _ => {
                    continue;
                }
            }
        }

        free(|cs| {
            let mut globals = I2C_GLOBAL.borrow(cs).borrow_mut();
            if stop {
                self.stop_bus();
            }

            if let BusState::Error(err) = globals.state {
                globals.reset();
                return Err(err);
            };

            globals.reset();
            Ok(())
        })
    }
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

// TODO: Is there a nice way to do this without a ton of custom logic?

// impl<I: I2cInstance> I2c for I2cBus<I> {
//     fn transaction(
//         &mut self,
//         address: u8,
//         operations: &mut [Operation<'_>],
//     ) -> Result<(), Self::Error> {
//         // First op in plan
//         let mut ops = operations.iter_mut();
//
//         // Do the first op
//         if let Some(prev_op) = ops.next() {
//             // do I need a "prep" method?
//             // Step 1, do the first operation with a start
//             match prev_op {
//                 Operation::Read(rb) => self.read_blocking(address, rb, true, false)?,
//                 Operation::Write(wb) => self.write_blocking(address, wb, true, false)?,
//             }
//
//             for op in ops {
//                 match (&prev_op, &op) {
//                     (Operation::Read(_), Operation::Write())
//
//                 }
//             }
//         }
//
//         // All done, stop bus.
//         self.stop_bus();
//         Ok(())
//     }
// }
