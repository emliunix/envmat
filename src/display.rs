use core::fmt::{self, Write};

use display_interface::DisplayError;
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};

pub struct TextBuffer {
    buf: [u8; 256],
    len: usize,
}

impl TextBuffer {
    fn new() -> Self {
        TextBuffer {
            buf: [0; 256],
            len: 0,
        }
    }
    fn as_str<'a>(&'a self) -> &'a str{
        core::str::from_utf8(&self.buf[..self.len]).unwrap()
    }
}

impl fmt::Write for TextBuffer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let bytes = s.as_bytes();
        let space_remaining = self.buf.len() - self.len;

        if bytes.len() > space_remaining {
            return Err(fmt::Error);
        }

        let start = self.len;
        let end = start + bytes.len();
        self.buf[start..end].copy_from_slice(bytes);
        self.len += bytes.len();

        Ok(())
    }
}

const MONO_TEXT_STYLE: MonoTextStyle<'_, BinaryColor> = MonoTextStyleBuilder::new()
    .font(&FONT_6X10)
    .text_color(BinaryColor::On)
    .build();

pub fn draw<D>(d: &mut D, temp: f32) where
    D: DrawTarget<Color = BinaryColor, Error = DisplayError>
{
    // Create the text to display, including the temperature variable
    let mut buf = TextBuffer::new();
    write!(&mut buf, "Temperature: {:.1}°C", temp).unwrap();

    // Draw the text on the display
    defmt::debug!("Drawing text: {}", buf.as_str());
    Text::with_baseline(buf.as_str(), Point::zero(), MONO_TEXT_STYLE, Baseline::Top)
        .draw(d)
        .unwrap();
}
