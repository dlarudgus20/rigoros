use core::{fmt, str};
use core::fmt::Write;
use core::panic::PanicInfo;
use volatile::Volatile;
use lazy_static::lazy_static;
use spin::Mutex;
use arrayvec::{ArrayVec, ArrayString};
use pc_keyboard::{KeyCode, DecodedKey};
use x86_64::instructions::port::Port;
use x86_64::instructions::interrupts::without_interrupts;

use crate::fixed_writer::FixedWriter;
use crate::halt_loop;
use crate::ring_buffer::RingBuffer;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGrey = 7,
    DarkGrey = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yello = 14,
    White = 15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ColorCode(u8);

#[allow(dead_code)]
impl ColorCode {
    pub const fn new(fg: Color, bg: Color) -> ColorCode {
        ColorCode((bg as u8) << 4 | (fg as u8))
    }

    pub const DEFAULT: ColorCode = ColorCode::new(Color::LightGrey, Color::Black);
    pub const LOG: ColorCode = ColorCode::new(Color::LightGreen, Color::Black);
    pub const STATUS: ColorCode = ColorCode::new(Color::White, Color::LightGrey);
    pub const INPUT: ColorCode = ColorCode::new(Color::White, Color::Black);
    pub const PANIC: ColorCode = ColorCode::new(Color::Red, Color::White);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputStatus { Input, Waiting }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusLineKind { Front, Back }

pub struct LineInfo {
    pub cur_col: usize,
    pub cur_row: usize,
    pub screen: usize,
    pub width: usize,
    pub height: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct VideoChar {
    character: u8,
    color: ColorCode
}

type VideoRow = [VideoChar; VIDEO_WIDTH];
type VideoBuffer = [VideoRow; VIDEO_HEIGHT];

struct Terminal {
    cur_col: usize,
    cur_row: usize,
    scr_row: usize,

    status_front_len: usize,
    status_back_len: usize,

    input_cur: usize,
    input_status: InputStatus,
    input: &'static mut ArrayVec<u8, INPUT_MAXSIZE>,

    buffer: RingBuffer<'static, VideoRow>,
    video: Volatile<&'static mut VideoBuffer>,
}

struct TerminalWriter<'a> {
    term: &'a mut Terminal,
    color: ColorCode,
}

struct StatusLineWriter<'a> {
    term: &'a mut Terminal,
    kind: StatusLineKind,
    line: usize,
    cur: usize,
}

const VIDEO_MEMORY: usize = 0xffff80001feb8000;

const VIDEO_HEIGHT: usize = 25;
const VIDEO_WIDTH: usize = 80;

const INPUT_MAXSIZE: usize = 1024;
const BUFFER_HEIGHT: usize = 256;

const EMPTY_CHAR: VideoChar = VideoChar { character: 0, color: ColorCode::DEFAULT };
const EMPTY_ROW: VideoRow = [EMPTY_CHAR; VIDEO_WIDTH];

lazy_static! {
    static ref TERM: Mutex<Terminal> = Mutex::new(unsafe {
        static mut BUFFER: [VideoRow; BUFFER_HEIGHT] = [EMPTY_ROW; BUFFER_HEIGHT];
        static mut INPUT: ArrayVec<u8, INPUT_MAXSIZE> = ArrayVec::new_const();
        Terminal {
            cur_col: 0,
            cur_row: 0,
            scr_row: 0,
            status_front_len: 0,
            status_back_len: 0,
            input_cur: 0,
            input_status: InputStatus::Waiting,
            input: &mut INPUT,
            buffer: RingBuffer::new(&mut BUFFER),
            video: Volatile::new(&mut *(VIDEO_MEMORY as *mut VideoBuffer)),
        }
    });
}

pub unsafe fn init_term() {
    let mut term = TERM.lock();
    term.redraw_status_lines();
    term.clear();

    enable_cursor(true);
}

fn enable_cursor(enable: bool) {
    unsafe {
        let mut port1 = Port::<u8>::new(0x3d4);
        let mut port2 = Port::<u8>::new(0x3d5);

        if enable {
            let mut data: u8;

            port1.write(0x0a);
            data = port2.read();
            port2.write((data & 0xc0) | 13);

            port1.write(0x0b);
            data = port2.read();
            port2.write((data & 0xc0) | 15);
        }
        else {
            port1.write(0x0a);
            port2.write(0x20);
        }
    }
}

fn set_cursor(pos: u16) {
    unsafe {
        let mut port1 = Port::<u8>::new(0x3d4);
        let mut port2 = Port::<u8>::new(0x3d5);

        port1.write(0x0f);
        port2.write((pos & 0xff) as u8);
        port1.write(0x0e);
        port2.write((pos >> 8 & 0xff) as u8);
    }
}

