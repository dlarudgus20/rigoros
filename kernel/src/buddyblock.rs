use core::mem::{size_of, MaybeUninit, transmute};
use core::slice::from_raw_parts_mut;

use num_integer::div_ceil;

pub const UNIT_SIZE: usize = 4096;

pub struct BuddyBlockInfo {
    len: usize,
    addr: usize,
    metadata_len: usize,
    metadata_offset: usize,
    units: usize,
    levels: u32,
}

pub struct BuddyBlock<'a> {
    info: BuddyBlockInfo,
    used: usize,
    bitmaps: &'a mut [BlockBitmap],
}

#[repr(C)]
struct BlockBitmap {
    bits: *mut u8,
    count: usize,
}

unsafe impl Send for BlockBitmap {}

impl BuddyBlockInfo {
    pub fn empty() -> Self {
        Self { len: 0, addr: 0, metadata_len: 0, metadata_offset: 0, units: 0, levels: 0 }
    }

    pub fn new(raw_addr: usize, total_len: usize) -> Self {
        assert!(total_len > UNIT_SIZE);

        let mut levels: u32 = 0;
        let mut bits = 0;

        let units = (total_len - 1) / UNIT_SIZE + 1;
        let mut block_count = units;

        loop {
            levels += 1;
            bits += (block_count - 1) / 8 + 1;

            if block_count == 1 {
                break;
            }

            block_count >>= 1;
        }

        let metadata_len = (levels as usize) * size_of::<BlockBitmap>() + bits;
        let addr_offset = (metadata_len + UNIT_SIZE - 1) / UNIT_SIZE * UNIT_SIZE;

        assert!(metadata_len < total_len);

        BuddyBlockInfo {
            len: total_len - addr_offset,
            addr: raw_addr + addr_offset,
            metadata_len,
            metadata_offset: div_ceil(metadata_len, UNIT_SIZE) * UNIT_SIZE,
            units,
            levels,
        }
    }

    pub fn metadata_offset(&self) -> usize {
        self.metadata_offset
    }
}

impl<'a> BuddyBlock<'a> {
    pub fn empty() -> Self {
        Self { info: BuddyBlockInfo::empty(), used: 0, bitmaps: &mut [] }
    }

    pub unsafe fn new(metadata: *mut u8, info: BuddyBlockInfo) -> Self {
        let bitmaps_len = info.levels as usize;
        let bitmaps_bytes = bitmaps_len * size_of::<BlockBitmap>();

        let bitmaps = unsafe { from_raw_parts_mut(metadata as *mut MaybeUninit<BlockBitmap>, bitmaps_len) };
        let total_bits = unsafe { from_raw_parts_mut(metadata.add(bitmaps_bytes), info.metadata_len - bitmaps_bytes) };

        total_bits.fill(0);

        let mut block_count = info.units;
        let mut bits_idx = 0;
        let mut idx = 0;
        loop {
            let bits_len = (block_count - 1) / 8 + 1;
            let bits = &mut total_bits[bits_idx..bits_idx + bits_len];

            bits_idx += bits_len;
            let count = if block_count % 2 != 0 {
                bits[bits_len - 1] = 1 << (block_count % 8 - 1);
                1
            }
            else {
                0
            };

            bitmaps[idx].write(BlockBitmap {
                bits: bits.as_mut_ptr(),
                count,
            });

            idx += 1;
            block_count >>= 1;

            if block_count == 0 {
                break;
            }
        }

        assert_eq!(bits_idx, total_bits.len());
        assert_eq!(idx, bitmaps.len());

        Self {
            info,
            used: 0,
            bitmaps: unsafe { transmute(bitmaps) },
        }
    }

    pub fn alloc(&mut self, len: usize) -> Option<usize> {
        todo!();
    }

    pub fn dealloc(&mut self, addr: usize, len: usize) -> Option<()> {
        todo!();
    }
}
