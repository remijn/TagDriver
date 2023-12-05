#![allow(clippy::type_complexity)]
use std::{
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
    display::{bwr_color::BWRColor, FILL_STYLE_FG, OUTLINE_STYLE_FG},
    log,
    state::{app::ApplicationState, value::StateValueType},
};

use super::{ApplicationStateConsumer, DisplayAreaType, DisplayComponent, IconComponent};

pub struct BarDialog {
    pub name: &'static str,
    pub property: &'static str,
    pub display: u8,
    close_at: Instant,
    old_state: ApplicationState, // Values last drawn
    _draw_icon: Box<dyn Fn(&mut Canvas<BWRColor>, f64, Point)>,
}

const OPEN_TIME: Duration = Duration::from_secs(5);
impl BarDialog {
    pub fn new(
        name: &'static str,
        property: &'static str,
        display: u8,
        initial_state: ApplicationState,
        draw_icon: Box<dyn Fn(&mut Canvas<BWRColor>, f64, Point)>,
    ) -> Self {
        Self {
            name,
            property,
            display,
            old_state: initial_state,
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
    fn get_display(&self) -> u8 {
        self.display
    }
    fn state_consumer(&self) -> Option<&dyn ApplicationStateConsumer> {
        Some(self)
    }
    fn state_consumer_mut(&mut self) -> Option<&mut dyn ApplicationStateConsumer> {
        Some(self)
    }

    fn get_z_index(&self, values: &ApplicationState) -> u32 {
        let res = values.get(self.property);

        if res.is_none() {
            println!(
                "{} Can't get z-index, property {} does not exist in values",
                log::ERROR,
                self.property
            );
            return 0;
        }

        let value = res.unwrap_or_else(|| {
            panic!(
                "Can't get z-index, property {} does not exist in values",
                self.property
            )
        });

        if let Some(val) = self.old_state.get(self.property) {
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

        0
    }

    fn draw(
        &mut self,
        target: &mut Canvas<BWRColor>,
        values: &ApplicationState,
    ) -> Result<(), Box<dyn Error>> {
        let res = values.get(self.property);
        if res.is_none() {
            println!(
                "{} Can't get z-index, property {} does not exist in values",
                log::ERROR,
                self.property
            );
            return Ok(());
        }
        let value = res.unwrap_or_else(|| {
            panic!(
                "Can't draw component, property {} does not exist in values",
                self.property
            )
        });

        if let Some(val) = self.old_state.get(self.property) {
            if val != value {
                // Different value, we reset timeout and open
                self.close_at = Instant::now() + OPEN_TIME;
            }
        }
        self.old_state = values.clone();

        let bar_width: u32 = 155;
        let bar_height: u32 = 60;
        let bar_x: i32 = ((target.size().width - bar_width) / 2) as i32 + 30;
        let bar_y: i32 = ((target.size().height - bar_height) / 2) as i32;

        let icon_center = Point {
            x: bar_x - 40,
            y: (target.size().height / 2) as i32,
        };

        let float_value = match value {
            StateValueType::F64(val) => *val,
            StateValueType::U64(val) => *val as f64,
            StateValueType::I64(val) => *val as f64,
            _ => {
                panic!("Cannot convert to f64");
            }
        } / 100.0;

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

        Ok(())
    }

    fn get_refresh_at(&self) -> Option<Instant> {
        if self.close_at > Instant::now() {
            return Some(self.close_at);
        }
        None
    }
}

impl ApplicationStateConsumer for BarDialog {
    fn needs_refresh(&self, new_state: &ApplicationState) -> bool {
        let property = self.property;

        let new_value = new_state
            .map
            .get(property)
            .expect("Property not found in app state");

        if let Some(new_value_type) = &new_value.get() {
            let old_value = self
                .old_state
                .map
                .get(property)
                .expect("Property not found in old app state")
                .get();

            match new_value_type {
                StateValueType::F64(val) => {
                    if let Some(StateValueType::F64(old)) = old_value {
                        return old != *val;
                    } else {
                        return true; // only new value, no old value, we should refresh
                    }
                }
                StateValueType::U64(val) => {
                    if let Some(StateValueType::U64(old)) = old_value {
                        return old != *val;
                    } else {
                        return true; // only new value, no old value, we should refresh
                    }
                }
                StateValueType::I64(val) => {
                    if let Some(StateValueType::I64(old)) = old_value {
                        return old != *val;
                    } else {
                        return true; // only new value, no old value, we should refresh
                    }
                }
                StateValueType::String(val) => {
                    if let Some(StateValueType::String(old)) = &old_value {
                        return *old != *val;
                    } else {
                        return true; // only new value, no old value, we should refresh
                    }
                }
                StateValueType::NetworkState(val) => {
                    if let Some(StateValueType::NetworkState(old)) = &old_value {
                        return *old != *val;
                    } else {
                        return true; // only new value, no old value, we should refresh
                    }
                }
            };
        }
        // our key was not found
        false
    }
}
