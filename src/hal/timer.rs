/// This eventually needs to be something where you request a timer and if there
/// is one available you can get it, if not you get an error or false or something.
/// Arduino does this basically by keeping a list of how many timers are currently
/// running and passing the first available channel.
use core::marker::PhantomData;

/// Enable the GPT 32bit and 16bit timers
///
/// # Safety
///
/// Don't write to this register if timers are already running I think?
pub unsafe fn enable_gptimers() {
    let gpt_stopreg = crate::pac::MSTP::PTR;
    // Enable gpt_16 & gpt_32 timer module
    unsafe {
        (*gpt_stopreg)
            .mstpcrd
            .modify(|r, w| w.bits(r.bits() & !(0b11 << 5)));
    }
}

/// Enable the AGT 16bit timers
///
/// # Safety
///
/// Don't write to this register if timers are already running I think?
/// Also don't use this function if you're already writing to it somewhere else
pub unsafe fn enable_agtimers() {
    let p = unsafe { ra4m1::Peripherals::steal() };
    p.MSTP
        .mstpcrd
        .modify(|_, w| w.mstpd2().clear_bit().mstpd3().clear_bit());
}

#[allow(non_camel_case_types)]
#[repr(u8)]
pub enum Prescaler {
    PCLKD_1 = 0,
    PCLKD_4 = 1,
    PCLKD_16 = 2,
    PCLKD_64 = 3,
    PCLKD_256 = 4,
    PCLKD_1024 = 5,
}
impl Prescaler {
    pub fn divisor(&self) -> u32 {
        match self {
            Prescaler::PCLKD_1 => 1,
            Prescaler::PCLKD_4 => 4,
            Prescaler::PCLKD_16 => 16,
            Prescaler::PCLKD_64 => 64,
            Prescaler::PCLKD_256 => 256,
            Prescaler::PCLKD_1024 => 1024,
        }
    }
}

#[allow(non_camel_case_types)]
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum TimerMode {
    /// Timer restarts after period elapses.
    PERIODIC = 0,
    /// Timer stops after period elapses.
    ONE_SHOT = 1,
}

/// Select the type of PWM shape (can also use sawtooth with compare match regs)
#[allow(non_camel_case_types)]
#[repr(u8)]
#[derive(Debug, Copy, Clone)]
pub enum PwmMode {
    /// Sawtooth Wave Mode (must set compare match register)
    SAWTOOTH_WAVE_PWM = 0,
    /// Timer generates symmetric triangle-wave PWM output.
    TRIANGLE_WAVE_SYMMETRIC_PWM = 4,
    /// Timer generates asymmetric triangle-wave PWM output.
    TRIANGLE_WAVE_ASYMMETRIC_PWM = 5,
    /// Timer generates Asymmetric Triangle-wave PWM output. In PWM mode 3, the duty cycle does
    /// not need to be updated at each tough/crest interrupt. Instead, the trough and crest duty cycle values can be
    /// set once and only need to be updated when the application needs to change the duty cycle.
    TRIANGLE_WAVE_ASYMMETRIC_PWM_MODE3 = 6,
}

// Not sure if I need these or if these should be connected to the specific timer.
// Some sort of big ol' match statement?
#[derive(Debug, Clone, Copy)]
pub enum CountDir {
    Down,
    Up,
}
impl From<bool> for CountDir {
    fn from(value: bool) -> Self {
        if value { CountDir::Up } else { CountDir::Down }
    }
}
impl From<CountDir> for bool {
    fn from(value: CountDir) -> Self {
        match value {
            CountDir::Down => false,
            CountDir::Up => true,
        }
    }
}

// Type states for timer readiness
pub struct Unconfigured;
pub struct Configured;

// Type states for timer mode
pub struct NotSet;
pub struct Periodic;
pub struct OneShot;
pub struct Pwm;
pub struct InputCapture; // Less important

// /// Idk if I need to do all of this sealing stuff, this isn't a lib yet.
// /// LLMs like to do it because they see it in library code but... eh...
// mod private {
//     pub trait Sealed {}
// }

pub trait TimerSize {
    // TODO: Some check here that we have an actual valid unsigned int.
    // LLMS want to add From<u32> but that fails for u16.
    //
    // If I leave this as From<u16> will this work? It's just a trait bound...
    type CounterType: Copy + Into<u32> + defmt::Format;
    fn max_value() -> Self::CounterType;
}

pub struct Bits32;
impl TimerSize for Bits32 {
    type CounterType = u32;
    fn max_value() -> u32 {
        Self::CounterType::MAX
    }
}

