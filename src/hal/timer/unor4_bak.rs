use super::*;
use crate::pac::GPT162;
use cortex_m::asm;

// This should get macroed and used for all GPTs. Macro includes GPT number,
// type (u16 vs u32), and probably the overflow interrupt id?
impl<MODE> GptRegs for Gpt<GPT162, MODE> {
    type Counter = u16;

    #[inline]
    fn disable(&self) {
        // All of this assumes we're still on the software start/stop/clear
        self.regs.gtcr.modify(|_, w| w.cst().clear_bit());
        // ChatGPT says to wait here, so eh?
        while self.regs.gtcr.read().cst().bit_is_set() {
            asm::nop();
        }
    }

    #[inline]
    fn enable(&self) {
        self.regs.gtcr.modify(|_, w| w.cst().set_bit());
    }

    #[inline]
    fn set_reload(&self, value: u16) {
        // This is the period register max counts
        self.regs
            .gtpr
            .write(|w| unsafe { w.gtpr().bits(value.into()) });
    }

    #[inline]
    fn set_prescaler(&self, prescaler: Prescaler) {
        self.regs
            .gtcr
            .modify(|_, w| unsafe { w.tpcs().bits(prescaler as u8) });
    }

    #[inline]
    fn clear_counter(&self) {
        self.regs.gtclr.write(|w| w.cclr2().set_bit());
    }

    #[inline]
    fn set_compare_a(&self, value: u16) {
        self.regs.gtccra.write(|w| unsafe { w.bits(value.into()) });
    }

    /// This sets the timer mode to sawtooth repeating. Should probably be changed
    /// to take an enum that lets you pick between repeating and one shot modes.
    /// Also resets the pclck divider.
    #[inline]
    fn set_timer_mode(&self) {
        self.regs.gtcr.modify(|_, w| w.md()._000().tpcs()._000());
    }

    /// This sets the timer mode to the provided PwmMode
    /// Also resets the pclck divider.
    #[inline]
    fn set_pwm_mode(&self, mode: PwmMode) {
        self.regs
            .gtcr
            .modify(|_, w| unsafe { w.md()._100().tpcs().bits(mode as u8) });
    }

    /// This should enable the overflow interrupt
    /// TODO: I think this flag needs tog et turned on somewhere else. or maybe
    /// it's always enabled?
    #[inline]
    fn enable_overflow_irq(&self) {
        // This is Event Number 0x06D in the IELSRn reg
        self.regs.gtst.write(|w| w.tcfpo()._0());
    }

    #[inline]
    fn clear_overflow_irq_flag(&self) {
        self.regs.gtst.write(|w| w.tcfpo()._0());
    }

    #[inline]
    fn overflow_irq_pending(&self) -> bool {
        self.regs.gtst.read().tcfpo().bit_is_set()
    }
}
