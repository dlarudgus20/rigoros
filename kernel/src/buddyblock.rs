use core::mem::{size_of, MaybeUninit, transmute};
use core::slice::from_raw_parts_mut;
use num_integer::div_ceil;

use crate::memory::PAGE_SIZE;
use crate::{println, print};

pub const UNIT_SIZE: usize = PAGE_SIZE as usize;

pub struct BuddyBlockInfo {
    len: usize,
    addr: usize,
    metadata_len: usize,
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
        Self { len: 0, addr: 0, metadata_len: 0, units: 0, levels: 0 }
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
            units,
            levels,
        }
    }

    pub fn metadata_offset(&self) -> usize {
        (self.metadata_len + UNIT_SIZE - 1) / UNIT_SIZE * UNIT_SIZE
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
        assert_ne!(len, 0);

        let aligned_len = div_ceil(len, UNIT_SIZE) * UNIT_SIZE;
        let bitmap_idx_fit = bitmap_index_for_size(aligned_len);
        let bitmap_len = self.bitmaps.len();

        if bitmap_idx_fit >= bitmap_len {
            return None
        }

        for bitmap_idx in bitmap_idx_fit..bitmap_len {
            let (bits, count) = self.get_bits(bitmap_idx);

            if *count == 0 {
                continue;
            }

            let bits_idx = bits.iter().position(|&x| x != 0).unwrap();
            let mut offset = 0;
            while (bits[bits_idx] & (1 << offset)) == 0 {
                offset += 1;
            }

            bits[bits_idx] &= !(1 << offset);
            *count -= 1;

            let block_idx = bits_idx * 8 + offset;

            if bitmap_idx != bitmap_idx_fit {
                let mut below_block_idx = block_idx;
                for below in (bitmap_idx_fit..bitmap_idx).rev() {
                    below_block_idx <<= 1;
                    self.bitset_1(below, below_block_idx + 1);
                }
            }

            self.used += aligned_len;
            return Some(self.info.addr + block_idx * (UNIT_SIZE << bitmap_idx));
        }

        unreachable!();
    }

    pub fn dealloc(&mut self, addr: usize, len: usize) {
        if len == 0 {
            return;
        }

        assert!((self.info.addr..self.info.addr + self.info.len).contains(&addr));

        let aligned_addr = addr / UNIT_SIZE * UNIT_SIZE;
        let aligned_len = div_ceil(len, UNIT_SIZE) * UNIT_SIZE;
        let bitmap_idx_fit = bitmap_index_for_size(aligned_len);
        let bitmap_len = self.bitmaps.len();

        assert!(bitmap_idx_fit < bitmap_len);

        let mut block_idx = (aligned_addr - self.info.addr) / (UNIT_SIZE << bitmap_idx_fit);
        assert!(!self.bitget(bitmap_idx_fit, block_idx));
        self.bitset_1(bitmap_idx_fit, block_idx);

        let mut above = bitmap_idx_fit;
        loop {
            let buddy_idx = block_idx ^ 1;
            if self.bitget(above, buddy_idx) {
                if above + 1 >= bitmap_len {
                    break;
                }

                self.bitset_0(above, buddy_idx);
                self.bitset_0(above, block_idx);

                block_idx <<= 1;
                above += 1;
                self.bitset_1(above, block_idx);
            }
            else {
                break;
            }
        }

        self.used -= aligned_len;
    }

    fn get_bits(&mut self, idx: usize) -> (&'a mut [u8], &mut usize) {
        let block_count = self.info.units >> idx;
        let bitmap = &mut self.bitmaps[idx];
        let bits = bitmap.bits;
        let bits_len = (block_count - 1) / 8 + 1;
        (unsafe { from_raw_parts_mut(bits, bits_len) }, &mut bitmap.count)
    }

    fn bitget(&mut self, bitmap_idx: usize, block_idx: usize) -> bool {
        let (bits, _) = self.get_bits(bitmap_idx);
        (bits[block_idx / 8] & (1 << block_idx % 8)) != 0
    }

    fn bitset_1(&mut self, bitmap_idx: usize, block_idx: usize) {
        let (bits, count) = self.get_bits(bitmap_idx);
        bits[block_idx / 8] |= 1 << block_idx % 8;
        *count += 1;
    }

    fn bitset_0(&mut self, bitmap_idx: usize, block_idx: usize) {
        let (bits, count) = self.get_bits(bitmap_idx);
        bits[block_idx / 8] &= !(1 << block_idx % 8);
        *count -= 1;
    }

    pub fn test_seq(&mut self) {
        for level in 0..self.bitmaps.len() {
            println!("Bitmap Level #{}", level);
            print!("Allocation & Compare : ");

            let block_count = self.info.units >> level;
            let size = UNIT_SIZE << level;

            for count in 0..block_count {
                if let Some(addr) = self.alloc(size) {
                    let slice = unsafe { from_raw_parts_mut(addr as *mut u32, size / 4) };
                    for (idx, x) in slice.iter_mut().enumerate() {
                        unsafe { core::ptr::write_volatile(&mut *x, idx as u32) };
                    }
                    for (idx, x) in slice.iter().enumerate() {
                        let data = unsafe { core::ptr::read_volatile(&*x) };
                        if data != idx as u32 {
                            println!("comparison fail: Level[{}] Size[{}] Count[{}]", level, size, count);
                        }
                    }
                    print!(".");
                }
                else {
                    println!("alloc() fail: Level[{}] Size[{}] Count[{}]", level, size, count);
                    return;
                }
            }

            print!("Free : ");
            for count in 0..block_count {
                let addr = self.info.addr + size * count;
                self.dealloc(addr, size);
                print!(".");
            }

            println!();
        }
        println!("Sequencial Test Completed");
    }
}

fn bitmap_index_for_size(size: usize) -> usize {
    let mut idx = 0;
    while (UNIT_SIZE << idx) < size {
        idx += 1;
    }
    idx
}
