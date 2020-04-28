use core::ptr::copy;
use volatile::Volatile;

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
#[repr(C)]
struct ScreenChar {
    character: u8,
    color: ColorCode
}

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

const SCREEN_HEIGHT: usize = 24;

#[repr(transparent)]
struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

pub struct Terminal {
    cur_col: usize,
    cur_row: usize,
    buffer: &'static mut Buffer,
}

impl Terminal {
    pub fn create() -> Terminal {
        Terminal {
            cur_col: 0,
            cur_row: 0,
            buffer: unsafe { &mut *(0xffff80001feb8000 as *mut Buffer) }
        }
    }

    pub fn write_string(&mut self, color: ColorCode, s: &str) {
        for ch in s.bytes() {
            self.write_char(color, ch);
        }
        self.update_cursor();
    }

    fn write_char(&mut self, color: ColorCode, ch: u8) {
        match ch {
            b'\n' => {
                self.new_line();
            }
            ch => {
                self.buffer.chars[self.cur_row][self.cur_col].write(ScreenChar {
                    character: ch,
                    color: color
                });

                self.cur_col += 1;
                if self.cur_col >= BUFFER_WIDTH {
                    self.new_line();
                }
            }
        }
    }

    fn new_line(&mut self) {
        self.cur_col = 0;
        self.cur_row += 1;
        if self.cur_row >= SCREEN_HEIGHT {
            self.scroll();
        }
    }

    fn scroll(&mut self) {
        if self.cur_row == 0 {
            self.cur_col = 0;
        }
        else {
            self.cur_row -= 1;
        }

        unsafe {
            copy(&self.buffer.chars[1], &mut self.buffer.chars[0], SCREEN_HEIGHT - 1);
        }
        let empty = ScreenChar { character: 0, color: ColorCode::DEFAULT };
        for item in self.buffer.chars[SCREEN_HEIGHT - 1].iter_mut() {
            item.write(empty);
        }
    }

    fn update_cursor(&self) {
        let pos = self.cur_row * BUFFER_WIDTH + self.cur_col;
        out8(0x3d4, 0x0f);
        out8(0x3d4 + 1, (pos & 0xff) as u8);
        out8(0x3d4, 0x0e);
        out8(0x3d4 + 1, (pos >> 8 & 0xff) as u8);
    }
}

fn out8(port: u16, data: u8) {
    unsafe {
        asm!("out %al, %dx" : : "{al}"(data), "{dx}"(port) : : "volatile");
    }
}
