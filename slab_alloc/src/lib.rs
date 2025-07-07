#![cfg_attr(not(test), no_std)]
#![deny(unsafe_op_in_unsafe_fn)]

/// A `SlabAllocator` is a memory allocator designed for efficient allocation and deallocation
/// of fixed-size objects. It uses a slab-based approach, where memory is divided into pages,
/// and each page is further divided into fixed-size slots for objects. This allocator is
/// particularly useful for scenarios where frequent allocations and deallocations of objects
/// of the same size are required.
///
/// The memory `SlabAllocator` uses is allocated by the struct implmenting `PageAllocator` trait,
/// which is responsible for managing the allocation and deallocation of memory pages.
/// It must allocates memory sized in `PAGE_SIZE` bytes, and aligned in `PAGE_SIZE` bytes.
///
/// Each page managed by the `SlabAllocator` consists of the following components:
///
/// 1. **Page Header**:
///    - Located at the beginning of the page.
///    - Contains metadata about the page, such as:
///      - `next`: Pointer to the next page in the linked list of pages.
///      - `free`: Offset to the first free object in the free object list. The free objects are managed as a singly linked list.
///      - `count`: Number of currently allocated objects in the page.
///
/// 2. **Object Slots**:
///    - Rest of the page are filled with an array of object slots.
///    - Each object slot consists of:
///      - **Object Header**:
///        - Contains metadata for the object, such as:
///          - `magic`: Filled with `OBJECT_MAGIC` when the slot is allocated.
///          - `next`: Offset to the next free object in the free object list of the page. 0 when this slot is used.
///      - **Redzone (before payload)**:
///        - Two redzones (before and after the payload) are used to detect memory corruption.
///        - Filled with a predefined pattern (`REDZONE_FILL`) to ensure integrity.
///      - **Padding (before payload)**:
///        - Padding added to ensure proper alignment of the payload.
///      - **Payload**:
///        - The actual memory region allocated for the object.
///        - Its first byte is filled with a predefined pattern (`UNUSED_FILL`) when the slot is not used.
///      - **Redzone (after payload)**
///      - **Padding (tail)**:
///        - Padding added to ensure proper alignment of the next object header.
///
/// # Allocation & Deallocation Behavior
///
/// - When an object is allocated:
///   - The allocator checks if there are free objects available in the current page.
///   - If no free objects are available, a new page is allocated from the `PageAllocator`.
///   - The first free object is removed from the free list, and its metadata is updated to
///     mark it as allocated.
///   - A pointer to the payload region of the object is returned.
///
/// - When an object is deallocated:
///   - The allocator verifies the integrity of the object using the magic number and redzones.
///   - The object is marked as free and added back to the free list of its page.
///   - If the page becomes completely free, it may be deallocated and returned to the
///     `PageAllocator`.
///

use core::mem::{align_of, size_of, MaybeUninit};
use core::ptr::{NonNull, null_mut, write_bytes};
use core::slice::from_raw_parts;

pub const PAGE_SIZE: usize = 4096;
pub const REDZONE_SIZE: u16 = 16;

const OBJECT_MAGIC: u16 = 0x6b5c;
const REDZONE_FILL: u8 = 0xf1;
const UNUSED_FILL: u8 = 0xe2;

pub unsafe trait PageAllocator {
    // return value must be aligned in PAGE_SIZE
    fn allocate(&mut self) -> Option<NonNull<[u8; PAGE_SIZE]>>;
    // Safety: ptr is an address of an allocated page
    unsafe fn deallocate(&mut self, ptr: NonNull<[u8; PAGE_SIZE]>);
}

pub struct SlabAllocator<PA: PageAllocator> {
    payload_size: u16,
    payload_align: u16,
    front_size: u16,                // size between slot object's start and payload's start.
    object_size: u16,               // size of the total slot object.
    avail_start: PageHeader,        // dummy PageHeader, next pointer to the first available page in the available page list.
    page_allocator: PA,
}

#[repr(C)]
struct PageHeader {
    prev: *mut PageHeader,
    next: *mut PageHeader,
    free: u16,
    count: u16,
}

fn page_list_is_tail(page: &PageHeader) -> bool {
    page.next.is_null()
}

fn page_list_is_singleton(page: &PageHeader) -> bool {
    page.next.is_null() || page.prev.is_null()
}

// Safety: page is tail, new_page is valid and head
unsafe fn page_list_push_tail(page: &mut PageHeader, new_page: *mut PageHeader) {
    page.next = new_page;
    unsafe { (*new_page).prev = &mut *page; }
}

fn page_list_pop_next(page: &mut PageHeader) -> *mut PageHeader {
    let n = page.next;
    if !n.is_null() {
        unsafe {
            page.next = (*n).next;
            if !page.next.is_null() {
                (*page.next).prev = page;
            }
            (*n).prev = null_mut();
            (*n).next = null_mut();
        }
    }
    n
}

// Safety: new_page is valid and singleton
unsafe fn page_list_push_next(page: &mut PageHeader, new_page: *mut PageHeader) {
    unsafe {
        (*new_page).next = page.next;
        (*new_page).prev = page;
        if !page.next.is_null() {
            (*page.next).prev = new_page;
        }
        page.next = new_page;
    }
}

