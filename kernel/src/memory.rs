use lazy_static::lazy_static;
use num_enum::{TryFromPrimitive, IntoPrimitive};
use num_integer::div_ceil;
use num_iter::range_step;
use x86_64::instructions::tlb;
use x86_64::structures::paging::page_table::PageTableEntry;
use x86_64::{VirtAddr, PhysAddr};
use x86_64::structures::paging::{PageTable, PageTableFlags};

use crate::log;
use crate::irq_mutex::IrqMutex;
use crate::buddyblock::{BuddyBlock, BuddyBlockInfo};
use crate::terminal::ColorCode;

#[derive(Copy, Clone)]
struct MemoryMap {
    entries: &'static [MemoryMapEntry],
}

#[derive(Debug, PartialEq, Eq)]
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
    map: MemoryMap,
}

struct MemoryData {
    total_len: usize,
    page_table_len: usize,
    buddy_len: usize,
    buddyblock: BuddyBlock<'static>,
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

pub unsafe fn init_memory() {
    print_memory("Memory map report");
    unsafe {
        init_memory_map();
        init_dyn_page();
        init_dyn_alloc();
    }
    print_dynmem_map();
}

unsafe fn init_memory_map() {
    let entries = unsafe { get_memory_map_mut() };

    let len = extract_dynmem_map(entries);

    unsafe {
        set_memory_map_len(len as u16);
    }
}

fn extract_dynmem_map(entries: &mut [MemoryMapEntry]) -> usize {
    entries.sort_unstable_by_key(|x| x.base);

    let mut prev_end = DYNMEM_START_PHYS;
    for entry in &mut *entries {
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

    slice_remove(entries, |x| x.mem_type != MemoryEntryType::Usable.into())
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
    }
}

unsafe fn create_dyn_page(pml4t: &mut PageTable, map: MemoryMap, start_virt: u64) -> (usize, usize) {
    let flags = PageTableFlags::WRITABLE | PageTableFlags::PRESENT;

    let mut walker = unsafe {
        PageInitWalker::new(&mut *pml4t, start_virt as *mut PageTable, 1, map)
    };
    let mut page_count = 0;

    for entry in map.entries {
        let start = entry.base;
        let stop = entry.base + entry.size;
        for base in range_step(start, stop, PAGE_SIZE) {
            page_count += 1;
            walker.next_set(PhysAddr::new(base), flags);
        }
    }

    pml4t[1].set_unused();

    let total_len = page_count * (PAGE_SIZE as usize);
    let page_table_len = walker.count() * (PAGE_SIZE as usize);

    (total_len, page_table_len)
}

impl PageInitWalker {
    unsafe fn new(pml4t: *mut PageTable, pdpt: *mut PageTable, first_dir: u16, map: MemoryMap) -> Self {
        unsafe {
            let pdt = pdpt.add(1);
            let pt = pdt.add(1);
            let next_table = pt.add(1);
            Self {
                start_table: pdpt,
                next_table,
                tables: [pml4t, pdpt, pdt, pt],
                indices: [1, 1, first_dir + 1, 0],
                map,
            }
        }
    }

    fn count(&self) -> usize {
        unsafe { self.next_table.offset_from(self.start_table) as usize }
    }

    fn next_set(&mut self, addr: PhysAddr, flags: PageTableFlags) {
        self.next_set_recur(3, addr, flags);
    }

    fn next_set_recur(&mut self, level: usize, addr: PhysAddr, flags: PageTableFlags) {
        let table = unsafe { &mut (*self.tables[level]) };
        table[self.indices[level] as usize].set_addr(addr, flags);
        self.indices[level] += 1;

        if self.indices[level] >= 512 {
            assert_ne!(level, 0, "PageTableWalker::next() out of bound");

            let next = self.next_table;
            unsafe {
                (*next).zero();
            }
            self.tables[level] = next;
            self.indices[level] = 0;
            self.next_table = unsafe { next.add(1) };

            let next_phys = virt_to_phys_dynmem(VirtAddr::from_ptr(next), self.map);
            self.next_set_recur(level - 1, next_phys, flags);
        }
    }
}

fn virt_to_phys_dynmem(virt: VirtAddr, map: MemoryMap) -> PhysAddr {
    let offset = virt.as_u64() - DYNMEM_START_VIRT;

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

fn phys_to_virt_dynmem(phys: PhysAddr, map: MemoryMap) -> VirtAddr {
    let mut sum = 0;
    for entry in map.entries {
        if (entry.base..entry.base + entry.size).contains(&phys.as_u64()) {
            let virt = DYNMEM_START_VIRT + sum + (phys.as_u64() - entry.base);
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
        phys_to_virt_dynmem(phys, get_memory_map())
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
    let end = (DYNMEM_START_VIRT as usize) + data.total_len;

    let info = BuddyBlockInfo::new(start, end - start);
    data.buddy_len = info.metadata_offset();

    data.buddyblock = unsafe { BuddyBlock::new(start as *mut u8, info) };
}

pub fn allocate(len: usize) -> Option<usize> {
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

pub fn test_dyn_seq() {
    let mut data = MEMORY_DATA.lock();
    data.buddyblock.test_seq();
}

pub fn print_dynmem_map() {
    print_memory("Dynamic Memory");
}

fn print_memory(title: &str) {
    let map = get_memory_map();

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
                        let v = VirtAddr::new((virt << 9 | first as u64) << 12).as_u64();
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

fn get_memory_map() -> MemoryMap {
    MemoryMap { entries: unsafe { get_memory_map_mut() } }
}

unsafe fn get_memory_map_mut() -> &'static mut [MemoryMapEntry] {
    unsafe {
        let len = *(MEMORY_MAP_ADDR as *const u16) as usize;
        core::slice::from_raw_parts_mut((MEMORY_MAP_ADDR + 8) as *mut MemoryMapEntry, len)
    }
}

unsafe fn set_memory_map_len(len: u16) {
    unsafe { *(MEMORY_MAP_ADDR as *mut u16) = len; }
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

    #[test]
    fn test_dynmem_map() {
        let mut test_map = [
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

        let len = extract_dynmem_map(&mut test_map);
        let result = &test_map[0..len];
        assert_eq!(result, expected);
    }

    /*
    // std::vec?
    #[test]
    fn test_dyn_page() {
        let map = [
            MemoryMapEntry { base: 0x00800000, size: 0x00840000, mem_type: 1, attrib: 0 },
            MemoryMapEntry { base: 0x01040000, size: 0x00fc0000, mem_type: 1, attrib: 0 },
            MemoryMapEntry { base: 0x02000000, size: 0x01000000, mem_type: 1, attrib: 0 },
            MemoryMapEntry { base: 0x03000000, size: 0x01000000, mem_type: 1, attrib: 0 },
        ];

        let mut pml4t = PageTable::new();
        let mem = vec![0u8; 0x04000000];
        let (total_len, page_table_len) = unsafe {
            create_dyn_page(&mut pml4t, MemoryMap { entries: &map }, &mem[0] as u64)
        };

        todo!();
    }
    */
}
