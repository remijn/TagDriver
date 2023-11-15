use std::collections::HashMap;

use embedded_graphics::geometry::Point;

use crate::{
    dbus::{DBusPropertyAdress, DBusValue, DBusValueMap},
    display::bwr_display::BWRDisplay,
};

use super::{DBusConsumer, DisplayComponent, IconComponent};

pub struct StateItem {
    pub name: &'static str,
    pub center: Point,
    pub properties: Vec<DBusPropertyAdress>,
    pub screen: u8,
    old_values: Box<DBusValueMap>, // Values last drawn
    _draw_icon: Box<dyn Fn(&mut BWRDisplay, f64, Point)>,
}

impl StateItem {
    pub fn new(
        name: &'static str,
        center: Point,
        properties: Vec<DBusPropertyAdress>,
        screen: u8,
        draw_icon: Box<dyn Fn(&mut BWRDisplay, f64, Point)>,
    ) -> Self {
        Self {
            name,
            center,
            screen,
            old_values: Box::new(HashMap::new()),
            properties,
            _draw_icon: draw_icon,
        }
    }
}

impl IconComponent for StateItem {
    fn draw_icon(&self, target: &mut BWRDisplay, value: f64, center: Point) {
        (self._draw_icon)(target, value, center);
    }
}

impl DisplayComponent for StateItem {
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
        Some(self)
    }

    fn dbus_mut(&mut self) -> Option<&mut dyn super::DBusConsumer> {
        Some(self)
    }
}

impl DBusConsumer for StateItem {
    fn wanted_dbus_values(&self) -> Vec<DBusPropertyAdress> {
        return self.properties.clone();
    }

    fn needs_refresh(&self, new_values: &DBusValueMap) -> bool {
        for property in self.properties.clone() {
            if new_values.contains_key(&property) {
                let new_v = new_values.get(&property).expect("");
                let old_v = self.old_values.get(&property);

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
        }
        // our key was not found
        return false;
    }

    fn set_initial(&mut self, new_values: &DBusValueMap) {
        self.old_values = Box::new(new_values.clone());
    }
}
