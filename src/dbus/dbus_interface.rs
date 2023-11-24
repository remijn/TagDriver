use dbus::{
    arg::RefArg,
    blocking::{
        stdintf::{self, org_freedesktop_dbus::Properties},
        Connection,
    },
    Message,
};
use itertools::Itertools;
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    Mutex,
};

use std::{collections::HashMap, error::Error, sync::Arc, time::Duration};

use stdintf::org_freedesktop_dbus::PropertiesPropertiesChanged;

use crate::log;

use super::{BusType, DBusPropertyAdress, DBusUpdate, DBusValue, DBusValueMap};

#[allow(dead_code)]
fn print_refarg(value: &dyn RefArg) {
    // We don't know what type the value is. We'll try a few and fall back to
    // debug printing if the value is more complex than that.

    if let Some(s) = value.as_str() {
        println!("{}", s);
    } else if let Some(i) = value.as_i64() {
        println!("{}", i);
    } else {
        println!("{:?}", value);
    }
}

pub struct DBusInterface {
    session_conn: Connection,
    system_conn: Connection,
    pub values: Arc<Mutex<DBusValueMap>>,
    pub initialised: bool,
    rx: Receiver<Vec<DBusUpdate>>,
    tx: Sender<Vec<DBusUpdate>>,
    update_tx: Sender<bool>,
}

impl DBusInterface {
    pub fn new(update_tx: Sender<bool>) -> Result<Self, Box<dyn Error>> {
        let session_conn = Connection::new_session().expect("Error connecting to Session DBus");

        let system_conn = Connection::new_system().expect("Error connecting to System DBus");

        let (tx, rx) = mpsc::channel::<Vec<DBusUpdate>>(20);

        let values: Arc<Mutex<DBusValueMap>> = Arc::new(Mutex::new(HashMap::new()));

        Ok(Self {
            session_conn,
            system_conn,
            values,
            initialised: false,
            rx,
            tx,
            update_tx,
        })
    }

    pub async fn init(
        &mut self,
        properties: Vec<&'static DBusPropertyAdress>,
    ) -> Result<DBusValueMap, &str> {
        if self.initialised {
            // print!("DBus init can only be called once!!!");
            return Err("DBus init can only be called once!!!");
        }
        self.initialised = true;

        // let values = self.values.lock().expect("cant lock");

        let proxies = properties.iter().into_group_map_by(|v| v.proxy.clone());

        let mut values = self.values.lock().await;

        // self.conn.with_proxy(dest, path, timeout);

        // let conn = self.conn

        for (proxy, proxy_properties) in proxies {
            println!("{} Init Proxy {} {}", log::DBUS, proxy.dest, proxy.path);

            let clone_tx = self.tx.clone();

            // let clone_proxy: DBusProxyAdress = proxy.clone();

            let connection = match proxy.bus {
                BusType::SESSION => &self.session_conn,
                BusType::SYSTEM => &self.system_conn,
            };

            let conn_proxy = connection.with_proxy(proxy.dest, proxy.path, Duration::from_secs(2));

            for property in proxy_properties {
                // Get initial value

                let res = conn_proxy.get::<Box<dyn RefArg>>(property.interface, &property.property);

                if let Ok(result) = res {
                    values.insert(property, DBusValue::from_ref_arg(&result));
                } else {
                    println!("{} Unable to get property {}", log::ERROR, property);
                }

                // conn_proxy.method_call(
                //     m,
                //     args,
                // )

                // let value = self.values.get(property).expect("Unknown value");
            }

            let props = properties.clone();

            println!("Match proxy {:?}", proxy);

            conn_proxy
                .match_signal(
                    move |h: PropertiesPropertiesChanged, _: &Connection, _: &Message| {
                        // let values = self.values.lock().expect("Could not lock values mutex");

                        // let iface: String = h.interface_name.as_str().clone();
                        let iface = h.interface_name;

                        let mut updates: Vec<DBusUpdate> = Vec::new();

                        for (key, value) in h.changed_properties {
                            for prop in props.iter() {
                                if prop.proxy == &proxy
                                    && prop.interface == iface.as_str()
                                    && prop.property == key.as_str()
                                {
                                    updates.push((prop, Some(value.0.box_clone())));
                                }
                            }

                            // print_refarg(&value.1.expect("huh?"));
                        }
                        if updates.len() > 0 {
                            println!("{} {} Values {:?} ", log::DBUS, iface, updates);
                            clone_tx.try_send(updates).expect("Could not send");
                        }
                        true
                    },
                )
                .expect("error");
        }
        return Ok(values.clone());
    }

    pub async fn run(&mut self) {
        let values = self.values.lock().await;
        println!(
            "{} Starting DBus Thread with {} watched properties:",
            log::THREAD,
            { values.len() }
        );
        for property in values.keys() {
            println!(" - {} {}", log::DBUS, property);
        }
        drop(values);

        loop {
            self.session_conn
                .process(Duration::from_millis(10))
                .expect("Could not process session dbus messages");

            self.system_conn
                .process(Duration::from_millis(10))
                .expect("Could not process system dbus messages");

            while let Ok(dbus_values) = self.rx.try_recv() {
                if dbus_values.len() == 0 {
                    continue;
                }
                let mut updated = false;
                let mut values = self.values.lock().await;
                for (key, new_value_option) in dbus_values {
                    match values.get(&key) {
                        Some(_val) if new_value_option.is_some() => {
                            values.insert(
                                key,
                                DBusValue::from_ref_arg(&new_value_option.expect("!")),
                            );
                            updated = true;
                            // value = new_value_option.expect("impossible").box_clone();
                        } // let Some(new_value) = new_value_option => {}
                        Some(_val) => println!("{} Recieved empty value????", log::ERROR),
                        None => {
                            println!("{} Could not match: \n{} {}", log::WARN, log::DBUS, key);

                            let matches = values
                                .keys()
                                .filter(|k| {
                                    k.property == key.property || k.interface == key.interface
                                })
                                .into_iter();

                            println!("{} Did you mean any of these:", log::WARN);
                            for match_item in matches {
                                println!(" - {} {}", log::DBUS, match_item);
                            }
                        }
                    }
                }
                if updated {
                    self.update_tx
                        .send(true)
                        .await
                        .expect("Could not send dbus update");
                }
            }
        }
    }
}

#[allow(unused_attributes)]
#[feature(test)]
mod test {

    #[tokio::test]
    async fn can_create() {
        use super::DBusInterface;
        use tokio::sync::mpsc;

        let (update_tx, _update_rx) = mpsc::channel::<bool>(20);
        let bus = DBusInterface::new(update_tx);
        match bus {
            Err(_e) => assert!(false),
            Ok(_v) => assert!(true),
        }
    }
}