fn page_list_remove(page: &mut PageHeader) {
    unsafe {
        let n = page.next;
        let p = page.prev;
        if !n.is_null() { (*n).prev = p; }
        if !p.is_null() { (*p).next = n; }
        page.next = null_mut();
        page.prev = null_mut();
    }
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
            avail_start: PageHeader {
                prev: null_mut(),
                next: null_mut(),
                free: 0,
                count: 0,
            },
            page_allocator,
        }
    }

    pub fn alloc(&mut self) -> Option<NonNull<u8>> {
        if page_list_is_tail(&self.avail_start) {
            let new_page = self.alloc_page()?;
            unsafe {
                page_list_push_tail(&mut self.avail_start, new_page);
            }
        }

        Some(unsafe { self.alloc_from_free() })
    }

    // Safety: self.avail_page is not null
    unsafe fn alloc_from_free(&mut self) -> NonNull<u8> {
        let page = self.avail_start.next;
        unsafe {
            assert_ne!((*page).free, 0, "slab is poisoned");

            let addr = page as usize + (*page).free as usize;
            let object = addr as *mut ObjectHeader;

            assert_eq!((*object).magic, 0, "slab is poisoned");
            assert!(self.check_redzone(addr), "slab is poisoned");
            assert!(self.check_unused(addr), "dangling pointer exists or slab is poisoned");

            (*object).magic = OBJECT_MAGIC;
            (*page).count += 1;

            if (*object).next != 0 {
                (*page).free = (*object).next;
                (*object).next = 0;
            }
            else {
                (*page).free = 0;
                self.kick_full_page();
            }

            NonNull::new_unchecked((addr + self.front_size as usize) as *mut u8)
        }
    }

    unsafe fn kick_full_page(&mut self) {
        let kicked = page_list_pop_next(&mut self.avail_start);
        assert!(!kicked.is_null(), "slab is poisoned");
    }

    fn alloc_page(&mut self) -> Option<*mut PageHeader> {
        let mut offset = self.page_offset();

        let header_size = size_of::<ObjectHeader>();
        let right_offset = (self.front_size + self.payload_size) as usize;

        let addr = self.page_allocator.allocate()?.as_ptr() as usize;
        let header = {
            let header_uninit = addr as *mut MaybeUninit<PageHeader>;
            unsafe {
                (*header_uninit).write(PageHeader {
                    prev: null_mut(),
                    next: null_mut(),
                    free: offset,
                    count: 0,
                });
            }
            header_uninit as *mut PageHeader
        };

        unsafe {
            loop {
                let obj_addr = addr + offset as usize;
                let object = obj_addr as *mut ObjectHeader;

                (*object).magic = 0;
                write_bytes((obj_addr + header_size) as *mut u8, REDZONE_FILL, REDZONE_SIZE as usize);
                *((obj_addr + self.front_size as usize) as *mut u8) = UNUSED_FILL;
                write_bytes((obj_addr + right_offset) as *mut u8, REDZONE_FILL, REDZONE_SIZE as usize);

                offset += self.object_size;
                if offset + self.object_size <= PAGE_SIZE as u16 {
                    (*object).next = offset;
                }
                else {
                    (*object).next = 0;
                    break;
                }
            }
        }

        Some(header)
    }

    fn page_offset(&self) -> u16 {
        align_ceil(size_of::<PageHeader>() as u16, ObjectHeader::align(self.payload_align))
    }

    // Safety: ptr is currently allocated
    pub unsafe fn dealloc(&mut self, ptr: NonNull<u8>) {
        let payload_addr = ptr.as_ptr() as usize;
        let addr = payload_addr - self.front_size as usize;
        let ptr_header = addr as *mut ObjectHeader;

        unsafe {
            let header = &mut *ptr_header;
            assert!(header.magic == OBJECT_MAGIC && header.next == 0, "invalid dealloc() or slab is poisoned");
            assert!(self.check_redzone(addr), "invalid dealloc() or slab is poisoned");

            *(payload_addr as *mut u8) = UNUSED_FILL;

            let page = page_from_object(ptr_header);
            (*page).count -= 1;

            header.magic = 0;
            if (*page).free != 0 {
                header.next = (*page).free;
            }
            (*page).free = (addr - page as usize) as u16;

            if (*page).count == 0 {
                self.dealloc_page(page);
            }
            else if self.avail_start.next != page {
                self.insert_avail_page(page);
            }
        }
    }

    unsafe fn dealloc_page(&mut self, page: *mut PageHeader) {
        unsafe {
            page_list_remove(&mut *page);
            self.page_allocator.deallocate(NonNull::new_unchecked(page as *mut [u8; PAGE_SIZE]));
        }
    }

    unsafe fn insert_avail_page(&mut self, page: *mut PageHeader) {
        unsafe {
            assert!(page_list_is_singleton(&mut *page), "slab is poisoned");
            page_list_push_next(&mut self.avail_start, page);
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

#[cfg(test)]
mod test;
