use dbus::{
    arg::RefArg,
    blocking::{
        stdintf::{self, org_freedesktop_dbus::Properties},
        Connection,
    },
    channel::MatchingReceiver,
    Message,
};

use dbus_crossroads::{Context, Crossroads};
use itertools::Itertools;
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

    session_conn.request_name("io.remijn.tagdriver", false, true, false)?;

    let mut cr = Crossroads::new();

    // Let's build a new interface, which can be used for "Hello" objects.
    let iface_token = cr.register("io.remijn.tagdriver", |b| {
        // This row advertises (when introspected) that we can send a HelloHappened signal.
        // We use the single-tuple to say that we have one single argument, named "sender" of type "String".
        // The msg_fn returns a boxed function, which when called constructs the message to be emitted.
        // let state_changed = b.signal::<(String,), _>("StateChanged", ("json",)).msg_fn();

        // Let's add a method to the interface. We have the method name, followed by
        // names of input and output arguments (used for introspection). The closure then controls
        // the types of these arguments. The last argument to the closure is a tuple of the input arguments.

        let clone_tx = tx.clone();
        b.method(
            "SetImage",
            ("png", "display"),
            ("reply",),
            move |_ctx: &mut Context,
                  _state: &mut Arc<Mutex<ApplicationState>>,
                  (png, display): (String, u32)| {
                // And here's what happens when the method is called.
                println!("{} SetImage called for display {}", log::DBUS, display);

                clone_tx
                    .try_send(vec![DBusUpdate::MethodShowImage(png, display)])
                    .expect("Could not send");
                let reply = format!("Display on screen {}", display);
                Ok((reply,))
            },
        );
        let clone_tx = tx.clone();
        b.method(
            "SetWorkspaces",
            ("active", "count"),
            ("reply",),
            move |_ctx: &mut Context,
                  _state: &mut Arc<Mutex<ApplicationState>>,
                  (active, count): (u32, u32)| {
                // And here's what happens when the method is called.
                let mut workspaces = vec![false; count as usize];
                workspaces[active as usize] = true;
                let string_indicator = workspaces
                    .iter()
                    .map(|v| match v {
                        true => "🔵",
                        false => "⚪",
                    })
                    .join("");

                println!(
                    "{} Method SetWorkspaces called {}",
                    log::DBUS,
                    string_indicator
                );

                clone_tx
                    .try_send(vec![DBusUpdate::MethodSetWorkspaces(active, count)])
                    .expect("Could not send");
                Ok(("ok",))
            },
        );
    });
    cr.insert("/", &[iface_token], state.clone());

    session_conn.start_receive(
        dbus::message::MatchRule::new_method_call(),
        Box::new(move |msg, conn| {
            cr.handle_message(msg, conn).unwrap();
            true
        }),
    );

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

        let clone_tx = tx.clone();

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
                                updates.push(DBusUpdate::PropertyUpdate((
                                    prop,
                                    Some(value.0.box_clone()),
                                )));
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
            for update in dbus_values {
                match update {
                    DBusUpdate::PropertyUpdate((key, new_value_option)) => {
                        let old_value = state_lock.get_value_dbus(key)?;

                        match old_value {
                            Some(_val) if new_value_option.is_some() => {
                                state_lock
                                    .update_dbus(key, &new_value_option.expect(""))
                                    .expect("Error applying DBus update to state");
                                updated = true;
                            }
                            Some(_val) => println!("{} Recieved empty value????", log::ERROR),
                            None => {
                                println!(
                                    "{} Could not match into Application state: \n{} {}",
                                    log::WARN,
                                    log::DBUS,
                                    key
                                );
                            }
                        }
                    }
                    DBusUpdate::MethodShowImage(_png, display) => {
                        print!("update method show image on display {}", display);
                    }
                    DBusUpdate::MethodSetWorkspaces(active, count) => {
                        state_lock
                            .update("workspace:active", Some(StateValueType::U64(active as u64)))?;
                        state_lock
                            .update("workspace:count", Some(StateValueType::U64(count as u64)))?;
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