pub struct Bits16;
impl TimerSize for Bits16 {
    type CounterType = u16;
    fn max_value() -> u16 {
        Self::CounterType::MAX
    }
}

pub enum TimerRegBlock {
    Block32(*const ra4m1::gpt320::RegisterBlock),
    Block16(*const ra4m1::gpt162::RegisterBlock),
}

/// Generic trait across both GPT types (not AGT!)
/// I can't put the register here because there's a 320 block and a 162 block.
pub trait TimerInstance {
    type Width: TimerSize;
    const CHANNEL: u8;
    const BLOCK: TimerRegBlock;
}

// Is there a better way to do this? Unsure as of now.
pub trait TimerExt {
    type Timer;
    fn into_timer(self) -> Self::Timer;
}

macro_rules! gptimer {
    ($GPTNAME: ident, $Gptname: ident, $Bits: ty, $Block: ident, $Ch: expr) => {
        impl TimerInstance for $Gptname {
            type Width = $Bits;
            const CHANNEL: u8 = $Ch;
            const BLOCK: TimerRegBlock = TimerRegBlock::$Block(ra4m1::$GPTNAME::PTR);
        }

        impl TimerExt for $crate::pac::$GPTNAME {
            type Timer = GPTimer<$Gptname, Unconfigured, NotSet>;
            fn into_timer(self) -> GPTimer<$Gptname, Unconfigured, NotSet> {
                GPTimer::<$Gptname, Unconfigured, NotSet>::new()
            }
        }
    };
}

// Stick these here to help rust analyzer a bit
pub struct Gpt0;
pub struct Gpt1;
pub struct Gpt2;
pub struct Gpt3;
pub struct Gpt4;
pub struct Gpt5;
pub struct Gpt6;
pub struct Gpt7;

gptimer!(GPT320, Gpt0, Bits32, Block32, 0);
gptimer!(GPT321, Gpt1, Bits32, Block32, 1);
gptimer!(GPT162, Gpt2, Bits16, Block16, 2);
gptimer!(GPT163, Gpt3, Bits16, Block16, 3);
gptimer!(GPT164, Gpt4, Bits16, Block16, 4);
gptimer!(GPT165, Gpt5, Bits16, Block16, 5);
gptimer!(GPT166, Gpt6, Bits16, Block16, 6);
gptimer!(GPT167, Gpt7, Bits16, Block16, 7);

// THis should be read from somewhere, preferably as part of the board init.
const MCU_FREQ: u32 = 48_000_000;

/// Generic General Purpose timer struct
///
/// - `T` is the Timer Instance, this holds some associated data such as the
///   register block, the timer channel, and the timer width in bits.
/// - `CFG` is the configuration status, timers can only run if they're configured
/// - `MODE` is a timer mode (PWM vs Periodic) maybe also One Shot?
pub struct GPTimer<T: TimerInstance, CFG, MODE> {
    _instance: PhantomData<T>,
    _cfg: PhantomData<CFG>,
    _mode: PhantomData<MODE>,
}

// Think really hard about what should be attached to the GPTimer and what should
// be attached to the TimerInstance trait.
#[allow(clippy::new_without_default)]
impl<T: TimerInstance, CFG, MODE> GPTimer<T, CFG, MODE> {
    pub fn new() -> Self {
        Self {
            _instance: PhantomData,
            _cfg: PhantomData,
            _mode: PhantomData,
        }
    }

    fn _set_periodic_mode(&self, pmode: TimerMode) {
        // There has got to be some way to make this less verbose. I guess
        // I could dispatch each of the 28 fields to the enum?
        match T::BLOCK {
            TimerRegBlock::Block32(val) => {
                unsafe { (*val).gtcr.write(|w| w.md().bits(pmode as u8)) };
            }
            TimerRegBlock::Block16(val) => {
                unsafe { (*val).gtcr.write(|w| w.md().bits(pmode as u8)) };
            }
        }
    }

    fn _set_pwm_mode(&self, pwmmode: PwmMode) {
        match T::BLOCK {
            TimerRegBlock::Block32(val) => {
                unsafe { (*val).gtcr.write(|w| w.md().bits(pwmmode as u8)) };
            }
            TimerRegBlock::Block16(val) => {
                unsafe { (*val).gtcr.write(|w| w.md().bits(pwmmode as u8)) };
            }
        }
    }

