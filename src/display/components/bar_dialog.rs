use std::{collections::HashMap, error::Error, time::Instant};

use embedded_graphics::{
    prelude::{OriginDimensions, Point, Size},
    primitives::{Primitive, Rectangle},
    Drawable,
};

use crate::{
    dbus::{DBusPropertyAdress, DBusValue, DBusValueMap},
    display::{FILL_STYLE_FG, OUTLINE_STYLE_FG},
};

use super::super::bwr_display::BWRDisplay;

use super::{DBusConsumer, DisplayAreaType, DisplayComponent};

pub struct BarDialog {
    pub name: &'static str,
    pub property: DBusPropertyAdress,
    pub screen: u8,
    pub is_open: bool,
    pub close_at: Box<Instant>,
    old_values: Box<DBusValueMap>,
}

impl BarDialog {
    pub fn new(name: &'static str, property: DBusPropertyAdress, screen: u8) -> Self {
        Self {
            name,
            property,
            screen,
            is_open: false,
            old_values: Box::new(HashMap::new()),
            close_at: Box::new(Instant::now()),
        }
    }
}

impl DisplayComponent for BarDialog {
    fn get_name(&self) -> &str {
        self.name
    }
    fn get_type(&self) -> DisplayAreaType {
        DisplayAreaType::Dialog
    }
    fn get_screen(&self) -> u8 {
        self.screen
    }
    fn dbus(&self) -> Option<&dyn DBusConsumer> {
        Some(self)
    }

    fn draw(
        &mut self,
        target: &mut BWRDisplay,
        values: Box<DBusValueMap>,
    ) -> Result<(), Box<dyn Error>> {
        self.old_values = values.clone();

        let bar_width: u32 = 200;
        let bar_height: u32 = 60;
        let bar_x: i32 = ((target.size().width - bar_width) / 2) as i32;
        let bar_y: i32 = ((target.size().height - bar_height) / 2) as i32;

        let value = values.get(&self.property).expect(
            format!(
                "Can't draw component, property {} does not exist in values",
                self.property
            )
            .as_str(),
        );

        let percentage = match value {
            DBusValue::F64(val) => *val,
            DBusValue::U64(val) => *val as f64,
            DBusValue::I64(val) => *val as f64,
            _ => 69.0,
        };

        let filled_width = (percentage / 100.0 * bar_width as f64) as u32;

        // Draw outline
        Rectangle::new(
            Point { x: bar_x, y: bar_y },
            Size {
                width: bar_width,
                height: bar_height,
            },
        )
        .into_styled(OUTLINE_STYLE_FG)
        .draw(target)?;

        // Draw fill
        Rectangle::new(
            Point { x: bar_x, y: bar_y },
            Size {
                width: filled_width,
                height: bar_height,
            },
        )
        .into_styled(FILL_STYLE_FG)
        .draw(target)?;

        return Ok(());
    }

    fn get_z_index(&self, _values: &DBusValueMap) -> u32 {
        todo!()
    }
}

impl DBusConsumer for BarDialog {
    fn wanted_dbus_values(&self) -> Vec<&DBusPropertyAdress> {
        return [&self.property].to_vec();
    }

    fn needs_refresh(&self, new_values: &DBusValueMap) -> bool {
        if new_values.contains_key(&self.property) {
            let new_v = new_values.get(&self.property).expect("");
            let old_v = self.old_values.get(&self.property);

            match new_v {
                DBusValue::F64(val) => {
                    if let Some(DBusValue::F64(old)) = old_v {
                        return *old != *val; // return true if value is not the same
                    } else {
                        return true; // only new value, no old value, we should refresh
                    }
                }
                DBusValue::U64(val) => {
                    if let Some(DBusValue::U64(old)) = old_v {
                        return *old != *val; // return true if value is not the same
                    } else {
                        return true; // only new value, no old value, we should refresh
                    }
                }
                DBusValue::I64(val) => {
                    if let Some(DBusValue::I64(old)) = old_v {
                        return *old != *val; // return true if value is not the same
                    } else {
                        return true; // only new value, no old value, we should refresh
                    }
                }
                _ => 69.0 as f64,
            };
        }
        // our key was not found
        return false;
    }
}
