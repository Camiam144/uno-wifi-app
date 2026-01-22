use crate::hal::gpio::Input;
use crate::hal::gpio::Pin;
use crate::hal::gpio::erased::DynamicPinErased;
use crate::hal::timer::CountDir;
use crate::hal::timer::GPTSourceT;
use crate::hal::timer::GPTimer;
use crate::hal::timer::NotSet;
use crate::hal::timer::Periodic;
use crate::hal::timer::{Configured, PeriodicCfg, TimerInstance, Unconfigured};
use crate::millis_timer::millis;
use core::cell::{Cell, RefCell};
use cortex_m::interrupt::{Mutex, free};
use ra4m1::interrupt;

// TODO: Figure out the graphics library

pub const NUM_LEDS: u8 = 96;

/// These are all of the pin combinations to turn on and off each LED. The indices
/// start at 0 in the top left. Each index is given as [HI, LO] to turn on that
/// specific LED.
const PINS: [[u8; 2]; 96] = [
    [7, 3],
    [3, 7],
    [7, 4],
    [4, 7],
    [3, 4],
    [4, 3],
    [7, 8],
    [8, 7],
    [3, 8],
    [8, 3],
    [4, 8],
    [8, 4],
    [7, 0],
    [0, 7],
    [3, 0],
    [0, 3],
    [4, 0],
    [0, 4],
    [8, 0],
    [0, 8],
    [7, 6],
    [6, 7],
    [3, 6],
    [6, 3],
    [4, 6],
    [6, 4],
    [8, 6],
    [6, 8],
    [0, 6],
    [6, 0],
    [7, 5],
    [5, 7],
    [3, 5],
    [5, 3],
    [4, 5],
    [5, 4],
    [8, 5],
    [5, 8],
    [0, 5],
    [5, 0],
    [6, 5],
    [5, 6],
    [7, 1],
    [1, 7],
    [3, 1],
    [1, 3],
    [4, 1],
    [1, 4],
    [8, 1],
    [1, 8],
    [0, 1],
    [1, 0],
    [6, 1],
    [1, 6],
    [5, 1],
    [1, 5],
    [7, 2],
    [2, 7],
    [3, 2],
    [2, 3],
    [4, 2],
    [2, 4],
    [8, 2],
    [2, 8],
    [0, 2],
    [2, 0],
    [6, 2],
    [2, 6],
    [5, 2],
    [2, 5],
    [1, 2],
    [2, 1],
    [7, 10],
    [10, 7],
    [3, 10],
    [10, 3],
    [4, 10],
    [10, 4],
    [8, 10],
    [10, 8],
    [0, 10],
    [10, 0],
    [6, 10],
    [10, 6],
    [5, 10],
    [10, 5],
    [1, 10],
    [10, 1],
    [2, 10],
    [10, 2],
    [7, 9],
    [9, 7],
    [3, 9],
    [9, 3],
    [4, 9],
    [9, 4],
];

const LEDPORT0BITMASK: u16 = (1 << 3) | (1 << 4) | (1 << 11) | (1 << 12) | (1 << 13) | (1 << 15);
const LEDPORT2BITMASK: u16 = (1 << 4) | (1 << 5) | (1 << 6) | (1 << 12) | (1 << 13);

// This is super ugly, but it will work for now
// TODO: Make this less ugly, can I get an array of each individual pin block?
// const PIN_PFS_BASE: *const ra4m1::pfs::RegisterBlock = ra4m1::PFS::PTR;
const PIN_PFS_BASE: usize = 0x4004_0800;
const PIN_PFS_OFFSET: usize = 0x04;
const PIN_OFFSETS: [usize; 11] = [
    PIN_PFS_OFFSET * 3,
    PIN_PFS_OFFSET * 4,
    PIN_PFS_OFFSET * 11,
    PIN_PFS_OFFSET * 12,
    PIN_PFS_OFFSET * 13,
    PIN_PFS_OFFSET * 15,
    PIN_PFS_OFFSET * (32 + 4),
    PIN_PFS_OFFSET * (32 + 5),
    PIN_PFS_OFFSET * (32 + 6),
    PIN_PFS_OFFSET * (32 + 12),
    PIN_PFS_OFFSET * (32 + 13),
];

