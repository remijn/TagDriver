use super::super::{HEIGHT, REAL_HEIGHT, WIDTH};
use super::bwr_color::BWRColor;

use core::convert::TryInto;
use embedded_graphics::prelude::*;
pub struct BWRDisplay {
    /// The framebuffer with a single byte per pixel
    framebuffer: [u8; (WIDTH * HEIGHT) as usize],
    // black_buffer: [u8; (WIDTH * HEIGHT / 8) as usize],
    // red_buffer: [u8; (WIDTH * HEIGHT / 8) as usize],
}

impl BWRDisplay {
    pub fn new() -> Self {
        Self {
            framebuffer: [0; (WIDTH * HEIGHT) as usize],
        }
    }

    pub fn get_fixed_buffer(&mut self) -> (Vec<u8>, Vec<u8>) {
        // New correctly sized buffer
        let mut black_buffer: [u8; (WIDTH * HEIGHT / 8) as usize] =
            [0; (WIDTH * HEIGHT / 8) as usize];
        let mut red_buffer: [u8; (WIDTH * HEIGHT / 8) as usize] =
            [0; (WIDTH * HEIGHT / 8) as usize];

        let mut i = 0;
        while i < WIDTH * HEIGHT / 8 {
            //Iterate through new buff, I is bytes
            let mut black_byte: u8 = 0b0000_0000;
            let mut red_byte: u8 = 0b0000_0000;
            let bit: u8 = 0b1000_0000;

            let mut j: u8 = 0;
            while j < 8 {
                //Iterate through bits

                if (self.framebuffer[((i * 8) + j as u32) as usize]) == 1 {
                    black_byte |= bit >> j;
                }
                if (self.framebuffer[((i * 8) + j as u32) as usize]) == 2 {
                    red_byte |= bit >> j;
                }

                j += 1;
            }
            black_buffer[i as usize] = (black_byte) ^ 0xFF; // Toggle all the bits, to invert the colors
            red_buffer[i as usize] = red_byte;
            i += 1;
        }
        (black_buffer.to_vec(), red_buffer.to_vec())
    }

    #[allow(dead_code)]
    pub fn partial_buffer(&mut self, black_buffer: &[u8], point: Point, size: Size) -> Vec<u8> {
        let mut pbuf: Vec<u8> = Vec::new();
        pbuf.resize((size.width * size.height / 8) as usize, 0);

        for tx in 0..size.width {
            for ty in 0..size.height / 8 {
                let buf_i: u32 = tx * size.height / 8 + ty;

                let old_i: u32 = (tx + point.x as u32) * HEIGHT / 8 + ty + (point.y as u32 / 8);

                pbuf[buf_i as usize] = black_buffer[old_i as usize];
            }
        }
        return pbuf;
    }
}

impl DrawTarget for BWRDisplay {
    type Color = BWRColor;
    // `ExampleDisplay` uses a framebuffer and doesn't need to communicate with the display
    // controller to draw pixel, which means that drawing operations can never fail. To reflect
    // this the type `Infallible` was chosen as the `Error` type.
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(coord, color) in pixels.into_iter() {
            // Check if the pixel coordinates are out of bounds (negative or greater than
            // (250,128)). `DrawTarget` implementation are required to discard any out of bounds
            // pixels without returning an error or causing a panic.

            if let Ok((x @ 0..=249, y @ 0..=121)) = coord.try_into() {
                // Calculate the index in the framebuffer.
                let index: u32 = x * 128 + y;
                self.framebuffer[index as usize] = (color) as u8;
            }
        }

        Ok(())
    }
}

impl OriginDimensions for BWRDisplay {
    fn size(&self) -> Size {
        Size::new(WIDTH, REAL_HEIGHT)
    }
}

#[cfg(test)]
mod tests {

    use super::super::bwr_color::BWRColor;
    use embedded_graphics::{
        mono_font::{ascii::FONT_10X20, MonoTextStyle},
        prelude::*,
        primitives::{Circle, Line, PrimitiveStyle, Rectangle},
        text::Text,
    };
    use embedded_graphics_simulator::{
        BinaryColorTheme, OutputSettingsBuilder, SimulatorDisplay, Window,
    };

    #[test]
    fn display() -> Result<(), core::convert::Infallible> {
        // Start simulator
        let mut display = SimulatorDisplay::<BWRColor>::new(Size::new(250, 128));

        //Set styles
        let line_style = PrimitiveStyle::with_stroke(BWRColor::On, 1);
        let text_style = MonoTextStyle::new(&FONT_10X20, BWRColor::On);

        // Shapes
        Circle::new(Point::new(72, 8), 48)
            .into_styled(line_style)
            .draw(&mut display)?;

        Line::new(Point::new(48, 16), Point::new(8, 16))
            .into_styled(line_style)
            .draw(&mut display)?;

        Line::new(Point::new(48, 16), Point::new(64, 32))
            .into_styled(line_style)
            .draw(&mut display)?;

        Rectangle::new(Point::new(79, 15), Size::new(34, 34))
            .into_styled(line_style)
            .draw(&mut display)?;

        Text::new("Hello World!", Point::new(5, 5), text_style).draw(&mut display)?;

        // Output
        let output_settings = OutputSettingsBuilder::new()
            .theme(BinaryColorTheme::Default)
            .build();

        Window::new("Hello World", &output_settings).show_static(&display);

        Ok(())
    }
}
