use x86_64::registers::control::Cr3;
use x86_64::structures::paging::page_table::PageTableEntry;
use x86_64::{VirtAddr, PhysAddr};
use x86_64::structures::paging::{PageTable, PageTableFlags};

use crate::println;

const PAGE_TABLE_ADDR: u64 = 0xffff8000003f0000;

const KERNEL_START_VIRT: u64 = 0xffff800000000000;
const KERNEL_START_PHYS: u64 = 0x00200000;

pub unsafe fn init_page() {
    static mut PDPT: PageTable = PageTable::new();
    static mut PDT: PageTable = PageTable::new();
    static mut PT: PageTable = PageTable::new();

    let table = get_table_mut();

    let start = 0x01000000u64;
    let flags = PageTableFlags::WRITABLE | PageTableFlags::PRESENT;

    table[0].set_addr(phys_addr_in_kernel(VirtAddr::from_ptr(&PDPT)), flags);
    PDPT[0].set_addr(phys_addr_in_kernel(VirtAddr::from_ptr(&PDT)), flags);
    PDT[1].set_addr(phys_addr_in_kernel(VirtAddr::from_ptr(&PT)), flags);

    for (idx, entry) in PT.iter_mut().enumerate() {
        let addr = start + (idx as u64) * 4096;
        entry.set_addr(PhysAddr::new(addr), flags);
    }

    invalidate_page_table();
}

fn phys_addr_in_kernel(virt: VirtAddr) -> PhysAddr {
    let addr = virt.as_u64() - KERNEL_START_VIRT + KERNEL_START_PHYS;
    PhysAddr::new(addr)
}

fn virt_addr_in_kernel(phys: PhysAddr) -> VirtAddr {
    let addr = phys.as_u64() - KERNEL_START_PHYS + KERNEL_START_VIRT;
    VirtAddr::new(addr)
}

pub fn print_page() {
    print_table_r(get_table(), &["PML4E", " PDPE", "  PDE"], 0, 0);
}

fn print_table_r(table: &PageTable, names: &[&str], depth: usize, virt: u64) {
    for (idx, entry) in table.iter().enumerate() {
        if entry.flags().contains(PageTableFlags::PRESENT) {
            println!("{} {:#5x} to {:#x}: {:?}", names[depth], idx, entry.addr().as_u64(), entry.flags());

            if !entry.flags().contains(PageTableFlags::HUGE_PAGE) {
                let addr = virt_addr_in_kernel(entry.addr());
                let subtable: &PageTable = unsafe { &*addr.as_ptr() };

                if depth + 1 < names.len() {
                    print_table_r(subtable, names, depth + 1, virt << 9 | idx as u64);
                }
                else {
                    print_pages(subtable, virt << 9 | idx as u64);
                }
            }
        }
    }

    fn print_pages(table: &PageTable, virt: u64) {
        let mut found = None;
        for idx in 0..513 {
            let present = idx < 512 && table[idx].flags().contains(PageTableFlags::PRESENT);
            match found {
                None => {
                    if present {
                        found = Some(idx);
                    }
                }
                Some(first) => {
                    if !present || table[idx - 1].addr().as_u64() + 0x1000 != table[idx].addr().as_u64() {
                        let page: &PageTableEntry = &table[first];
                        let len = (idx - first) as u64;
                        let v = VirtAddr::new((virt << 9 | first as u64) << 12).as_u64();
                        let p = page.addr().as_u64();
                        println!("   PT {:#018x}-{:#018x} to {:#x}-{:#x}", v, v + len * 4096, p, p + len * 4096);

                        found = if present { Some(idx) } else { None };
                    }
                }
            }
        }
    }
}

fn get_table() -> &'static PageTable {
    get_table_mut()
}

fn get_table_mut() -> &'static mut PageTable {
    unsafe {
        &mut *(PAGE_TABLE_ADDR as *mut PageTable)
    }
}

fn invalidate_page_table() {
    unsafe {
        let (table, flag) = Cr3::read();
        Cr3::write(table, flag);
    }
}