/// Turn the whole grid off, then turn on the specific index if called for
/// idx must be 0 <= idx <= 95 (for our 96 grid LED)
fn turn_led(idx: usize, on: bool) {
    // Unsafe write to set the whole LED screen to low
    // *TECHNICALLY* we should be doing this pin by pin but this is probably faster
    // Should *TECHNICALLY* be safe enough because we own all of the pins
    let p1 = ra4m1::PORT0::PTR;
    let p2 = ra4m1::PORT2::PTR;
    unsafe {
        (*p1)
            .pcntr1()
            .modify(|r, w| w.pdr().bits(r.pdr().bits() & !LEDPORT0BITMASK));
        (*p2)
            .pcntr1()
            .modify(|r, w| w.pdr().bits(r.pdr().bits() & !LEDPORT2BITMASK));
    }

    // Since the pins are owned by the struct, they can't come out until the
    // struct goes out of scope, at which point we should reset them in the
    // struct's drop function?
    let [hi, lo] = PINS[idx];
    if on {
        unsafe {
            let hi_addr = PIN_PFS_BASE + (PIN_OFFSETS[hi as usize]);
            let lo_addr = PIN_PFS_BASE + (PIN_OFFSETS[lo as usize]);
            let hi_block = &*(hi_addr as *const ra4m1::pfs::P000PFS);
            hi_block.write(|w| w.bits(1 | 1 << 2));
            let lo_block = &*(lo_addr as *const ra4m1::pfs::P000PFS);
            lo_block.write(|w| w.bits(1 << 2));
        }
    }
}

#[derive(defmt::Format)]
pub enum AnimationData {
    Animation(&'static [[u32; 4]]),
    SingleFrame([u32; 4]),
}

impl AnimationData {
    fn get_frame(&self, idx: usize) -> Option<&[u32; 4]> {
        match self {
            AnimationData::Animation(frames) => frames.get(idx),
            AnimationData::SingleFrame(frame) if idx == 0 => Some(frame),
            _ => None,
        }
    }

