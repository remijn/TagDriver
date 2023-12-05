use std::collections::HashMap;

use crate::dbus::{BusType, DBusPropertyAdress, DBusProxyAdress};

pub mod app;
pub mod value;

use app::ApplicationState;

use self::value::{Filter, FilterMultiply, FilterRound, StateValue};

// pub struct PowerState {
//     battery_percentage: u32,
//     battery_state: BatteryState,
// }

// pub struct HardwareState {
//     power_state: PowerState,
// }

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
        StateValue::dbus(&BRIGHTNESS_PROPERTY, vec![]),
    );

    map.insert(
        "player:volume",
        StateValue::dbus(
            &PLAYER_VOLUME_PROPERTY,
            vec![Filter::Multiply(FilterMultiply { factor: 100.0 })],
        ),
    );
    map.insert(
        "battery:level",
        StateValue::dbus(&BATTERY_LEVEL_PROPERTY, vec![]),
    );
    map.insert(
        "battery:state",
        StateValue::dbus(&BATTERY_STATE_PROPERTY, vec![]),
    );

    map.insert("wifi:state", StateValue::new(None));
    map.insert(
        "wifi:strength",
        StateValue::filtered(None, vec![Filter::Round(FilterRound { to: 20.0 })]),
    );
    map.insert("eth:state", StateValue::new(None));
    map.insert("workspace:active", StateValue::new(None));
    map.insert("workspace:count", StateValue::new(None));

    ApplicationState { map }
}
