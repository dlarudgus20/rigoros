use lazy_static::lazy_static;
use num_enum::{TryFromPrimitive, IntoPrimitive};
use num_integer::div_ceil;
use num_iter::range_step;
use x86_64::instructions::tlb;
use x86_64::structures::paging::page_table::PageTableEntry;
use x86_64::{VirtAddr, PhysAddr};
use x86_64::structures::paging::{PageTable, PageTableFlags};

use buddyblock::{BuddyBlock, BuddyBlockInfo};

use crate::log;
use crate::irq_mutex::IrqMutex;
use crate::terminal::ColorCode;

#[derive(Clone, Copy)]
struct MemoryMap {
    entries: &'static [MemoryMapEntry],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct MemoryMapEntry {
    base: u64,
    size: u64,
    mem_type: u32,
    attrib: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
#[repr(u32)]
enum MemoryEntryType {
    Usable = 1,
    Reserved = 2,
    AcpiReclaimable = 3,
    AcpiNVS = 4,
    BadArea = 5,
}

struct PageInitWalker {
    start_table: *mut PageTable,
    next_table: *mut PageTable,
    tables: [*mut PageTable; 4],
    indices: [u16; 4],
    skip: u32,
}

struct MemoryData {
    total_len: usize,
    page_table_len: usize,
    buddy_len: usize,
    buddyblock: BuddyBlock<'static>,
}

pub struct AllocatorInfo {
    pub buddy: BuddyBlockInfo,
    pub used: usize,
}

pub struct AllocatorSizeInfo {
    pub len: usize,
    pub used: usize,
}

const DYNMEM_START_PHYS: u64 = 0x00800000;
const DYNMEM_START_VIRT: u64 = 0x00200000;

const PAGE_TABLE_ADDR: u64 = 0xffff8000003f0000;
const MEMORY_MAP_ADDR: u64 = 0xffff80001fe06000;

const KERNEL_START_VIRT: u64 = 0xffff800000000000;
const KERNEL_START_PHYS: u64 = 0x00200000;

const KSTACK_START_VIRT: u64 = 0xffff80000f000000;
const KSTACK_START_PHYS: u64 = 0x00600000;

pub const PAGE_SIZE: u64 = 4096;

lazy_static! {
    static ref MEMORY_DATA: IrqMutex<MemoryData> = IrqMutex::new(MemoryData {
        total_len: 0,
        page_table_len: 0,
        buddy_len: 0,
        buddyblock: BuddyBlock::empty(),
    });
}

lazy_static! {
    static ref DYNMEM_MAP: MemoryMap = {
        const BUFSIZE: usize = 1024;
        static mut BUFFER: [MemoryMapEntry; BUFSIZE] = [MemoryMapEntry::zero(); BUFSIZE];

        let e820 = get_e820_map();
        let buffer = unsafe { &mut *&raw mut BUFFER };
        let len = create_dynmem_map(e820.entries, buffer);

        MemoryMap { entries: &buffer[0..len] }
    };
}

impl MemoryMapEntry {
    const fn zero() -> Self {
        Self { attrib: 0, mem_type: 0, base: 0, size: 0 }
    }
}

pub unsafe fn init_memory() {
    get_memory_map(); // lazy-initialize

    unsafe {
        init_dyn_page();
        init_dyn_alloc();
    }
}

fn get_memory_map() -> MemoryMap {
    *DYNMEM_MAP
}

fn create_dynmem_map(e820_entries: &[MemoryMapEntry], map_buffer: &mut [MemoryMapEntry]) -> usize {
    let e820_len = e820_entries.len().min(map_buffer.len());
    let buffer = &mut map_buffer[0..e820_len];
    buffer.copy_from_slice(e820_entries);
    buffer.sort_unstable_by_key(|x| x.base);

    let mut prev_end = DYNMEM_START_PHYS;
    for entry in &mut *buffer {
        if MemoryEntryType::try_from(entry.mem_type) == Ok(MemoryEntryType::Usable) {
            let start = entry.base;
            let end = entry.base + entry.size;

            let align_start = prev_end.max(div_ceil(start, PAGE_SIZE) * PAGE_SIZE);
            let align_end = end / PAGE_SIZE * PAGE_SIZE;

            if align_start < align_end {
                entry.base = align_start;
                entry.size = align_end - align_start;
                prev_end = align_end;
            }
            else {
                entry.mem_type = 0;
            }
        }
    }

    slice_remove(buffer, |x| x.mem_type != MemoryEntryType::Usable.into())
}

unsafe fn init_dyn_page() {
    let pml4t = get_table_mut();
    let map = get_memory_map();

    create_tmp_page(pml4t);

    let (total_len, page_table_len) = unsafe {
        create_dyn_page(pml4t, map, DYNMEM_START_VIRT)
    };
    tlb::flush_all();

    let mut data = MEMORY_DATA.lock();
    data.total_len = total_len;
    data.page_table_len = page_table_len;
}

fn create_tmp_page(pml4t: &mut PageTable) {
    fn stk_phys<T>(ptr: *const T) -> PhysAddr {
        let addr = (ptr as u64) - KSTACK_START_VIRT + KSTACK_START_PHYS;
        PhysAddr::new(addr)
    }

    fn dyn_phys(idx: u64) -> PhysAddr {
        PhysAddr::new(DYNMEM_START_PHYS + idx * PAGE_SIZE)
    }

    let flags = PageTableFlags::WRITABLE | PageTableFlags::PRESENT;

    unsafe {
        // map first 3 pages by recursive paging
        let mut tmptable = PageTable::new();
        let phy_table = stk_phys(&tmptable);
        pml4t[1].set_addr(phy_table, flags);
        tmptable[0].set_addr(phy_table, flags);
        tmptable[1].set_addr(phy_table, flags);
        for idx in 0..3 {
            let addr = DYNMEM_START_PHYS + (idx as u64) * PAGE_SIZE;
            tmptable[2 + idx].set_addr(PhysAddr::new(addr), flags);
        }
        tlb::flush_all();

        // PML4:1, PDP: 0, PD: 1, PT: 2
        let pdpt = (1u64 << 39 | 1u64 << 21 | 2u64 << 12) as *mut PageTable;
        let pdt = pdpt.add(1);
        let pt = pdt.add(1);
        pml4t[0].set_addr(dyn_phys(0), flags);
        (*pdpt)[0].set_addr(dyn_phys(1), flags);
        (*pdt)[1].set_addr(dyn_phys(2), flags);
        for idx in 0..3 {
            let addr = DYNMEM_START_PHYS + (idx as u64) * PAGE_SIZE;
            (*pt)[idx].set_addr(PhysAddr::new(addr), flags);
        }

        pml4t[1].set_unused();
    }
}

unsafe fn create_dyn_page(pml4t: &mut PageTable, map: MemoryMap, start_virt: u64) -> (usize, usize) {
    let flags = PageTableFlags::WRITABLE | PageTableFlags::PRESENT;

    let mut walker = unsafe {
        PageInitWalker::new(&mut *pml4t, start_virt as *mut PageTable, 1)
    };
    let mut page_count = 0;

    for entry in map.entries {
        let start = entry.base;
        let stop = entry.base + entry.size;
        for base in range_step(start, stop, PAGE_SIZE) {
            page_count += 1;
            walker.next_set(PhysAddr::new(base), flags, map, start_virt);
        }
    }

    let total_len = page_count * (PAGE_SIZE as usize);
    let page_table_len = walker.count() * (PAGE_SIZE as usize);

    (total_len, page_table_len)
}

impl PageInitWalker {
    unsafe fn new(pml4t: *mut PageTable, pdpt: *mut PageTable, first_dir: u16) -> Self {
        unsafe {
            let pdt = pdpt.add(1);
            let pt = pdt.add(1);
            let next_table = pt.add(1);
            Self {
                start_table: pdpt,
                next_table,
                tables: [pml4t, pdpt, pdt, pt],
                indices: [1, 1, first_dir + 1, 3],
                skip: 3,
            }
        }
    }

