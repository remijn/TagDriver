use std::{collections::HashMap, fmt::Display};

use dbus::arg::{ArgType, RefArg};

use crate::log;

pub mod dbus_interface;
mod playerctld;

// Extend RefArg with Eq
pub trait RefArgEq: RefArg + Eq + PartialEq + Clone {
    // fn eq(&self, other: &Self) -> bool {
    //     match self.arg_type() {
    //         // ArgType::Array | ArgType::DictEntry | ArgType::Variant => self.as
    //     }
    // }
}

// Implement RefArgEq for any type that implements RefArg
impl<T: RefArg + Eq + PartialEq + Clone> RefArgEq for T {}

type DBusUpdate = (DBusPropertyAdress, Option<Box<dyn RefArg>>);

#[allow(dead_code)]
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub enum BusType {
    SESSION,
    SYSTEM,
}

// Define DBus structs
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct DBusProxyAdress {
    bus: BusType,
    dest: &'static str,
    path: &'static str,
}
impl DBusProxyAdress {
    pub const fn new(bus: BusType, dest: &'static str, path: &'static str) -> DBusProxyAdress {
        DBusProxyAdress { bus, dest, path }
    }
}

// pub type DBusValueMap = HashMap<DBusPropertyAdress, Box<dyn RefArg>>;

#[derive(PartialEq, Clone, Debug)]
pub enum DBusValue {
    F64(f64),
    I64(i64),
    U64(u64),
    STRING(String),
}

impl DBusValue {
    pub fn from_ref_arg(ref_arg: &dyn RefArg) -> DBusValue {
        return match ref_arg.arg_type() {
            ArgType::Int16 | ArgType::Int32 | ArgType::Int64 => {
                DBusValue::I64(ref_arg.as_i64().expect("Cast error"))
            }

            ArgType::UInt16
            | ArgType::UInt32
            | ArgType::UInt64
            | ArgType::Byte
            | ArgType::Boolean => DBusValue::U64(ref_arg.as_u64().expect("Cast error")),
            ArgType::String => DBusValue::STRING(ref_arg.as_str().expect("Cast error").to_string()),

            ArgType::Double => DBusValue::F64(ref_arg.as_f64().expect("Cast error")),

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

pub type DBusValueMap = HashMap<DBusPropertyAdress, DBusValue>;

#[derive(Hash, Eq, PartialEq, Clone)]
pub struct DBusPropertyAdress {
    pub proxy: DBusProxyAdress,
    pub interface: &'static str,
    pub property: &'static str,
}

impl Display for DBusPropertyAdress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?} {} {} {}",
            self.proxy.bus, self.proxy.dest, self.interface, self.property
        )
    }
}
impl DBusPropertyAdress {
    /// Creates a new DBusProperty
    pub const fn new(
        proxy: DBusProxyAdress,
        interface: &'static str,
        property: &'static str,
    ) -> DBusPropertyAdress {
        DBusPropertyAdress {
            proxy,
            interface,
            property,
        }
    }
}
