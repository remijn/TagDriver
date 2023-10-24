use std::time::{Duration, Instant};

mod dbus;
mod display;
mod eink;

use debug_print::debug_println;

use display::generate_image;
use eink::{EInkCommand, EInkResponse};

use embedded_graphics::prelude::{Point, Size};
use serialport::{DataBits, Parity, StopBits};
use tokio::{
    sync::mpsc::{self, Receiver, Sender},
    time::sleep,
};
use tokio_serial::{SerialPort, SerialPortBuilderExt};

const WIDTH: u32 = 250;
const REAL_HEIGHT: u32 = 122;
const HEIGHT: u32 = 128;

async fn main_thread(tx: Sender<EInkCommand>, mut rx: Receiver<EInkResponse>) {
    let mut state: EInkResponse = EInkResponse::READY;
    let mut display = display::bwr_display::BWRDisplay::new();

    generate_image(&mut display, 0).expect("?");
    let mut br_buffers = display.get_fixed_buffer();
    let both = [br_buffers.0, br_buffers.1].concat();
    let mut update = EInkCommand::full(both);
    tx.send(update).await.expect("Error sending to main thread");

    let start = Instant::now();

    let mut next_refresh = Instant::now();

    loop {
        // Proccess from the serial thread without blocking
        if let Ok(message) = rx.try_recv() {
            debug_println!("main got state {}", message);
            // Handle the received message.
            state = message;
        }
        if next_refresh > Instant::now() {
            sleep(Duration::from_millis(50)).await;
            continue;
        }

        match state {
            EInkResponse::READY => {
                state = EInkResponse::BUSY; //Asume the state here
                next_refresh = Instant::now() + Duration::from_secs(10);

                let i = (start.elapsed().as_millis() as f32 / 1000.0 % 5.0 * 20.0) as u32;

                generate_image(&mut display, i).expect("?");

                let p = Point::new(100, 0);
                let s = Size::new(50, 128);

                br_buffers = display.get_fixed_buffer();
                let partial = display.partial_buffer(br_buffers.0.as_slice(), p, s);

                // both = [br_buffers.0, br_buffers.1].concat();
                update = EInkCommand::partial(partial, p.x as u32, p.y as u32, s.width, s.height);
                tx.send(update).await.expect("Error sending to main thread");
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
            _ => {}
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // --------- UART SETUP ---------
    // Create the serial port
    let mut port = tokio_serial::new(eink::PORT, eink::BAUD_RATE)
        .timeout(Duration::from_millis(1000))
        .open_native_async()
        .expect(format!("Failed to connect to device {}", eink::PORT).as_str());

    port.set_exclusive(false)?;

    const CONFIG_ERROR: &str = "error configuring port";
    port.set_data_bits(DataBits::Eight).expect(CONFIG_ERROR);
    port.set_stop_bits(StopBits::One).expect(CONFIG_ERROR);
    port.set_parity(Parity::None).expect(CONFIG_ERROR);

    // let port_clone = port.try_clone().expect("Could not clone port");

    // Create a channel for communication between threads
    let (thread_tx, rx) = mpsc::channel::<EInkResponse>(512);
    let (tx, thread_rx) = mpsc::channel::<EInkCommand>(1024);

    // Spawn the serial thread
    tokio::spawn(async move {
        eink::thread::run_thread(Box::new(port), thread_tx, thread_rx)
            .await
            .expect("Could not spawn thread");
    });

    // --------- DBUS SETUP ---------

    let mut dbus_interface = dbus::interface::DBusInterface::new().expect("Could not init DBus");

    let player = dbus::DBusProxyAdress::new(
        "org.mpris.MediaPlayer2.playerctld".to_string(),
        "/org/mpris/MediaPlayer2".to_string(),
    );

    let playback_status = dbus::DBusPropertyAdress::new(
        player.clone(),
        "org.mpris.MediaPlayer2.Player".to_string(),
        "PlaybackStatus".to_string(),
    );

    let metadata = dbus::DBusPropertyAdress::new(
        player.clone(),
        "org.mpris.MediaPlayer2.Player".to_string(),
        "Metadata".to_string(),
    );

    dbus_interface.register_property(playback_status)?;
    dbus_interface.register_property(metadata)?;

    let power = dbus::DBusProxyAdress::new(
        "org.gnome.SettingsDaemon.Power".to_string(),
        "/org/gnome/SettingsDaemon/Power".to_string(),
    );

    let backlight = dbus::DBusPropertyAdress::new(
        power.clone(),
        "org.gnome.SettingsDaemon.Power.Screen".to_string(),
        "Brightness".to_string(),
    );

    dbus_interface.register_property(backlight)?;

    tokio::spawn(async move {
        dbus_interface.init(); //.expect("Could not init DBus");
        dbus::thread::run_thread(dbus_interface).await;
    });

    // Run the main thread
    if false {
        main_thread(tx, rx).await;
    }

    Ok(())
}
