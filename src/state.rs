use std::collections::HashMap;

use dbus::arg::{ArgType, RefArg};
use serde::{Deserialize, Serialize};

use crate::{
    dbus::{BusType, DBusPropertyAdress, DBusProxyAdress},
    log,
};

// pub struct PowerState {
//     battery_percentage: u32,
//     battery_state: BatteryState,
// }

// pub struct HardwareState {
//     power_state: PowerState,
// }

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum StateValueType {
    U64(u64),
    I64(i64),
    F64(f64),
    String(String),
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

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct StateValue {
    pub value: Option<StateValueType>,

    #[serde(skip_deserializing)]
    pub dbus_property: Option<&'static DBusPropertyAdress>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'static"))]
pub struct ApplicationState {
    pub map: HashMap<&'static str, StateValue>,
}

impl ApplicationState {
    pub fn get_value_dbus(&self, property: &DBusPropertyAdress) -> Option<&StateValueType> {
        for value in self.map.values() {
            if value.dbus_property.is_some() && value.dbus_property.expect("") == property {
                return value.value.as_ref();
            }
        }
        None
    }

    pub fn update_dbus(&mut self, property: &DBusPropertyAdress, val: &dyn RefArg) {
        for (_key, value) in self.map.iter_mut() {
            if value.dbus_property.is_some() && value.dbus_property.expect("") == property {
                // let mut v = value.clone();
                value.value = Some(StateValueType::from_ref_arg(val));
            }
        }
        // for (key, value) in self.map.iter_mut() {
        //     if value.dbus_property.is_some() && value.dbus_property.expect("") == property {
        //         // let v = value.c;
        //         value.value = Some(StateValueType::from_ref_arg(val));
        //     }
        // }
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
        },
    );

    map.insert(
        "player:volume",
        StateValue {
            value: None,
            dbus_property: Some(&PLAYER_VOLUME_PROPERTY),
        },
    );
    map.insert(
        "battery:level",
        StateValue {
            value: None,
            dbus_property: Some(&BATTERY_LEVEL_PROPERTY),
        },
    );
    map.insert(
        "battery:state",
        StateValue {
            value: None,
            dbus_property: Some(&BATTERY_STATE_PROPERTY),
        },
    );

    ApplicationState { map }
}
