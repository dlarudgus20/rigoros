use super::*;
use std::vec::Vec;
use std::alloc::{alloc_zeroed, dealloc, Layout};
use std::panic::{catch_unwind, AssertUnwindSafe};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use rand::seq::SliceRandom;

struct MockPageAllocator {
    layout: Layout,
    pages: Vec<NonNull<[u8; PAGE_SIZE]>>,
    deallocated: Vec<NonNull<[u8; PAGE_SIZE]>>,
    on_after_alloc: Option<fn (&mut MockPageAllocator) -> ()>,
    on_before_dealloc: Option<fn (&mut MockPageAllocator) -> ()>,
}

impl MockPageAllocator {
    fn new() -> Self {
        Self {
            layout: Layout::from_size_align(PAGE_SIZE, PAGE_SIZE).unwrap(),
            pages: Vec::new(),
            deallocated: Vec::new(),
            on_after_alloc: None,
            on_before_dealloc: None,
        }
    }

    #[allow(dead_code)]
    fn is_vaild(&self, ptr: *mut u8) -> bool {
        self.pages.iter().any(|&p| p.as_ptr() as *mut u8 == ptr)
    }
}

unsafe impl PageAllocator for MockPageAllocator {
    fn allocate(&mut self) -> Option<NonNull<[u8; PAGE_SIZE]>> {
        let page = unsafe { alloc_zeroed(self.layout) as *mut [u8; PAGE_SIZE] };
        let ptr = NonNull::new(page).unwrap();
        self.pages.push(ptr);
        self.on_after_alloc.map(|f| f(self));
        Some(ptr)
    }