pub fn process_input(input: DecodedKey) {
    without_interrupts(|| { TERM.lock().process_input(input) })
}

pub fn set_status_lines_front(lines: usize) {
    without_interrupts(|| {
        TERM.lock().set_status_lines(StatusLineKind::Front, lines);
    });
}

pub fn set_status_lines_back(lines: usize) {
    without_interrupts(|| {
        TERM.lock().set_status_lines(StatusLineKind::Back, lines);
    });
}

pub fn scroll(page: isize) {
    without_interrupts(|| { TERM.lock().scroll_page(page); });
}

pub fn line_info() -> LineInfo {
    without_interrupts(|| { TERM.lock().line_info() })
}

impl Terminal {
    pub fn clear(&mut self) {
        self.buffer.push_force(EMPTY_ROW);

        self.cur_col = 0;
        self.cur_row = self.buffer.len() - 1;
        self.scr_row = self.cur_row;

        self.scroll_to_cursor();
        self.update_cursor();
    }

    pub fn write_string(&mut self, color: ColorCode, s: &str) {
        for ch in s.bytes() {
            self.write_char(color, ch);
        }
        self.scroll_to_cursor();
        self.update_cursor();
    }

    pub fn process_input(&mut self, input: DecodedKey) {
        match (input, self.input_status) {
            (DecodedKey::RawKey(KeyCode::PageUp), _) => {
                self.scroll_page(-1);
                self.print_line_status();
            }
            (DecodedKey::RawKey(KeyCode::PageDown), _) => {
                self.scroll_page(1);
                self.print_line_status();
            }
            (DecodedKey::Unicode('\n'), InputStatus::Input) => {
                self.clear_cur_line_status();
            }
            (DecodedKey::RawKey(KeyCode::Delete), InputStatus::Input) => {
                self.delete_char();
                self.print_cursor_status();
            }
            (DecodedKey::Unicode('\x08'), InputStatus::Input) => {
                self.backspace();
                self.print_cursor_status();
            }
            (DecodedKey::Unicode(ch), InputStatus::Input) => {
                if ch.is_ascii() && !ch.is_ascii_control() {
                    self.put_char(ch as u8);
                    self.print_cursor_status();
                }
            }
            _ => {}
        }
    }

    fn put_char(&mut self, ch: u8) {
        if let Ok(_) = self.input.try_insert(self.input_cur, ch) {
            let mut buf = ArrayVec::<u8, INPUT_MAXSIZE>::new();
            buf.try_extend_from_slice(&self.input[self.input_cur..]).unwrap();

            let s = unsafe { str::from_utf8_unchecked(&buf) };
            self.write_string(ColorCode::DEFAULT, s);
            todo!("write_string_at ?");
        }
    }

    fn delete_char(&mut self) {
        todo!();
    }

    fn backspace(&mut self) {
        todo!();
    }

    fn print_line_status(&mut self) {
        if self.status_back_len > 0 {
            let screen = self.scr_row;
            let height = self.screen_height();
            let total = self.buffer.len();

            let scr_page = (screen + height - 1) / height;
            let scr_reminder = screen % height;
            let total_page =
                (total - scr_reminder + height - 1) / height
                + if scr_reminder > 0 { 1 } else { 0 };

            let mut writer = StatusLineWriter { term: self, kind: StatusLineKind::Back, line: 0, cur: 0 };
            write!(writer, "page {} / {}, line {} / {}", scr_page + 1, total_page, screen + 1, total).unwrap();
        }
    }

    fn print_cursor_status(&mut self) {
        if self.status_back_len > 0 {
            let cur_col = self.cur_col;
            let cur_row = self.cur_row;
            let total = self.buffer.len();

            let mut writer = StatusLineWriter { term: self, kind: StatusLineKind::Back, line: 0, cur: 0 };
            write!(writer, "row {} / {}, col {} / {}", cur_row + 1, total, cur_col + 1, VIDEO_WIDTH).unwrap();
        }
    }

    fn clear_cur_line_status(&mut self) {
        if self.status_back_len > 0 {
            self.clear_status_line(StatusLineKind::Back, 0, 0);
        }
    }

    pub fn write_status_line(&mut self, kind: StatusLineKind, line: usize, s: &str, cur: usize) -> usize {
        let row = self.get_status_line_row(kind, line);
        let mut pos = cur;

        for ch in s.bytes() {
            if pos >= VIDEO_WIDTH {
                break;
            }

            self.video_ch_mut(row, pos).write(VideoChar {
                character: ch,
                color: ColorCode::STATUS,
            });

            pos += 1;
        }

        if pos < VIDEO_WIDTH { pos } else { VIDEO_WIDTH }
    }

