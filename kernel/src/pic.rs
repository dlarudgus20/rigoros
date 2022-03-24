use bitflags::bitflags;
use pic8259::ChainedPics;

#[allow(dead_code)]
#[repr(u8)]
pub enum Irq {
    TIMER = 0,
    KEYBOARD = 1,
    SLAVE = 2,
    SERIAL1 = 3,
    SERIAL2 = 4,
    PARALLEL1 = 5,
    FLOPPY = 6,
    PARALLEL2 = 7,
    RTC = 8,
    MOUSE = 12,
    COPROC = 13,
    HDD1 = 14,
    HDD2 = 15,
}

bitflags! {
    pub struct Mask: u16 {
        const TIMER = 1 << Irq::TIMER as u8;
        const KEYBOARD = 1 << Irq::KEYBOARD as u8;
        const SLAVE = 1 << Irq::SLAVE as u8;
        const SERIAL1 = 1 << Irq::SERIAL1 as u8;
        const SERIAL2 = 1 << Irq::SERIAL2 as u8;
        const PARALLEL1 = 1 << Irq::PARALLEL1 as u8;
        const FLOPPY = 1 << Irq::FLOPPY as u8;
        const PARALLEL2 = 1 << Irq::PARALLEL2 as u8;
        const RTC = 1 << Irq::RTC as u8;
        const MOUSE = 1 << Irq::MOUSE as u8;
        const COPROC = 1 << Irq::COPROC as u8;
        const HDD1 = 1 << Irq::HDD1 as u8;
        const HDD2 = 1 << Irq::HDD2 as u8;
    }
}

pub const PIC_INT_OFFSET: u8 = 0x20;

static PIC: spin::Mutex<ChainedPics> = spin::Mutex::new(unsafe {
    ChainedPics::new(PIC_INT_OFFSET, PIC_INT_OFFSET + 8)
});

pub fn init_pic() {
    let mut pic = PIC.lock();
    unsafe {
        pic.initialize();
        pic.write_masks(!Mask::SLAVE.bits as u8, 0xff);
    }
}

pub unsafe fn set_mask(mask: Mask) {
    let mut pic = PIC.lock();
    let bits = !mask.bits;
    pic.write_masks(bits as u8, (bits >> 8) as u8);
}

pub unsafe fn send_eoi(irq: Irq) {
    let mut pic = PIC.lock();
    pic.notify_end_of_interrupt(PIC_INT_OFFSET + irq as u8);
}

impl Irq {
    pub fn as_intn(self) -> usize {
        (PIC_INT_OFFSET + self as u8).into()
    }
}