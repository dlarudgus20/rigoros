#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]

use core::mem::{size_of, MaybeUninit, transmute};
use core::slice::from_raw_parts_mut;
use num_integer::div_ceil;

pub const UNIT_SIZE: usize = 4096;

pub struct BuddyBlock<'a> {
    info: BuddyBlockInfo,
    used: usize,
    bitmaps: &'a mut [BlockBitmap],
}

#[derive(Debug, Clone, Copy)]
pub struct BuddyBlockInfo {
    raw_addr: usize,
    total_len: usize,
    metadata_len: usize,
    data_offset: usize,
    units: usize,
    levels: u32,
}

#[repr(C)]
struct BlockBitmap {
    bits: *mut u8,
    count: usize,
}

struct BlockBitmapRef<'a> {
    bits: &'a mut [u8],
    count: &'a mut usize,
}

unsafe impl Send for BlockBitmap {}

impl BuddyBlockInfo {
    pub fn empty() -> Self {
        Self { raw_addr: 0, total_len: 0, metadata_len: 0, data_offset: 0, units: 0, levels: 0 }
    }

    pub fn raw_addr(&self) -> usize {
        self.raw_addr
    }

    pub fn total_len(&self) -> usize {
        self.total_len
    }

    pub fn metadata_len(&self) -> usize {
        self.metadata_len
    }

    pub fn data_offset(&self) -> usize {
        self.data_offset
    }

    pub fn units(&self) -> usize {
        self.units
    }

    pub fn levels(&self) -> u32 {
        self.levels
    }

    pub fn data_addr(&self) -> usize {
        self.raw_addr + self.data_offset
    }

    pub fn data_len(&self) -> usize {
        self.total_len - self.data_offset
    }

    fn from_chunk(addr: usize, len: usize) -> Self {
        assert!(len > UNIT_SIZE);

        let mut levels: u32 = 0;
        let mut bits = 0;

        let units: usize = (len - 1) / UNIT_SIZE + 1;
        let mut block_count = units;

        loop {
            levels += 1;
            bits += (block_count - 1) / 8 + 1;

            if block_count == 1 {
                break;
            }

            block_count /= 2;
        }

        let metadata_len = (levels as usize) * size_of::<BlockBitmap>() + bits;
        assert!(metadata_len < len);

        BuddyBlockInfo {
            raw_addr: addr,
            total_len: len,
            metadata_len,
            data_offset: 0,
            units,
            levels,
        }
    }

    pub fn new(raw_addr: usize, total_len: usize) -> Self {
        let tmp = BuddyBlockInfo::from_chunk(raw_addr, total_len);
        let data_offset = div_ceil(tmp.metadata_len, UNIT_SIZE) * UNIT_SIZE;

        let info = BuddyBlockInfo::from_chunk(raw_addr + data_offset, total_len - data_offset);
        assert!(info.metadata_len < data_offset);

        Self {
            raw_addr,
            total_len,
            data_offset,
            ..info
        }
    }
}

impl<'a> BuddyBlock<'a> {
    pub fn empty() -> Self {
        Self { info: BuddyBlockInfo::empty(), used: 0, bitmaps: &mut [] }
    }

    pub fn info(&self) -> &BuddyBlockInfo {
        &self.info
    }

    pub fn used(&self) -> usize {
        self.used
    }

    pub fn left(&self) -> usize {
        self.info.data_len() - self.used()
    }

    pub unsafe fn new(raw_addr: usize, total_len: usize) -> Self {
        let info = BuddyBlockInfo::new(raw_addr, total_len);

        let bitmaps_len = info.levels as usize;
        let bitmaps_bytes = bitmaps_len * size_of::<BlockBitmap>();

        let bitmaps = unsafe {
            from_raw_parts_mut(raw_addr as *mut MaybeUninit<BlockBitmap>, bitmaps_len)
        };
        let total_bits = unsafe {
            from_raw_parts_mut((raw_addr + bitmaps_bytes) as *mut u8, info.metadata_len - bitmaps_bytes)
        };

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
            block_count /= 2;

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
        assert_ne!(len, 0);

        let aligned_len = div_ceil(len, UNIT_SIZE) * UNIT_SIZE;
        let bitmap_idx_fit = bitmap_index_for_size(aligned_len);
        let bitmap_len = self.bitmaps.len() as u32;

        if bitmap_idx_fit >= bitmap_len {
            // requested memory is too large
            return None
        }

        for bitmap_idx in bitmap_idx_fit..bitmap_len {
            let mut bitmap = self.get_bits(bitmap_idx);

            if bitmap.empty() {
                continue;
            }

            let block_idx = bitmap.first_1();
            bitmap.set_0(block_idx);

            let mut below_block_idx = block_idx;
            for below in (bitmap_idx_fit..bitmap_idx).rev() {
                below_block_idx *= 2;
                let mut below_bitmap = self.get_bits(below);
                below_bitmap.set_1(below_block_idx + 1);
            }

            self.used += aligned_len;

            let data_addr = self.info.raw_addr + self.info.data_offset;
            return Some(data_addr + block_idx * (UNIT_SIZE << bitmap_idx));
        }

        // there is no memory to allocate
        None
    }

