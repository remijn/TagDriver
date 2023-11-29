use embedded_canvas::Canvas;
use embedded_graphics::{geometry::Point, image::Image, Drawable};
use tinybmp::Bmp;

use crate::{display::bwr_color::BWRColor, state::ApplicationState};

use super::DisplayComponent;

pub struct ImageBackground {
    pub name: &'static str,
    pub screen: u8,
    image: Box<Bmp<'static, BWRColor>>,
}

impl ImageBackground {
    pub fn new(name: &'static str, screen: u8, image: Box<Bmp<'static, BWRColor>>) -> Self {
        Self {
            name,
            screen,
            image,
        }
    }
}

impl DisplayComponent for ImageBackground {
    fn get_screen(&self) -> u8 {
        self.screen
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
        return Ok(());
    }

    fn get_z_index(&self, _state: &ApplicationState) -> u32 {
        10
    }
}