    fn _set_period_count(&self, count: u32) {
        // Silently drop the extra counts if we somehow try to shove 32 bits into
        // a 16 bit timer.
        match T::BLOCK {
            TimerRegBlock::Block32(val) => {
                unsafe {
                    (*val).gtpr.write(|w| w.gtpr().bits(count));
                    (*val).gtpbr.write(|w| w.gtpbr().bits(count));
                };
            }
            TimerRegBlock::Block16(val) => {
                unsafe {
                    (*val).gtpr.write(|w| w.gtpr().bits(count));
                    (*val).gtpbr.write(|w| w.gtpbr().bits(count));
                };
            }
        }
    }

    fn _set_prescaler(&self, prescaler: Prescaler) {
        match T::BLOCK {
            TimerRegBlock::Block32(val) => {
                unsafe { (*val).gtcr.modify(|_, w| w.tpcs().bits(prescaler as u8)) };
            }
            TimerRegBlock::Block16(val) => {
                unsafe { (*val).gtcr.modify(|_, w| w.tpcs().bits(prescaler as u8)) };
            }
        }
    }

    fn _set_count_dir(&self, dir: CountDir) {
        match T::BLOCK {
            TimerRegBlock::Block32(val) => {
                unsafe {
                    (*val).gtuddtyc.write(|w| w.ud().bit(dir.into()));
                };
            }
            TimerRegBlock::Block16(val) => {
                unsafe {
                    (*val).gtuddtyc.write(|w| w.ud().bit(dir.into()));
                };
            }
        }
    }

    fn _set_gtssr(&self, gtssr_src: GPTSourceT) {
        match T::BLOCK {
            TimerRegBlock::Block32(val) => {
                unsafe {
                    (*val).gtssr.write(|w| w.bits(gtssr_src as u32));
                };
            }
            TimerRegBlock::Block16(val) => {
                unsafe {
                    (*val).gtssr.write(|w| w.bits(gtssr_src as u32));
                };
            }
        }
    }

    fn _set_gtpsr(&self, gtpsr_src: GPTSourceT) {
        match T::BLOCK {
            TimerRegBlock::Block32(val) => {
                unsafe {
                    (*val).gtpsr.write(|w| w.bits(gtpsr_src as u32));
                };
            }
            TimerRegBlock::Block16(val) => {
                unsafe {
                    (*val).gtpsr.write(|w| w.bits(gtpsr_src as u32));
                };
            }
        }
    }
    fn _set_gtcsr(&self, gtcsr_src: GPTSourceT) {
        match T::BLOCK {
            TimerRegBlock::Block32(val) => {
                unsafe {
                    (*val).gtcsr.write(|w| w.bits(gtcsr_src as u32));
                };
            }
            TimerRegBlock::Block16(val) => {
                unsafe {
                    (*val).gtcsr.write(|w| w.bits(gtcsr_src as u32));
                };
            }
        }
    }
    // I probably need some checks so this doesn't get called incorrectly?
    // Plus I think this only works if the GTSSR is set to software.
    fn _start(&self) {
        match T::BLOCK {
            TimerRegBlock::Block32(val) => {
                unsafe {
                    (*val)
                        .gtstr
                        .modify(|r, w| w.bits(r.bits() | 1 << T::CHANNEL));
                };
            }
            TimerRegBlock::Block16(val) => {
                unsafe {
                    (*val)
                        .gtstr
                        .modify(|r, w| w.bits(r.bits() | 1 << T::CHANNEL));
                };
            }
        }
    }
    fn _stop(&self) {
        match T::BLOCK {
            TimerRegBlock::Block32(val) => {
                unsafe {
                    (*val)
                        .gtstp
                        .modify(|r, w| w.bits(r.bits() | 1 << T::CHANNEL));
                };
            }
            TimerRegBlock::Block16(val) => {
                unsafe {
                    (*val)
                        .gtstp
                        .modify(|r, w| w.bits(r.bits() | 1 << T::CHANNEL));
                };
            }
        }
    }
    fn _clear(&self) {
        match T::BLOCK {
            TimerRegBlock::Block32(val) => {
                unsafe {
                    (*val).gtclr.write(|w| w.bits(1 << T::CHANNEL));
                };
            }
            TimerRegBlock::Block16(val) => {
                unsafe {
                    (*val).gtclr.write(|w| w.bits(1 << T::CHANNEL));
                };
            }
        }
    }
    fn _has_overflowed(&self) -> bool {
        match T::BLOCK {
            TimerRegBlock::Block32(val) => unsafe { (*val).gtst.read().tcfpo().bit_is_set() },
            TimerRegBlock::Block16(val) => unsafe { (*val).gtst.read().tcfpo().bit_is_set() },
        }
    }
    fn _clear_overflow_flag(&self) {
        match T::BLOCK {
            TimerRegBlock::Block32(val) => unsafe {
                (*val).gtst.modify(|_, w| w.tcfpo().clear_bit());
            },
            TimerRegBlock::Block16(val) => unsafe {
                (*val).gtst.modify(|_, w| w.tcfpo().clear_bit());
            },
        }
    }

