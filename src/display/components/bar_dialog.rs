use std::{
    collections::HashMap,
    error::Error,
    time::{Duration, Instant},
};

use embedded_canvas::Canvas;
use embedded_graphics::{
    prelude::{OriginDimensions, Point, Size},
    primitives::{Primitive, Rectangle},
    Drawable,
};

use crate::{
    dbus::{DBusPropertyAdress, DBusValue, DBusValueMap},
    display::{bwr_color::BWRColor, FILL_STYLE_FG, OUTLINE_STYLE_FG},
    log,
};

use super::IconComponent;

use super::{DBusConsumer, DisplayAreaType, DisplayComponent};

pub struct BarDialog {
    pub name: &'static str,
    pub property: &'static DBusPropertyAdress,
    pub screen: u8,
    close_at: Instant,
    old_values: Box<DBusValueMap>, // Values last drawn
    _draw_icon: Box<dyn Fn(&mut Canvas<BWRColor>, f64, Point)>,
}

const OPEN_TIME: Duration = Duration::from_secs(5);
impl BarDialog {
    pub fn new(
        name: &'static str,
        property: &'static DBusPropertyAdress,
        screen: u8,
        draw_icon: Box<dyn Fn(&mut Canvas<BWRColor>, f64, Point)>,
    ) -> Self {
        Self {
            name,
            property,
            screen,
            old_values: Box::new(HashMap::new()),
            close_at: Instant::now() - OPEN_TIME,
            _draw_icon: draw_icon,
        }
    }
}

impl IconComponent for BarDialog {
    fn draw_icon(&self, target: &mut Canvas<BWRColor>, value: f64, center: Point) {
        (self._draw_icon)(target, value, center);
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
    fn dbus_mut(&mut self) -> Option<&mut dyn DBusConsumer> {
        Some(self)
    }

    fn get_z_index(&self, values: &DBusValueMap) -> u32 {
        let res = values.get(&self.property);

        if let None = res {
            println!(
                "{} Can't get z-index, property {} does not exist in values",
                log::ERROR,
                self.property
            );
            return 0;
        }

        let value = res.expect(
            format!(
                "Can't get z-index, property {} does not exist in values",
                self.property
            )
            .as_str(),
        );

        if let Some(val) = self.old_values.get(&self.property) {
            if val != value {
                return 100; //changed value
            }
        } else {
            return 100; //new value
        }

        if Instant::now() < self.close_at {
            let elapsed = Instant::now() - self.close_at;
            if elapsed < OPEN_TIME {
                return 90 - (elapsed.as_millis() / 100) as u32;
            }
        }

        return 0;
    }

    fn draw(
        &mut self,
        target: &mut Canvas<BWRColor>,
        values: Box<DBusValueMap>,
    ) -> Result<(), Box<dyn Error>> {
        let res = values.get(&self.property);
        if let None = res {
            println!(
                "{} Can't get z-index, property {} does not exist in values",
                log::ERROR,
                self.property
            );
            return Ok(());
        }
        let value = res.expect(
            format!(
                "Can't draw component, property {} does not exist in values",
                self.property
            )
            .as_str(),
        );

        if let Some(val) = self.old_values.get(&self.property) {
            if val != value {
                // Different value, we reset timeout and open
                self.close_at = Instant::now() + OPEN_TIME;
            }
        }
        self.old_values = values.clone();

        let bar_width: u32 = 155;
        let bar_height: u32 = 60;
        let bar_x: i32 = ((target.size().width - bar_width) / 2) as i32 + 30;
        let bar_y: i32 = ((target.size().height - bar_height) / 2) as i32;

        let float_value = match value {
            DBusValue::F64(val) => *val,
            DBusValue::U64(val) => *val as f64 / 100.0,
            DBusValue::I64(val) => *val as f64 / 100.0,
            _ => 69.0,
        };

        let icon_center = Point {
            x: bar_x - 40,
            y: (target.size().height / 2) as i32,
        };

        self.draw_icon(target, float_value, icon_center);

        let filled_width = (float_value * bar_width as f64) as u32;

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

    fn get_refresh_at(&self) -> Option<Instant> {
        if self.close_at > Instant::now() {
            return Some(self.close_at);
        }
        return None;
    }
}

impl DBusConsumer for BarDialog {
    fn wanted_dbus_values(&self) -> Vec<&'static DBusPropertyAdress> {
        return [self.property].to_vec();
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

    fn set_initial(&mut self, new_values: &DBusValueMap) {
        self.old_values = Box::new(new_values.clone());
    }
}
