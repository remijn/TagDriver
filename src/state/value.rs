use dbus::arg::{ArgType, RefArg};
use serde::{Deserialize, Serialize};

use crate::{
    dbus::{networkmanager::NMDeviceState, DBusPropertyAdress},
    log,
};

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
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

impl std::fmt::Display for StateValueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            StateValueType::F64(val) => write!(f, "f{}", val),
            StateValueType::I64(val) => write!(f, "i{}", val),
            StateValueType::U64(val) => write!(f, "u{}", val),
            StateValueType::String(val) => write!(f, "{}", val),
            _ => write!(f, "{:?}", &self),
        }
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
pub struct FilterRound {
    pub to: f64,
}

impl FilterTrait for FilterRound {
    fn apply(&mut self, input: StateValueType) -> StateValueType {
        let mult = 1.0 / self.to;
        match input {
            StateValueType::U64(value) => {
                StateValueType::U64(((value as f64 * mult).round() / mult).round() as u64)
            }
            StateValueType::I64(value) => {
                StateValueType::I64(((value as f64 * mult).round() / mult).round() as i64)
            }
            StateValueType::F64(value) => StateValueType::F64((value * mult).round() / mult),
            _ => panic!("Cannot round"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum Filter {
    Multiply(FilterMultiply),
    Round(FilterRound),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct StateValue {
    value: Option<StateValueType>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    filters: Vec<Filter>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(skip_deserializing)]
    pub dbus_property: Option<&'static DBusPropertyAdress>,
}

impl StateValue {
    pub fn new(default: Option<StateValueType>) -> Self {
        Self {
            value: default,
            dbus_property: None,
            filters: vec![],
        }
    }
    pub fn filtered(default: Option<StateValueType>, filters: Vec<Filter>) -> Self {
        Self {
            value: default,
            dbus_property: None,
            filters,
        }
    }
    pub fn dbus(property: &'static DBusPropertyAdress, filters: Vec<Filter>) -> Self {
        Self {
            value: None,
            dbus_property: Some(property),
            filters,
        }
    }
    pub fn get(&self) -> Option<StateValueType> {
        self.value.clone()
    }
    pub fn get_ref(&self) -> Option<&StateValueType> {
        self.value.as_ref()
    }

    pub fn set(&mut self, value: Option<StateValueType>) -> Option<&StateValueType> {
        let mut value = value;

        for filter in self.filters.iter_mut() {
            match filter {
                Filter::Multiply(filter) if value.is_some() => {
                    value = Some(filter.apply(value.unwrap()));
                }
                Filter::Round(filter) if value.is_some() => {
                    value = Some(filter.apply(value.unwrap()));
                }
                _ => {
                    println!(
                        "{} Filter {:?} cannot be used on {:?}",
                        log::ERROR,
                        filter,
                        value
                    );
                }
            }
        }

        self.value = value;
        return self.value.as_ref();
    }
}

impl std::fmt::Display for StateValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.value {
            None => write!(f, "None"),
            Some(val) => write!(f, "{}", val),
        }
    }
}
