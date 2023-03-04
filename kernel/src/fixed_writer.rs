use core::fmt::{Write, Error};

// https://stackoverflow.com/questions/71678897/how-can-i-write-a-formatted-string-up-to-buffer-size-in-rust

pub struct FixedWriter<'a, W>(&'a mut W);

impl<'a, W: Write> FixedWriter<'a, W> {
    pub fn new(writer: &'a mut W) -> Self {
        Self(writer)
    }
}

impl<'a, W: Write> Write for FixedWriter<'a, W> {
    fn write_str(&mut self, s: &str) -> Result<(), Error> {
        for c in s.chars() {
            self.0.write_char(c)?;
        }
        Ok(())
    } 
}
