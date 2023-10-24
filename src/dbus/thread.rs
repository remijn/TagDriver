use super::interface::DBusInterface;

pub(crate) async fn run_thread(mut interface: DBusInterface) {
    println!("() Starting DBus Thread");

    loop {
        interface.process();
    }
}
