use embedded_canvas::Canvas;
use embedded_graphics::{geometry::Point, image::Image, Drawable};
use tinybmp::Bmp;

use crate::{display::bwr_color::BWRColor, state::app::ApplicationState};

use super::DisplayComponent;

pub struct ImageBackground {
    pub name: &'static str,
    pub display: u8,
    image: Box<Bmp<'static, BWRColor>>,
}

impl ImageBackground {
    pub fn new(name: &'static str, display: u8, image: Box<Bmp<'static, BWRColor>>) -> Self {
        Self {
            name,
            display,
            image,
        }
    }
}

impl DisplayComponent for ImageBackground {
    fn get_display(&self) -> u8 {
        self.display
    }

    fn get_type(&self) -> super::DisplayAreaType {
        super::DisplayAreaType::Fullscreen
    }

    fn get_name(&self) -> &str {
        self.name
    }

    fn draw(
        &mut self,
        target: &mut Canvas<BWRColor>,
        _state: &ApplicationState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Image::new(self.image.as_ref(), Point::new(0, 0)).draw(target)?;
        Ok(())
    }

    fn get_z_index(&self, _state: &ApplicationState) -> u32 {
        10
    }
}
