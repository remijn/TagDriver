use std::{error::Error, time::Instant};

pub mod bar_dialog;
pub mod image_background;
pub mod simple_item;
pub mod state_item;

use embedded_canvas::Canvas;
use embedded_graphics::prelude::Point;

use super::{super::dbus::*, bwr_color::BWRColor};

pub trait DBusConsumer {
    fn needs_refresh(&self, new_values: &DBusValueMap) -> bool;
    fn wanted_dbus_values(&self) -> Vec<DBusPropertyAdress>;
    fn set_initial(&mut self, new_values: &DBusValueMap);
}

pub type NextRefresh = (Instant, RefreshType);

pub enum RefreshType {
    Full,
    Partial,
}

#[derive(PartialEq, Eq)]
pub enum DisplayAreaType {
    Area(u32, u32), // item contained in area, i.e. icon for bluetooth
    Fullscreen,     // Fullscreen thing, large clock or image, stays open
    Dialog, // Fullscreen dialog that shows on change, (volume or brightness popup, notifications maybe), and dissapears after
}
pub trait DisplayComponent {
    fn get_screen(&self) -> u8;
    fn get_type(&self) -> DisplayAreaType;
    fn get_name(&self) -> &str;
    fn draw(
        &mut self,
        target: &mut Canvas<BWRColor>,
        values: Box<DBusValueMap>,
    ) -> Result<(), Box<dyn Error>>;
    fn get_z_index(&self, values: &DBusValueMap) -> u32;
    fn get_refresh_at(&self) -> Option<Instant> {
        None
    }

    fn dbus(&self) -> Option<&dyn DBusConsumer> {
        None
    }
    fn dbus_mut(&mut self) -> Option<&mut dyn DBusConsumer> {
        None
    }
}

pub trait IconComponent {
    fn draw_icon(&self, target: &mut Canvas<BWRColor>, value: f64, center: Point);
}
