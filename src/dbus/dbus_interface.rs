use dbus::{
    arg::RefArg,
    blocking::{
        stdintf::{self, org_freedesktop_dbus::Properties},
        Connection,
    },
    Message,
};

use tokio::sync::{
    mpsc::{self, Sender},
    Mutex,
};

use std::{error::Error, sync::Arc, time::Duration};

use stdintf::org_freedesktop_dbus::PropertiesPropertiesChanged;

use crate::{log, state::ApplicationState};

use super::{BusType, DBusPropertyAdress, DBusProxyAdress, DBusUpdate};

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

pub async fn run_dbus_thread(
    update_tx: Sender<bool>,
    state: Arc<Mutex<ApplicationState>>,
) -> Result<(), Box<dyn Error>> {
    let session_conn = Connection::new_session().expect("Error connecting to Session DBus");

    let system_conn = Connection::new_system().expect("Error connecting to System DBus");

    let (tx, mut rx) = mpsc::channel::<Vec<DBusUpdate>>(20);

    // let values: Arc<Mutex<DBusValueMap>> = Arc::new(Mutex::new(HashMap::new()));

    let mut state_lock: tokio::sync::MutexGuard<'_, ApplicationState> = state.lock().await;

    // Get initial values and start listening for updates

    let mut proxies: Vec<&DBusProxyAdress> = Vec::new();
    let mut properties: Vec<&DBusPropertyAdress> = Vec::new();

    for key in state_lock.map.keys() {
        if let Some(state_value) = state_lock.map.get(key) {
            if let Some(prop) = state_value.dbus_property {
                properties.push(prop);
                proxies.push(prop.proxy);
            }
        }
    }

    for proxy in proxies {
        println!("{} Init Proxy {} {}", log::DBUS, proxy.dest, proxy.path);

        let clone_tx = tx.clone();

        // let clone_proxy: DBusProxyAdress = proxy.clone();

        let connection = match proxy.bus {
            BusType::Session => &session_conn,
            BusType::System => &system_conn,
        };

        let conn_proxy = connection.with_proxy(proxy.dest, proxy.path, Duration::from_secs(2));

        for property in properties.clone() {
            if property.proxy != proxy {
                continue;
            }
            // Get initial value
            let res = conn_proxy.get::<Box<dyn RefArg>>(property.interface, property.property);

            if let Ok(result) = res {
                state_lock.update_dbus(property, &result);
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

        conn_proxy
            .match_signal(
                move |h: PropertiesPropertiesChanged, _: &Connection, _: &Message| {
                    // let values = self.values.lock().expect("Could not lock values mutex");

                    // let iface: String = h.interface_name.as_str().clone();
                    let iface = h.interface_name;

                    let mut updates: Vec<DBusUpdate> = Vec::new();

                    for (key, value) in h.changed_properties {
                        for prop in props.iter() {
                            if prop.proxy == proxy
                                && prop.interface == iface.as_str()
                                && prop.property == key.as_str()
                            {
                                updates.push((prop, Some(value.0.box_clone())));
                            }
                        }

                        // print_refarg(&value.1.expect("huh?"));
                    }
                    if !updates.is_empty() {
                        println!("{} {} Values {:?} ", log::DBUS, iface, updates);
                        clone_tx.try_send(updates).expect("Could not send");
                    }
                    true
                },
            )
            .expect("error");
    }

    drop(state_lock);

    loop {
        session_conn
            .process(Duration::from_millis(10))
            .expect("Could not process session dbus messages");

        system_conn
            .process(Duration::from_millis(10))
            .expect("Could not process system dbus messages");

        while let Ok(dbus_values) = rx.try_recv() {
            if dbus_values.is_empty() {
                continue;
            }
            let mut updated = false;
            let mut state_lock = state.lock().await;
            for (key, new_value_option) in dbus_values {
                let old_value = state_lock.get_value_dbus(key);

                match old_value {
                    Some(_val) if new_value_option.is_some() => {
                        state_lock.update_dbus(key, &new_value_option.expect(""));
                        updated = true;
                        // value = new_value_option.expect("impossible").box_clone();
                    } // let Some(new_value) = new_value_option => {}
                    Some(_val) => println!("{} Recieved empty value????", log::ERROR),
                    None => {
                        println!(
                            "{} Could not match into Application state: \n{} {}",
                            log::WARN,
                            log::DBUS,
                            key
                        );

                        // let matches = state
                        //     .keys()
                        //     .filter(|k| {
                        //         k.property == key.property || k.interface == key.interface
                        //     })
                        //     .into_iter();

                        // println!("{} Did you mean any of these:", log::WARN);
                        // for match_item in matches {
                        //     println!(" - {} {}", log::DBUS, match_item);
                        // }
                    }
                }
            }
            drop(state_lock);

            if updated {
                update_tx
                    .send(true)
                    .await
                    .expect("Could not send dbus update");
            }
        }
    }
}
