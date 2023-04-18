use arrayvec::ArrayVec;

use crate::{print, println};
use crate::terminal::{ColorCode, INPUT_MAXSIZE, start_inputting};
use crate::{pit, memory, task};

struct Command(&'static str, fn (args: &ArrayVec<&str, INPUT_MAXSIZE>), &'static str, Option<&'static str>);

const COMMAND: [Command; 8] = [
    Command("help",         cmd_help,           "show help",            Some("help (specific command)")),
    Command("tick",         cmd_tick,           "show tick count",      None),
    Command("printpage",    cmd_print_page,     "print page table",     None),
    Command("printmmap",    cmd_print_mmap,     "print memory map",     None),
    Command("meminfo",      cmd_mem_info,       "print memory info",    None),
    Command("testtask",     cmd_test_task,      "run test task",        Some("testtask (--quit)")),
    Command("testdynseq",   cmd_test_dyn_seq,   "test dynamic memory in sequencial order", None),
    Command("testdynran",   cmd_test_dyn_ran,   "test dynamic memory in random order", None),
];

pub fn prompt() {
    print!("> ");
    start_inputting();
}

pub fn input_line(input: &str) {
    let args: ArrayVec<&str, INPUT_MAXSIZE> = input.split_whitespace().collect();

    if args.len() > 0 {
        if let Some(cmd) = COMMAND.iter().find(|x| x.0 == args[0]) {
            cmd.1(&args);
        }
        else {
            println!(color: ColorCode::ERROR, "'{}': command not found", args[0]);
        }
    }
}

fn cmd_help(args: &ArrayVec<&str, INPUT_MAXSIZE>) {
    fn show_help(cmd: &Command) {
        println!("{} : {}", cmd.0, cmd.2);
    }

    if args.len() <= 1 {
        COMMAND.iter().for_each(show_help);
        println!("To see more detail help of specific command, enter 'help [command]'");
    }
    else {
        if let Some(cmd) = COMMAND.iter().find(|x| x.0 == args[1]) {
            show_help(cmd);
            if let Some(detail) = cmd.3 {
                println!("Usage) {}", detail);
            }
        }
        else {
            println!("'{}' is not command. type <help> to see help of whole commands.", args[1]);
        }
    }
}

fn cmd_tick(_args: &ArrayVec<&str, INPUT_MAXSIZE>) {
    println!("tick: {}", pit::tick());
}

fn cmd_print_page(_args: &ArrayVec<&str, INPUT_MAXSIZE>) {
    memory::print_page();
}

fn cmd_print_mmap(_args: &ArrayVec<&str, INPUT_MAXSIZE>) {
    memory::print_e820_map();
    memory::print_dynmem_map();
}

fn cmd_mem_info(_args: &ArrayVec<&str, INPUT_MAXSIZE>) {
    let info = memory::allocator_info();
    println!("===dynamic memory allocator infomation===");
    println!("metadata address     : {:#018x}", info.buddy.raw_addr());
    println!("metadata size        : {:#018x}", info.buddy.metadata_len());
    println!("count of unit blocks : {:#018x}", info.buddy.units());
    println!("total bitmap level   : {}", info.buddy.levels());
    println!("=========================================");
    println!("start address        : {:#018x}", info.buddy.data_addr());
    println!("dynmem size          : {:#018x}", info.buddy.data_len());
    println!("used size            : {:#018x}", info.used);
    println!("=========================================");
}

fn cmd_test_task(args: &ArrayVec<&str, INPUT_MAXSIZE>) {
    let quit = args.len() >= 2 && args[1] == "--quit";
    task::test_task(quit);
}

fn cmd_test_dyn_seq(_args: &ArrayVec<&str, INPUT_MAXSIZE>) {
    use core::slice::from_raw_parts_mut;
    use memory::{PAGE_SIZE, alloc_zero, deallocate, allocator_info, allocator_size_info};

    let info = allocator_info();

    println!("memory chunk starts at {:#x}", info.buddy.data_addr());
    println!("data range: [{:#x}, {:#x})", info.buddy.data_addr(), info.buddy.raw_addr() + info.buddy.total_len());

    for level in 0..info.buddy.levels() {
        let block_count = info.buddy.units() >> level;
        let size = (PAGE_SIZE as usize) << level;

        println!("Bitmap Level #{} (block_count={}, size={:#x})", level, block_count, size);

        let mut szinfo = allocator_size_info();
        assert_eq!(szinfo.used, 0);

        print!("Alloc & Comp : ");
        for index in 0..block_count {
            if let Some(addr) = alloc_zero(size) {
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

        szinfo = allocator_size_info();
        assert_eq!(szinfo.used, szinfo.len / size * size);

        print!("\nDeallocation : ");
        for index in 0..block_count {
            let addr = info.buddy.data_addr() + size * index;
            deallocate(addr, size);
            print!(".");
        }

        szinfo = allocator_size_info();
        assert_eq!(szinfo.used, 0);

        println!();
    }
}

fn cmd_test_dyn_ran(_args: &ArrayVec<&str, INPUT_MAXSIZE>) {
    todo!();
}
