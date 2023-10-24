use std::time::Instant;

use dbus::arg::RefArg;

pub mod interface;
mod playerctld;
pub mod thread;

// Extend RefArg with Eq
// trait RefArgEq: RefArg + Eq + PartialEq {}

// // Implement RefArgEq for any type that implements RefArg
// impl<T: RefArg + Eq + PartialEq> RefArgEq for T {}

// Define DBus structs
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct DBusProxyAdress {
    dest: String,
    path: String,
}
impl DBusProxyAdress {
    pub fn new(dest: String, path: String) -> DBusProxyAdress {
        DBusProxyAdress { dest, path }
    }
}

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct DBusPropertyAdress {
    proxy: DBusProxyAdress,
    interface: String,
    property: String,
}
impl DBusPropertyAdress {
    /// Creates a new DBusProperty
    pub fn new(proxy: DBusProxyAdress, interface: String, property: String) -> DBusPropertyAdress {
        DBusPropertyAdress {
            proxy,
            interface,
            property,
        }
    }
}
