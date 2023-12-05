use std::collections::HashMap;

use dbus::arg::RefArg;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{dbus::DBusPropertyAdress, log};

use super::value::{StateValue, StateValueType};

#[derive(Error, Debug)]
pub enum ApplicationStateError {
    #[error("Key '{0}' does not exist")]
    DoesNotExistError(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'static"))]
pub struct ApplicationState {
    pub map: HashMap<&'static str, StateValue>,
}

fn print_update(key: &str, old: &StateValue, new: &StateValue) {
    if old == new {
        return;
    }
    println!("{} Updated {} old: {}, new: {}", log::STATE, key, old, new);
}
impl ApplicationState {
    pub fn get_value_dbus(
        &self,
        property: &DBusPropertyAdress,
    ) -> Result<Option<StateValueType>, ApplicationStateError> {
        for value in self.map.values() {
            if value.dbus_property.is_some() && value.dbus_property.expect("") == property {
                return Ok(value.get());
            }
        }
        Err(ApplicationStateError::DoesNotExistError(
            property.to_string(),
        ))
    }

    pub fn update_dbus(
        &mut self,
        property: &DBusPropertyAdress,
        val: &dyn RefArg,
    ) -> Result<Option<StateValueType>, ApplicationStateError> {
        for (key, value) in self.map.iter_mut() {
            if value.dbus_property.is_some() && value.dbus_property.expect("") == property {
                // let mut v = value.clone();
                let old = value.clone();
                value.set(Some(StateValueType::from_ref_arg(val)));
                print_update(key, &old, value);

                return Ok(old.get());
            }
        }
        Err(ApplicationStateError::DoesNotExistError(
            property.to_string(),
        ))
    }

    pub fn update(
        &mut self,
        property: &str,
        value: Option<StateValueType>,
    ) -> Result<bool, ApplicationStateError> {
        if !self.map.contains_key(property) {
            return Err(ApplicationStateError::DoesNotExistError(
                property.to_string(),
            ));
        }
        let Some(state_value) = self.map.get_mut(property) else {
            return Err(ApplicationStateError::DoesNotExistError(
                property.to_string(),
            ));
        };
        let old = state_value.clone();
        let updated = old.get() != value;

        state_value.set(value);

        if updated {
            print_update(property, &old, state_value);
        }
        Ok(updated)
    }

    pub fn update_multiple(
        &mut self,
        properties: HashMap<&str, Option<StateValueType>>,
    ) -> Result<bool, ApplicationStateError> {
        let mut updated = false;
        for (key, value) in properties {
            updated |= self.update(key, value)?;
        }
        Ok(updated)
    }

    pub fn get(&self, key: &str) -> Option<&StateValueType> {
        let Some(value) = self.map.get(key) else {
            return None;
        };

        return value.get_ref();
    }
}