    fn len(&self) -> usize {
        match self {
            AnimationData::Animation(frames) => frames.len(),
            AnimationData::SingleFrame(_) => 1,
        }
    }
}

// Globals so the interrupt handler can... well... handle things
#[derive(defmt::Format)]
struct DisplayState {
    current_pin_idx: usize,
    framebuffer: [u32; 3],
    animation: Option<AnimationData>, // Only works with compiled animations
    current_frame_idx: usize,
    curr_frame_duration: u32,
    frame_start_millis: u32,
    should_loop: bool,
}
impl DisplayState {
    fn new() -> Self {
        Self {
            current_pin_idx: 0,
            framebuffer: [0, 0, 0],
            animation: None,
            current_frame_idx: 0,
            curr_frame_duration: 0,
            frame_start_millis: 0,
            should_loop: false,
        }
    }
}

static DISPLAY_STATE: Mutex<RefCell<DisplayState>> = Mutex::new(RefCell::new(DisplayState {
    current_pin_idx: 0,
    framebuffer: [0, 0, 0],
    animation: None,
    current_frame_idx: 0,
    curr_frame_duration: 0,
    frame_start_millis: 0,
    should_loop: false,
}));

// Hold the timer channel so we can clear the flag
static TIMER_CHANNEL: Mutex<Cell<Option<u8>>> = Mutex::new(Cell::new(None));

/// The 12x8 LED matrix on the board. This struct will hold all of the logic
/// to get the thing to work.
pub struct LEDMatrix<T: TimerInstance> {
    /// These are the dynamic erased pins for internal use. Once the pins go in
    /// to this driver, they never come out.
    _dynpins: [DynamicPinErased; 11],
    // The timer shouldn't be optional, this struct should own a timer.
    timer: GPTimer<T, Configured, Periodic>,
}

impl<T: TimerInstance> LEDMatrix<T> {
    /// Initialize the matrix with the proper set of pins, all set to low.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        p003: Pin<0, 3, Input>,
        p004: Pin<0, 4, Input>,
        p011: Pin<0, 11, Input>,
        p012: Pin<0, 12, Input>,
        p013: Pin<0, 13, Input>,
        p015: Pin<0, 15, Input>,
        p204: Pin<2, 4, Input>,
        p205: Pin<2, 5, Input>,
        p206: Pin<2, 6, Input>,
        p212: Pin<2, 12, Input>,
        p213: Pin<2, 13, Input>,
        timer: GPTimer<T, Unconfigured, NotSet>,
    ) -> Self {
        let dynpins: [DynamicPinErased; 11] = [
            p003.into_fully_erased_dynamic(),
            p004.into_fully_erased_dynamic(),
            p011.into_fully_erased_dynamic(),
            p012.into_fully_erased_dynamic(),
            p013.into_fully_erased_dynamic(),
            p015.into_fully_erased_dynamic(),
            p204.into_fully_erased_dynamic(),
            p205.into_fully_erased_dynamic(),
            p206.into_fully_erased_dynamic(),
            p212.into_fully_erased_dynamic(),
            p213.into_fully_erased_dynamic(),
        ];

        // Load the pins into the global lookup for the interrupt
        // free(|cs| PIN_LOOKUP.borrow(cs).set(Some(dynpins)));

        // Set up the timer
        let ledtimercfg: PeriodicCfg = PeriodicCfg {
            gtssr: GPTSourceT::SOFTWARE,
            gtpsr: GPTSourceT::SOFTWARE,
            gtcsr: GPTSourceT::SOFTWARE,
            count_dir: CountDir::Up,
            freq_hz: 9600.0,
        };
        let ledtimer = timer.into_periodic().configure(ledtimercfg);

        // Load timer channel into global space
        free(|cs| TIMER_CHANNEL.borrow(cs).set(Some(ledtimer.get_channel())));

        // unmask the interrupt
        unsafe {
            ra4m1::NVIC::unmask(ra4m1::interrupt::IEL9);
        }

        // Attach timer overflow to interrupt.
        let p = unsafe { ra4m1::Peripherals::steal() };
        // TODO: get hardcoded 9 from the trait when it's done
        // 0x57 is the start of the GPTimer flag block, there are 8 flags per timer
        p.ICU.ielsr[9].write(|w| unsafe { w.iels().bits(0x57 + 8 * ledtimer.get_channel() + 6) });

        // defmt::println!("Channel is {}", ledtimer.get_channel());
        // defmt::println!(
        //     "Linked event is {:03x}",
        //     p.ICU.ielsr[9].read().iels().bits()
        // );
        // Initialize the global state
        free(|cs| {
            *DISPLAY_STATE.borrow(cs).borrow_mut() = DisplayState::new();
        });

        // Start the timer
        ledtimer.start();

        Self {
            _dynpins: dynpins,
            timer: ledtimer,
        }
    }

    pub fn stop(&self) {
        self.timer.stop();
    }

    pub fn start(&self) {
        self.timer.start();
    }

    pub fn on(&mut self, led_idx: usize) {
        turn_led(led_idx, true);
    }
    pub fn off(&mut self, led_idx: usize) {
        turn_led(led_idx, false);
    }
    pub fn clear(&mut self) {
        self.load_frame([0, 0, 0]);
    }

    /// Load a single frame into the frame storage, I can't think of an easier
    /// way right now
    pub fn load_frame(&mut self, frame: [u32; 3]) {
        free(|cs| {
            let mut state = DISPLAY_STATE.borrow(cs).borrow_mut();

            state.animation = Some(AnimationData::SingleFrame([
                frame[0], frame[1], frame[2], 0,
            ]));
            state.current_frame_idx = 0;
            state.curr_frame_duration = 0;
            state.frame_start_millis = millis();
            state.should_loop = false;
        });
        next();
    }

