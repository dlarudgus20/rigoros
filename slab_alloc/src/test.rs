use super::*;
use std::vec::Vec;
use std::alloc::{alloc_zeroed, dealloc, Layout};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use rand::seq::SliceRandom;

struct MockPageAllocator {
    layout: Layout,
    pages: Vec<NonNull<[u8; PAGE_SIZE]>>,
    deallocated: Vec<NonNull<[u8; PAGE_SIZE]>>,
}

impl MockPageAllocator {
    fn new() -> Self {
        Self {
            layout: Layout::from_size_align(PAGE_SIZE, PAGE_SIZE).unwrap(),
            pages: Vec::new(),
            deallocated: Vec::new(),
        }
    }
    fn is_vaild(&self, ptr: *mut u8) -> bool {
        self.pages.iter().any(|&p| p.as_ptr() as *mut u8 == ptr)
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
        //unsafe { dealloc(ptr.as_ptr() as *mut u8, self.layout); }
        self.pages.remove(index);
        for page in &self.pages {
            let ptr = page.as_ptr() as *const usize;
            for i in 0..PAGE_SIZE / core::mem::size_of::<usize>() {
                assert_ne!(unsafe { *ptr.add(i) }, ptr as usize);
            }
        }
        self.deallocated.push(ptr);
        unsafe { core::ptr::write_bytes(ptr.as_ptr() as *mut u8, 0xdd, PAGE_SIZE); }
    }
}

impl Drop for MockPageAllocator {
    fn drop(&mut self) {
        for page in &self.deallocated {
            let slice = unsafe { core::slice::from_raw_parts(page.as_ptr() as *const u8, PAGE_SIZE) };
            for i in 0..PAGE_SIZE {
                assert_eq!(slice[i], 0xdd);
            }
            unsafe { dealloc(page.as_ptr() as *mut u8, self.layout); }
        }
        for page in &self.pages {
            unsafe { dealloc(page.as_ptr() as *mut u8, self.layout); }
        }
    }
}

#[test]
fn test_slab_allocator_creation() {
    let page_allocator = MockPageAllocator::new();
    let slab_allocator = SlabAllocator::new(64, 8, page_allocator);
    assert_eq!(slab_allocator.payload_size, 64);
    assert_eq!(slab_allocator.payload_align, 8);
}

#[test]
fn test_slab_allocation() {
    let page_allocator = MockPageAllocator::new();
    let mut slab_allocator = SlabAllocator::new(64, 8, page_allocator);

    let ptr = slab_allocator.alloc();
    assert!(ptr.is_some());
}

#[test]
fn test_slab_deallocation() {
    let page_allocator = MockPageAllocator::new();
    let mut slab_allocator = SlabAllocator::new(64, 8, page_allocator);

    let ptr = slab_allocator.alloc().unwrap();
    unsafe {
        slab_allocator.dealloc(ptr);
    }
}

#[test]
fn test_multiple_allocations() {
    let page_allocator = MockPageAllocator::new();
    let mut slab_allocator = SlabAllocator::new(64, 8, page_allocator);

    let ptr1 = slab_allocator.alloc().unwrap();
    let ptr2 = slab_allocator.alloc().unwrap();
    assert_ne!(ptr1, ptr2);

    unsafe {
        slab_allocator.dealloc(ptr1);
        slab_allocator.dealloc(ptr2);
    }
}

#[test]
fn test_redzone_integrity() {
    let page_allocator = MockPageAllocator::new();
    let mut slab_allocator = SlabAllocator::new(64, 8, page_allocator);

    let ptr = slab_allocator.alloc().unwrap();
    let addr = ptr.as_ptr() as usize;

    unsafe {
        let redzone_offset = slab_allocator.front_size as usize - size_of::<ObjectHeader>();
        let redzone_start = addr - redzone_offset;
        let redzone_end = addr + 64;

        for i in 0..(REDZONE_SIZE as usize) {
            assert_eq!(*((redzone_start + i) as *const u8), REDZONE_FILL);
            assert_eq!(*((redzone_end + i) as *const u8), REDZONE_FILL);
        }

        slab_allocator.dealloc(ptr);
    }
}

#[test]
fn test_multiple_pages_allocation_and_deallocation() {
    let page_allocator = MockPageAllocator::new();
    let mut slab_allocator = SlabAllocator::new(64, 8, page_allocator);

    let mut allocated_ptrs = Vec::new();

    // Allocate enough objects to span multiple pages
    for _ in 0..(PAGE_SIZE / 64 * 3) {
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
fn test_random_allocation_and_deallocation_sequencial() {
    let page_allocator = MockPageAllocator::new();
    let mut slab_allocator = SlabAllocator::new(64, 8, page_allocator);

    let mut allocated_ptrs = Vec::new();

    let seed = rand::random();//15936561931664768008;
    let mut rng = SmallRng::seed_from_u64(seed);

    println!("Random seed: {}", seed);

    // Allocate enough objects to exceed 3 times the page size
    for _ in 0..(PAGE_SIZE / 64 * 4) {
        let ptr = slab_allocator.alloc().unwrap();
        allocated_ptrs.push(ptr);
        println!("Allocated: {:?}", ptr);
    }

    // Shuffle the allocated pointers to randomize deallocation order
    allocated_ptrs.shuffle(&mut rng);

    // Deallocate in random order
    for (i, &ptr) in allocated_ptrs.iter().enumerate() {
        unsafe {
            println!("Deallocating: {:?}   #{}", ptr, i);
            check_avail_list(&slab_allocator);
            slab_allocator.dealloc(ptr);
        }
    }

    // Ensure allocator can still allocate after random deallocation
    let ptr = slab_allocator.alloc();
    assert!(ptr.is_some());
}

fn check_avail_list(slab: &SlabAllocator<MockPageAllocator>) {
    let pa = &slab.page_allocator;
    let mut p = slab.avail_start.next;
    let mut pp = Vec::new();
    while !p.is_null() {
        if !pa.is_vaild(p as *mut u8) {
            panic!("corrupted avail_page list");
        }
        if pp.contains(&p) {
            panic!("circular loop in avail_page list");
        }
        pp.push(p);
        p = unsafe { (*p).next };
    }
}

#[test]
fn test_random_allocation_and_deallocation_interleaved() {
    let page_allocator = MockPageAllocator::new();
    let mut slab_allocator = SlabAllocator::new(64, 8, page_allocator);

    let mut allocated_ptrs = Vec::new();

    let seed = 7734348131707548111;//rand::random();
    let mut rng = SmallRng::seed_from_u64(seed);

    println!("Random seed: {}", seed);

    // Interleave allocation and deallocation
    let repeat = PAGE_SIZE / 64 * 4;
    for i in 0..repeat {
        if rng.random_bool((i as f64) / (repeat as f64)) && !allocated_ptrs.is_empty() {
            // Randomly deallocate an object if possible
            let index = rng.random_range(0..allocated_ptrs.len());
            let ptr: NonNull<u8> = allocated_ptrs.swap_remove(index);
            unsafe {
                slab_allocator.dealloc(ptr);
            }
        } else {
            // Allocate a new object
            let ptr = slab_allocator.alloc().unwrap();
            allocated_ptrs.push(ptr);
        }
        check_avail_list(&slab_allocator);
    }

    // Deallocate left pointers
    for ptr in allocated_ptrs {
        unsafe {
            slab_allocator.dealloc(ptr);
        }
        check_avail_list(&slab_allocator);
    }

    // Ensure allocator can still allocate after random deallocation
    let ptr = slab_allocator.alloc();
    assert!(ptr.is_some());
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
