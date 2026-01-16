use crate::hal::gpio::*;

pub type P200<Input> = Pin<2, 0, Input>;
// pub type P201<Input> = Pin<2, 1, Input>;
pub type P204<Input> = Pin<2, 4, Input>;
pub type P205<Input> = Pin<2, 5, Input>;
pub type P206<Input> = Pin<2, 6, Input>;
pub type P212<Input> = Pin<2, 12, Input>;
pub type P213<Input> = Pin<2, 13, Input>;
pub type P214<Input> = Pin<2, 14, Input>;
pub type P215<Input> = Pin<2, 15, Input>;

pub struct Port2Pins {
    pub p200: P200<Input>,
    // pub p201: P201<Input>,
    pub p204: P204<Input>,
    pub p205: P205<Input>,
    pub p206: P206<Input>,
    pub p212: P212<Input>,
    pub p213: P213<Input>,
    pub p214: P214<Input>,
    pub p215: P215<Input>,
}

impl GpioExt for crate::pac::PORT2 {
    type Parts = Port2Pins;
    fn split(self) -> Port2Pins {
        Port2Pins {
            p200: P200::new(unsafe { (*PMNPFS_BLOCK_BASE).p200pfs() }),
            // p201: P201::new(unsafe { (*PMNPFS_BLOCK_BASE).p201pfs() }),
            p204: P204::new(unsafe { (*PMNPFS_BLOCK_BASE).p204pfs() }),
            p205: P205::new(unsafe { (*PMNPFS_BLOCK_BASE).p205pfs() }),
            p206: P206::new(unsafe { (*PMNPFS_BLOCK_BASE).p206pfs() }),
            p212: P212::new(unsafe { (*PMNPFS_BLOCK_BASE).p212pfs() }),
            p213: P213::new(unsafe { (*PMNPFS_BLOCK_BASE).p213pfs() }),
            p214: P214::new(unsafe { (*PMNPFS_BLOCK_BASE).p214pfs() }),
            p215: P215::new(unsafe { (*PMNPFS_BLOCK_BASE).p215pfs() }),
        }
    }
}