    fn count(&self) -> usize {
        unsafe { self.next_table.offset_from(self.start_table) as usize }
    }

    fn next_set(&mut self, addr: PhysAddr, flags: PageTableFlags, map: MemoryMap, start_virt: u64) {
        if self.skip == 0 {
            self.next_set_recur(3, addr, flags, map, start_virt);
        }
        else {
            self.skip -= 1;
        }
    }

    fn next_set_recur(&mut self, level: usize, addr: PhysAddr, flags: PageTableFlags, map: MemoryMap, start_virt: u64) {
        assert!(!(level == 0 && self.indices[level] >= 256), "PageTableWalker::next() out of bound");

        if self.indices[level] >= 512 {
            let next = self.next_table;
            unsafe {
                (*next).zero();
            }
            self.tables[level] = next;
            self.indices[level] = 0;
            self.next_table = unsafe { next.add(1) };

            let next_phys = virt_to_phys_dynmem(VirtAddr::from_ptr(next), map, start_virt);
            self.next_set_recur(level - 1, next_phys, flags, map, start_virt);
        }

        let table = unsafe { &mut (*self.tables[level]) };
        table[self.indices[level] as usize].set_addr(addr, flags);
        self.indices[level] += 1;
    }
}

fn virt_to_phys_dynmem(virt: VirtAddr, map: MemoryMap, start_virt: u64) -> PhysAddr {
    let offset = virt.as_u64() - start_virt;

    let mut sum = 0;
    for entry in map.entries {
        let oldsum = sum;
        sum += entry.size;

        if offset < sum {
            let phys = entry.base + (offset - oldsum);
            return PhysAddr::new(phys);
        }
    }

    panic!("invalid dynmem virtual address");
}

fn phys_to_virt_dynmem(phys: PhysAddr, map: MemoryMap, start_virt: u64) -> VirtAddr {
    let mut sum = 0;
    for entry in map.entries {
        if (entry.base..entry.base + entry.size).contains(&phys.as_u64()) {
            let virt = start_virt + sum + (phys.as_u64() - entry.base);
            return VirtAddr::new(virt);
        }

        sum += entry.size;
    }

    panic!("invalid dynmem physical address");
}

#[allow(dead_code)]
fn virt_to_phys_kernel(virt: VirtAddr) -> PhysAddr {
    let addr = virt.as_u64() - KERNEL_START_VIRT + KERNEL_START_PHYS;
    PhysAddr::new(addr)
}

fn phys_to_virt_kernel(phys: PhysAddr) -> VirtAddr {
    let addr = phys.as_u64() - KERNEL_START_PHYS + KERNEL_START_VIRT;
    VirtAddr::new(addr)
}

fn phys_to_virt(phys: PhysAddr) -> VirtAddr {
    if phys.as_u64() >= DYNMEM_START_PHYS {
        phys_to_virt_dynmem(phys, get_memory_map(), DYNMEM_START_VIRT)
    }
    else if phys.as_u64() < KSTACK_START_PHYS {
        phys_to_virt_kernel(phys)
    }
    else {
        panic!("invalid physical address")
    }
}

unsafe fn init_dyn_alloc() {
    let mut data = MEMORY_DATA.lock();
    let start = (DYNMEM_START_VIRT as usize) + data.page_table_len;
    let len = data.total_len - data.page_table_len;

    data.buddyblock = unsafe {
        BuddyBlock::new(start, len)
    };
    data.buddy_len = data.buddyblock.info().data_offset();
}

pub fn allocator_info() -> AllocatorInfo {
    let data = MEMORY_DATA.lock();
    AllocatorInfo {
        buddy: *data.buddyblock.info(),
        used: data.buddyblock.used(),
    }
}

pub fn allocator_size_info() -> AllocatorSizeInfo {
    let data = MEMORY_DATA.lock();
    AllocatorSizeInfo {
        len: data.buddyblock.info().data_len(),
        used: data.buddyblock.used(),
    }
}

pub fn alloc_zero(len: usize) -> Option<usize> {
    let mut data = MEMORY_DATA.lock();
    data.buddyblock.alloc(len).map(|addr| {
        unsafe {
            core::ptr::write_bytes(addr as *mut u8, 0, len);
        }
        addr
    })
}

pub fn deallocate(addr: usize, len: usize) {
    let mut data = MEMORY_DATA.lock();
    data.buddyblock.dealloc(addr, len);
}

pub fn print_e820_map() {
    print_memory(get_e820_map(), "BIOS e820 Memory Map");
}

pub fn print_dynmem_map() {
    print_memory(get_memory_map(), "Dynamic Memory Map");
}

fn print_memory(map: MemoryMap, title: &str) {
    log!(color: ColorCode::DEFAULT, "{}: {} entries", title, map.entries.len());
    for entry in map.entries {
        log!(nosep, color: ColorCode::DEFAULT, "    [{:#018x}, {:#018x})", entry.base, entry.base + entry.size);
        if let Ok(t) = MemoryEntryType::try_from(entry.mem_type) {
            log!(color: ColorCode::DEFAULT, " {:?}", t);
        }
        else {
            log!(color: ColorCode::DEFAULT, " (unknown)");
        }
    }
}

pub fn print_page() {
    print_table_r(get_table(), &["PML4E", " PDPE", "  PDE"], 0, 0);
}

fn print_table_r(table: &PageTable, names: &[&str], depth: usize, virt: u64) {
    for (idx, entry) in table.iter().enumerate() {
        if entry.flags().contains(PageTableFlags::PRESENT) {
            log!(color: ColorCode::DEFAULT, "{} {:#5x} to {:#x}: {:?}", names[depth], idx, entry.addr().as_u64(), entry.flags());

            if !entry.flags().contains(PageTableFlags::HUGE_PAGE) {
                let addr = phys_to_virt(entry.addr());
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
                        let flag = 0xffff800000000000;
                        let v_raw = (virt << 9 | first as u64) << 12;
                        let v_extended = if v_raw & flag == 0 { v_raw } else { v_raw | flag };
                        let v = VirtAddr::new(v_extended).as_u64();
                        let p = page.addr().as_u64();
                        log!(color: ColorCode::DEFAULT, "   PT {:#018x}-{:#018x} to {:#x}-{:#x}", v, v + len * PAGE_SIZE, p, p + len * PAGE_SIZE);

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

fn get_e820_map() -> MemoryMap {
    unsafe {
        let len = *(MEMORY_MAP_ADDR as *const u16) as usize;
        MemoryMap {
            entries: core::slice::from_raw_parts((MEMORY_MAP_ADDR + 8) as *mut MemoryMapEntry, len)
        }
    }
}

// https://en.cppreference.com/w/cpp/algorithm/remove
fn slice_remove<T, F : Fn(&T) -> bool>(slice: &mut [T], predicate: F) -> usize {
    let len = slice.len();
    let mut first = slice.iter().position(&predicate).unwrap_or(len);
    let mut p = first + 1;
    while p < len {
        if !predicate(&slice[p]) {
            //slice[first] = slice[p];
            slice.swap(first, p);
            first += 1;
        }
        p += 1;
    }
    first
}

#[cfg(test)]
mod tests {
    use super::{*};
    use std::vec;

    #[test]
    fn test_create_dynmem_map() {
        let test_map = [
            MemoryMapEntry { base: 0x03000000, size: 0x01000000, mem_type: 1, attrib: 0 },
            MemoryMapEntry { base: 0x00000000, size: 0x01040000, mem_type: 1, attrib: 0 },
            MemoryMapEntry { base: 0x02000000, size: 0x01000000, mem_type: 1, attrib: 0 },
            MemoryMapEntry { base: 0x01000000, size: 0x01000000, mem_type: 1, attrib: 0 },
        ];
        let expected = [
            MemoryMapEntry { base: 0x00800000, size: 0x00840000, mem_type: 1, attrib: 0 },
            MemoryMapEntry { base: 0x01040000, size: 0x00fc0000, mem_type: 1, attrib: 0 },
            MemoryMapEntry { base: 0x02000000, size: 0x01000000, mem_type: 1, attrib: 0 },
            MemoryMapEntry { base: 0x03000000, size: 0x01000000, mem_type: 1, attrib: 0 },
        ];
        let mut buffer = [MemoryMapEntry::zero(); 16];
        let len = create_dynmem_map(&test_map, &mut buffer);
        let result = &buffer[0..len];
        assert_eq!(result, expected);
    }

    #[derive(Clone, Copy)]
    #[repr(C, align(4096))]
    struct TestPage {
        data: [u8; 4096]
    }

    fn test_create_dyn_page<const N: usize>(map: &'static [MemoryMapEntry; N], dyn_size: u64) {
        let mut pml4t = PageTable::new();
        let mem = vec![TestPage { data: [0; 4096] }; dyn_size as usize / 4096];
        let start_virt = mem.as_ptr() as u64;

        let (total_len, page_table_len) = unsafe {
            create_dyn_page(&mut pml4t, MemoryMap { entries: map }, start_virt)
        };

        let pages = div_ceil(dyn_size, 4096);
        let pts = div_ceil(pages, 512);
        let directories = div_ceil(pts, 512);
        let pdts = div_ceil(directories, 512);
        let pointers = div_ceil(pts, 512);
        let pdpts = div_ceil(pointers, 512);
        let expected_len = pts * 4096 + pdts * 4096 + pdpts * 4096;
        println!("total={:#x} ptlen={:#x} expected={:#x}", total_len, page_table_len, expected_len);

        assert_eq!(total_len as u64, dyn_size, "total_len");
        assert_eq!(page_table_len as u64, expected_len, "page_table_len");
    }

    #[test]
    fn test_create_dyn_page_single_entry() {
        const MEMMAP: [MemoryMapEntry; 1] = [
            MemoryMapEntry { base: 0x00800000, size: 0x01000000, mem_type: 1, attrib: 0 },
        ];

        test_create_dyn_page(&MEMMAP, 0x01000000);
    }

    #[test]
    fn test_create_dyn_page_multiple_entries() {
        const MEMMAP: [MemoryMapEntry; 4] = [
            MemoryMapEntry { base: 0x00800000, size: 0x00840000, mem_type: 1, attrib: 0 },
            MemoryMapEntry { base: 0x01040000, size: 0x00fc0000, mem_type: 1, attrib: 0 },
            MemoryMapEntry { base: 0x02000000, size: 0x01000000, mem_type: 1, attrib: 0 },
            MemoryMapEntry { base: 0x03000000, size: 0x01000000, mem_type: 1, attrib: 0 },
        ];

        test_create_dyn_page(&MEMMAP, 0x03800000);
    }
}
