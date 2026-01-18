use crate::gpio_port;

gpio_port!(
    Port0,
    PORT0,
    port0,
    0,
    [
        (P000, p000, 0, p000pfs),
        (P001, p001, 1, p001pfs),
        (P002, p002, 2, p002pfs),
        (P003, p003, 3, p003pfs),
        (P004, p004, 4, p004pfs),
        (P010, p010, 10, p010pfs),
        (P011, p011, 11, p011pfs),
        (P012, p012, 12, p012pfs),
        (P013, p013, 13, p013pfs),
        (P014, p014, 14, p014pfs),
        (P015, p015, 15, p015pfs),
    ]
);

pub mod port1 {
    use crate::hal::gpio::*;
    pub struct Port1;

    pub struct Parts {
        pub p100: P100<Input>,
        pub p101: P101<Input>,
        pub p102: P102<Input>,
        pub p103: P103<Input>,
        pub p104: P104<Input>,
        pub p105: P105<Input>,
        pub p106: P106<Input>,
        pub p107: P107<Input>,
        // pub p108: P108<Input>,
        // pub p109: P109<Input>,
        // pub p110: P110<Input>,
        pub p111: P111<Input>,
        pub p112: P112<Input>,
        pub p113: P113<Input>,
    }

    impl GpioExt for crate::pac::PORT1 {
        type Parts = Parts;

        fn split(self) -> Parts {
            Parts {
                p100: P100::new(unsafe { (*PMNPFS_BLOCK_BASE).p100pfs().get_unchecked(0) }),
                p101: P101::new(unsafe { (*PMNPFS_BLOCK_BASE).p100pfs().get_unchecked(1) }),
                p102: P102::new(unsafe { (*PMNPFS_BLOCK_BASE).p100pfs().get_unchecked(2) }),
                p103: P103::new(unsafe { (*PMNPFS_BLOCK_BASE).p100pfs().get_unchecked(3) }),
                p104: P104::new(unsafe { (*PMNPFS_BLOCK_BASE).p100pfs().get_unchecked(4) }),
                p105: P105::new(unsafe { (*PMNPFS_BLOCK_BASE).p100pfs().get_unchecked(5) }),
                p106: P106::new(unsafe { (*PMNPFS_BLOCK_BASE).p100pfs().get_unchecked(6) }),
                p107: P107::new(unsafe { (*PMNPFS_BLOCK_BASE).p100pfs().get_unchecked(7) }),
                // type is p108
                // p108: P108::new(unsafe { (*PMNPFS_BLOCK_BASE).p108pfs() }),
                // type is p109
                // p109: P109::new(unsafe { (*PMNPFS_BLOCK_BASE).p109pfs() }),
                // type is p108
                // p110: P110::new(unsafe { (*PMNPFS_BLOCK_BASE).p110() }),
                p111: P111::new(unsafe { (*PMNPFS_BLOCK_BASE).p111pfs() }),
                p112: P112::new(unsafe { (*PMNPFS_BLOCK_BASE).p112pfs() }),
                p113: P113::new(unsafe { (*PMNPFS_BLOCK_BASE).p113pfs() }),
            }
        }
    }
    pub type P100<Input> = Pin<1, 0, Input>;
    pub type P101<Input> = Pin<1, 1, Input>;
    pub type P102<Input> = Pin<1, 2, Input>;
    pub type P103<Input> = Pin<1, 3, Input>;
    pub type P104<Input> = Pin<1, 4, Input>;
    pub type P105<Input> = Pin<1, 5, Input>;
    pub type P106<Input> = Pin<1, 6, Input>;
    pub type P107<Input> = Pin<1, 7, Input>;
    // pub type P108<Input> = Pin<1, 8, Input>;
    // pub type P109<Input> = Pin<1, 9, Input>;
    // pub type P110<Input> = Pin<1, 10, Input>;
    pub type P111<Input> = Pin<1, 11, Input>;
    pub type P112<Input> = Pin<1, 12, Input>;
    pub type P113<Input> = Pin<1, 13, Input>;
}

// Still need to implement the input only ports on Port 2
gpio_port!(
    Port2,
    PORT2,
    port2,
    2,
    [
        (P200, p200, 0, p200pfs),
        (P204, p204, 4, p204pfs),
        (P205, p205, 5, p205pfs),
        (P206, p206, 6, p206pfs),
        (P212, p212, 12, p212pfs),
        (P213, p213, 13, p213pfs),
        (P214, p214, 14, p214pfs),
        (P215, p215, 15, p215pfs),
    ]
);

gpio_port!(
    Port3,
    PORT3,
    port3,
    3,
    [
        (P301, p301, 1, p301pfs),
        (P302, p302, 2, p302pfs),
        (P303, p303, 3, p303pfs),
        (P304, p304, 4, p304pfs),
    ]
);

pub mod port4 {
    use crate::hal::gpio::*;
    pub struct Port4;

    pub struct Parts {
        pub p400: P400<Input>,
        pub p401: P401<Input>,
        pub p402: P402<Input>,
        pub p407: P407<Input>,
        // pub p408: P408<Input>,
        pub p409: P409<Input>,
        pub p410: P410<Input>,
        pub p411: P411<Input>,
    }

    impl GpioExt for crate::pac::PORT4 {
        type Parts = Parts;

        fn split(self) -> Parts {
            Parts {
                p400: P400::new(unsafe { (*PMNPFS_BLOCK_BASE).p400pfs().get_unchecked(0) }),
                p401: P401::new(unsafe { (*PMNPFS_BLOCK_BASE).p400pfs().get_unchecked(1) }),
                p402: P402::new(unsafe { (*PMNPFS_BLOCK_BASE).p400pfs().get_unchecked(2) }),
                p407: P407::new(unsafe { (*PMNPFS_BLOCK_BASE).p400pfs().get_unchecked(7) }),
                // type is p408
                // p408: P408::new(unsafe { (*PMNPFS_BLOCK_BASE).p408pfs() }),
                p409: P409::new(unsafe { (*PMNPFS_BLOCK_BASE).p409pfs() }),
                p410: P410::new(unsafe { (*PMNPFS_BLOCK_BASE).p410pfs() }),
                p411: P411::new(unsafe { (*PMNPFS_BLOCK_BASE).p411pfs() }),
            }
        }
    }
    pub type P400<Input> = Pin<4, 0, Input>;
    pub type P401<Input> = Pin<4, 1, Input>;
    pub type P402<Input> = Pin<4, 2, Input>;
    pub type P407<Input> = Pin<4, 7, Input>;
    // pub type P408<Input> = Pin<4, 8, Input>;
    pub type P409<Input> = Pin<4, 9, Input>;
    pub type P410<Input> = Pin<4, 10, Input>;
    pub type P411<Input> = Pin<4, 11, Input>;
}
