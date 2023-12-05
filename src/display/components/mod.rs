use std::{error::Error, time::Instant};

pub mod bar_dialog;
pub mod image_background;
pub mod simple_item;
pub mod state_item;

use embedded_canvas::Canvas;
use embedded_graphics::prelude::Point;

use crate::state::ApplicationState;

use super::bwr_color::BWRColor;

pub trait ApplicationStateConsumer {
    fn needs_refresh(&self, new_values: &ApplicationState) -> bool;
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
        values: &ApplicationState,
    ) -> Result<(), Box<dyn Error>>;
    fn get_z_index(&self, values: &ApplicationState) -> u32;
    fn get_refresh_at(&self) -> Option<Instant> {
        None
    }

    fn state_consumer(&self) -> Option<&dyn ApplicationStateConsumer> {
        None
    }
    fn state_consumer_mut(&mut self) -> Option<&mut dyn ApplicationStateConsumer> {
        None
    }
}

pub trait IconComponent {
    fn draw_icon(&self, target: &mut Canvas<BWRColor>, value: f64, center: Point);
}
