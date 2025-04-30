use slab_alloc::{SlabAllocator, PageAllocator, PAGE_SIZE};
use std::ptr::NonNull;
use std::alloc::{alloc_zeroed, dealloc, Layout};
use std::vec::Vec;

struct MockPageAllocator {
    layout: Layout,
    pages: Vec<NonNull<[u8; PAGE_SIZE]>>,
}

impl MockPageAllocator {
    fn new() -> Self {
        Self {
            layout: Layout::from_size_align(PAGE_SIZE, PAGE_SIZE).unwrap(),
            pages: Vec::new(),
        }
    }
}

unsafe impl PageAllocator for MockPageAllocator {
    fn allocate(&mut self) -> Option<NonNull<[u8; PAGE_SIZE]>> {
        let page = unsafe { alloc_zeroed(self.layout) as *mut [u8; PAGE_SIZE] };
        let ptr = NonNull::new(page).unwrap();
        self.pages.push(ptr);
        Some(ptr)
    }

    unsafe fn deallocate(&mut self, ptr: NonNull<[u8; PAGE_SIZE]>) {
        let index = self.pages.iter().position(|&p| p == ptr).unwrap();
        unsafe { dealloc(ptr.as_ptr() as *mut u8, self.layout); }
        self.pages.remove(index);
    }
}

impl Drop for MockPageAllocator {
    fn drop(&mut self) {
        for page in &self.pages {
            unsafe { dealloc(page.as_ptr() as *mut u8, self.layout); }
        }
    }
}

#[test]
fn test_large_allocation_and_deallocation() {
    let page_allocator = MockPageAllocator::new();
    let mut slab_allocator = SlabAllocator::new(128, 16, page_allocator);

    let mut allocated_ptrs = Vec::new();

    // Allocate enough objects to span multiple pages
    for _ in 0..(PAGE_SIZE / 128 * 5) {
        let ptr = slab_allocator.alloc().unwrap();
        allocated_ptrs.push(ptr);
    }

    // Ensure all pointers are unique
    for i in 0..allocated_ptrs.len() {
        for j in (i + 1)..allocated_ptrs.len() {
            assert_ne!(allocated_ptrs[i], allocated_ptrs[j]);
        }
    }

    // Deallocate all objects
    for ptr in allocated_ptrs {
        unsafe {
            slab_allocator.dealloc(ptr);
        }
    }

    // Ensure allocator can reuse pages after deallocation
    let ptr = slab_allocator.alloc();
    assert!(ptr.is_some());
}

#[test]
fn test_interleaved_allocation_and_deallocation() {
    let page_allocator = MockPageAllocator::new();
    let mut slab_allocator = SlabAllocator::new(64, 8, page_allocator);

    let mut allocated_ptrs = Vec::new();

    // Interleave allocation and deallocation
    for i in 0..100 {
        if i % 3 == 0 && !allocated_ptrs.is_empty() {
            let ptr = allocated_ptrs.pop().unwrap();
            unsafe {
                slab_allocator.dealloc(ptr);
            }
        } else {
            let ptr = slab_allocator.alloc().unwrap();
            allocated_ptrs.push(ptr);
        }
    }

    // Deallocate remaining objects
    for ptr in allocated_ptrs {
        unsafe {
            slab_allocator.dealloc(ptr);
        }
    }

    // Ensure allocator can still allocate after interleaved operations
    let ptr = slab_allocator.alloc();
    assert!(ptr.is_some());
}

#[test]
fn test_fragmentation_handling() {
    let page_allocator = MockPageAllocator::new();
    let mut slab_allocator = SlabAllocator::new(128, 16, page_allocator);

    let mut allocated_ptrs = Vec::new();

    // Allocate and deallocate in a pattern to create fragmentation
    for i in 0..50 {
        let ptr = slab_allocator.alloc().unwrap();
        if i % 2 == 0 {
            unsafe {
                slab_allocator.dealloc(ptr);
            }
        } else {
            allocated_ptrs.push(ptr);
        }
    }

    // Deallocate remaining objects
    for ptr in allocated_ptrs {
        unsafe {
            slab_allocator.dealloc(ptr);
        }
    }

    // Ensure allocator can still allocate after fragmentation
    let ptr = slab_allocator.alloc();
    assert!(ptr.is_some());
}

#[test]
fn test_stress_allocation_and_deallocation() {
    let page_allocator = MockPageAllocator::new();
    let mut slab_allocator = SlabAllocator::new(256, 32, page_allocator);

    let mut allocated_ptrs = Vec::new();

    // Stress test with a large number of allocations and deallocations
    for i in 0..1000 {
        if i % 5 == 0 && !allocated_ptrs.is_empty() {
            let ptr = allocated_ptrs.pop().unwrap();
            unsafe {
                slab_allocator.dealloc(ptr);
            }
        } else {
            let ptr = slab_allocator.alloc().unwrap();
            allocated_ptrs.push(ptr);
        }
    }

    // Deallocate remaining objects
    for ptr in allocated_ptrs {
        unsafe {
            slab_allocator.dealloc(ptr);
        }
    }

    // Ensure allocator can still allocate after stress test
    let ptr = slab_allocator.alloc();
    assert!(ptr.is_some());
}
