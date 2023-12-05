use std::fmt::Display;

use dbus::arg::RefArg;
use serde::{ser::SerializeStruct, Deserialize, Serialize, Serializer};

pub mod dbus_interface;
pub mod networkmanager;
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

type DBusPropertyUpdate = (&'static DBusPropertyAdress, Option<Box<dyn RefArg>>);

#[derive(Debug)]
pub enum DBusUpdate {
    PropertyUpdate(DBusPropertyUpdate),
    MethodShowImage(String, u32),
    MethodSetWorkspaces(u32, u32),
}

#[allow(dead_code)]
#[derive(Hash, Eq, PartialEq, Clone, Debug, Serialize, Deserialize)]
pub enum BusType {
    Session,
    System,
}

// Define DBus structs
#[derive(Hash, Eq, PartialEq, Clone, Debug, Serialize, Deserialize)]
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

// pub type DBusValueMap = HashMap<&'static DBusPropertyAdress, DBusValue>;

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct DBusPropertyAdress {
    pub proxy: &'static DBusProxyAdress,
    pub interface: &'static str,
    pub property: &'static str,
}

// This is the trait that informs Serde how to serialize DBusPropertyAdress
impl Serialize for DBusPropertyAdress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut adress = serializer.serialize_struct("DBusPropertyAdress", 3)?;
        adress.serialize_field("interface", &self.interface)?;
        adress.serialize_field("property", &self.property)?;
        adress.serialize_field("bus", &self.proxy.bus)?;
        adress.serialize_field("destination", &self.proxy.dest)?;
        adress.serialize_field("path", &self.proxy.path)?;
        adress.end()
    }
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
        proxy: &'static DBusProxyAdress,
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
