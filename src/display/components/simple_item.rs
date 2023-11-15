use embedded_graphics::geometry::Point;

use crate::display::bwr_display::BWRDisplay;

use super::{DisplayComponent, IconComponent};

pub struct SimpleItem {
    pub name: &'static str,
    pub center: Point,
    pub screen: u8,
    _draw_icon: Box<dyn Fn(&mut BWRDisplay, Point)>,
}

impl SimpleItem {
    pub fn new(
        name: &'static str,
        center: Point,
        screen: u8,
        draw_icon: Box<dyn Fn(&mut BWRDisplay, Point)>,
    ) -> Self {
        Self {
            name,
            center,
            screen,
            _draw_icon: draw_icon,
        }
    }
}

impl IconComponent for SimpleItem {
    fn draw_icon(&self, target: &mut BWRDisplay, _value: f64, center: Point) {
        (self._draw_icon)(target, center);
    }
}

impl DisplayComponent for SimpleItem {
    fn get_screen(&self) -> u8 {
        self.screen
    }

    fn get_type(&self) -> super::DisplayAreaType {
        super::DisplayAreaType::Area
    }

    fn get_name(&self) -> &str {
        self.name
    }

    fn draw(
        &mut self,
        target: &mut crate::display::bwr_display::BWRDisplay,
        _values: Box<crate::dbus::DBusValueMap>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.draw_icon(target, 0.0, self.center);

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
