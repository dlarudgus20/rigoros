#![cfg_attr(not(test), no_std)]
#![deny(unsafe_op_in_unsafe_fn)]

use core::marker::PhantomData;
use core::mem::{align_of, size_of};
use core::ptr::{NonNull, null_mut, write_bytes};
use core::slice::from_raw_parts;

pub const PAGE_SIZE: usize = 4096;
pub const REDZONE_SIZE: u16 = 16;

const EMPTY_MAGIC: u16 = 0x3a49;
const OBJECT_MAGIC: u16 = 0x6b5c;
const REDZONE_FILL: u8 = 0xf1;
const UNUSED_FILL: u8 = 0xf2;

pub unsafe trait PageAllocator {
    // return value must be aligned in PAGE_SIZE
    fn allocate(&mut self) -> Option<NonNull<[u8; PAGE_SIZE]>>;
    // Safety: ptr is an address of an allocated page
    unsafe fn deallocate(&mut self, ptr: NonNull<[u8; PAGE_SIZE]>);
}

pub struct SlabAllocator<T, PA: PageAllocator> {
    partial_list: PageList,
    page_allocator: PA,
    _phantom: PhantomData<T>,
}

unsafe impl<T, PA: PageAllocator> Send for SlabAllocator<T, PA> {}

struct PageList {
    head: *mut PageLink,
    tail: *mut PageLink,
}

#[repr(C)]
struct PageLink {
    next: *mut PageLink,
    prev: *mut PageLink,
}

#[repr(C)]
struct SlotPage<T> {
    link: PageLink,
    free_index: u16,
    alloc_count: u16,
    _phantom: PhantomData<T>,
}

#[repr(C)]
struct SlotObject<T> {
    magic: u16,
    next: u16,
    _phantom: PhantomData<T>,
}

const fn align_ceil(x: usize, align: usize) -> usize {
    let mask = align - 1;
    (x + mask) & !mask
}

impl<T> SlotObject<T> {
    const fn align_of() -> usize {
        let a = align_of::<SlotObject<T>>();
        let b = align_of::<T>();
        if a > b { a } else { b }
    }
    const fn redzone1_offset() -> usize {
        size_of::<SlotObject<T>>()
    }
    const fn redzone1_size() -> usize {
        Self::payload_offset() - Self::redzone1_offset()
    }
    const fn payload_offset() -> usize {
        align_ceil(Self::redzone1_offset() + REDZONE_SIZE as usize, align_of::<T>())
    }
    const fn redzone2_offset() -> usize {
        Self::payload_offset() + size_of::<T>()
    }
    const fn redzone2_size() -> usize {
        Self::size_of() - Self::redzone2_offset()
    }
    const fn size_of() -> usize {
        align_ceil(Self::redzone2_offset() + REDZONE_SIZE as usize, Self::align_of())
    }

    fn init(&mut self) {
        let raw = self as *mut Self as *mut u8;
        self.magic = EMPTY_MAGIC;
        self.next = 0;
        unsafe {
            write_bytes(raw.add(Self::redzone1_offset()), REDZONE_FILL, Self::redzone1_size());
            write_bytes(raw.add(Self::payload_offset()), UNUSED_FILL, size_of::<T>());
            write_bytes(raw.add(Self::redzone2_offset()), REDZONE_FILL, Self::redzone2_size());
        }
    }

    fn check_redzone(&self) -> bool {
        let raw = self as *const Self as *const u8;
        let redzone1 = unsafe {
            from_raw_parts(raw.add(Self::redzone1_offset()), Self::redzone1_size())
        };
        let redzone2 = unsafe {
            from_raw_parts(raw.add(Self::redzone2_offset()), Self::redzone2_size())
        };

        redzone1.iter().all(|&b| b == REDZONE_FILL) &&
            redzone2.iter().all(|&b| b == REDZONE_FILL)
    }

    fn check_unused(&self) -> bool {
        let raw = self as *const Self as *const u8;
        let payload = unsafe {
            from_raw_parts(raw.add(Self::payload_offset()), size_of::<T>())
        };

        payload.iter().all(|&b| b == UNUSED_FILL)
    }

    fn write_unused(&mut self, b: u8) {
        let raw = self as *mut Self as *mut u8;
        unsafe {
            write_bytes(raw.add(Self::payload_offset()), b, size_of::<T>());
        }
    }

    fn payload(&mut self) -> NonNull<T> {
        let raw = self as *mut Self as *mut u8;
        unsafe { NonNull::new_unchecked(raw.add(Self::payload_offset()) as *mut T) }
    }

    fn on_alloc(&mut self) {
        assert!(self.magic == EMPTY_MAGIC && self.next == 0, "slab is poisoned");
        assert!(self.check_redzone(), "redzone is corrupted");
        assert!(self.check_unused(), "slab is poisoned");
        (*self).magic = OBJECT_MAGIC;
        (*self).write_unused(0);
    }

    fn on_dealloc(&mut self) {
        assert!(self.magic == OBJECT_MAGIC && self.next == 0, "try to deallocate an object that is not allocated");
        assert!(self.check_redzone(), "redzone is corrupted");
        (*self).magic = EMPTY_MAGIC;
        (*self).write_unused(UNUSED_FILL);
    }

    // Safety: `self` is a valid object inside page
    unsafe fn page_from_object(&mut self) -> *mut SlotPage<T> {
        let raw = self as *mut SlotObject<T> as usize;
        (raw & !(PAGE_SIZE as usize - 1)) as *mut SlotPage<T>
    }
}