    fn set_frequency(&self, freq_hz: f32) -> Result<(), TimerError> {
        let max_count: <T::Width as TimerSize>::CounterType = <T::Width>::max_value();

        if freq_hz <= 0.0 || freq_hz > MCU_FREQ as f32 {
            return Err(TimerError::InvalidFrequencySetting);
        }

        self.set_period_counts(freq_hz, max_count)
    }

    fn set_period_counts(
        &self,
        freq_hz: f32,
        max: <T::Width as TimerSize>::CounterType,
    ) -> Result<(), TimerError> {
        let period_counts: u32;
        let prescaler: Prescaler;
        let target_counts: f32 = MCU_FREQ as f32 / freq_hz;

        if (target_counts as u32) < max.into() {
            period_counts = target_counts as u32;
            prescaler = Prescaler::PCLKD_1;
        } else if ((target_counts / 4.0) as u32) < max.into() {
            period_counts = (target_counts / 4.0) as u32;
            prescaler = Prescaler::PCLKD_4;
        } else if ((target_counts / 16.0) as u32) < max.into() {
            period_counts = (target_counts / 16.0) as u32;
            prescaler = Prescaler::PCLKD_16;
        } else if ((target_counts / 256.0) as u32) < max.into() {
            period_counts = (target_counts / 256.0) as u32;
            prescaler = Prescaler::PCLKD_256;
        } else if ((target_counts / 1024.0) as u32) < max.into() {
            period_counts = (target_counts / 1024.0) as u32;
            prescaler = Prescaler::PCLKD_1024;
        } else {
            return Err(TimerError::InvalidFrequencySetting);
        }

        self._set_period_count(period_counts);
        self._set_prescaler(prescaler);

        Ok(())
    }

    pub fn get_channel(&self) -> u8 {
        T::CHANNEL
    }
}
impl<T: TimerInstance> GPTimer<T, Unconfigured, NotSet> {
    /// Set mode to periodic and change typstate.
    pub fn into_periodic(self) -> GPTimer<T, Unconfigured, Periodic> {
        // Set mode to periodic before passing over
        self._set_periodic_mode(TimerMode::PERIODIC);
        GPTimer {
            _instance: PhantomData,
            _cfg: PhantomData,
            _mode: PhantomData,
        }
    }
    /// Set mode to PWM triangle and change typestate
    pub fn into_pwm(self) -> GPTimer<T, Unconfigured, Pwm> {
        self._set_pwm_mode(PwmMode::TRIANGLE_WAVE_SYMMETRIC_PWM);
        GPTimer {
            _instance: PhantomData,
            _cfg: PhantomData,
            _mode: PhantomData,
        }
    }
}

// Do we want to pass some sort of cfg struct?
pub struct PeriodicCfg {
    pub gtssr: GPTSourceT,
    pub gtpsr: GPTSourceT,
    pub gtcsr: GPTSourceT,
    pub count_dir: CountDir,
    /// This should be "reasonable" for the board for now
    pub freq_hz: f32,
}

impl<T: TimerInstance> GPTimer<T, Unconfigured, Periodic> {
    pub fn configure(self, cfg: PeriodicCfg) -> GPTimer<T, Configured, Periodic> {
        self._set_count_dir(cfg.count_dir);
        self._set_gtssr(cfg.gtssr);
        self._set_gtpsr(cfg.gtpsr);
        self._set_gtcsr(cfg.gtcsr);
        // TODO: Handle this error instead of unwrapping.
        self.set_frequency(cfg.freq_hz).unwrap();
        GPTimer {
            _instance: PhantomData,
            _cfg: PhantomData,
            _mode: PhantomData,
        }
    }
}

impl<T: TimerInstance, MODE> GPTimer<T, Configured, MODE> {
    pub fn start(&self) {
        self._start();
    }
    pub fn stop(&self) {
        self._stop();
    }
    pub fn clear(&self) {
        self._clear();
    }

