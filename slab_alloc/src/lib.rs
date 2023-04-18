#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]

use core::mem::{size_of, align_of};
use core::ptr::{NonNull, null_mut, write_bytes};
use core::slice::from_raw_parts;

pub const PAGE_SIZE: usize = 4096;
pub const REDZONE_SIZE: u16 = 16;

const OBJECT_MAGIC: u16 = 0x6b5c;
const REDZONE_FILL: u8 = 0xf1;
const UNUSED_FILL: u8 = 0xe2;

pub unsafe trait PageAllocator {
    fn allocate(&mut self) -> Option<NonNull<[u8; PAGE_SIZE]>>;
    unsafe fn deallocate(&mut self, ptr: NonNull<[u8; PAGE_SIZE]>);
}

pub struct SlabAllocator<PA: PageAllocator> {
    payload_size: u16,
    payload_align: u16,
    front_size: u16,
    object_size: u16,
    free: *mut ObjectHeader,
    page_allocator: PA,
}

#[repr(C)]
struct PageHeader {
    next: *mut PageHeader,
    free: u16,
    count: u16,
}

#[repr(C)]
struct ObjectHeader {
    magic: u16,
    next: u16,
}

impl ObjectHeader {
    fn align(payload_align: u16) -> u16 {
        payload_align.max(align_of::<ObjectHeader>() as u16)
    }
}

impl<PA: PageAllocator> SlabAllocator<PA> {
    pub fn new(payload_size: u16, payload_align: u16, page_allocator: PA) -> Self {
        assert!(PAGE_SIZE.is_power_of_two());

        assert!(payload_align.is_power_of_two(), "invalid slab alignment");
        assert!(payload_align <= (PAGE_SIZE as u16) / 4, "invalid slab alignment");
        assert!(payload_size < (PAGE_SIZE as u16) / 2, "invalid slab size");
        assert!(payload_size > 0, "invalid slab size");

        let object_align = ObjectHeader::align(payload_align);
        let header_size = size_of::<ObjectHeader>() as u16;
        let front_size = align_ceil(header_size + REDZONE_SIZE, object_align);
        let object_size = align_ceil(front_size + payload_size + REDZONE_SIZE, object_align);

        assert!(object_size <= (PAGE_SIZE as u16) / 2, "invalid slab size");

        Self {
            payload_size,
            payload_align,
            front_size,
            object_size,
            free: null_mut(),
            page_allocator,
        }
    }

    pub fn alloc(&mut self) -> Option<NonNull<u8>> {
        if self.free.is_null() {
            self.free = self.alloc_page()?;
        }

        Some(unsafe { self.alloc_from_free() })
    }

    // Safety: self.free is not null
    unsafe fn alloc_from_free(&mut self) -> NonNull<u8> {
        let object = self.free;
        let addr = object as usize;
        unsafe {
            assert_eq!((*object).magic, 0, "slab is poisoned");
            assert!(self.check_redzone(addr), "slab is poisoned");
            assert!(self.check_unused(addr), "dangling pointer exists or slab is poisoned");

            (*object).magic = OBJECT_MAGIC;

            let page = page_from_object(object);
            (*page).count += 1;

            if (*object).next != 0 {
                let next_addr = (page as usize) + (*object).next as usize;
                self.free = next_addr as *mut ObjectHeader;
                (*page).free = (*object).next;
                (*object).next = 0;
            }
            else if !(*page).next.is_null() {
                let np = (*page).next;
                let next_addr = (np as usize) + (*np).free as usize;
                self.free = next_addr as *mut ObjectHeader;
                (*page).free = 0;
                (*page).next = null_mut();
            }
            else {
                self.free = null_mut();
                (*page).free = 0;
            }

            NonNull::new_unchecked((addr + self.front_size as usize) as *mut u8)
        }
    }

