use arrayvec::ArrayVec;

use crate::{print, println};
use crate::terminal::{ColorCode, INPUT_MAXSIZE, start_inputting};
use crate::pit;
use crate::memory;
use crate::task;

struct Command(&'static str, fn (args: &ArrayVec<&str, INPUT_MAXSIZE>), &'static str, Option<&'static str>);

const COMMAND: [Command; 4] = [
    Command("help",         cmd_help,       "show help",        Some("help (specific command)")),
    Command("tick",         cmd_tick,       "show tick count",  None),
    Command("print-page",   cmd_print_page, "print page table", None),
    Command("test-task",    cmd_test_task,  "run test task",    None),
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

fn cmd_test_task(_args: &ArrayVec<&str, INPUT_MAXSIZE>) {
    task::test_task();
}