    /// Check the TCPFO flag on the register to see if we've overflowed
    pub fn has_overflowed(&self) -> bool {
        self._has_overflowed()
    }
    pub fn clear_overflow_flag(&self) {
        self._clear_overflow_flag();
    }

    /// Release the timer by stopping, clearing, and returning to an unconfigured
    /// state.
    /// TODO: Run a full reset of all registers before releasing
    /// Should this be a drop? Or is returning an unconfigured not set timer enough?
    pub fn release(self) -> GPTimer<T, Unconfigured, NotSet> {
        self.stop();
        self.clear();
        GPTimer {
            _instance: PhantomData,
            _cfg: PhantomData,
            _mode: PhantomData,
        }
    }
}

/// Errors maybe?
#[derive(Debug)]
pub enum TimerError {
    InvalidFrequencySetting,
}

/// Sources can be used to start the timer, stop the timer, count up, or count down.
/// These enumerations represent a bitmask. Multiple sources can be ORed together.
/// We will almost exclusively be using the Software start on bit 31.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum GPTSourceT {
    // No active event sources.
    NONE = 0,
    // Action performed on GTETRGA rising edge.
    GTETRGA_RISING = (1 << 0),
    // Action performed on GTETRGA falling edge.
    GTETRGA_FALLING = (1 << 1),
    // Action performed on GTETRGB rising edge.
    GTETRGB_RISING = (1 << 2),
    // Action performed on GTETRGB falling edge.
    GTETRGB_FALLING = (1 << 3),
    // Action performed when GTIOCA input rises while GTIOCB is low.
    GTIOCA_RISING_WHILE_GTIOCB_LOW = (1 << 8),
    // Action performed when GTIOCA input rises while GTIOCB is high.
    GTIOCA_RISING_WHILE_GTIOCB_HIGH = (1 << 9),
    // Action performed when GTIOCA input falls while GTIOCB is low.
    GTIOCA_FALLING_WHILE_GTIOCB_LOW = (1 << 10),
    // Action performed when GTIOCA input falls while GTIOCB is high.
    GTIOCA_FALLING_WHILE_GTIOCB_HIGH = (1 << 11),
    // Action performed when GTIOCB input rises while GTIOCA is low.
    GTIOCB_RISING_WHILE_GTIOCA_LOW = (1 << 12),
    // Action performed when GTIOCB input rises while GTIOCA is high.
    GTIOCB_RISING_WHILE_GTIOCA_HIGH = (1 << 13),
    // Action performed when GTIOCB input falls while GTIOCA is low.
    GTIOCB_FALLING_WHILE_GTIOCA_LOW = (1 << 14),
    // Action performed when GTIOCB input falls while GTIOCA is high.
    GTIOCB_FALLING_WHILE_GTIOCA_HIGH = (1 << 15),
    // Action performed on ELC GPTA event.
    GPT_A = (1 << 16),
    // Action performed on ELC GPTB event.
    GPT_B = (1 << 17),
    // Action performed on ELC GPTC event.
    GPT_C = (1 << 18),
    // Action performed on ELC GPTD event.
    GPT_D = (1 << 19),
    // Action performed on ELC GPTE event.
    GPT_E = (1 << 20),
    // Action performed on ELC GPTF event.
    GPT_F = (1 << 21),
    // Action performed on ELC GPTG event.
    GPT_G = (1 << 22),
    // Action performed on ELC GPTH event.
    GPT_H = (1 << 23),
    // Action performed on Software Source event.
    // Enables the GTSTR, GTSTP, and GTCLR registers when used appropriately
    SOFTWARE = (1 << 31),
}

// =====================================
//              AGTimer Stuff
// =====================================

#[allow(non_camel_case_types)]
#[repr(u8)]
pub enum AGTPrescaler {
    PCLKD_1 = 0,
    PCLKD_8 = 1,
    PCLKD_2 = 3,
    Divided_AGTLCLK = 4,
    Underflow_AGT0 = 5,
    Divided_AGTSCLK = 6,
}

pub struct AGTimer0;
pub struct AGTimer1;

pub trait AGTimerInstance {
    type Width: TimerSize;
    const CHANNEL: u8;
    const BLOCK: *const ra4m1::agt0::RegisterBlock;
}
// Only 2 timers so we'll just copy-paste instead of macroing.
// Ideally (maybe) the AGTimers could be just another timer type, it does add
// another arm to the match enum.
impl AGTimerInstance for AGTimer0 {
    type Width = Bits16;
    const CHANNEL: u8 = 0;
    const BLOCK: *const ra4m1::agt0::RegisterBlock = ra4m1::AGT0::PTR;
}
impl AGTimerInstance for AGTimer1 {
    type Width = Bits16;
    const CHANNEL: u8 = 1;
    const BLOCK: *const ra4m1::agt0::RegisterBlock = ra4m1::AGT1::PTR;
}