impl PageLink {
    fn null() -> Self {
        PageLink {
            next: null_mut(),
            prev: null_mut(),
        }
    }
}

impl PageList {
    fn new() -> Self {
        PageList {
            head: null_mut(),
            tail: null_mut(),
        }
    }

    fn assign_singleton(&mut self, link: &mut PageLink) {
        link.next = null_mut();
        link.prev = null_mut();
        self.head = link;
        self.tail = link;
    }

    fn push_back(&mut self, link: &mut PageLink) {
        link.next = null_mut();
        link.prev = self.tail;
        if !self.tail.is_null() {
            unsafe { (*self.tail).next = link; }
        }
        else if self.head.is_null() {
            self.head = link;
        }
        self.tail = link;
    }

    // Safety: `link` must be a valid link in the list
    unsafe fn remove(&mut self, link: &mut PageLink) {
        if link.prev.is_null() {
            self.head = link.next;
        } else {
            unsafe { (*link.prev).next = link.next; }
        }

        if link.next.is_null() {
            self.tail = link.prev;
        } else {
            unsafe { (*link.next).prev = link.prev; }
        }

        link.next = null_mut();
        link.prev = null_mut();
    }
}

impl<T> SlotPage<T> {
    const SIZE_ASSERT: () = assert!(Self::object_offset() + SlotObject::<T>::size_of() <= PAGE_SIZE, "object size is too big for a page");

    const fn object_offset() -> usize {
        align_ceil(size_of::<SlotPage<T>>(), SlotObject::<T>::align_of())
    }

    // Safety: `addr` must be aligned to PAGE_SIZE and point to a valid memory region sized of PAGE_SIZE
    unsafe fn init(addr: usize) -> *mut SlotPage<T> {
        let header = addr as *mut SlotPage<T>;

        let mut offset = Self::object_offset();

        unsafe {
            core::ptr::write(header, SlotPage {
                link: PageLink::null(),
                free_index: offset as u16,
                alloc_count: 0,
                _phantom: PhantomData,
            });

            loop {
                let obj_addr = addr + offset;
                let obj_size = SlotObject::<T>::size_of();
                let next_offset = offset + obj_size;

                let obj = obj_addr as *mut SlotObject<T>;
                (*obj).init();

                if next_offset + obj_size < PAGE_SIZE {
                    (*obj).next = next_offset as u16;
                    offset = next_offset;
                } else {
                    (*obj).next = 0;
                    break;
                }
            }
        }
        header
    }

    fn pop_front_object(&mut self) -> (*mut SlotObject<T>, bool) {
        assert!(self.free_index != 0, "slab is corrupted: try to pop object from an fully-allocated page");

        let page_addr = self as *mut SlotPage<T> as usize;
        let obj_addr = page_addr + self.free_index as usize;
        let obj = obj_addr as *mut SlotObject<T>;

        unsafe {
            let full =
                if (*obj).next == 0 {
                    self.free_index = 0;
                    true
                } else {
                    self.free_index = (*obj).next;
                    false
                };

            (*obj).next = 0;
            self.alloc_count += 1;
            (obj, full)
        }
    }

    // Safety: `obj` must be a valid object inside page
    unsafe fn push_front_object(&mut self, obj: *mut SlotObject<T>) {
        let page_addr = self as *mut SlotPage<T> as usize;
        let obj_addr = obj as usize;
        unsafe { (*obj).next = self.free_index; }
        self.free_index = (obj_addr - page_addr) as u16;
        self.alloc_count -= 1;
    }
}

impl<T, PA: PageAllocator> SlabAllocator<T, PA> {
    pub fn new(page_allocator: PA) -> Self {
        let _ = SlotPage::<T>::SIZE_ASSERT; // Ensure that the object fits in a page
        Self {
            partial_list: PageList::new(),
            page_allocator,
            _phantom: PhantomData,
        }
    }

    pub fn alloc(&mut self) -> Option<NonNull<T>> {
        if self.partial_list.head.is_null() {
            self.alloc_page()?;
        }

        let page = unsafe { &mut *(self.partial_list.head as *mut SlotPage<T>) };
        let (obj, full) = page.pop_front_object();

        if full {
            unsafe { self.partial_list.remove(&mut page.link); }
        }

        unsafe {
            (*obj).on_alloc();
            Some((*obj).payload())
        }
    }

    fn alloc_page(&mut self) -> Option<()> {
        let page_ptr = self.page_allocator.allocate()?;
        let page_addr = page_ptr.as_ptr() as usize;
        unsafe {
            let page = &mut *SlotPage::<T>::init(page_addr);
            self.partial_list.assign_singleton(&mut page.link);
        }
        Some(())
    }

    // Safety: `ptr` must be a valid pointer to an allocated object
    pub unsafe fn dealloc(&mut self, ptr: NonNull<T>) {
        let payload_addr = ptr.as_ptr() as usize;
        let obj_addr = payload_addr - SlotObject::<T>::payload_offset();

        unsafe {
            let obj = obj_addr as *mut SlotObject<T>;
            (*obj).on_dealloc();

            let page = (*obj).page_from_object();
            let was_full = (*page).free_index == 0;
            (*page).push_front_object(obj);

            if (*page).alloc_count == 0 {
                if !was_full {
                    self.partial_list.remove(&mut (*page).link);
                }
                self.page_allocator.deallocate(NonNull::new_unchecked(page as *mut [u8; PAGE_SIZE]));
            }
            else if was_full {
                self.partial_list.push_back(&mut (*page).link);
            }
        }
    }
}

#[cfg(test)]
mod test_slab;

#[cfg(test)]
mod test_pagelist;
