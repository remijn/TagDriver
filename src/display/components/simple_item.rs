use embedded_canvas::Canvas;
use embedded_graphics::{
    geometry::{Point, Size},
    image::Image,
    Drawable,
};
use embedded_icon::{EmbeddedIcon, Icon};

use crate::{display::bwr_color::BWRColor, state::app::ApplicationState};

use super::{DisplayComponent, IconComponent};

pub struct SimpleItem<T: EmbeddedIcon> {
    pub name: &'static str,
    pub display: u8,
    pub size: Size,
    pub icon: Icon<BWRColor, T>,
}

impl<T: EmbeddedIcon> SimpleItem<T> {
    pub fn new(name: &'static str, display: u8, icon: Icon<BWRColor, T>) -> Self {
        Self {
            name,
            display,
            size: Size::new(50, 50),
            icon,
        }
    }
}

impl<T: EmbeddedIcon> IconComponent for SimpleItem<T> {
    fn draw_icon(&self, target: &mut Canvas<BWRColor>, _value: f64, center: Point) {
        Image::with_center(&self.icon, center).draw(target).ok();
    }
}

impl<T: EmbeddedIcon> DisplayComponent for SimpleItem<T> {
    fn get_display(&self) -> u8 {
        self.display
    }

    fn get_type(&self) -> super::DisplayAreaType {
        super::DisplayAreaType::Icon(self.size)
    }

    fn get_name(&self) -> &str {
        self.name
    }

    fn draw(
        &mut self,
        target: &mut Canvas<BWRColor>,
        _values: &ApplicationState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let center = Point::new((self.size.width / 2) as i32, (self.size.height / 2) as i32);
        self.draw_icon(target, 0.0, center);

        Ok(())
    }

    fn get_z_index(&self, _values: &ApplicationState) -> u32 {
        20
    }

    fn state_consumer(&self) -> Option<&dyn super::ApplicationStateConsumer> {
        None
    }

    fn state_consumer_mut(&mut self) -> Option<&mut dyn super::ApplicationStateConsumer> {
        None
    }
}