    unsafe fn deallocate(&mut self, ptr: NonNull<[u8; PAGE_SIZE]>) {
        self.on_before_dealloc.map(|f| f(self));
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
    #[repr(align(8))]
    struct Chunk { _data: [u8; 64] }
    let _slab_allocator: SlabAllocator<Chunk, _> = SlabAllocator::new(page_allocator);
}

#[test]
fn test_slab_alloc_dealloc_once() {
    let page_allocator = MockPageAllocator::new();
    #[repr(align(8))]
    struct Chunk { _data: [u8; 64] }
    let mut slab_allocator: SlabAllocator<Chunk, _> = SlabAllocator::new(page_allocator);

    // Allocate a chunk
    let chunk = slab_allocator.alloc();
    assert!(chunk.is_some());

    // Deallocate the chunk
    if let Some(ptr) = chunk {
        unsafe { slab_allocator.dealloc(ptr); }
    }
}

#[test]
fn test_slab_alignment_8() {
    let page_allocator = MockPageAllocator::new();
    #[repr(align(8))]
    struct Chunk8 { _data: [u8; 24] }
    let mut slab_allocator: SlabAllocator<Chunk8, _> = SlabAllocator::new(page_allocator);

    let ptr = slab_allocator.alloc().unwrap();
    assert_eq!((ptr.as_ptr() as usize) % 8, 0, "Pointer is not 8-byte aligned");
    unsafe { slab_allocator.dealloc(ptr); }
}

#[test]
fn test_slab_alignment_16() {
    let page_allocator = MockPageAllocator::new();
    #[repr(align(16))]
    struct Chunk16 { _data: [u8; 32] }
    let mut slab_allocator: SlabAllocator<Chunk16, _> = SlabAllocator::new(page_allocator);

    let ptr = slab_allocator.alloc().unwrap();
    assert_eq!((ptr.as_ptr() as usize) % 16, 0, "Pointer is not 16-byte aligned");
    unsafe { slab_allocator.dealloc(ptr); }
}

#[test]
fn test_slab_alignment_64() {
    let page_allocator = MockPageAllocator::new();
    #[repr(align(64))]
    struct Chunk64 { _data: [u8; 128] }
    let mut slab_allocator: SlabAllocator<Chunk64, _> = SlabAllocator::new(page_allocator);

    let ptr = slab_allocator.alloc().unwrap();
    assert_eq!((ptr.as_ptr() as usize) % 64, 0, "Pointer is not 64-byte aligned");
    unsafe { slab_allocator.dealloc(ptr); }
}

#[test]
fn test_slab_alignment_multiple_chunks() {
    let page_allocator = MockPageAllocator::new();
    #[repr(align(32))]
    struct Chunk32 { _data: [u8; 64] }
    let mut slab_allocator: SlabAllocator<Chunk32, _> = SlabAllocator::new(page_allocator);

    let mut ptrs = Vec::new();
    for _ in 0..10 {
        let ptr = slab_allocator.alloc().unwrap();
        assert_eq!((ptr.as_ptr() as usize) % 32, 0, "Pointer is not 32-byte aligned");
        ptrs.push(ptr);
    }
    for ptr in ptrs {
        unsafe { slab_allocator.dealloc(ptr); }
    }
}

#[test]
fn test_slab_alignment_random_sizes() {
    let page_allocator = MockPageAllocator::new();
    #[repr(align(128))]
    struct Chunk128 { _data: [u8; 200] }
    let mut slab_allocator: SlabAllocator<Chunk128, _> = SlabAllocator::new(page_allocator);

    let mut ptrs = Vec::new();
    for _ in 0..5 {
        let ptr = slab_allocator.alloc().unwrap();
        assert_eq!((ptr.as_ptr() as usize) % 128, 0, "Pointer is not 128-byte aligned");
        ptrs.push(ptr);
    }
    for ptr in ptrs {
        unsafe { slab_allocator.dealloc(ptr); }
    }
}
/*
#[test]
fn test_slab_alloc_all_alignment() {
    let page_allocator = MockPageAllocator::new();
    #[repr(align(32))]
    struct Chunk {
        _data: [u8; 32],
    }
    let mut slab_allocator: SlabAllocator<Chunk, _> = SlabAllocator::new(page_allocator);

    let chunks_per_page = PAGE_SIZE / std::mem::size_of::<Chunk>();
    let mut ptrs = Vec::new();
    for _ in 0..chunks_per_page {
        let ptr = slab_allocator.alloc().unwrap();
        assert_eq!((ptr.as_ptr() as usize) % 32, 0, "Pointer is not 32-byte aligned");
        ptrs.push(ptr);
    }
    for ptr in ptrs {
        slab_allocator.dealloc(ptr);
    }
}*/
#[test]
fn test_slab_exhaustion_and_reuse() {
    let page_allocator = MockPageAllocator::new();
    #[repr(align(16))]
    struct Chunk { _data: [u8; 32] }
    let mut slab_allocator: SlabAllocator<Chunk, _> = SlabAllocator::new(page_allocator);

    let chunks_per_page = PAGE_SIZE / std::mem::size_of::<Chunk>();
    let mut ptrs = Vec::new();

    // Allocate all possible chunks in a page
    for _ in 0..chunks_per_page {
        let ptr = slab_allocator.alloc().expect("Should allocate chunk");
        ptrs.push(ptr);
    }

    // Allocating one more should allocate a new page (should not panic or return None)
    let extra = slab_allocator.alloc();
    assert!(extra.is_some(), "Should allocate chunk from new page");

    // Deallocate all and ensure reuse
    for ptr in ptrs {
        unsafe { slab_allocator.dealloc(ptr); }
    }
    if let Some(ptr) = extra {
        unsafe { slab_allocator.dealloc(ptr); }
    }

    // After deallocation, allocation should succeed again
    let ptr = slab_allocator.alloc();
    assert!(ptr.is_some());
    unsafe { slab_allocator.dealloc(ptr.unwrap()); }
}

#[test]
fn test_slab_random_alloc_dealloc_pattern() {
    let page_allocator = MockPageAllocator::new();
    #[repr(align(8))]
    struct Chunk { _data: [u8; 40] }
    let mut slab_allocator: SlabAllocator<Chunk, _> = SlabAllocator::new(page_allocator);

    let mut rng = SmallRng::seed_from_u64(42);
    let mut ptrs = Vec::new();

    // Randomly allocate and deallocate
    for _ in 0..100 {
        if rng.random_bool(0.6) || ptrs.is_empty() {
            if let Some(ptr) = slab_allocator.alloc() {
                ptrs.push(ptr);
            }
        } else {
            let idx = rng.random_range(0..ptrs.len());
            let ptr = ptrs.swap_remove(idx);
            unsafe { slab_allocator.dealloc(ptr); }
        }
    }

    // Clean up
    for ptr in ptrs {
        unsafe { slab_allocator.dealloc(ptr); }
    }
}

#[test]
fn test_slab_null_alloc_returns_none() {
    struct FailingAllocator;
    unsafe impl PageAllocator for FailingAllocator {
        fn allocate(&mut self) -> Option<NonNull<[u8; PAGE_SIZE]>> {
            None
        }
        unsafe fn deallocate(&mut self, _ptr: NonNull<[u8; PAGE_SIZE]>) {}
    }

    #[repr(align(8))]
    struct Chunk {
        _data: [u8; 64],
    }
    let mut slab_allocator: SlabAllocator<Chunk, FailingAllocator> = SlabAllocator::new(FailingAllocator);

    let ptr = slab_allocator.alloc();
    assert!(ptr.is_none(), "Should return None if page allocation fails");
}

#[test]
fn test_slab_large_chunk() {
    let page_allocator = MockPageAllocator::new();
    #[repr(align(8))]
    struct LargeChunk { _data: [u8; PAGE_SIZE / 2] }
    let mut slab_allocator: SlabAllocator<LargeChunk, _> = SlabAllocator::new(page_allocator);

    // Only two chunks should fit in a page
    let ptr1 = slab_allocator.alloc();
    let ptr2 = slab_allocator.alloc();
    assert!(ptr1.is_some());
    assert!(ptr2.is_some());
    let ptr3 = slab_allocator.alloc();
    assert!(ptr3.is_some(), "Should allocate from a new page");

    // Clean up
    unsafe {
        slab_allocator.dealloc(ptr1.unwrap());
        slab_allocator.dealloc(ptr2.unwrap());
        slab_allocator.dealloc(ptr3.unwrap());
    }
}

#[test]
fn test_slab_interleaved_alloc_dealloc_multiple_types() {
    let page_allocator_a = MockPageAllocator::new();
    let page_allocator_b = MockPageAllocator::new();

    #[repr(align(16))]
    struct ChunkA { _data: [u8; 48] }
    #[repr(align(32))]
    struct ChunkB { _data: [u8; 64] }

    let mut slab_a: SlabAllocator<ChunkA, _> = SlabAllocator::new(page_allocator_a);
    let mut slab_b: SlabAllocator<ChunkB, _> = SlabAllocator::new(page_allocator_b);

    let mut ptrs_a = Vec::new();
    let mut ptrs_b = Vec::new();

    // Interleaved allocation
    for i in 0..20 {
        if i % 2 == 0 {
            ptrs_a.push(slab_a.alloc().unwrap());
        } else {
            ptrs_b.push(slab_b.alloc().unwrap());
        }
    }

    // Interleaved deallocation
    for (a, b) in ptrs_a.drain(..).zip(ptrs_b.drain(..)) {
        unsafe { 
            slab_a.dealloc(a);
            slab_b.dealloc(b);
        }
    }
}

#[test]
fn test_slab_stress_many_pages() {
    let page_allocator = MockPageAllocator::new();
    #[repr(align(8))]
    struct Chunk { _data: [u8; 32] }
    let mut slab: SlabAllocator<Chunk, _> = SlabAllocator::new(page_allocator);

    let chunks_per_page = PAGE_SIZE / std::mem::size_of::<Chunk>();
    let total_chunks = chunks_per_page * 50; // 50 pages

    let mut ptrs = Vec::with_capacity(total_chunks);
    for _ in 0..total_chunks {
        ptrs.push(slab.alloc().unwrap());
    }
    for ptr in ptrs {
        unsafe { slab.dealloc(ptr); }
    }
}

#[test]
fn test_slab_repeated_alloc_dealloc_cycles() {
    let page_allocator = MockPageAllocator::new();
    #[repr(align(16))]
    struct Chunk { _data: [u8; 48] }
    let mut slab: SlabAllocator<Chunk, _> = SlabAllocator::new(page_allocator);

    let chunks_per_page = PAGE_SIZE / std::mem::size_of::<Chunk>();
    for _ in 0..10 {
        let mut ptrs = Vec::new();
        for _ in 0..chunks_per_page * 3 {
            ptrs.push(slab.alloc().unwrap());
        }
        ptrs.shuffle(&mut rand::rng());
        for ptr in ptrs {
            unsafe { slab.dealloc(ptr); }
        }
    }
}

#[test]
fn test_slab_double_free_panics() {
    let page_allocator = MockPageAllocator::new();
    #[repr(align(8))]
    struct Chunk { _data: [u8; 32] }
    let mut slab: SlabAllocator<Chunk, _> = SlabAllocator::new(page_allocator);

    let ptr = slab.alloc().unwrap();
    unsafe { slab.dealloc(ptr); }
    // Double free should panic or cause debug assertion failure
    let result = catch_unwind(AssertUnwindSafe(|| {
        unsafe { slab.dealloc(ptr); }
    }));
    assert!(result.is_err(), "Double free should panic or fail");
}

#[test]
fn test_slab_alloc_dealloc_pattern_with_gaps() {
    let page_allocator = MockPageAllocator::new();
    #[repr(align(8))]
    struct Chunk { _data: [u8; 24] }
    let mut slab: SlabAllocator<Chunk, _> = SlabAllocator::new(page_allocator);

    let mut ptrs = Vec::new();
    for _ in 0..30 {
        ptrs.push(slab.alloc().unwrap());
    }
    // Deallocate every third chunk
    for i in (0..ptrs.len()).step_by(3) {
        unsafe { slab.dealloc(ptrs[i]); }
    }
    // Allocate again, should reuse freed slots
    let mut new_ptrs = Vec::new();
    for _ in 0..10 {
        new_ptrs.push(slab.alloc().unwrap());
    }
    // Clean up
    for (i, ptr) in ptrs.into_iter().enumerate() {
        if i % 3 != 0 {
            unsafe { slab.dealloc(ptr); }
        }
    }
    for ptr in new_ptrs {
        unsafe { slab.dealloc(ptr); }
    }
}

#[test]
fn test_slab_fragmentation_and_reuse() {
    let page_allocator = MockPageAllocator::new();
    #[repr(align(8))]
    struct Chunk { _data: [u8; 40] }
    let mut slab: SlabAllocator<Chunk, _> = SlabAllocator::new(page_allocator);

    let mut ptrs = Vec::new();
    for _ in 0..100 {
        ptrs.push(slab.alloc().unwrap());
    }
    // Free every even-indexed chunk
    for i in (0..ptrs.len()).step_by(2) {
        unsafe { slab.dealloc(ptrs[i]); }
    }
    // Allocate again, should fill freed slots
    let mut reused = Vec::new();
    for _ in 0..50 {
        reused.push(slab.alloc().unwrap());
    }
    // Clean up
    for i in (1..ptrs.len()).step_by(2) {
        unsafe { slab.dealloc(ptrs[i]); }
    }
    for ptr in reused {
        unsafe { slab.dealloc(ptr); }
    }
}

#[test]
fn test_slab_old_multiple_pages_allocation_and_deallocation() {
    let page_allocator = MockPageAllocator::new();
    #[repr(align(8))]
    struct Chunk { _data: [u8; 64] }
    let mut slab_allocator: SlabAllocator<Chunk, _> = SlabAllocator::new(page_allocator);

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
fn test_slab_old_random_allocation_and_deallocation_sequencial() {
    let page_allocator = MockPageAllocator::new();
    #[repr(align(8))]
    struct Chunk { _data: [u8; 64] }
    let mut slab_allocator: SlabAllocator<Chunk, _> = SlabAllocator::new(page_allocator);

    let mut allocated_ptrs = Vec::new();

    let seed = rand::random();//15936561931664768008;
    let mut rng = SmallRng::seed_from_u64(seed);

    println!("Random seed: {}", seed);

    // Allocate enough objects to exceed 3 times the page size
    for _ in 0..(PAGE_SIZE / 64 * 4) {
        let ptr = slab_allocator.alloc().unwrap();
        allocated_ptrs.push(ptr);
    }

    // Shuffle the allocated pointers to randomize deallocation order
    allocated_ptrs.shuffle(&mut rng);

    // Deallocate in random order
    for ptr in allocated_ptrs {
        unsafe {
            slab_allocator.dealloc(ptr);
        }
    }

    // Ensure allocator can still allocate after random deallocation
    let ptr = slab_allocator.alloc();
    assert!(ptr.is_some());
}

#[test]
fn test_slab_old_random_allocation_and_deallocation_interleaved() {
    let page_allocator = MockPageAllocator::new();
    #[repr(align(8))]
    struct Chunk { _data: [u8; 64] }
    let mut slab_allocator: SlabAllocator<Chunk, MockPageAllocator> = SlabAllocator::new(page_allocator);

    let mut allocated_ptrs = Vec::new();

    let seed = rand::random();//7734348131707548111;
    let mut rng = SmallRng::seed_from_u64(seed);

    println!("Random seed: {}", seed);

    // Interleave allocation and deallocation
    let repeat = PAGE_SIZE / 64 * 4;
    for i in 0..repeat {
        if rng.random_bool((i as f64) / (repeat as f64)) && !allocated_ptrs.is_empty() {
            // Randomly deallocate an object if possible
            let index = rng.random_range(0..allocated_ptrs.len());
            let ptr = allocated_ptrs.swap_remove(index);
            unsafe {
                slab_allocator.dealloc(ptr);
            }
        } else {
            // Allocate a new object
            let ptr = slab_allocator.alloc().unwrap();
            allocated_ptrs.push(ptr);
        }
    }

    // Deallocate left pointers
    for ptr in allocated_ptrs {
        unsafe {
            slab_allocator.dealloc(ptr);
        }
    }

    // Ensure allocator can still allocate after random deallocation
    let ptr = slab_allocator.alloc();
    assert!(ptr.is_some());
}

#[test]
fn test_slab_old_large_allocation_and_deallocation() {
    let page_allocator = MockPageAllocator::new();
    #[repr(align(16))]
    struct Chunk { _data: [u8; 128] }
    let mut slab_allocator: SlabAllocator<Chunk, _> = SlabAllocator::new(page_allocator);

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
fn test_slab_old_interleaved_allocation_and_deallocation() {
    let page_allocator = MockPageAllocator::new();
    #[repr(align(8))]
    struct Chunk { _data: [u8; 64] }
    let mut slab_allocator: SlabAllocator<Chunk, _> = SlabAllocator::new(page_allocator);

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
fn test_slab_old_fragmentation_handling() {
    let page_allocator = MockPageAllocator::new();
    #[repr(align(16))]
    struct Chunk { _data: [u8; 128] }
    let mut slab_allocator: SlabAllocator<Chunk, _> = SlabAllocator::new(page_allocator);

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
fn test_slab_old_stress_allocation_and_deallocation() {
    let page_allocator = MockPageAllocator::new();
    #[repr(align(32))]
    struct Chunk { _data: [u8; 256] }
    let mut slab_allocator: SlabAllocator<Chunk, _> = SlabAllocator::new(page_allocator);

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

// These tests are intended to fail in compilation.
/*
#[test]
fn test_slab_object_too_large_for_page() {
    let page_allocator = MockPageAllocator::new();
    const MINIMAL_METADATA_SIZE: usize = 60;
    #[repr(align(1))]
    struct HugeChunk { _data: [u8; PAGE_SIZE - MINIMAL_METADATA_SIZE + 1] }
    SlabAllocator::<HugeChunk, _>::new(page_allocator);
}
#[test]
fn test_slab_object_too_large_for_page_by_align() {
    let page_allocator = MockPageAllocator::new();
    #[repr(align(128))]
    struct HugeChunk { _data: [u8; PAGE_SIZE - 128 * 3 + 1] }
    SlabAllocator::<HugeChunk, _>::new(page_allocator);
}
*/

#[test]
fn test_slab_redzone_detection_on_overflow() {
    let page_allocator = MockPageAllocator::new();
    #[repr(align(8))]
    struct Chunk { _data: [u8; 32] }
    let mut slab: SlabAllocator<Chunk, _> = SlabAllocator::new(page_allocator);

    let ptr = slab.alloc().unwrap();
    let chunk_ptr = ptr.as_ptr() as *mut u8;

    // Write past the end of the chunk to trigger redzone detection.
    // The redzone is assumed to be immediately after the chunk.
    let result = catch_unwind(AssertUnwindSafe(|| unsafe {
        // Write 1 byte past the end of the chunk
        *chunk_ptr.add(std::mem::size_of::<Chunk>()) = 0xAA;
        slab.dealloc(ptr);
    }));
    assert!(result.is_err(), "Redzone overflow should panic or fail");
}

#[test]
fn test_slab_redzone_detection_on_underflow() {
    let page_allocator = MockPageAllocator::new();
    #[repr(align(8))]
    struct Chunk { _data: [u8; 32] }
    let mut slab: SlabAllocator<Chunk, _> = SlabAllocator::new(page_allocator);

    let ptr = slab.alloc().unwrap();
    let chunk_ptr = ptr.as_ptr() as *mut u8;

    let result = catch_unwind(AssertUnwindSafe(|| unsafe {
        // Write 1 byte before the start of the chunk
        *chunk_ptr.offset(-1) = 0xBB;
        slab.dealloc(ptr);
    }));
    assert!(result.is_err(), "Redzone underflow should panic or fail");
}

#[test]
fn test_slab_redzone_integrity_on_normal_use() {
    // This test ensures that normal allocation and deallocation does not trigger redzone errors.

    let page_allocator = MockPageAllocator::new();
    #[repr(align(8))]
    struct Chunk { _data: [u8; 32] }
    let mut slab: SlabAllocator<Chunk, _> = SlabAllocator::new(page_allocator);

    let ptr = slab.alloc().unwrap();
    let chunk_ptr = ptr.as_ptr() as *mut u8;

    // Write within the chunk boundaries
    unsafe {
        for i in 0..std::mem::size_of::<Chunk>() {
            *chunk_ptr.add(i) = i as u8;
        }
        slab.dealloc(ptr);
    }
    // If no panic occurs, redzone is intact for normal use
}

#[test]
fn test_slab_alloc_dealloc_full_page_cycle() {
    let page_allocator = MockPageAllocator::new();
    #[repr(align(8))]
    struct Chunk { _data: [u8; 32] }
    let mut slab: SlabAllocator<Chunk, _> = SlabAllocator::new(page_allocator);

    let chunks_per_page = (PAGE_SIZE - SlotPage::<Chunk>::object_offset()) / SlotObject::<Chunk>::size_of();
    let mut ptrs = Vec::with_capacity(chunks_per_page);

    slab.page_allocator.on_after_alloc = Some(|pa| {
        assert!(pa.pages.len() == 1, "Should allocate exactly one page");
    });

    // Fill a page
    for _ in 0..chunks_per_page {
        ptrs.push(slab.alloc().unwrap());
    }

    // Deallocate all
    for ptr in ptrs {
        unsafe { slab.dealloc(ptr); }
    }

    // Allocate again
    for _ in 0..chunks_per_page {
        assert!(slab.alloc().is_some());
    }
}

#[test]
fn test_slab_alloc_dealloc_interleaved_pages() {
    let page_allocator = MockPageAllocator::new();
    #[repr(align(8))]
    struct Chunk { _data: [u8; 64] }
    let mut slab: SlabAllocator<Chunk, _> = SlabAllocator::new(page_allocator);

    let chunks_per_page = (PAGE_SIZE - SlotPage::<Chunk>::object_offset()) / SlotObject::<Chunk>::size_of();
    let mut ptrs = Vec::new();

    println!("Chunks per page: {}", chunks_per_page);

    slab.page_allocator.on_after_alloc = Some(|pa| {
        assert!(pa.pages.len() <= 2, "Should allocate two pages at most");
        println!("Allocated page: #{}", pa.pages.len());
    });

    slab.page_allocator.on_before_dealloc = Some(|_| {
        panic!("Should not deallocate any pages during this test");
    });

    // Allocate enough for two pages
    for _ in 0..(chunks_per_page * 2) {
        ptrs.push(slab.alloc().unwrap());
    }

    // Deallocate every other chunk
    for i in (0..ptrs.len()).step_by(2) {
        unsafe { slab.dealloc(ptrs[i]); }
    }

    // Allocate again, should fill freed slots before allocating new pages
    for i in 0..chunks_per_page {
        println!("Reallocating chunk #{}", i * 2);
        assert!(slab.alloc().is_some());
    }
}

#[test]
fn test_slab_alloc_dealloc_after_drop() {
    let page_allocator = MockPageAllocator::new();
    #[repr(align(8))]
    struct Chunk { _data: [u8; 32] }

    let _ = {
        let mut slab: SlabAllocator<Chunk, _> = SlabAllocator::new(page_allocator);
        let ptr = slab.alloc().unwrap();
        unsafe { slab.dealloc(ptr); }
        ptr
    };
    // After drop, memory should be deallocated, but we can't access ptr anymore.
    // This test just ensures no panic or UB occurs on drop.
}

#[test]
fn test_slab_alloc_dealloc_many_types() {
    let page_allocator = MockPageAllocator::new();
    #[repr(align(8))]
    struct ChunkA { _data: [u8; 16] }
    #[repr(align(16))]
    struct ChunkB { _data: [u8; 32] }
    #[repr(align(32))]
    struct ChunkC { _data: [u8; 64] }

    let mut slab_a: SlabAllocator<ChunkA, _> = SlabAllocator::new(page_allocator);
    let mut slab_b: SlabAllocator<ChunkB, _> = SlabAllocator::new(MockPageAllocator::new());
    let mut slab_c: SlabAllocator<ChunkC, _> = SlabAllocator::new(MockPageAllocator::new());

    let ptr_a = slab_a.alloc().unwrap();
    let ptr_b = slab_b.alloc().unwrap();
    let ptr_c = slab_c.alloc().unwrap();

    unsafe {
        slab_a.dealloc(ptr_a);
        slab_b.dealloc(ptr_b);
        slab_c.dealloc(ptr_c);
    }
}

#[test]
fn test_slab_alloc_dealloc_zero_sized_type() {
    let page_allocator = MockPageAllocator::new();
    #[repr(align(8))]
    struct Chunk;
    let mut slab: SlabAllocator<Chunk, _> = SlabAllocator::new(page_allocator);

    // Zero-sized types: alloc should still return a unique pointer
    let ptr1 = slab.alloc().unwrap();
    let ptr2 = slab.alloc().unwrap();
    assert_ne!(ptr1, ptr2);

    unsafe {
        slab.dealloc(ptr1);
        slab.dealloc(ptr2);
    }
}

#[test]
fn test_slab_alloc_dealloc_with_unusual_alignment() {
    let page_allocator = MockPageAllocator::new();
    #[repr(align(256))]
    struct Chunk { _data: [u8; 128] }
    let mut slab: SlabAllocator<Chunk, _> = SlabAllocator::new(page_allocator);

    let ptr = slab.alloc().unwrap();
    assert_eq!((ptr.as_ptr() as usize) % 256, 0);
    unsafe { slab.dealloc(ptr); }
}

#[test]
fn test_slab_alloc_dealloc_with_minimum_size() {
    let page_allocator = MockPageAllocator::new();
    #[repr(align(1))]
    struct Chunk { _data: [u8; 1] }
    let mut slab: SlabAllocator<Chunk, _> = SlabAllocator::new(page_allocator);

    let ptr = slab.alloc().unwrap();
    unsafe { slab.dealloc(ptr); }
}

#[test]
fn test_slab_alloc_dealloc_with_large_alignment() {
    let page_allocator = MockPageAllocator::new();
    #[repr(align(1024))]
    struct Chunk { _data: [u8; 32] }
    let mut slab: SlabAllocator<Chunk, _> = SlabAllocator::new(page_allocator);

    let ptr = slab.alloc().unwrap();
    assert_eq!((ptr.as_ptr() as usize) % 1024, 0);
    unsafe { slab.dealloc(ptr); }
}

#[test]
fn test_slab_alloc_dealloc_with_multiple_allocators() {
    let mut allocators = Vec::new();
    for _ in 0..4 {
        allocators.push(MockPageAllocator::new());
    }
    #[repr(align(8))]
    struct Chunk { _data: [u8; 32] }
    let mut slabs: Vec<SlabAllocator<Chunk, _>> = allocators
        .into_iter()
        .map(SlabAllocator::new)
        .collect();

    let mut ptrs = Vec::new();
    for slab in &mut slabs {
        ptrs.push(slab.alloc().unwrap());
    }
    for (slab, ptr) in slabs.iter_mut().zip(ptrs) {
        unsafe { slab.dealloc(ptr); }
    }
}
