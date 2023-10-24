use dbus::{
    arg::RefArg,
    blocking::{stdintf, Connection},
    Message,
};
use itertools::Itertools;
use tokio::sync::mpsc::{self, Receiver, Sender};

use std::{collections::HashMap, error::Error, time::Duration};

use stdintf::org_freedesktop_dbus::PropertiesPropertiesChanged;

use super::{DBusPropertyAdress, DBusProxyAdress};

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

pub type DBusUpdate = (DBusPropertyAdress, Option<Box<dyn RefArg>>);

pub struct DBusInterface {
    conn: Connection,
    values: HashMap<DBusPropertyAdress, Option<Box<dyn RefArg>>>,
    initialised: bool,
    rx: Receiver<DBusUpdate>,
    tx: Sender<DBusUpdate>,
}

impl DBusInterface {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let conn = Connection::new_session().expect("Error connecting to DBus");

        let (tx, rx) = mpsc::channel::<DBusUpdate>(20);

        Ok(Self {
            conn,
            values: HashMap::new(),
            initialised: false,
            rx,
            tx,
        })
    }

    pub fn register_property(
        &mut self,
        dbus_property: DBusPropertyAdress,
    ) -> Result<(), Box<dyn Error>> {
        assert!(
            !self.initialised,
            "Properties can only be registered bofore init()"
        );
        self.values.insert(dbus_property, None);
        Ok(())
        // self.values.insert(dbus_property, None);
    }

    pub fn init(&mut self) {
        assert!(!self.initialised, "init() can't be called twice");
        self.initialised = true;

        self.do_init();
    }

    pub fn do_init(&self) {
        // let values = self.values.lock().expect("cant lock");

        let proxies = self.values.keys().into_group_map_by(|v| v.proxy.clone());

        // self.conn.with_proxy(dest, path, timeout);

        // let conn = self.conn

        for (proxy, proxy_properties) in proxies {
            println!("Init Proxy {} {}", proxy.dest, proxy.path);

            let clone_tx = self.tx.clone();

            let clone_proxy: DBusProxyAdress = proxy.clone();

            let conn_proxy = self.conn.with_proxy(
                clone_proxy.dest.clone(),
                clone_proxy.path.clone(),
                Duration::from_secs(2),
            );

            for property in proxy_properties {
                // Get initial value
                let value = self.values.get(property).expect("Unknown value");
            }

            conn_proxy
                .match_signal(
                    move |h: PropertiesPropertiesChanged, _: &Connection, _: &Message| {
                        // let values = self.values.lock().expect("Could not lock values mutex");
                        // print!(
                        //     "PropChange CB for {} {} changed: ",
                        //     clone_proxy.dest, h.interface_name
                        // );
                        // let iface: String = h.interface_name.as_str().clone();
                        let iface = h.interface_name;

                        for (key, value) in h.changed_properties {
                            let prop = DBusPropertyAdress::new(
                                clone_proxy.clone(),
                                iface.clone(),
                                key.clone(),
                            );

                            let value: DBusUpdate = (prop, Some(value.0));

                            // print_refarg(&value.1.expect("huh?"));

                            clone_tx.try_send(value).expect("Could not send");
                        }
                        true
                    },
                )
                .expect("error");
        }
    }

    pub fn process(&mut self) {
        self.conn
            .process(Duration::from_millis(200))
            .expect("Could not process dbus messages");

        while let Ok((key, value)) = self.rx.try_recv() {
            // Should we forward this value?
            let val = self.values.get_mut(&key);

            if val.is_some() && value.is_some() {
                let val3 = val.expect("impossible");
                let val_ref = val3.insert(value.expect("impossible"));
                // val3 = Some(value.expect("impossible"));
                print!("{:?}: ", key);
                print_refarg(val_ref);
            } else {
                println!("Could not match {:?}", key);
                if let Some(val) = self
                    .values
                    .keys()
                    .find(|k| k.property == key.property || k.interface == k.interface)
                {
                    println!("Looks like {:?}", val);
                }
            }

            // if let Some(v) = self.values.get(&key) {
            //     let refBox = v.as_ref().expect(msg);
            // } else {
            //     // for(self)
            // }
        }
    }
}

#[allow(unused_attributes)]
#[feature(test)]
mod test {

    #[tokio::test]
    async fn can_create() {
        use super::DBusInterface;
        let bus = DBusInterface::new();
        match bus {
            Err(_e) => assert!(false),
            Ok(_v) => assert!(true),
        }
    }
}
