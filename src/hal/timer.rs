// This eventually needs to be something where you request a timer and if there
// is one available you can get it, if not you get an error or false or something.
// Arduino does this basically by keeping a list of how many timers are currently
// running and passing the first available channel.

use embedded_hal::digital::PinState;
use ra4m1::{MSTP, gpt320::RegisterBlock};

/// Sources can be used to start the timer, stop the timer, count up, or count down. These enumerations represent a bitmask. Multiple sources can be ORed together.
#[allow(non_camel_case_types)]
#[repr(u32)]
pub enum GPTSourceT {
    // No active event sources.
    GPT_SOURCE_NONE = 0,
    // Action performed on GTETRGA rising edge.
    GPT_SOURCE_GTETRGA_RISING = (1 << 0),
    // Action performed on GTETRGA falling edge.
    GPT_SOURCE_GTETRGA_FALLING = (1 << 1),
    // Action performed on GTETRGB rising edge.
    GPT_SOURCE_GTETRGB_RISING = (1 << 2),
    // Action performed on GTETRGB falling edge.
    GPT_SOURCE_GTETRGB_FALLING = (1 << 3),
    // // Action performed on GTETRGC rising edge.
    // GPT_SOURCE_GTETRGC_RISING = (1 << 4),
    // // Action performed on GTETRGC falling edge.
    // GPT_SOURCE_GTETRGC_FALLING = (1 << 5),
    // // Action performed on GTETRGB rising edge.
    // GPT_SOURCE_GTETRGD_RISING = (1 << 6),
    // // Action performed on GTETRGB falling edge.
    // GPT_SOURCE_GTETRGD_FALLING = (1 << 7),
    // Action performed when GTIOCA input rises while GTIOCB is low.
    GPT_SOURCE_GTIOCA_RISING_WHILE_GTIOCB_LOW = (1 << 8),
    // Action performed when GTIOCA input rises while GTIOCB is high.
    GPT_SOURCE_GTIOCA_RISING_WHILE_GTIOCB_HIGH = (1 << 9),
    // Action performed when GTIOCA input falls while GTIOCB is low.
    GPT_SOURCE_GTIOCA_FALLING_WHILE_GTIOCB_LOW = (1 << 10),
    // Action performed when GTIOCA input falls while GTIOCB is high.
    GPT_SOURCE_GTIOCA_FALLING_WHILE_GTIOCB_HIGH = (1 << 11),
    // Action performed when GTIOCB input rises while GTIOCA is low.
    GPT_SOURCE_GTIOCB_RISING_WHILE_GTIOCA_LOW = (1 << 12),
    // Action performed when GTIOCB input rises while GTIOCA is high.
    GPT_SOURCE_GTIOCB_RISING_WHILE_GTIOCA_HIGH = (1 << 13),
    // Action performed when GTIOCB input falls while GTIOCA is low.
    GPT_SOURCE_GTIOCB_FALLING_WHILE_GTIOCA_LOW = (1 << 14),
    // Action performed when GTIOCB input falls while GTIOCA is high.
    GPT_SOURCE_GTIOCB_FALLING_WHILE_GTIOCA_HIGH = (1 << 15),
    // Action performed on ELC GPTA event.
    GPT_SOURCE_GPT_A = (1 << 16),
    // Action performed on ELC GPTB event.
    GPT_SOURCE_GPT_B = (1 << 17),
    // Action performed on ELC GPTC event.
    GPT_SOURCE_GPT_C = (1 << 18),
    // Action performed on ELC GPTD event.
    GPT_SOURCE_GPT_D = (1 << 19),
    // Action performed on ELC GPTE event.
    GPT_SOURCE_GPT_E = (1 << 20),
    // Action performed on ELC GPTF event.
    GPT_SOURCE_GPT_F = (1 << 21),
    // Action performed on ELC GPTG event.
    GPT_SOURCE_GPT_G = (1 << 22),
    // Action performed on ELC GPTH event.
    GPT_SOURCE_GPT_H = (1 << 23),
    // Action performed on Software Source event.
    // Enables the GTSTR, GTSTP, and GTCLR registers when used appropriately
    GPT_SOURCE_SOFTWARE = (1 << 31),
}

