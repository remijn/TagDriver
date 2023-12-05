use std::collections::HashMap;

use dbus::arg::{ArgType, RefArg};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    dbus::{networkmanager::NMDeviceState, BusType, DBusPropertyAdress, DBusProxyAdress},
    log,
};

// pub struct PowerState {
//     battery_percentage: u32,
//     battery_state: BatteryState,
// }

// pub struct HardwareState {
//     power_state: PowerState,
// }

#[derive(Error, Debug)]
pub enum ApplicationStateError {
    #[error("Key '{0}' does not exist")]
    DoesNotExistError(String),
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum NetworkState {
    Unknown,
    Connecting,
    Connected,
    Disconnected,
    Disabled,
}

impl From<NMDeviceState> for NetworkState {
    fn from(value: NMDeviceState) -> Self {
        match value {
            NMDeviceState::Unavailable | NMDeviceState::Unmanaged => Self::Disabled,
            NMDeviceState::Prepare
            | NMDeviceState::Config
            | NMDeviceState::NeedAuth
            | NMDeviceState::Secondaries
            | NMDeviceState::IpConfig
            | NMDeviceState::IpCheck => Self::Connecting,
            NMDeviceState::Activated => Self::Connected,
            NMDeviceState::Disconnected | NMDeviceState::Deactivating | NMDeviceState::Failed => {
                Self::Disconnected
            }
            NMDeviceState::Unknown => Self::Unknown,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum StateValueType {
    U64(u64),
    I64(i64),
    F64(f64),
    String(String),
    NetworkState(NetworkState),
}

impl StateValueType {
    pub fn from_ref_arg(ref_arg: &dyn RefArg) -> Self {
        return match ref_arg.arg_type() {
            ArgType::Int16 | ArgType::Int32 | ArgType::Int64 => {
                StateValueType::I64(ref_arg.as_i64().expect("Cast error"))
            }

            ArgType::UInt16
            | ArgType::UInt32
            | ArgType::UInt64
            | ArgType::Byte
            | ArgType::Boolean => StateValueType::U64(ref_arg.as_u64().expect("Cast error")),
            ArgType::String => {
                StateValueType::String(ref_arg.as_str().expect("Cast error").to_string())
            }

            ArgType::Double => StateValueType::F64(ref_arg.as_f64().expect("Cast error")),

            _ => {
                println!(
                    "{} Could not convert type {}",
                    log::ERROR,
                    ref_arg.arg_type().as_str()
                );
                todo!();
            }
        };
    }
}

pub trait FilterTrait {
    fn apply(&mut self, input: StateValueType) -> StateValueType;
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct FilterMultiply {
    pub factor: f64,
}

impl FilterTrait for FilterMultiply {
    fn apply(&mut self, input: StateValueType) -> StateValueType {
        match input {
            StateValueType::U64(value) => StateValueType::F64(value as f64 * self.factor),
            StateValueType::I64(value) => StateValueType::F64(value as f64 * self.factor),
            StateValueType::F64(value) => StateValueType::F64(value * self.factor),
            _ => panic!("Cannot multiply"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum Filter {
    Multiply(FilterMultiply),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct StateValue {
    pub value: Option<StateValueType>,
    pub filters: Vec<Filter>,

    #[serde(skip_deserializing)]
    pub dbus_property: Option<&'static DBusPropertyAdress>,
}

impl StateValue {}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'static"))]
pub struct ApplicationState {
    pub map: HashMap<&'static str, StateValue>,
}

impl ApplicationState {
    pub fn get_value_dbus(
        &self,
        property: &DBusPropertyAdress,
    ) -> Result<Option<StateValueType>, ApplicationStateError> {
        for value in self.map.values() {
            if value.dbus_property.is_some() && value.dbus_property.expect("") == property {
                return Ok(value.value.clone());
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
                let old = value.value.clone();
                value.value = Some(StateValueType::from_ref_arg(val));

                println!(
                    "{} Updated {} old: {:?}, new: {:?}",
                    log::STATE,
                    key,
                    old,
                    value.value
                );

                return Ok(old);
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
        let old = state_value.value.clone();
        let updated = old != value;

        if updated {
            println!(
                "{} Updated {} old: {:?}, new: {:?}",
                log::STATE,
                property,
                old,
                value
            );
        }

        state_value.value = value;

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

        return value.value.as_ref();
    }
}

// PROXY Backlight power settings
static BACKLIGHT_PROXY: DBusProxyAdress = DBusProxyAdress::new(
    BusType::Session,
    "org.gnome.SettingsDaemon.Power",
    "/org/gnome/SettingsDaemon/Power",
);

// PROP display brightness
static BRIGHTNESS_PROPERTY: DBusPropertyAdress = DBusPropertyAdress::new(
    &BACKLIGHT_PROXY,
    "org.gnome.SettingsDaemon.Power.Screen",
    "Brightness",
);

// PROXY playerctld Media player
static PLAYER_PROXY: DBusProxyAdress = DBusProxyAdress::new(
    BusType::Session,
    "org.mpris.MediaPlayer2.playerctld",
    "/org/mpris/MediaPlayer2",
);
// PROP Volume
static PLAYER_VOLUME_PROPERTY: DBusPropertyAdress =
    DBusPropertyAdress::new(&PLAYER_PROXY, "org.mpris.MediaPlayer2.Player", "Volume");

// PROXY Battery status
static BATTERY_PROXY: DBusProxyAdress = DBusProxyAdress::new(
    BusType::System,
    "org.freedesktop.UPower",
    "/org/freedesktop/UPower/devices/battery_BAT1",
);
// PROP Battery Percentage
static BATTERY_LEVEL_PROPERTY: DBusPropertyAdress = DBusPropertyAdress::new(
    &BATTERY_PROXY,
    "org.freedesktop.UPower.Device",
    "Percentage",
);

// PROP Battery State
static BATTERY_STATE_PROPERTY: DBusPropertyAdress =
    DBusPropertyAdress::new(&BATTERY_PROXY, "org.freedesktop.UPower.Device", "State");

pub fn build_state_map() -> ApplicationState {
    let mut map: HashMap<&'static str, StateValue> = HashMap::new();

    map.insert(
        "backlight:brightness",
        StateValue {
            value: None,
            dbus_property: Some(&BRIGHTNESS_PROPERTY),
            filters: vec![Filter::Multiply(FilterMultiply { factor: 100.0 })],
        },
    );

    map.insert(
        "player:volume",
        StateValue {
            value: None,
            dbus_property: Some(&PLAYER_VOLUME_PROPERTY),
            filters: vec![],
        },
    );
    map.insert(
        "battery:level",
        StateValue {
            value: None,
            dbus_property: Some(&BATTERY_LEVEL_PROPERTY),
            filters: vec![],
        },
    );
    map.insert(
        "battery:state",
        StateValue {
            value: None,
            dbus_property: Some(&BATTERY_STATE_PROPERTY),
            filters: vec![],
        },
    );

    map.insert(
        "wifi:state",
        StateValue {
            value: None,
            dbus_property: None,
            filters: vec![],
        },
    );
    map.insert(
        "wifi:strength",
        StateValue {
            value: None,
            dbus_property: None,
            filters: vec![],
        },
    );
    map.insert(
        "eth:state",
        StateValue {
            value: None,
            dbus_property: None,
            filters: vec![],
        },
    );
    map.insert(
        "workspace:active",
        StateValue {
            value: None,
            dbus_property: None,
            filters: vec![],
        },
    );
    map.insert(
        "workspace:count",
        StateValue {
            value: None,
            dbus_property: None,
            filters: vec![],
        },
    );

    ApplicationState { map }
}
