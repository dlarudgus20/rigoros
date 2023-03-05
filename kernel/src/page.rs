use x86_64::structures::paging::{PageTable, PageTableFlags};

use crate::println;

const PAGE_TABLE_ADDR_VIRTUAL: u64 = 0xffff8000003f0000;
const PAGE_TABLE_ADDR_PHYSICAL: u64 = 0x5f0000;

pub fn print_page() {
    print_table_r(&["PML4E", " PDPE", "  PDE"], 0, get_table());
}

fn print_table_r(names: &[&str], depth: usize, table: &PageTable) {
    for (i, entry) in table.iter().enumerate() {
        if entry.flags().contains(PageTableFlags::PRESENT) {
            println!("{} {:#5x} to {:#x}: {:?}", names[depth], i, entry.addr().as_u64(), entry.flags());

            if depth + 1 < names.len() && !entry.flags().contains(PageTableFlags::HUGE_PAGE) {
                let addr = entry.addr().as_u64() - PAGE_TABLE_ADDR_PHYSICAL + PAGE_TABLE_ADDR_VIRTUAL;
                let subtable: &PageTable = unsafe { &*(addr as *const PageTable) };
                print_table_r(names, depth + 1, subtable);
            }
        }
    }
}

fn get_table() -> &'static mut PageTable {
    unsafe {
        &mut *(PAGE_TABLE_ADDR_VIRTUAL as *mut PageTable)
    }
}
