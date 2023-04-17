use core::mem::size_of;
use core::slice::from_raw_parts_mut;
use buddyblock::{BuddyBlock, UNIT_SIZE};

#[repr(C, align(4096))]
#[derive(Clone, Copy)]
struct Page {
    buf: [u64; 512]
}

struct TestBuddy<'a>(BuddyBlock<'a>, Vec<Page>);

fn create_buddy<'a>() -> TestBuddy<'a> {
    let mem: Vec<Page> = vec![Page { buf: [0; 512] }; 0x00200000 / 4096];

    let buddy = unsafe {
        BuddyBlock::new(mem.as_ptr() as usize, mem.len() * size_of::<Page>())
    };

    TestBuddy(buddy, mem)
}

#[test]
fn test_new() {
    let TestBuddy(buddy, mem) = create_buddy();

    let begin = mem.as_ptr() as usize;
    let len = mem.len() * size_of::<Page>();

    fn calc_lev_bitlen(blocks: usize) -> (u32, usize) {
        let mut level = 0;
        let mut bitlen = 0;
        while (blocks >> level) != 0 {
            bitlen += ((blocks >> level) + 7) / 8;
            level += 1;
        }
        (level, bitlen)
    }

    let units = 0x1ff000 / UNIT_SIZE;
    let (level, bitlen) = calc_lev_bitlen(units);
    let metalen = (level as usize) * 16 + bitlen;

    println!("{:?}", buddy.info());
    assert_eq!(buddy.info().raw_addr(), begin);
    assert_eq!(buddy.info().total_len(), len);
    assert_eq!(buddy.info().metadata_len(), metalen);
    assert_eq!(buddy.info().data_offset(), UNIT_SIZE);
    assert_eq!(buddy.info().units(), units);
    assert_eq!(buddy.info().levels(), level);
    assert_eq!(buddy.info().data_addr(), begin + UNIT_SIZE);
}

#[test]
fn test_seq() {
    let TestBuddy(mut buddy, mem) = create_buddy();

    println!("memory chunk starts at {:#x}", mem.as_ptr() as usize);
    println!("data range: [{:#x}, {:#x})", buddy.info().data_addr(), buddy.info().raw_addr() + buddy.info().total_len());

    for level in 0..buddy.info().levels() {
        let block_count = buddy.info().units() >> level;
        let size = UNIT_SIZE << level;

        println!("Bitmap Level #{} (block_count={}, size={:#x})", level, block_count, size);

        assert_eq!(buddy.used(), 0);

        print!("Alloc & Comp : ");
        for index in 0..block_count {
            if let Some(addr) = buddy.alloc(size - 1) { // test unaligned
                let slice = unsafe { from_raw_parts_mut(addr as *mut u32, size / 4) };
                for (idx, x) in slice.iter_mut().enumerate() {
                    unsafe { core::ptr::write_volatile(&mut *x, idx as u32) };
                }
                for (idx, x) in slice.iter().enumerate() {
                    let data = unsafe { core::ptr::read_volatile(&*x) };
                    if data != idx as u32 {
                        println!("comparison fail: level={} size={} index={}", level, size, index);
                    }
                }
                print!(".");
            }
            else {
                println!("alloc() fail: level={} size={} index={}", level, size, index);
                return;
            }
        }

        assert_eq!(buddy.used(), buddy.info().data_len() / size * size);

        print!("\nDeallocation : ");
        for index in 0..block_count {
            let addr = buddy.info().data_addr() + size * index;
            buddy.dealloc(addr + 1, size - 1); // test unaligned
            print!(".");
        }

        assert_eq!(buddy.used(), 0);

        println!();
    }
}
