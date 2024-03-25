use core::fmt::{self, Write};

use display_interface::DisplayError;
use embedded_graphics::{
    mono_font::{iso_8859_1::FONT_10X20 as FONT, MonoTextStyle, MonoTextStyleBuilder}, pixelcolor::BinaryColor, prelude::*, text::{Baseline, Text}
};

pub struct TextBuffer<const SZ: usize> {
    buf: [u8; SZ],
    len: usize,
}

impl<const SZ: usize> TextBuffer<SZ> {
    fn new() -> Self {
        TextBuffer {
            buf: [0; SZ],
            len: 0,
        }
    }
    fn as_str<'a>(&'a self) -> &'a str{
        core::str::from_utf8(&self.buf[..self.len]).unwrap()
    }
}

impl<const SZ: usize> fmt::Write for TextBuffer<SZ> {
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
    .font(&FONT)
    .text_color(BinaryColor::On)
    .build();

pub fn draw<D>(d: &mut D, tmpr: f32, humi: f32) where
    D: DrawTarget<Color = BinaryColor, Error = DisplayError>
{
    // Create the text to display, including the temperature variable
    let mut buf = TextBuffer::<32>::new();
    write!(&mut buf, "Tmpr: {:.1}°C\nHumi: {:.1}%", tmpr, humi).unwrap();

    // Draw the text on the display
    d.clear(BinaryColor::Off).unwrap();
    defmt::debug!("Drawing text: {}", buf.as_str());
    Text::with_baseline(buf.as_str(), Point::zero(), MONO_TEXT_STYLE, Baseline::Top)
        .draw(d)
        .unwrap();
}
