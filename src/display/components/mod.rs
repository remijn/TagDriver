use std::{error::Error, time::Instant};

pub mod bar_dialog;

use super::{super::dbus::*, bwr_display::BWRDisplay};

pub trait DBusConsumer {
    fn needs_refresh(&self, new_values: &DBusValueMap) -> bool;
    fn wanted_dbus_values(&self) -> Vec<&DBusPropertyAdress>;
}

pub type NextRefresh = (Instant, RefreshType);

pub enum RefreshType {
    Full,
    Partial,
}

pub enum DisplayAreaType {
    Area,       // item contained in area, i.e. icon for bluetooth
    Fullscreen, // Fullscreen thing, large clock or image, stays open
    Dialog, // Fullscreen dialog that shows on change, (volume or brightness popup, notifications maybe), and dissapears after
}

pub trait DisplayComponent {
    fn get_screen(&self) -> u8;
    fn get_type(&self) -> DisplayAreaType;
    fn get_name(&self) -> &str;
    fn draw(
        &mut self,
        target: &mut BWRDisplay,
        values: Box<DBusValueMap>,
    ) -> Result<(), Box<dyn Error>>;
    fn get_z_index(&self, values: &DBusValueMap) -> u32;

    fn dbus(&self) -> Option<&dyn DBusConsumer>;
}
