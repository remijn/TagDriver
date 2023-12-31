use super::{bwr_color::BWRColor, DisplayFlip, DisplayRotation};

use embedded_graphics::prelude::*;

pub struct BWRDisplay {
    /// The framebuffer with a single byte per pixel
    framebuffer: Vec<u8>,
    width: u32,
    height: u32,
    buffer_height: u32, // black_buffer: [u8; (WIDTH * HEIGHT / 8) as usize],
    // red_buffer: [u8; (WIDTH * HEIGHT / 8) as usize],
    rotate: DisplayRotation,
    flip: DisplayFlip,
}

impl BWRDisplay {
    pub fn new(width: u32, height: u32, rotate: DisplayRotation, flip: DisplayFlip) -> Self {
        let mut buffer_height: u32 = height;

        if rotate == DisplayRotation::Rotate90 || rotate == DisplayRotation::Rotate270 {
            buffer_height = width;
        }

        if buffer_height % 8 != 0 {
            buffer_height = (buffer_height / 8 + 1) * 8;
        }

        let framebuffer = vec![0; (width * buffer_height) as usize];

        Self {
            framebuffer,
            width,
            height,
            buffer_height,
            rotate,
            flip,
        }
    }

    pub fn get_fixed_buffer(&mut self) -> (Vec<u8>, Vec<u8>) {
        // New correctly sized buffer
        let mut black_buffer = vec![0; (self.width * (self.buffer_height / 8)) as usize];

        let mut red_buffer = vec![0; (self.width * (self.buffer_height / 8)) as usize];

        let mut i = 0;
        while i < self.width * self.buffer_height / 8 {
            //Iterate through new buff, I is bytes
            let mut black_byte: u8 = 0b0000_0000;
            let mut red_byte: u8 = 0b0000_0000;
            let bit: u8 = 0b1000_0000;

            let mut j: u32 = 0;
            while j < 8 {
                //Iterate through bits

                if (self.framebuffer[((i * 8) + j) as usize]) == 1 {
                    black_byte |= bit >> j;
                }
                if (self.framebuffer[((i * 8) + j) as usize]) == 2 {
                    red_byte |= bit >> j;
                }

                j += 1;
            }
            black_buffer[i as usize] = black_byte;
            red_buffer[i as usize] = red_byte;
            i += 1;
        }
        (black_buffer, red_buffer)
    }

    #[allow(dead_code)]
    pub fn partial_buffer(&mut self, black_buffer: &[u8], point: Point, size: Size) -> Vec<u8> {
        let mut pbuf: Vec<u8> = vec![0; (size.width * size.height / 8) as usize];

        for tx in 0..size.width {
            for ty in 0..size.height / 8 {
                let buf_i: u32 = tx * size.height / 8 + ty;

                let old_i: u32 =
                    (tx + point.x as u32) * self.buffer_height / 8 + ty + (point.y as u32 / 8);

                pbuf[buf_i as usize] = black_buffer[old_i as usize];
            }
        }
        pbuf
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

            // if let Ok((x @ 0..self.width, y @ 0..=121)) = coord.try_into() {
            // Calculate the index in the framebuffer.

            let mut cx = coord.x;
            let mut cy = coord.y;

            // Flip upside down
            if self.rotate == DisplayRotation::Rotate180
                || self.rotate == DisplayRotation::Rotate270
            {
                cx = -cx + (self.width as i32 - 1);
                cy = -cy + (self.height as i32 - 1);
            }

            match self.flip {
                DisplayFlip::Horizontal => {
                    cx = -cx + (self.width as i32 - 1);
                }
                DisplayFlip::Vertical => {
                    cy = -cy + (self.height as i32 - 1);
                }
                DisplayFlip::None => {}
            }

            if self.rotate == DisplayRotation::Rotate90 || self.rotate == DisplayRotation::Rotate270
            {
                std::mem::swap(&mut cx, &mut cy);
            }

            if coord.x >= 0
                && coord.y >= 0
                && coord.x < self.width as i32
                && coord.y < self.height as i32
            {
                // Limit drawing outside buffer
                let index = cx as u32 * self.buffer_height + cy as u32;
                self.framebuffer[index as usize] = (color) as u8;
            }

            // }
        }

        Ok(())
    }
}

impl OriginDimensions for BWRDisplay {
    fn size(&self) -> Size {
        Size::new(self.width, self.height)
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