impl TimerExt for crate::pac::AGT0 {
    type Timer = AGTimer<AGTimer0, Unconfigured>;
    fn into_timer(self) -> AGTimer<AGTimer0, Unconfigured> {
        AGTimer::<AGTimer0, Unconfigured>::new()
    }
}
impl TimerExt for crate::pac::AGT1 {
    type Timer = AGTimer<AGTimer1, Unconfigured>;
    fn into_timer(self) -> AGTimer<AGTimer1, Unconfigured> {
        AGTimer::<AGTimer1, Unconfigured>::new()
    }
}

/// AGT timer struct
///
/// This follows a lot of the same conventions as the GPT timer stuff, but
/// adapted for AGT.
pub struct AGTimer<T: AGTimerInstance, CFG> {
    _instance: PhantomData<T>,
    _cfg: PhantomData<CFG>,
}

#[allow(clippy::new_without_default)]
impl<T: AGTimerInstance, CFG> AGTimer<T, CFG> {
    pub fn new() -> Self {
        Self {
            _instance: PhantomData,
            _cfg: PhantomData,
        }
    }
    // fn _set_periodic_mode(&self) {
    //     unsafe {
    //         (*T::BLOCK).agtmr1.modify(|_, w| w.tmod()._000());
    //     }
    // }

    fn _set_period_count(&self, count: u16) {
        // There is a bunch of timing stuff if you use the compare match registers
        // and a certain number of clock cycles before registers are enabled
        // or reloaded. Idk. Just don't mess it up.
        unsafe {
            (*T::BLOCK).agt.write(|w| w.agt().bits(count));
        }
    }
    fn _set_prescaler(&self, prescaler: AGTPrescaler) {
        unsafe {
            (*T::BLOCK)
                .agtmr1
                .modify(|_, w| w.tck().bits(prescaler as u8));
        }
    }
    fn _start(&self) {
        unsafe {
            (*T::BLOCK).agtcr.modify(|_, w| w.tstart().set_bit());
        };
    }
    fn _stop(&self) {
        unsafe {
            (*T::BLOCK).agtcr.modify(|_, w| w.tstop().clear_bit());
        };
    }
    fn _clear(&self) {
        unsafe {
            (*T::BLOCK).agt.write(|w| w.agt().bits(0xFFFF));
        };
    }

    fn _has_underflowed(&self) -> bool {
        unsafe { (*T::BLOCK).agtcr.read().tundf().bit_is_set() }
    }
    fn _clear_underflow_flag(&self) {
        unsafe {
            (*T::BLOCK).agtcr.modify(|_, w| w.tundf().clear_bit());
        }
    }
}
// Do we want to pass some sort of cfg struct?
pub struct AGTCfg {
    pub counts: u16,
    pub prescaler: AGTPrescaler,
}

impl<T: AGTimerInstance> AGTimer<T, Unconfigured> {
    pub fn configure(self, cfg: AGTCfg) -> AGTimer<T, Configured> {
        self._set_period_count(cfg.counts);
        self._set_prescaler(cfg.prescaler);
        // self._set_periodic_mode();
        AGTimer {
            _instance: PhantomData,
            _cfg: PhantomData,
        }
    }
}

impl<T: AGTimerInstance> AGTimer<T, Configured> {
    pub fn start(&self) {
        self._start();
    }
    pub fn stop(&self) {
        self._stop();
    }
    pub fn clear(&self) {
        self._clear();
    }
    /// Check the TCPFO flag on the register to see if we've overflowed
    pub fn has_underflowed(&self) -> bool {
        self._has_underflowed()
    }
    pub fn clear_underflow_flag(&self) {
        self._clear_underflow_flag();
    }

    /// Release the timer by stopping, clearing, and returning to an unconfigured
    /// state.
    /// TODO: Run a full reset of all registers before releasing
    /// Should this be a drop? Or is returning an unconfigured timer enough?
    pub fn release(self) -> AGTimer<T, Unconfigured> {
        self.stop();
        self.clear();
        AGTimer {
            _instance: PhantomData,
            _cfg: PhantomData,
        }
    }
}