    pub fn clear_status_line(&mut self, kind: StatusLineKind, line: usize, cur: usize) {
        let row = self.get_status_line_row(kind, line);

        for pos in cur..VIDEO_WIDTH {
            self.video_ch_mut(row, pos).write(VideoChar {
                character: 0,
                color: ColorCode::STATUS,
            });
        }
    }

    fn get_status_line_row(&self, kind: StatusLineKind, line: usize) -> usize {
        let limit = match kind {
            StatusLineKind::Front => self.status_front_len,
            StatusLineKind::Back => self.status_back_len,
        };

        if line >= limit {
            panic!("Invalid status line number");
        }

        match kind {
            StatusLineKind::Front => line,
            StatusLineKind::Back => VIDEO_HEIGHT - line - 1,
        }
    }

    pub fn set_status_lines(&mut self, kind: StatusLineKind, lines: usize) {
        match kind {
            StatusLineKind::Front => self.status_front_len = lines,
            StatusLineKind::Back => self.status_back_len = lines,
        }
        self.redraw();
    }

    pub fn scroll_page(&mut self, page: isize) {
        let buflen = self.buffer.len() as isize;
        let scrlen = self.screen_height() as isize;
        let to = (self.scr_row as isize) + page * scrlen;
        let row = if to < 0 { 0 } else if to >= buflen { buflen - 1 } else { to };
        self.scroll_to(row as usize);
    }

    pub fn line_info(&self) -> LineInfo {
        LineInfo {
            cur_col: self.cur_col,
            cur_row: self.cur_row,
            screen: self.scr_row,
            width: VIDEO_WIDTH,
            height: self.screen_height(),
            total: self.buffer.len(),
        }
    }

    fn redraw(&mut self) {
        self.redraw_status_lines();
        self.redraw_screen();
    }

    fn redraw_status_lines(&mut self) {
        let line = [VideoChar { character: 0, color: ColorCode::STATUS }; VIDEO_WIDTH];
        let back_start = VIDEO_HEIGHT - self.status_back_len;

        for row in 0..self.status_front_len {
            self.video_row_mut(row).write(line);
        }

        for row in back_start..VIDEO_HEIGHT {
            self.video_row_mut(row).write(line);
        }
    }

    fn redraw_screen(&mut self) {
        let scrlen = self.screen_height();

        for row in 0..scrlen {
            let line = *self.buffer.get(self.scr_row + row).unwrap_or(&EMPTY_ROW);
            self.video_row_mut(self.screen_start() + row).write(line);
        }
    }

    fn screen_start(&self) -> usize {
        self.status_front_len
    }

    fn screen_height(&self) -> usize {
        VIDEO_HEIGHT - self.status_front_len - self.status_back_len
    }

    fn video_row_mut(&mut self, row: usize) -> Volatile<&mut VideoRow> {
        self.video.map_mut(|x| &mut x[row])
    }

    fn video_ch_mut(&mut self, row: usize, col: usize) -> Volatile<&mut VideoChar> {
        self.video.map_mut(|x| &mut x[row][col])
    }

    fn write_char(&mut self, color: ColorCode, ch: u8) {
        match ch {
            b'\n' => {
                self.new_line();
            }
            ch => {
                let ch = VideoChar {
                    character: ch,
                    color: color,
                };

                self.buffer[self.cur_row][self.cur_col] = ch;
                if self.cursor_visible() {
                    let sr = self.screen_start() + self.cur_row - self.scr_row;
                    self.video_ch_mut(sr as usize, self.cur_col).write(ch);
                }

                self.cur_col += 1;
                if self.cur_col >= VIDEO_WIDTH {
                    self.new_line();
                }
            }
        }
    }

    fn write_char_at(&mut self, color: ColorCode, col: usize, row: usize, ch: u8) -> (usize, usize) {
        todo!();
        match ch {
            b'\n' => {
                self.new_line_at(row)
            }
            ch => {
                let ch = VideoChar {
                    character: ch,
                    color: color,
                };

                self.buffer[row][col] = ch;
                if self.row_visible(row) {
                    let sr = self.screen_start() + row - self.scr_row;
                    self.video_ch_mut(sr as usize, col).write(ch);
                }

                let nc = col + 1;
                if nc >= VIDEO_WIDTH {
                    (0, row + 1)
                } else {
                    (nc, row)
                }
            }
        }
    }

    fn new_line(&mut self) {
        self.cur_col = 0;
        self.cur_row += 1;
        self.buffer.insert_force(self.cur_row, EMPTY_ROW);
    }

    fn new_line_at(&mut self, row: usize) -> (usize, usize) {
        todo!();
        if self.cur_row > row {
            self.cur_row += 1;
        }
        if self.scr_row > row {
            self.scr_row += 1;
        }

        self.buffer.insert_force(row + 1, EMPTY_ROW);

        (0, row + 1)
    }

