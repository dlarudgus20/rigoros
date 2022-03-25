use core::{fmt, str};
use core::panic::PanicInfo;
use volatile::Volatile;
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::instructions::port::Port;
use x86_64::instructions::interrupts::without_interrupts;

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

    buffer: RingBuffer<'static, VideoRow>,
    video: Volatile<&'static mut VideoBuffer>,
}

struct TerminalWriter<'a>(&'a mut Terminal, ColorCode);

const VIDEO_HEIGHT: usize = 25;
const VIDEO_WIDTH: usize = 80;

const BUFFER_HEIGHT: usize = 256;

const EMPTY_CHAR: VideoChar = VideoChar { character: 0, color: ColorCode::DEFAULT };
const EMPTY_ROW: VideoRow = [EMPTY_CHAR; VIDEO_WIDTH];

lazy_static! {
    static ref TERM: Mutex<Terminal> = Mutex::new(unsafe {
        static mut BUFFER: [VideoRow; BUFFER_HEIGHT] = [EMPTY_ROW; BUFFER_HEIGHT];
        Terminal {
            cur_col: 0,
            cur_row: 0,
            scr_row: 0,
            status_front_len: 0,
            status_back_len: 0,
            buffer: RingBuffer::new(&mut BUFFER),
            video: Volatile::new(&mut *(0xffff80001feb8000 as *mut VideoBuffer)),
        }
    });
}

pub fn init_term() {
    let mut term = TERM.lock();
    term.redraw_status_lines();
    term.clear();
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
        self.update_cursor();
    }

    pub fn write_status(&mut self, kind: StatusLineKind, line: usize, s: &str) {
        let limit = match kind {
            StatusLineKind::Front => self.status_front_len,
            StatusLineKind::Back => self.status_back_len,
        };

        if line >= limit {
            panic!("Invalid status line number");
        }

        let row = match kind {
            StatusLineKind::Front => line,
            StatusLineKind::Back => VIDEO_HEIGHT - line - 1,
        };

        let bytes = s.as_bytes();

        for index in 0..VIDEO_WIDTH {
            let character = if index < bytes.len() { bytes[index] } else { 0 };
            self.video_ch_mut(row, index).write(VideoChar {
                character,
                color: ColorCode::STATUS,
            });
        }
        self.update_cursor();
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
                    let row = self.screen_start() + self.cur_row - self.scr_row;
                    self.video_ch_mut(row as usize, self.cur_col).write(ch);
                }

                self.cur_col += 1;
                if self.cur_col >= VIDEO_WIDTH {
                    self.new_line();
                } else {
                    self.scroll_to_cursor();
                }
            }
        }
    }

    fn new_line(&mut self) {
        self.cur_col = 0;
        self.cur_row += 1;
        self.buffer.insert_force(self.cur_row, EMPTY_ROW);
        self.scroll_to_cursor();
    }

    fn cursor_visible(&self) -> bool {
        let scrlen = self.screen_height();
        self.scr_row <= self.cur_row && self.cur_row < self.scr_row + scrlen
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
    }

    fn update_cursor(&self) {
        let pos = if self.cursor_visible() {
            (self.screen_start() + self.cur_row - self.scr_row) * VIDEO_WIDTH + self.cur_col
        } else {
            VIDEO_HEIGHT * VIDEO_WIDTH
        };

        unsafe {
            let mut port1 = Port::<u8>::new(0x3d4);
            let mut port2 = Port::<u8>::new(0x3d5);

            port1.write(0x0f);
            port2.write((pos & 0xff) as u8);
            port1.write(0x0e);
            port2.write((pos >> 8 & 0xff) as u8);
        }
    }
}

impl<'a> fmt::Write for TerminalWriter<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.write_string(self.1, s);
        Ok(())
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
    use fmt::Write;

    without_interrupts(|| {
        let mut term = TERM.lock();
        let c = color.unwrap_or(ColorCode::DEFAULT);
        TerminalWriter(&mut term, c).write_fmt(args).unwrap();
    });
}

#[doc(hidden)]
pub fn _print_status(front: StatusLineKind, line: usize, args: fmt::Arguments) {
    use core2::io::Write;

    let mut buffer: [u8; VIDEO_WIDTH] = [0; VIDEO_WIDTH];
    (&mut buffer[..]).write_fmt(args).unwrap();

    without_interrupts(|| {
        let mut term = TERM.lock();
        term.write_status(front, line, str::from_utf8(&buffer).unwrap());
    });
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    println!(color: ColorCode::PANIC; "panic!!");
    halt_loop();
}