#[allow(non_camel_case_types)]
#[repr(u8)]
pub enum GPTCaptureFilter {
    GPT_CAPTURE_FILTER_NONE = 0,         //< None - no filtering
    GPT_CAPTURE_FILTER_PCLKD_DIV_1 = 1,  //< PCLK/1 - fast sampling
    GPT_CAPTURE_FILTER_PCLKD_DIV_4 = 3,  //< PCLK/4
    GPT_CAPTURE_FILTER_PCLKD_DIV_16 = 5, //< PCLK/16
    GPT_CAPTURE_FILTER_PCLKD_DIV_64 = 7, //< PCLK/64 - slow sampling
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
pub enum TimerModeT {
    /// Timer restarts after period elapses.
    TIMER_MODE_PERIODIC = 0,
    /// Timer stops after period elapses.
    TIMER_MODE_ONE_SHOT = 1,
    // /// Timer generates saw-wave PWM output.
    // TIMER_MODE_PWM = 2,
    // /// Saw-wave one-shot pulse mode (fixed buffer operation).
    // TIMER_MODE_ONE_SHOT_PULSE = 3,
    /// Timer generates symmetric triangle-wave PWM output.
    TIMER_MODE_TRIANGLE_WAVE_SYMMETRIC_PWM = 4,
    /// Timer generates asymmetric triangle-wave PWM output.
    TIMER_MODE_TRIANGLE_WAVE_ASYMMETRIC_PWM = 5,
    /// Timer generates Asymmetric Triangle-wave PWM output. In PWM mode 3, the duty cycle does
    ///not need to be updated at each tough/crest interrupt. Instead, the trough and crest duty cycle values can be
    /// set once and only need to be updated when the application needs to change the duty cycle.
    timer_mode_triangle_wave_asymmetric_pwm_MODE3 = 6,
}

#[allow(non_camel_case_types)]
#[repr(u8)]
pub enum TimerSourceDivT {
    /// timer clock source divided by 1
    TIMER_SOURCE_DIV_1 = 0,
    /// Timer clock source divided by 2
    TIMER_SOURCE_DIV_2 = 1,
    /// Timer clock source divided by 4
    TIMER_SOURCE_DIV_4 = 2,
    /// Timer clock source divided by 8
    TIMER_SOURCE_DIV_8 = 3,
    /// Timer clock source divided by 16
    TIMER_SOURCE_DIV_16 = 4,
    /// Timer clock source divided by 32
    TIMER_SOURCE_DIV_32 = 5,
    /// Timer clock source divided by 64
    TIMER_SOURCE_DIV_64 = 6,
    /// Timer clock source divided by 128
    TIMER_SOURCE_DIV_128 = 7,
    /// Timer clock source divided by 256
    TIMER_SOURCE_DIV_256 = 8,
    /// Timer clock source divided by 512
    TIMER_SOURCE_DIV_512 = 9,
    /// Timer clock source divided by 1024
    TIMER_SOURCE_DIV_1024 = 10,
}

#[allow(non_camel_case_types)]
#[repr(i8)]
pub enum IRQn_Type {
    FSP_INVALID_VECTOR = -33,    // invalid vector for inits
    Reset_IRQn = -15,            //  1 Reset Vector invoked on Power up and warm reset
    NonMaskableInt_IRQn = -14,   //  2 Non maskable Interrupt cannot be stopped or preempted
    HardFault_IRQn = -13,        //  3 Hard Fault all classes of Fault
    MemoryManagement_IRQn = -12, //  4 Memory Management MPU mismatch, including Access Violation and No Match
    BusFault_IRQn = -11, //  5 Bus Fault Pre-Fetch-, Memory Access, other address/memory Fault
    UsageFault_IRQn = -10, //  6 Usage Fault i.e. Undef Instruction, Illegal State Transition
    SecureFault_IRQn = -9, //  7 Secure Fault Interrupt
    SVCall_IRQn = -5,    // 11 System Service Call via SVC instruction
    DebugMonitor_IRQn = -4, // 12 Debug Monitor
    PendSV_IRQn = -2,    // 14 Pendable request for system service
    SysTick_IRQn = -1,   // 15 System Tick Timer
    IIC1_RXI_IRQn = 0,   /* IIC1 RXI (Receive data full) */
    IIC1_TXI_IRQn = 1,   /* IIC1 TXI (Transmit data empty) */
    IIC1_TEI_IRQn = 2,   /* IIC1 TEI (Transmit end) */
    IIC1_ERI_IRQn = 3,   /* IIC1 ERI (Transfer error) */
    SPI1_RXI_IRQn = 4,   /* SPI1 RXI (Receive buffer full) */
    SPI1_TXI_IRQn = 5,   /* SPI1 TXI (Transmit buffer empty) */
    SPI1_TEI_IRQn = 6,   /* SPI1 TEI (Transmission complete event) */
    SPI1_ERI_IRQn = 7,   /* SPI1 ERI (Error) */
    ICU_IRQ0_IRQn = 8,   /* ICU IRQ0 (External pin interrupt 0) */
    ICU_IRQ1_IRQn = 9,   /* ICU IRQ1 (External pin interrupt 1) */
    USBFS_INT_IRQn = 10, /* USBFS INT (USBFS interrupt) */
    USBFS_RESUME_IRQn = 11, /* USBFS RESUME (USBFS resume interrupt) */
    USBFS_FIFO_0_IRQn = 12, /* USBFS FIFO 0 (DMA transfer request 0) */
    USBFS_FIFO_1_IRQn = 13, /* USBFS FIFO 1 (DMA transfer request 1) */
    RTC_ALARM_IRQn = 14, /* RTC ALARM (Alarm interrupt) */
    RTC_PERIOD_IRQn = 15, /* RTC PERIOD (Periodic interrupt) */
    RTC_CARRY_IRQn = 16, /* RTC CARRY (Carry interrupt) */
    AGT0_INT_IRQn = 17,  /* AGT0 INT (AGT interrupt) */
    SCI0_RXI_IRQn = 18,  /* SCI0 RXI (Receive data full) */
    SCI0_TXI_IRQn = 19,  /* SCI0 TXI (Transmit data empty) */
    SCI0_TEI_IRQn = 20,  /* SCI0 TEI (Transmit end) */
    SCI0_ERI_IRQn = 21,  /* SCI0 ERI (Receive error) */
    SCI1_RXI_IRQn = 22,  /* SCI1 RXI (Received data full) */
    SCI1_TXI_IRQn = 23,  /* SCI1 TXI (Transmit data empty) */
    SCI1_TEI_IRQn = 24,  /* SCI1 TEI (Transmit end) */
    SCI1_ERI_IRQn = 25,  /* SCI1 ERI (Receive error) */
    SCI2_TXI_IRQn = 26,  /* SCI2 TXI (Transmit data empty) */
    SCI2_TEI_IRQn = 27,  /* SCI2 TEI (Transmit end) */
    SCI2_RXI_IRQn = 28,  /* SCI2 RXI (Received data full) */
    SCI2_ERI_IRQn = 29,  /* SCI2 ERI (Receive error) */
    IIC0_RXI_IRQn = 30,  /* IIC0 RXI (Receive data full) */
    IIC0_TXI_IRQn = 31,  /* IIC0 TXI (Transmit data empty) */
}

/// This gets passed into the GPTimer creator to extend the default init
pub struct TimerCfg {
    pub mode: TimerModeT,
    pub period_counts: u32,
    pub source_div: TimerSourceDivT,
    pub duty_cycle_counts: u32,
    // Select the channel
    pub channel: Option<u8>,
    pub cycle_end_ipl: u8,
    pub cycle_end_irq: IRQn_Type,
    pub callback: Option<fn()>,
}

impl TimerCfg {
    pub fn new() -> Self {
        TimerCfg {
            mode: TimerModeT::TIMER_MODE_PERIODIC,
            period_counts: 0,
            source_div: TimerSourceDivT::TIMER_SOURCE_DIV_1,
            duty_cycle_counts: 0,
            channel: None,
            cycle_end_ipl: 0xFF,
            cycle_end_irq: IRQn_Type::FSP_INVALID_VECTOR,
            callback: None,
        }
    }
    pub fn do_callback(&self) {
        if let Some(cb) = self.callback {
            (cb)();
        }
    }
}
impl Default for TimerCfg {
    fn default() -> Self {
        TimerCfg::new()
    }
}

/// Just kinda ripping this from the FspTimer.h provided by Renesas
pub struct GPTimer {
    pub gtioca_output_enabled: bool,
    pub gtioca_stop_level: PinState,
    pub gtiocb_output_enabled: bool,
    pub gtiocb_stop_level: PinState,
    pub start_source: GPTSourceT,
    pub stop_source: GPTSourceT,
    pub clear_source: GPTSourceT,
    pub count_up_source: GPTSourceT,
    pub count_down_source: GPTSourceT,
    pub capture_a_source: GPTSourceT,
    pub capture_b_source: GPTSourceT,
    /// These are u8 vals, init at 0xFFu
    pub capture_a_ipl: u8,
    /// These are u8 vals, init at 0xFFu
    pub capture_b_ipl: u8,
    pub capture_a_irq: IRQn_Type,
    pub capture_b_irq: IRQn_Type,
    pub capture_filter_gtcioa: GPTCaptureFilter,
    pub capture_filter_gtciob: GPTCaptureFilter,
    pub gtior_setting: u8, // There is also a p_pwm_cfg inited to nullptr
}

impl GPTimer {
    fn new() -> Self {
        GPTimer {
            gtioca_output_enabled: false,
            gtioca_stop_level: PinState::Low,
            gtiocb_output_enabled: false,
            gtiocb_stop_level: PinState::Low,
            start_source: GPTSourceT::GPT_SOURCE_NONE,
            stop_source: GPTSourceT::GPT_SOURCE_NONE,
            clear_source: GPTSourceT::GPT_SOURCE_NONE,
            count_up_source: GPTSourceT::GPT_SOURCE_NONE,
            count_down_source: GPTSourceT::GPT_SOURCE_NONE,
            capture_a_source: GPTSourceT::GPT_SOURCE_NONE,
            capture_b_source: GPTSourceT::GPT_SOURCE_NONE,
            capture_a_ipl: 0xFF,
            capture_b_ipl: 0xFF,
            capture_a_irq: IRQn_Type::FSP_INVALID_VECTOR,
            capture_b_irq: IRQn_Type::FSP_INVALID_VECTOR,
            capture_filter_gtcioa: GPTCaptureFilter::GPT_CAPTURE_FILTER_NONE,
            capture_filter_gtciob: GPTCaptureFilter::GPT_CAPTURE_FILTER_NONE,
            gtior_setting: 0,
        }
    }
}

impl Default for GPTimer {
    fn default() -> Self {
        GPTimer::new()
    }
}

#[allow(non_camel_case_types)]
#[derive(PartialEq)]
pub enum TimerT {
    GPT_16_Timer,
    GPT_32_Timer,
    AGT_Timer,
}

#[derive(PartialEq)]
pub enum TimerAvail {
    TimerFree,
    TimerUsed,
}

pub const GPT_HOWMANY: usize = 8;
pub const GPT_32_HOWMANY: usize = 2;
pub const GPT_16_HOWMANY: usize = 6;
static mut GPT_USED_CHANNEL: [TimerAvail; GPT_HOWMANY] = [
    TimerAvail::TimerFree,
    TimerAvail::TimerFree,
    TimerAvail::TimerFree,
    TimerAvail::TimerFree,
    TimerAvail::TimerFree,
    TimerAvail::TimerFree,
    TimerAvail::TimerFree,
    TimerAvail::TimerFree,
];

const GPT_REG_BLOCK_PTRS: (
    *const ra4m1::gpt320::RegisterBlock,
    *const ra4m1::gpt320::RegisterBlock,
    *const ra4m1::gpt162::RegisterBlock,
    *const ra4m1::gpt162::RegisterBlock,
    *const ra4m1::gpt162::RegisterBlock,
    *const ra4m1::gpt162::RegisterBlock,
    *const ra4m1::gpt162::RegisterBlock,
    *const ra4m1::gpt162::RegisterBlock,
) = (
    ra4m1::GPT320::PTR,
    ra4m1::GPT321::PTR,
    ra4m1::GPT162::PTR,
    ra4m1::GPT163::PTR,
    ra4m1::GPT164::PTR,
    ra4m1::GPT165::PTR,
    ra4m1::GPT166::PTR,
    ra4m1::GPT167::PTR,
);

/// I'm still not sure how I want to do all of this
pub fn setup_timers() {
    // Enable timer module
    let mstp_reg = ra4m1::MSTP::PTR;
}

#[allow(static_mut_refs)]
/// Unsafe as long as we have to hold the used timers in the global state
pub fn get_available_timer_channel(t: TimerT) -> Option<u8> {
    let mut avail_idx = None;
    match t {
        TimerT::GPT_32_Timer => unsafe {
            for (i, timer) in GPT_USED_CHANNEL.iter().enumerate().take(GPT_32_HOWMANY) {
                if *timer == TimerAvail::TimerFree {
                    avail_idx = Some(i as u8);
                    break;
                }
            }
        },
        TimerT::GPT_16_Timer => unsafe {
            for (i, timer) in GPT_USED_CHANNEL.iter().enumerate().skip(GPT_32_HOWMANY) {
                if *timer == TimerAvail::TimerFree {
                    avail_idx = Some(i as u8);
                    break;
                }
            }
        },
        // I know channel 0 is reserved in the bootloader for some stuff
        TimerT::AGT_Timer => {}
    }
    avail_idx
}

pub struct Timer {
    pub gpt_timer: GPTimer,
    pub cfg: TimerCfg,
    pub timer_type: Option<TimerT>,
}

impl Timer {
    #[allow(clippy::too_many_arguments)]
    pub fn begin(
        &mut self,
        mode: TimerModeT,
        tp: TimerT,
        channel: u8,
        period_counts: u32,
        pulse_counts: u32,
        sd: TimerSourceDivT,
        callback: fn(),
    ) -> bool {
        let mut init_ok = false;
        let mut timer_cfg = TimerCfg {
            mode,
            source_div: sd,
            period_counts,
            duty_cycle_counts: pulse_counts,
            callback: Some(callback),
            ..Default::default()
        };

        if tp == TimerT::GPT_16_Timer {
            self.gpt_timer = GPTimer::new();
            self.timer_type = Some(tp);

            if (channel as usize) < GPT_HOWMANY {
                init_ok = true;
                timer_cfg.channel = Some(channel);
                self.cfg = timer_cfg;
            }
        }
        init_ok
    }

    /// Don't really know what this is for yet, the abstraction is too deep.
    /// Call it with priority = 12 for now.
    pub fn setup_overflow_irq(&mut self, priority: u8) {
        self.cfg.cycle_end_ipl = priority;
    }

    /// This should try and open the timer?
    pub fn open(&self) {
        if let Some(timer_type) = &self.timer_type {
            // Open the timer
        }
    }
}
