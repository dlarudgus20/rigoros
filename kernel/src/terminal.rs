use core::fmt;
use volatile::Volatile;
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::instructions::port::Port;
use x86_64::instructions::interrupts::without_interrupts;

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

const VIDEO_HEIGHT: usize = 25;
const VIDEO_WIDTH: usize = 80;

const SCREEN_HEIGHT: usize = 24;

type VideoBuffer = [[ScreenChar; VIDEO_WIDTH]; VIDEO_HEIGHT];

struct Terminal {
    cur_col: usize,
    cur_row: usize,
    video: Volatile<&'static mut VideoBuffer>,
}

lazy_static! {
    static ref TERM: Mutex<Terminal> = Mutex::new(Terminal {
        cur_col: 0,
        cur_row: 0,
        video: Volatile::new(unsafe { &mut *(0xffff80001feb8000 as *mut VideoBuffer) }),
    });
}

impl Terminal {
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
                let ch = ScreenChar {
                    character: ch,
                    color: color
                };

                self.video.map_mut(|x| &mut x[self.cur_row][self.cur_col]).write(ch);

                self.cur_col += 1;
                if self.cur_col >= VIDEO_WIDTH {
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
        } else {
            self.cur_row -= 1;
        }

        //self.video.copy_within(1..SCREEN_HEIGHT, 0);
        for row in 0..(SCREEN_HEIGHT - 1) {
            let line = self.video.map(|x| &x[row + 1]).read();
            self.video.map_mut(|x| &mut x[row]).write(line);
        }

        let empty = ScreenChar { character: 0, color: ColorCode::DEFAULT };
        let mut lastrow = self.video.map_mut(|x| &mut x[SCREEN_HEIGHT - 1]);
        for col in 0..VIDEO_WIDTH {
            lastrow.map_mut(|x| &mut x[col]).write(empty)
        }
    }

    fn update_cursor(&self) {
        unsafe {
            let mut port1 = Port::<u8>::new(0x3d4);
            let mut port2 = Port::<u8>::new(0x3d5);

            let pos = self.cur_row * VIDEO_WIDTH + self.cur_col;
            port1.write(0x0f);
            port2.write((pos & 0xff) as u8);
            port1.write(0x0e);
            port2.write((pos >> 8 & 0xff) as u8);
        }
    }
}

impl fmt::Write for Terminal {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(ColorCode::DEFAULT, s);
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::terminal::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    without_interrupts(|| {
        TERM.lock().write_fmt(args).unwrap();
    });
}
