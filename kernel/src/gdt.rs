use lazy_static::lazy_static;
use x86_64::structures::gdt::{GlobalDescriptorTable, Descriptor};
use x86_64::registers::segmentation::{SegmentSelector, Segment, CS, DS, ES, FS, GS, SS};
use x86_64::PrivilegeLevel;

lazy_static! {
    static ref GDT: GlobalDescriptorTable = {
        let mut gdt = GlobalDescriptorTable::new();
        gdt.add_entry(Descriptor::kernel_code_segment());
        gdt.add_entry(Descriptor::kernel_data_segment());
        gdt
    };
}

pub fn init_gdt() {
    GDT.load();
    unsafe {
        DS::set_reg(SegmentSelector::new(2, PrivilegeLevel::Ring0));
        ES::set_reg(SegmentSelector::new(2, PrivilegeLevel::Ring0));
        FS::set_reg(SegmentSelector::new(2, PrivilegeLevel::Ring0));
        GS::set_reg(SegmentSelector::new(2, PrivilegeLevel::Ring0));
        SS::set_reg(SegmentSelector::new(2, PrivilegeLevel::Ring0));
        CS::set_reg(SegmentSelector::new(1, PrivilegeLevel::Ring0));
    }
}
