use lazy_static::lazy_static;
use x86_64::structures::gdt::{GlobalDescriptorTable, Descriptor};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::registers::segmentation::{SegmentSelector, Segment, CS, DS, ES, FS, GS, SS};
use x86_64::instructions::tables::load_tss;
use x86_64::VirtAddr;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

const STACK_SIZE: usize = 8192;

struct Selectors {
    code: SegmentSelector,
    data: SegmentSelector,
    tss: SegmentSelector,
}

lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
            VirtAddr::from_ptr(unsafe { &STACK }) + STACK_SIZE
        };
        tss
    };

    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let code = gdt.add_entry(Descriptor::kernel_code_segment());
        let data = gdt.add_entry(Descriptor::kernel_data_segment());
        let tss = gdt.add_entry(Descriptor::tss_segment(&TSS));
        (gdt, Selectors { code, data, tss })
    };
}

pub unsafe fn init_gdt() {
    GDT.0.load();
    DS::set_reg(GDT.1.data);
    ES::set_reg(GDT.1.data);
    FS::set_reg(GDT.1.data);
    GS::set_reg(GDT.1.data);
    SS::set_reg(GDT.1.data);
    CS::set_reg(GDT.1.code);
    load_tss(GDT.1.tss);
}
