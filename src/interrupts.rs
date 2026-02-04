use ra4m1::Interrupt;

#[allow(non_camel_case_types)]
#[repr(i8)]
pub enum IRQn_Type {
    // Negative numbers are set by the ARM chip itself.
    // Reset_IRQn = -15,            //  1 Reset Vector invoked on Power up and warm reset
    // NonMaskableInt_IRQn = -14,   //  2 Non maskable Interrupt cannot be stopped or preempted
    // HardFault_IRQn = -13,        //  3 Hard Fault all classes of Fault
    // MemoryManagement_IRQn = -12, //  4 Memory Management MPU mismatch, including Access Violation and No Match
    // BusFault_IRQn = -11, //  5 Bus Fault Pre-Fetch-, Memory Access, other address/memory Fault
    // UsageFault_IRQn = -10, //  6 Usage Fault i.e. Undef Instruction, Illegal State Transition
    // SecureFault_IRQn = -9, //  7 Secure Fault Interrupt
    // SVCall_IRQn = -5,    // 11 System Service Call via SVC instruction
    // DebugMonitor_IRQn = -4, // 12 Debug Monitor
    // PendSV_IRQn = -2,    // 14 Pendable request for system service
    // SysTick_IRQn = -1,   // 15 System Tick Timer
    // These values are user selectable. IRQn 0-7 are set by the arduino bootloader,
    // probably for communication with the ESP32 chip and maybe something else.
    USBFS_USBI = 0,
    USBFS_USBR = 1,
    USBFS_D0FIFO = 2,
    USBFS_D1FIFO = 3,
    SCI9_TXI = 4,
    SCI9_TEI = 5,
    SCI9_RXI = 6,
    SCI9_ERI = 7,
}

// I stole this stuff from a blog post on the arduino uno R4 for interrupt handling
// and they got it by slightly editing the embassy hal stuff.

// Macro to bind interrupts to handlers.
//
// This defines the right interrupt handlers, and creates a unit struct (like `struct Irqs;`)
// and implements the right Binding for it. You can pass this struct to drivers to
// prove at compile-time that the right interrupts have been bound.
//
// Example of how to bind one interrupt:
//
// ```rust,ignore
// use embassy_stm32::{bind_interrupts, usb, peripherals};
//
// bind_interrupts!(struct Irqs {
//     OTG_FS => usb::InterruptHandler<peripherals::USB_OTG_FS>;
// });
// ```
//
// Example of how to bind multiple interrupts, and multiple handlers to each interrupt, in a single macro invocation:
//
// ```rust,ignore
// use embassy_stm32::{bind_interrupts, i2c, peripherals};
//
// bind_interrupts!(
//     /// Binds the I2C interrupts.
//     struct Irqs {
//         I2C1 => i2c::EventInterruptHandler<peripherals::I2C1>, i2c::ErrorInterruptHandler<peripherals::I2C1>;
//         I2C2_3 => i2c::EventInterruptHandler<peripherals::I2C2>, i2c::ErrorInterruptHandler<peripherals::I2C2>,
//             i2c::EventInterruptHandler<peripherals::I2C3>, i2c::ErrorInterruptHandler<peripherals::I2C3>;
//     }
// );
// ```
//
// Some chips collate multiple interrupt signals into a single interrupt vector. In the above example, I2C2_3 is a
// single vector which is activated by events and errors on both peripherals I2C2 and I2C3. Check your chip's list
// of interrupt vectors if you get an unexpected compile error trying to bind the standard name.
// #[macro_export]
// macro_rules! bind_interrupts {
//     ($(#[$outer:meta])* $vis:vis struct $name:ident {
//         $(
//             $(#[doc = $doc:literal])*
//             $(#[cfg($cond_irq:meta)])?
//             $irq:ident => $(
//                 $(#[cfg($cond_handler:meta)])?
//                 $handler:ty
//             ),*;
//         )*
//     }) => {
//         #[derive(Copy, Clone)]
//         $(#[$outer])*
//         $vis struct $name;
//
//         $(
//             #[allow(non_snake_case)]
//             #[unsafe(no_mangle)]
//             $(#[cfg($cond_irq)])?
//             $(#[doc = $doc])*
//             unsafe extern "C" fn $irq() {
//                 $(
//                     $(#[cfg($cond_handler)])?
//                     unsafe {<$handler as $crate::interrupts::Handler>::on_interrupt(ra4m1::Interrupt::$irq)};
//
//                 )*
//             }
//
//             $(#[cfg($cond_irq)])?
//             $crate::bind_interrupts!(@inner
//                 $(
//                     $(#[cfg($cond_handler)])?
//                     unsafe impl $crate::interrupts::Binding<$handler> for $name {
//                         fn interrupt() -> ra4m1::Interrupt {
//                             ra4m1::Interrupt::$irq
//                         }
//                     }
//                 )*
//             );
//         )*
//     };
//     (@inner $($t:tt)*) => {
//         $($t)*
//     }
// }

// This handles some simple binding to ensure we do the interrupts correctly I think
// it works for my needs for now so don't touch it too much
#[macro_export]
macro_rules! bind_interrupts {
    (struct $name:ident {
    $(
        $irq:ident => $handler:ty;
    )*
    }) => {
    #[derive(Copy, Clone)]
    struct $name;
    $(
    #[interrupt]
    fn $irq() {
        unsafe {
            <$handler as $crate::interrupts::Handler>::on_interrupt(ra4m1::Interrupt::$irq)
        };
    }
    )*
    $(
    unsafe impl $crate::interrupts::Binding<$handler> for $name {
        fn interrupt() -> ra4m1::Interrupt {
            ra4m1::Interrupt::$irq
        }
    }
    )*
    };
}

/// Trait for handling interrupts.
///
/// The `on_interrupt` method is called when an interrupt occurs
/// after binding with the provided macro.
pub trait Handler {
    /// This binds to a specific interrupt enum
    ///
    /// # Safety
    /// All safety rules related to calling interrupts applies.
    unsafe fn on_interrupt(interrupt: Interrupt);
}

/// Confirms the handler is bound to an interrupt.
///
/// # Safety
/// Don't use this outside of the provided macro
pub unsafe trait Binding<H: Handler> {
    fn interrupt() -> Interrupt;
}
