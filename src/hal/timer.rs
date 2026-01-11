// This eventually needs to be something where you request a timer and if there
// is one available you can get it, if not you get an error or false or something.
// Arduino does this basically by keeping a list of how many timers are currently
// running and passing the first available channel.

use core::sync::atomic::{AtomicU8, Ordering};

/// Sources can be used to start the timer, stop the timer, count up, or count down. These enumerations represent a bitmask. Multiple sources can be ORed together.
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
    // // Action performed on GTETRGC rising edge.
    // GTETRGC_RISING = (1 << 4),
    // // Action performed on GTETRGC falling edge.
    // GTETRGC_FALLING = (1 << 5),
    // // Action performed on GTETRGB rising edge.
    // GTETRGD_RISING = (1 << 6),
    // // Action performed on GTETRGB falling edge.
    // GTETRGD_FALLING = (1 << 7),
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

#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TimerT {
    GPT_16_Timer,
    GPT_32_Timer,
    // TODO: add AGT timers
}

#[allow(non_camel_case_types)]
#[repr(u8)]
pub enum TimerPrescalerSelect {
    PCLKD_1 = 0,
    PCLKD_4 = 1,
    PCLKD_16 = 2,
    PCLKD_64 = 3,
    PCLKD_256 = 4,
    PCLKD_1024 = 5,
}

#[allow(non_camel_case_types)]
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum TimerModeT {
    /// Timer restarts after period elapses.
    PERIODIC = 0,
    /// Timer stops after period elapses.
    ONE_SHOT = 1,
    // /// Timer generates saw-wave PWM output.
    // PWM = 2,
    // /// Saw-wave one-shot pulse mode (fixed buffer operation).
    // ONE_SHOT_PULSE = 3,
    /// Timer generates symmetric triangle-wave PWM output.
    TRIANGLE_WAVE_SYMMETRIC_PWM = 4,
    /// Timer generates asymmetric triangle-wave PWM output.
    TRIANGLE_WAVE_ASYMMETRIC_PWM = 5,
    /// Timer generates Asymmetric Triangle-wave PWM output. In PWM mode 3, the duty cycle does
    ///not need to be updated at each tough/crest interrupt. Instead, the trough and crest duty cycle values can be
    /// set once and only need to be updated when the application needs to change the duty cycle.
    TRIANGLE_WAVE_ASYMMETRIC_PWM_mODE3 = 6,
}

/// Enable the GPT 32bit and 16bit timers
pub fn enable_gptimers() {
    let gpt_stopreg = ra4m1::MSTP::PTR;
    // Enable gpt_16 & gpt_32
    unsafe {
        (*gpt_stopreg)
            .mstpcrd
            .modify(|r, w| w.bits(r.bits() & !(0b11 << 5)));
    }
}

/// Board-specific constants
pub const GPT_HOWMANY: usize = 8;
pub const GPT_32_HOWMANY: usize = 2;
pub const GPT_16_HOWMANY: usize = 6;

#[derive(Debug)]
pub struct TimerChannel(u8);

/// First 2 bits are 32 bit timers, next 6 are for 16 bit timers.
static GPT_USED_CHANNEL: AtomicU8 = AtomicU8::new(0);

#[derive(Debug)]
pub enum TimerError {
    NoTimersAvailable,
    TimerChannelNotClaimed,
    TimerAlreadyRunning,
    InvalidFrequencySetting,
}