    fn alloc_page(&mut self) -> Option<*mut ObjectHeader> {
        let addr = self.page_allocator.allocate()?.as_ptr() as usize;
        let header = addr as *mut PageHeader;

        let mut offset = self.page_offset();
        let first = (addr + offset as usize) as *mut ObjectHeader;

        let header_size = size_of::<ObjectHeader>();
        let right_offset = (self.front_size + self.payload_size) as usize;

        unsafe {
            (*header).next = null_mut();
            (*header).free = offset;
            (*header).count = 0;
            loop {
                let obj_addr = addr + offset as usize;
                let object = obj_addr as *mut ObjectHeader;

                (*object).magic = 0;
                write_bytes((obj_addr + header_size) as *mut u8, REDZONE_FILL, REDZONE_SIZE as usize);
                *((obj_addr + self.front_size as usize) as *mut u8) = UNUSED_FILL;
                write_bytes((obj_addr + right_offset) as *mut u8, REDZONE_FILL, REDZONE_SIZE as usize);

                offset += self.object_size;
                if offset < PAGE_SIZE as u16 {
                    (*object).next = offset;
                }
                else {
                    (*object).next = 0;
                    break;
                }
            }
        }

        Some(first)
    }

    fn page_offset(&self) -> u16 {
        align_ceil(size_of::<PageHeader>() as u16, ObjectHeader::align(self.payload_align))
    }

    // Safety: ptr is currently allocated
    pub unsafe fn dealloc(&mut self, ptr: NonNull<u8>) {
        let payload_addr = ptr.as_ptr() as usize;
        let addr = payload_addr - self.front_size as usize;
        let header = addr as *mut ObjectHeader;

        unsafe {
            assert!((*header).magic == OBJECT_MAGIC && (*header).next == 0, "invalid dealloc() or slab is poisoned");
            assert!(self.check_redzone(addr), "invalid dealloc() or slab is poisoned");

            *(payload_addr as *mut u8) = UNUSED_FILL;

            let page = page_from_object(header);
            (*page).count -= 1;

            (*header).magic = 0;
            (*header).next = (*page).free;
            (*page).free = (addr - page as usize) as u16;

            if self.free.is_null() || page_from_object(self.free) == page {
                self.free = header;

                if (*page).count == 0 {
                    self.dealloc_page(page);
                }
            }
            else {
                let pp = page_from_object(self.free);
                (*page).next = (*pp).next;
                (*pp).next = page;
            }
        }
    }

    // Safety: page is valid
    unsafe fn dealloc_page(&mut self, page: *mut PageHeader) {
        unsafe {
            if (*page).next.is_null() {
                self.free = null_mut();
            }
            else {
                let np = (*page).next;
                let next_addr = (np as usize) + (*np).free as usize;
                self.free = next_addr as *mut ObjectHeader;
            }
            self.page_allocator.deallocate(NonNull::new_unchecked(page as *mut [u8; PAGE_SIZE]));
        }
    }

    // Safety: addr is address of slab object
    unsafe fn check_redzone(&self, addr: usize) -> bool {
        let right_offset = (self.front_size + self.payload_size) as usize;
        unsafe {
            let front = from_raw_parts((addr + size_of::<ObjectHeader>()) as *mut u8, REDZONE_SIZE as usize);
            let back = from_raw_parts((addr + right_offset) as *mut u8, REDZONE_SIZE as usize);

            front.iter().all(|&x| x == REDZONE_FILL) && back.iter().all(|&x| x == REDZONE_FILL)
        }
    }

    // Safety: addr is address of slab object
    unsafe fn check_unused(&self, addr: usize) -> bool {
        unsafe { *((addr + self.front_size as usize) as *mut u8) == UNUSED_FILL }
    }
}

// Safety: object is valid
unsafe fn page_from_object(object: *mut ObjectHeader) -> *mut PageHeader {
    ((object as usize) & !(PAGE_SIZE as usize - 1)) as *mut PageHeader
}

fn align_ceil(x: u16, align: u16) -> u16 {
    let mask = align - 1;
    (x + mask) & !mask
}
