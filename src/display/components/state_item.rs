#![allow(clippy::type_complexity)]
use embedded_canvas::Canvas;
use embedded_graphics::geometry::{Point, Size};

use crate::{
    display::bwr_color::BWRColor,
    state::{app::ApplicationState, value::StateValueType},
};

use super::{ApplicationStateConsumer, DisplayComponent, IconComponent};

pub struct StateItem {
    pub name: &'static str,
    pub properties: Vec<&'static str>,
    pub display: u8,
    pub size: Size,
    old_state: ApplicationState, // Values last drawn
    _draw_icon: Box<dyn Fn(&mut Canvas<BWRColor>, &ApplicationState, Point)>,
}

impl StateItem {
    pub fn new(
        name: &'static str,
        properties: Vec<&'static str>,
        display: u8,
        initial_state: ApplicationState,
        draw_icon: Box<dyn Fn(&mut Canvas<BWRColor>, &ApplicationState, Point)>,
    ) -> Self {
        Self {
            name,
            display,
            size: Size::new(50, 50),
            old_state: initial_state,
            properties,
            _draw_icon: draw_icon,
        }
    }
}

impl IconComponent for StateItem {
    fn draw_icon(&self, target: &mut Canvas<BWRColor>, _value: f64, center: Point) {
        (self._draw_icon)(target, &self.old_state, center);
    }
}

impl DisplayComponent for StateItem {
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
        values: &ApplicationState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.old_state = values.clone();

        let center = Point::new((self.size.width / 2) as i32, (self.size.height / 2) as i32);

        self.draw_icon(target, 100.0, center);

        Ok(())
    }

    fn get_z_index(&self, _values: &ApplicationState) -> u32 {
        20
    }

    fn state_consumer(&self) -> Option<&dyn super::ApplicationStateConsumer> {
        Some(self)
    }

    fn state_consumer_mut(&mut self) -> Option<&mut dyn super::ApplicationStateConsumer> {
        Some(self)
    }
}

impl ApplicationStateConsumer for StateItem {
    fn needs_refresh(&self, new_state: &ApplicationState) -> bool {
        for property in self.properties.as_slice() {
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
                            if old == *val {
                                continue;
                            } else {
                                return true;
                            }
                        } else {
                            return true; // only new value, no old value, we should refresh
                        }
                    }
                    StateValueType::U64(val) => {
                        if let Some(StateValueType::U64(old)) = old_value {
                            if old == *val {
                                continue;
                            } else {
                                return true;
                            }
                        } else {
                            return true; // only new value, no old value, we should refresh
                        }
                    }
                    StateValueType::I64(val) => {
                        if let Some(StateValueType::I64(old)) = old_value {
                            if old == *val {
                                continue;
                            } else {
                                return true;
                            }
                        } else {
                            return true; // only new value, no old value, we should refresh
                        }
                    }
                    StateValueType::String(val) => {
                        if let Some(StateValueType::String(old)) = &old_value {
                            if *old == *val {
                                continue;
                            } else {
                                return true;
                            }
                        } else {
                            return true; // only new value, no old value, we should refresh
                        }
                    }
                    StateValueType::NetworkState(val) => {
                        if let Some(StateValueType::NetworkState(old)) = &old_value {
                            if *old == *val {
                                continue;
                            } else {
                                return true;
                            }
                        } else {
                            return true; // only new value, no old value, we should refresh
                        }
                    }
                };
            }
        }
        // our key was not found
        false
    }
}
