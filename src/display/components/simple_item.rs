use embedded_canvas::Canvas;
use embedded_graphics::geometry::Point;

use crate::display::bwr_color::BWRColor;

use super::{DisplayComponent, IconComponent};

pub struct SimpleItem {
    pub name: &'static str,
    pub screen: u8,
    pub width: u32,
    pub height: u32,
    _draw_icon: Box<dyn Fn(&mut Canvas<BWRColor>, Point)>,
}

impl SimpleItem {
    pub fn new(
        name: &'static str,
        screen: u8,
        draw_icon: Box<dyn Fn(&mut Canvas<BWRColor>, Point)>,
    ) -> Self {
        Self {
            name,
            screen,
            width: 50,
            height: 50,
            _draw_icon: draw_icon,
        }
    }
}

impl IconComponent for SimpleItem {
    fn draw_icon(&self, target: &mut Canvas<BWRColor>, _value: f64, center: Point) {
        (self._draw_icon)(target, center);
    }
}

impl DisplayComponent for SimpleItem {
    fn get_screen(&self) -> u8 {
        self.screen
    }

    fn get_type(&self) -> super::DisplayAreaType {
        super::DisplayAreaType::Area(self.width, self.height)
    }

    fn get_name(&self) -> &str {
        self.name
    }

    fn draw(
        &mut self,
        target: &mut Canvas<BWRColor>,
        _values: Box<crate::dbus::DBusValueMap>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.draw_icon(
            target,
            0.0,
            Point::new(self.width as i32 / 2, self.height as i32 / 2),
        );

        return Ok(());
    }

    fn get_z_index(&self, _values: &crate::dbus::DBusValueMap) -> u32 {
        20
    }

    fn dbus(&self) -> Option<&dyn super::DBusConsumer> {
        None
    }

    fn dbus_mut(&mut self) -> Option<&mut dyn super::DBusConsumer> {
        None
    }
}
