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