/// I might need a better way to do this, for safety do not mutate the resulting
/// TimerChannel you get returned from this instance. I might want to move this
/// logic to the timer creation field but then what happens if it fails to create
/// a timer?
pub fn claim_timer(timertype: &TimerT) -> Result<TimerChannel, TimerError> {
    // Get our range depending on our timer type
    let (lower, upper) = match timertype {
        TimerT::GPT_32_Timer => (0, GPT_32_HOWMANY),
        TimerT::GPT_16_Timer => (GPT_32_HOWMANY, GPT_HOWMANY),
    };

    loop {
        let current_val = GPT_USED_CHANNEL.load(Ordering::Acquire);

        if (current_val == 0b11 && *timertype == TimerT::GPT_32_Timer)
            || (current_val >= 0b11111100 && *timertype == TimerT::GPT_16_Timer)
        {
            return Err(TimerError::NoTimersAvailable);
        }

        // Find the first available timer within that range (0 bit)
        for channel in lower..upper {
            let bit = 1 << channel;
            // Open timer at this shift
            if current_val & bit == 0 {
                let new_value = current_val | bit;
                // This bit is from the docs and I don't really get it
                if GPT_USED_CHANNEL
                    .compare_exchange(current_val, new_value, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
                {
                    return Ok(TimerChannel(channel.try_into().unwrap()));
                }
            }
        }
    }
}

fn release_timer(channel: TimerChannel) {
    let bit = 1 << channel.0;
    GPT_USED_CHANNEL.fetch_and(!bit, Ordering::Release);
}

// Not sure if I need these or if these should be connected to the specific timer.
// Some sort of big ol' match statement?
const GPT_32_PTRS: [*const ra4m1::gpt320::RegisterBlock; 2] =
    [ra4m1::GPT320::PTR, ra4m1::GPT321::PTR];

const GPT_16_PTRS: [*const ra4m1::gpt162::RegisterBlock; 6] = [
    ra4m1::GPT162::PTR,
    ra4m1::GPT163::PTR,
    ra4m1::GPT164::PTR,
    ra4m1::GPT165::PTR,
    ra4m1::GPT166::PTR,
    ra4m1::GPT167::PTR,
];

enum GPTRegBlockPtr {
    GPT32RegBlock(*const ra4m1::gpt320::RegisterBlock),
    GPT16RegBlock(*const ra4m1::gpt162::RegisterBlock),
}

#[derive(Debug, Clone, Copy)]
pub enum CountDir {
    Up,
    Down,
}

/// Build a timer config to pass into a GPTimer instance.
#[derive(Debug, Clone, Copy)]
pub struct TimerCfg {
    pub timer_type: TimerT,
    pub count_direction: CountDir,
    pub gtssr: GPTSourceT,
    pub gtpsr: GPTSourceT,
    pub gtcsr: GPTSourceT,
    pub mode: TimerModeT,
    pub freq: u32, // Do I need a check to make a reasonable frequency?
}

// What does a timer need to function?
pub struct GPTimer {
    timer_type: TimerT,
    duty_cycle_pct: f32,
    period_counts: u32, // Limited to u16 on 16 bit timers
    mode: TimerModeT,
    prescaler: TimerPrescalerSelect,
    channel: TimerChannel,
    count_direction: CountDir,
    // GPTimer specific settings to be extended later, this part will be different for AGTimers?
    gtssr: GPTSourceT,
    gtpsr: GPTSourceT,
    gtcsr: GPTSourceT,
    count_up_source: Option<GPTSourceT>,
    count_down_source: Option<GPTSourceT>,
    enable_buffering: bool,
    reg_block_ptr: GPTRegBlockPtr,
    // Some stuff to track timer state
    is_running: bool,
    is_stopped: bool,
}
impl GPTimer {
    /// Instantiates a new timer from a config and a channel. Takes ownership of
    /// the channel. Should this be allowed to fail? Maybe? Technically if we
    /// have an open channel there's no way this should ever fail.
    pub fn new_from_config(cfg: TimerCfg, channel: TimerChannel) -> Self {
        let reg_ptr = match &cfg.timer_type {
            TimerT::GPT_16_Timer => {
                GPTRegBlockPtr::GPT16RegBlock(GPT_16_PTRS[(channel.0 - 2) as usize])
            }
            TimerT::GPT_32_Timer => GPTRegBlockPtr::GPT32RegBlock(GPT_32_PTRS[channel.0 as usize]),
        };
        let mut timer = GPTimer {
            timer_type: cfg.timer_type,
            duty_cycle_pct: 0.50,
            period_counts: 0,
            mode: cfg.mode,
            prescaler: TimerPrescalerSelect::PCLKD_1,
            channel,
            count_direction: cfg.count_direction,
            gtssr: cfg.gtssr,
            gtpsr: cfg.gtpsr,
            gtcsr: cfg.gtcsr,
            count_up_source: None,
            count_down_source: None,
            enable_buffering: true,
            reg_block_ptr: reg_ptr,
            is_running: false,
            is_stopped: true,
        };
        timer.set_count_dir();
        timer.set_count(0);
        timer
    }
    fn set_count_dir(&mut self) {
        let gpt_block_ptr = &self.reg_block_ptr;
        match gpt_block_ptr {
            GPTRegBlockPtr::GPT32RegBlock(ptr) => unsafe {
                (**ptr).gtuddtyc.modify(|r, w| w.bits(r.bits() | 1));
            },
            GPTRegBlockPtr::GPT16RegBlock(ptr) => unsafe {
                (**ptr).gtuddtyc.modify(|r, w| w.bits(r.bits() | 1));
            },
        }
    }
    /// Start the timer count
    pub fn start(&mut self) -> Result<(), TimerError> {
        if self.is_running {
            return Err(TimerError::TimerAlreadyRunning);
        }
        if self.gtssr == GPTSourceT::SOFTWARE {
            let gpt_block_ptr = &self.reg_block_ptr;
            match gpt_block_ptr {
                GPTRegBlockPtr::GPT32RegBlock(ptr) => unsafe {
                    (**ptr)
                        .gtstr
                        .modify(|r, w| w.bits(r.bits() | 1 << &self.channel.0));
                },
                GPTRegBlockPtr::GPT16RegBlock(ptr) => unsafe {
                    (**ptr)
                        .gtstr
                        .modify(|r, w| w.bits(r.bits() | 1 << &self.channel.0));
                },
            }
            self.is_running = true;
            self.is_stopped = false;
            Ok(())
        } else {
            defmt::todo!(" Only Software GTSSR supported for now")
        }
    }
    /// Stop the timer if it's running
    pub fn stop(&mut self) {
        if self.is_stopped {
            return;
        }
        // TODO: Lol this only works if the gtpsr source is GPT_SOURCE_SOFTWARE
        if self.gtpsr == GPTSourceT::SOFTWARE {
            let gpt_block_ptr = &self.reg_block_ptr;
            match gpt_block_ptr {
                GPTRegBlockPtr::GPT32RegBlock(ptr) => unsafe {
                    (**ptr)
                        .gtstp
                        .modify(|r, w| w.bits(r.bits() | 1 << &self.channel.0));
                },
                GPTRegBlockPtr::GPT16RegBlock(ptr) => unsafe {
                    (**ptr)
                        .gtstp
                        .modify(|r, w| w.bits(r.bits() | 1 << &self.channel.0));
                },
            }
            self.is_running = false;
            self.is_stopped = true;
        } else {
            defmt::todo!("Only Software GTPSR support for now")
        }
    }
    pub fn clear(&mut self) -> Result<(), TimerError> {
        // Timer must be stopped before it can be cleared
        if self.is_running() {
            return Err(TimerError::TimerAlreadyRunning);
        }
        self.set_count(0);
        Ok(())
    }
    pub fn is_stopped(&self) -> bool {
        self.is_stopped
    }
    pub fn is_running(&self) -> bool {
        self.is_running
    }
    pub fn get_timer_type(&self) -> TimerT {
        self.timer_type
    }
    pub fn set_frequency(&mut self, freq_hz: f32) -> Result<(), TimerError> {
        let max_count = match self.timer_type {
            TimerT::GPT_16_Timer => u16::MAX as u32,
            TimerT::GPT_32_Timer => u32::MAX,
        };
        // This also should fail if freq_hz is "too small" as really anything
        // slower than a couple of seconds isn't valid for the GPT clocks
        // Really slow stuff needs the AGT or some more complex chained overflows.
        if freq_hz <= 0.0 {
            return Err(TimerError::InvalidFrequencySetting);
        }
        self.set_period_counts(freq_hz, max_count)
    }
    fn set_period_counts(&mut self, period: f32, max: u32) -> Result<(), TimerError> {
        // TODO: This should be read from somewhere (chip? Global?) Not hardcoded.
        let base_freq_hz = 48_000_000; // 48Mhz base oscillator
        if period * base_freq_hz as f32 > max as f32 {
            self.period_counts = (period * base_freq_hz as f32) as u32;
            self.prescaler = TimerPrescalerSelect::PCLKD_1;
        } else if period * base_freq_hz as f32 / 4.0 > max as f32 {
            self.period_counts = (period * base_freq_hz as f32 / 4.0) as u32;
            self.prescaler = TimerPrescalerSelect::PCLKD_4;
        } else if period * base_freq_hz as f32 / 16.0 > max as f32 {
            self.period_counts = (period * base_freq_hz as f32 / 16.0) as u32;
            self.prescaler = TimerPrescalerSelect::PCLKD_16;
        } else if period * base_freq_hz as f32 / 256.0 > max as f32 {
            self.period_counts = (period * base_freq_hz as f32 / 256.0) as u32;
            self.prescaler = TimerPrescalerSelect::PCLKD_256;
        } else if period * base_freq_hz as f32 / 1024.0 > max as f32 {
            self.period_counts = (period * base_freq_hz as f32 / 1024.0) as u32;
            self.prescaler = TimerPrescalerSelect::PCLKD_1024;
        } else {
            return Err(TimerError::InvalidFrequencySetting);
        }
        Ok(())
    }

    /// If you try to set a 32 bit value for a 16 bit timer, data will be silently lost
    /// Unsafe, don't write dumb stuff to the count register. Timer must be stopped to set count
    pub fn set_count(&mut self, counts: u32) {
        // Since this is a pub function, I need a guard so we don't write to in-use registers
        // I don't want to return a result since this also runs in the Drop trait.
        match self.timer_type {
            TimerT::GPT_32_Timer => self.set_count_32(counts),
            TimerT::GPT_16_Timer => self.set_count_16(counts as u16), // this loses data
        }
    }
    fn set_count_32(&mut self, counts: u32) {
        let gpt_block_ptr = &self.reg_block_ptr;
        match gpt_block_ptr {
            GPTRegBlockPtr::GPT32RegBlock(ptr) => unsafe {
                (**ptr).gtcnt.write(|w| w.gtcnt().bits(counts));
            },
            GPTRegBlockPtr::GPT16RegBlock(_) => {
                defmt::unreachable!("Can't have a 16 bit reg block on a 32 bit timer")
            }
        }
    }
    fn set_count_16(&mut self, counts: u16) {
        let gpt_block_ptr = &self.reg_block_ptr;
        match gpt_block_ptr {
            GPTRegBlockPtr::GPT32RegBlock(_) => {
                defmt::unreachable!("Can't have a 32 bit reg block on a 16 bit timer")
            }
            GPTRegBlockPtr::GPT16RegBlock(ptr) => unsafe {
                (**ptr).gtcnt.write(|w| w.gtcnt().bits(counts as u32));
            },
        }
    }
}

/// I'm not sure if this is the best way to do a destructor. I'm trying to follow
/// RAII principles where this timer can only exist if you have a valid channel.
/// You can have as many timer configs as you want but if you claim a timer you
/// have ownership of that timer until it goes out of scope.
impl Drop for GPTimer {
    fn drop(&mut self) {
        self.stop();
        // Don't use clear b/c we want no errors in our drop code?
        self.set_count(0);
        let bit = 1 << self.channel.0;
        GPT_USED_CHANNEL.fetch_and(!bit, Ordering::Release);
    }
}
