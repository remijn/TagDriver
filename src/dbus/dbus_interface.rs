use dbus::{
    arg::RefArg,
    blocking::{
        stdintf::{self, org_freedesktop_dbus::Properties},
        Connection,
    },
    Message,
};

// use dbus_crossroads::Crossroads;
use networkmanager::{
    devices::{Any, Device, Wireless},
    NetworkManager,
};
use tokio::{
    sync::{
        mpsc::{self, Sender},
        Mutex,
    },
    time::Instant,
};

use std::{collections::HashMap, error::Error, sync::Arc, time::Duration};

use stdintf::org_freedesktop_dbus::PropertiesPropertiesChanged;

use crate::{
    dbus::networkmanager::NMDeviceState,
    log,
    state::{ApplicationState, NetworkState, StateValueType},
};

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

fn update_data_nm(
    system_conn: &Connection,
    state: &mut ApplicationState,
) -> Result<bool, networkmanager::Error> {
    let nm = &NetworkManager::new(system_conn);

    let mut values: HashMap<&str, Option<StateValueType>> = HashMap::new();

    // Get wifi device
    let Device::WiFi(wifi) = nm.get_device_by_ip_iface("wlp1s0").expect("") else {
        panic!();
    };
    // Get wifi state
    let nm_wifi_state = NMDeviceState::from_int(wifi.state()?);
    let wifi_state = NetworkState::from(nm_wifi_state);

    // Update the strength
    if wifi_state == NetworkState::Connected {
        let ap = wifi.active_access_point();

        if ap.is_err() {
            values.insert("wifi:strength", Some(StateValueType::F64(0.0)));
        } else {
            values.insert(
                "wifi:strength",
                Some(StateValueType::F64(ap?.strength()? as f64)),
            );
        }
    } else {
        values.insert("wifi:strength", None);
    }

    values.insert("wifi:state", Some(StateValueType::NetworkState(wifi_state)));

    // Get eth device
    let Device::Ethernet(eth) = nm.get_device_by_ip_iface("enp2s0").expect("") else {
        panic!();
    };

    let nm_eth_state = NMDeviceState::from_int(eth.state()?);
    let eth_state = NetworkState::from(nm_eth_state);
    values.insert("eth:state", Some(StateValueType::NetworkState(eth_state)));

    let updated = state
        .update_multiple(values.clone())
        .expect("Error updating state");

    Ok(updated)
}

pub async fn run_dbus_thread(
    update_tx: Sender<bool>,
    state: Arc<Mutex<ApplicationState>>,
) -> Result<(), Box<dyn Error>> {
    let session_conn = Connection::new_session().expect("Error connecting to Session DBus");

    let system_conn: Connection =
        Connection::new_system().expect("Error connecting to System DBus");

    let (tx, mut rx) = mpsc::channel::<Vec<DBusUpdate>>(20);

    // let values: Arc<Mutex<DBusValueMap>> = Arc::new(Mutex::new(HashMap::new()));

    // Start the DBus Server

    // system_conn.request_name("com.example.dbustest", false, true, false)?;

    // let mut cr = Crossroads::new();
    // cr.set_async_support(Some((
    //     system_conn.clone(),
    //     Box::new(|x| {
    //         tokio::spawn(x);
    //     }),
    // )));

    let mut state_lock: tokio::sync::MutexGuard<'_, ApplicationState> = state.lock().await;

    let mut proxies: Vec<&DBusProxyAdress> = Vec::new();
    let mut properties: Vec<&DBusPropertyAdress> = Vec::new();

    // Get the properties we monitor from the ApplicationState
    for key in state_lock.map.keys() {
        if let Some(state_value) = state_lock.map.get(key) {
            if let Some(prop) = state_value.dbus_property {
                properties.push(prop);
                proxies.push(prop.proxy);
            }
        }
    }

    // Get initial values and start listening for updates
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
                state_lock
                    .update_dbus(property, &result)
                    .expect("Error setting initial DBus values");
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

    let mut next_nm_tick = Instant::now();
    let nm_duration = Duration::from_secs(1);

    loop {
        session_conn
            .process(Duration::from_millis(10))
            .expect("Could not process session dbus messages");

        system_conn
            .process(Duration::from_millis(10))
            .expect("Could not process system dbus messages");

        let mut updated = false;

        if next_nm_tick <= Instant::now() {
            let mut state_lock = state.lock().await;
            updated |= update_data_nm(&system_conn, &mut state_lock).expect("NetworkManager error");
            drop(state_lock);
            next_nm_tick = Instant::now() + nm_duration;
        }

        while let Ok(dbus_values) = rx.try_recv() {
            if dbus_values.is_empty() {
                continue;
            }
            let mut state_lock = state.lock().await;
            for (key, new_value_option) in dbus_values {
                let old_value = state_lock.get_value_dbus(key)?;

                match old_value {
                    Some(_val) if new_value_option.is_some() => {
                        state_lock
                            .update_dbus(key, &new_value_option.expect(""))
                            .expect("Error applying DBus update to state");
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
        }
        if updated {
            update_tx
                .send(true)
                .await
                .expect("Could not send dbus update");
        }
    }
}