    pub fn dealloc(&mut self, addr: usize, len: usize) {
        if len == 0 {
            return;
        }

        let data_addr = self.info.data_addr();
        let data_len = self.info.data_len();

        let aligned_addr = addr / UNIT_SIZE * UNIT_SIZE;
        let aligned_end = div_ceil(addr + len, UNIT_SIZE) * UNIT_SIZE;
        let aligned_len = aligned_end - aligned_addr;

        assert!(data_addr <= aligned_addr && aligned_addr < data_addr + data_len);
        assert!(data_addr < aligned_end && aligned_end <= data_addr + data_len);

        let bitmap_idx_fit = bitmap_index_for_size(aligned_len);
        let bitmap_len = self.bitmaps.len() as u32;

        assert!(bitmap_idx_fit < bitmap_len);

        let mut block_idx = (aligned_addr - data_addr) / (UNIT_SIZE << bitmap_idx_fit);
        let mut current = bitmap_idx_fit;
        loop {
            let mut current_bitmap = self.get_bits(current);
            assert!(!current_bitmap.get(block_idx));
            current_bitmap.set_1(block_idx);

            let buddy_idx = block_idx ^ 1;
            if current_bitmap.get(buddy_idx) {
                if current + 1 >= bitmap_len {
                    break;
                }

                current_bitmap.set_0(buddy_idx);
                current_bitmap.set_0(block_idx);

                block_idx /= 2;
                current += 1;
            }
            else {
                break;
            }
        }

        self.used -= aligned_len;
    }

    fn get_bits(&mut self, bitmap_idx: u32) -> BlockBitmapRef {
        BlockBitmapRef::from(&mut self.bitmaps, self.info.units, bitmap_idx)
    }
}

impl<'a> BlockBitmapRef<'a> {
    fn from(bitmaps: &'a mut [BlockBitmap], units: usize, bitmap_idx: u32) -> Self {
        let block_count = units >> bitmap_idx;
        let bitmap = &mut bitmaps[bitmap_idx as usize];
        let bits = bitmap.bits;
        let bits_len = (block_count - 1) / 8 + 1;
        Self {
            bits: unsafe { from_raw_parts_mut(bits, bits_len) },
            count: &mut bitmap.count
        }
    }

    fn get(&self, block_idx: usize) -> bool {
        (self.bits[block_idx / 8] & (1 << (block_idx % 8))) != 0
    }

    fn set_1(&mut self, block_idx: usize) {
        let prev = self.get(block_idx);
        self.bits[block_idx / 8] |= 1 << (block_idx % 8);
        if !prev {
            *self.count += 1;
        }
    }

    fn set_0(&mut self, block_idx: usize) {
        let prev = self.get(block_idx);
        self.bits[block_idx / 8] &= !(1 << (block_idx % 8));
        if prev {
            *self.count -= 1;
        }
    }

    fn empty(&self) -> bool {
        *self.count == 0
    }

    fn first_1(&self) -> usize {
        assert!(!self.empty());

        let mut bits_idx = 0;
        while self.bits[bits_idx] == 0 {
            bits_idx += 1;
        }

        let mut offset = 0;
        while (self.bits[bits_idx] & (1 << offset)) == 0 {
            offset += 1;
        }

        bits_idx * 8 + offset
    }
}

fn bitmap_index_for_size(size: usize) -> u32 {
    let mut idx = 0;
    while (UNIT_SIZE << idx) < size {
        idx += 1;
    }
    idx
}
