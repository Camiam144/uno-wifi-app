use crate::hal::gpio::*;

pub struct Port0;

pub struct Parts {
    pub p000: P000<Input>,
    pub p001: P001<Input>,
    pub p002: P002<Input>,
    pub p003: P003<Input>,
    pub p004: P004<Input>,
    pub p010: P010<Input>,
    pub p011: P011<Input>,
    pub p012: P012<Input>,
    pub p013: P013<Input>,
    pub p014: P014<Input>,
    pub p015: P015<Input>,
}

impl GpioExt for crate::pac::PORT0 {
    type Parts = Parts;
    fn split(self) -> Parts {
        Parts {
            p000: P000::new(unsafe { (*PMNPFS_BLOCK_BASE).p000pfs() }),
            p001: P001::new(unsafe { (*PMNPFS_BLOCK_BASE).p001pfs() }),
            p002: P002::new(unsafe { (*PMNPFS_BLOCK_BASE).p002pfs() }),
            p003: P003::new(unsafe { (*PMNPFS_BLOCK_BASE).p003pfs() }),
            p004: P004::new(unsafe { (*PMNPFS_BLOCK_BASE).p004pfs() }),
            p010: P010::new(unsafe { (*PMNPFS_BLOCK_BASE).p010pfs() }),
            p011: P011::new(unsafe { (*PMNPFS_BLOCK_BASE).p011pfs() }),
            p012: P012::new(unsafe { (*PMNPFS_BLOCK_BASE).p012pfs() }),
            p013: P013::new(unsafe { (*PMNPFS_BLOCK_BASE).p013pfs() }),
            p014: P014::new(unsafe { (*PMNPFS_BLOCK_BASE).p014pfs() }),
            p015: P015::new(unsafe { (*PMNPFS_BLOCK_BASE).p015pfs() }),
        }
    }
}

pub type P000<Input> = Pin<0, 0, Input>;
pub type P001<Input> = Pin<0, 1, Input>;
pub type P002<Input> = Pin<0, 2, Input>;
pub type P003<Input> = Pin<0, 3, Input>;
pub type P004<Input> = Pin<0, 4, Input>;
pub type P010<Input> = Pin<0, 10, Input>;
pub type P011<Input> = Pin<0, 11, Input>;
pub type P012<Input> = Pin<0, 12, Input>;
pub type P013<Input> = Pin<0, 13, Input>;
pub type P014<Input> = Pin<0, 14, Input>;
pub type P015<Input> = Pin<0, 15, Input>;