    fn cursor_visible(&self) -> bool {
        self.row_visible(self.cur_row)
    }

    fn row_visible(&self, row: usize) -> bool {
        let scrlen = self.screen_height();
        self.scr_row <= row && row < self.scr_row + scrlen
    }

    fn scroll_to_cursor(&mut self) {
        let scrlen = self.screen_height();

        if self.cur_row < self.scr_row {
            self.scroll_to(self.cur_row);
        } else if self.cur_row >= self.scr_row + scrlen {
            self.scroll_to(self.cur_row - scrlen + 1);
        }
    }

    fn scroll_to(&mut self, row: usize) {
        self.scr_row = row;
        self.redraw_screen();
        self.update_cursor();
    }

    fn update_cursor(&self) {
        let pos = if self.cursor_visible() {
            (self.screen_start() + self.cur_row - self.scr_row) * VIDEO_WIDTH + self.cur_col
        } else {
            VIDEO_HEIGHT * VIDEO_WIDTH
        };

        set_cursor(pos as u16);
    }
}

impl<'a> Write for TerminalWriter<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.term.write_string(self.color, s);
        Ok(())
    }
}

impl<'a> StatusLineWriter<'a> {
    pub fn done(&mut self) {
        self.term.clear_status_line(self.kind, self.line, self.cur);
        self.cur = VIDEO_WIDTH;
    }
}

impl<'a> Write for StatusLineWriter<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.cur = self.term.write_status_line(self.kind, self.line, s, self.cur);
        Ok(())
    }
}

impl<'a> Drop for StatusLineWriter<'a> {
    fn drop(&mut self) {
        self.done();
    }
}

#[macro_export]
macro_rules! print {
    (color: $c:expr; $($arg:tt)*) => ($crate::terminal::_print(Some($c), format_args!($($arg)*)));
    ($($arg:tt)*) => ($crate::terminal::_print(None, format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    (color: $c:expr; $($arg:tt)*) => ($crate::print!(color: $c; "{}\n", format_args!($($arg)*)));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => ($crate::println!(color: $crate::terminal::ColorCode::LOG; $($arg)*));
}

#[macro_export]
macro_rules! print_status {
    (front; $($arg:tt)*) => ($crate::terminal::_print_status($crate::terminal::StatusLineKind::Front, 0, format_args!($($arg)*)));
    (front: $n:expr; $($arg:tt)*) => ($crate::terminal::_print_status($crate::terminal::StatusLineKind::Front, $n, format_args!($($arg)*)));
    (back; $($arg:tt)*) => ($crate::terminal::_print_status($crate::terminal::StatusLineKind::Back, 0, format_args!($($arg)*)));
    (back: $n:expr; $($arg:tt)*) => ($crate::terminal::_print_status($crate::terminal::StatusLineKind::Back, $n, format_args!($($arg)*)));
    ($($arg:tt)*) => ($crate::terminal::_print_status($crate::terminal::StatusLineKind::Back, 0, format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(color: Option<ColorCode>, args: fmt::Arguments) {
    without_interrupts(|| {
        let mut term = TERM.lock();
        let c = color.unwrap_or(ColorCode::DEFAULT);
        TerminalWriter { term: &mut term, color: c }.write_fmt(args).unwrap();
    });
}

#[doc(hidden)]
pub fn _print_status(kind: StatusLineKind, line: usize, args: fmt::Arguments) {
    without_interrupts(|| {
        let mut term = TERM.lock();
        let mut writer = StatusLineWriter { term: &mut term, kind, line, cur: 0 };
        writer.write_fmt(args).unwrap();
    });
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    without_interrupts(|| {
        if let Some(mut term) = TERM.try_lock() {
            write!(TerminalWriter { term: &mut term, color: ColorCode::PANIC }, "[PANIC] {}", info).ok();
        } else {
            // manually write panic message on top of screen
            const SIZE: usize = VIDEO_WIDTH * VIDEO_HEIGHT;
            let mut s = ArrayString::<SIZE>::new();
            write!(FixedWriter::new(&mut s), "[PANIC in term-lock] {}", info).ok();

            let mut video = unsafe {
                Volatile::new(&mut *(VIDEO_MEMORY as *mut VideoBuffer))
            };

            for (idx, ch) in s.bytes().enumerate() {
                let col = idx % VIDEO_WIDTH;
                let row = idx / VIDEO_WIDTH;
                video.map_mut(|x| &mut x[row][col]).write(VideoChar {
                    character: ch,
                    color: ColorCode::PANIC
                });
            }
        }

        halt_loop();
    })
}
