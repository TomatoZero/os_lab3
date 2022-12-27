use core::fmt;
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::instructions::interrupts::without_interrupts;

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buf::_print(format_args!($($arg)*)));
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
        SCREEN.lock().write_fmt(args).unwrap();
    });
}

const BUF_HEIGHT: u32 = 25;
const BUF_WIDTH: u32 = 80;
const BUF_SIZE: usize = (BUF_HEIGHT * BUF_WIDTH * 2) as usize;

lazy_static! {
    pub static ref SCREEN: Mutex<Screen> = Mutex::new(
        {
            let mut screen = Screen {
                color: 0xa,
                buffer: unsafe {&mut *(0xb8000 as *mut [u8; BUF_SIZE])},
                line: 0,
                col: 0
            };
            screen.clear();
            screen
        }
    );
}

pub struct AsciiChar {
    pub char_byte: u8,
    pub color_byte: u8,
}

pub struct Screen {
    color: u8,
    buffer: &'static mut [u8; BUF_SIZE],
    line: u32,
    col: u32
}

impl core::fmt::Write for Screen {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.print(s);
        Ok(())
    }
}

impl Screen {

    pub fn clear(&mut self) {
        for i in 0..BUF_HEIGHT {
            for j in 0..BUF_WIDTH {
                self.write_char_byte(i * BUF_WIDTH + j, 0x00)
            }
        }
        self.col = 0;
        self.line = 0;
    }

    pub fn print(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                b'\n' => {
                    if self.line == BUF_HEIGHT - 1 {
                        self.scroll_up();
                    } else {
                        self.line += 1;
                    }
                    self.col = 0;
                }
                b => {
                    self.write_char_byte(self.line * BUF_WIDTH + self.col, b);
                    self.col += 1;
                    if self.col == BUF_WIDTH {
                        self.col = 0;
                        self.print("\n");
                    }
                }
            }
        }
    }

    fn scroll_up(&mut self) {
        for i in 0..self.line {
            for j in 0..BUF_WIDTH {
                let char_to_copy = self.read_char(BUF_WIDTH * (i + 1) + j);
                self.write_char(BUF_WIDTH * i + j, char_to_copy);
            }
        }
        for i in 0..BUF_WIDTH {
            self.write_char(self.line * BUF_WIDTH + i, AsciiChar { char_byte: b' ', color_byte: 0x00 });
        }
    }

    fn write_char_byte(&mut self, offset: u32, char_byte: u8) {
        self.write_char(offset, AsciiChar { char_byte, color_byte: self.color })
    }

    fn write_char(&mut self, offset: u32, char: AsciiChar) {
        self.buffer[offset as usize * 2] = char.char_byte;
        self.buffer[offset as usize * 2 + 1] = char.color_byte;
    }

    fn read_char(&self, offset: u32) -> AsciiChar {
        unsafe {
            return AsciiChar {
                char_byte: self.buffer[offset as usize * 2],
                color_byte: self.buffer[offset as usize * 2 + 1],
            };
        }
    }
}