    /// Arduino defines this function that just loads the wrapper. Idk why yet.
    /// maybe something with the text animation library I'm not sure.
    /// Currently this only works for animations with 'static lifetimes
    pub fn load_sequence(&mut self, frames: &'static [[u32; 4]], should_loop: bool) {
        // Load the frames into the global state
        free(|cs| {
            let mut state = DISPLAY_STATE.borrow(cs).borrow_mut();
            state.animation = Some(AnimationData::Animation(frames));
            state.current_frame_idx = 0;
            state.curr_frame_duration = if !frames.is_empty() { frames[0][3] } else { 0 };
            state.frame_start_millis = millis();
            state.should_loop = should_loop;
        });

        next();
    }
}
/// Shamelessly stolen from the arduino code, need to learn why this bit twiddling works
fn reverse(x: u32) -> u32 {
    let mut x = ((x >> 1) & 0x55555555_u32) | ((x & 0x55555555_u32) << 1);
    x = ((x >> 2) & 0x33333333_u32) | ((x & 0x33333333_u32) << 2);
    x = ((x >> 4) & 0x0f0f0f0f_u32) | ((x & 0x0f0f0f0f_u32) << 4);
    x = ((x >> 8) & 0x00ff00ff_u32) | ((x & 0x00ff00ff_u32) << 8);
    x = ((x >> 16) & 0x0000ffff_u32) | ((x & 0x0000ffff_u32) << 16);
    x
}

/// This function loads the next frame into the framebuffer for display
fn next() {
    free(|cs| {
        let mut state = DISPLAY_STATE.borrow(cs).borrow_mut();

        // Make sure we have something to do
        let animation = match &state.animation {
            Some(anim) => anim,
            None => return,
        };
        let num_frames = animation.len();
        let frame = match animation.get_frame(state.current_frame_idx) {
            // Get an owned frame value to pass to the framebuffer
            Some(f) => *f,
            None => return,
        };

        // Load the frame into the framebuffer
        state.framebuffer = [reverse(frame[0]), reverse(frame[1]), reverse(frame[2])];
        state.curr_frame_duration = frame[3];

        state.current_frame_idx = (state.current_frame_idx + 1) % num_frames;

        // Should we loop?
        if state.current_frame_idx == 0 && !state.should_loop {
            state.curr_frame_duration = 0;
        }
    })
}

/// This is the function that drives the whole display. Make it not suck?
/// I need to pass this in somehow, or figure out how the registering works
/// in main.rs so I can pass in the correct interrupt. What if I don't want
/// to use 9 by default?
#[interrupt]
unsafe fn IEL9() {
    let p = unsafe { ra4m1::Peripherals::steal() };

    // Clear ICU flag
    p.ICU.ielsr[9].modify(|_, w| w.ir().clear_bit());
    // Clear timer overflow flag (this is so gross ngl)
    let channel = free(|cs| TIMER_CHANNEL.borrow(cs).get());
    if let Some(ch) = channel {
        const GPTIMERBASE: usize = 0x4007_803C;
        const GPTIMEROFFSET: usize = 0x100;
        let timer_addr = GPTIMERBASE + (GPTIMEROFFSET * ch as usize);

        match ch {
            0..=1 => {
                let block = unsafe { &*(timer_addr as *const ra4m1::gpt320::RegisterBlock) };
                block.gtst.modify(|_, w| w.tcfpo().clear_bit());
            }
            2..=7 => {
                let block = unsafe { &*(timer_addr as *const ra4m1::gpt162::RegisterBlock) };
                block.gtst.modify(|_, w| w.tcfpo().clear_bit());
            }
            _ => {
                unreachable!();
            }
        }
    }

    // Figure out the animation?
    free(|cs| {
        let mut state = DISPLAY_STATE.borrow(cs).borrow_mut();

        let curr_idx = state.current_pin_idx;
        let word = curr_idx >> 5;
        let bit = curr_idx & 31;
        let is_on = (state.framebuffer[word] >> bit) & 1 == 1;

        turn_led(curr_idx, is_on);
        state.current_pin_idx = (curr_idx + 1) % NUM_LEDS as usize;

        // Time to advance?
        if state.curr_frame_duration != 0
            && millis() - state.frame_start_millis > state.curr_frame_duration
        {
            state.frame_start_millis = millis();
            // Claude says to release the borrow here, but was wrong about other references
            drop(state);
            next();
        }
    })
}
