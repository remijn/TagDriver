use std::{error::Error, time::Duration};

mod dbus;
mod display;
mod eink;

use display::{
    bwr_color::BWRColor,
    bwr_display::BWRDisplay,
    components::{bar_dialog::BarDialog, DisplayComponent},
};
use eink::{EInkCommand, EInkInterface, EInkResponse};

use embedded_graphics::prelude::DrawTarget;
use itertools::Itertools;
use serialport::{DataBits, Parity, StopBits};
use tokio::{sync::mpsc, time::sleep};
use tokio_serial::{SerialPort, SerialPortBuilderExt};

use dbus::{BusType, DBusPropertyAdress, DBusProxyAdress};

fn spawn_eink_thread(
    port: &str,
    baud: u32,
    width: u32,
    height: u32,
) -> Result<EInkInterface, Box<dyn Error>> {
    // Create the serial port
    let mut port = tokio_serial::new(port, baud)
        .timeout(Duration::from_millis(1000))
        .open_native_async()
        .expect(format!("Failed to connect to device {}", port).as_str());

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

    return Ok(EInkInterface {
        rx,
        tx,
        state: EInkResponse::OK,
        width,
        height,
    });
}

const SCREEN_COUNT: u8 = 2;
const BG_COLOR: BWRColor = BWRColor::Off;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let interface1: EInkInterface = spawn_eink_thread(
        "/dev/serial/by-id/usb-RemijnPi_Eink_Driver_E66038B71367A831-if00",
        912600,
        250,
        128,
    )?;

    let interface2: EInkInterface = spawn_eink_thread(
        "/dev/serial/by-id/usb-RemijnPi_Eink_Driver_E66038B71367A831-if04",
        912600,
        300,
        400,
    )?;

    let mut screens: [(BWRDisplay, EInkInterface); SCREEN_COUNT as usize] =
        [interface1, interface2].map(|interface| {
            (
                BWRDisplay::new(interface.width, interface.height),
                interface,
            )
        });

    // --------- DBUS SETUP ---------

    let (dbus_tx, mut dbus_rx) = mpsc::channel::<bool>(20);
    let mut dbus_interface =
        dbus::dbus_interface::DBusInterface::new(dbus_tx).expect("Could not connect to DBus");

    let mut display_components: Vec<&mut dyn DisplayComponent> = Vec::new();

    let backlight_proxy: DBusProxyAdress = DBusProxyAdress::new(
        BusType::SESSION,
        "org.gnome.SettingsDaemon.Power",
        "/org/gnome/SettingsDaemon/Power",
    );

    let brightness_property: DBusPropertyAdress = DBusPropertyAdress::new(
        backlight_proxy,
        "org.gnome.SettingsDaemon.Power.Screen",
        "Brightness",
    );

    let mut brightness_dialog = BarDialog::new("brightness dialog", brightness_property, 0);
    display_components.push(&mut brightness_dialog);
    // display_components.push(&mut brightness_dialog);

    let player_proxy: DBusProxyAdress = DBusProxyAdress::new(
        BusType::SESSION,
        "org.mpris.MediaPlayer2.playerctld",
        "/org/mpris/MediaPlayer2",
    );
    let player_volume_property: DBusPropertyAdress =
        DBusPropertyAdress::new(player_proxy, "org.mpris.MediaPlayer2.Player", "Volume");

    let mut player_volume_dialog =
        BarDialog::new("player volume dialog", player_volume_property, 0);

    display_components.push(&mut player_volume_dialog);

    // display_components.push(&mut player_volume_dialog);

    let mut properties: Vec<DBusPropertyAdress> = Vec::new();
    // Get all the wanted properties
    let iter = display_components.iter();
    for component in iter {
        if let Some(dbus) = component.dbus() {
            properties.append(
                &mut dbus
                    .wanted_dbus_values()
                    .iter()
                    .map(|v| (*v).clone())
                    .collect_vec(),
            );
        }
    }

    let dbus_values = dbus_interface.values.clone();

    tokio::spawn(async move {
        dbus_interface.init(properties).await; //.expect("Could not init DBus");
        dbus_interface.run().await;
    });

    // ----- Our Main Program Loop -----
    loop {
        // Proccess dmesg updates

        let mut needs_update: Vec<bool> = [false; SCREEN_COUNT as usize].to_vec().clone();

        while let Ok(_has_new) = dbus_rx.try_recv() {
            let values = &dbus_values.lock().await;

            for component in display_components.iter_mut() {
                let mut needs = false;

                if let Some(dbus) = component.dbus() {
                    // Is updated needed by dbus?
                    needs = dbus.needs_refresh(&values);
                }

                if needs {
                    println!(
                        "Component \"{}\" requests update on screen {}",
                        component.get_name(),
                        component.get_screen()
                    )
                }
                needs_update[component.get_screen() as usize] |= needs;
            }
        }
        for (i, (display, interface)) in screens.iter_mut().enumerate() {
            if needs_update[i] {
                // Screen I needs an update, lets wrender

                println!("Rendering screen {}", i);

                // clear the screen
                display.clear(BG_COLOR)?;

                // list of components filtered by the current screen
                let components = display_components
                    .iter_mut()
                    .filter(|component| component.get_screen() == i as u8);

                let values = Box::new(dbus_values.lock().await.clone());

                // Draw the components to the screen's framebuffer
                for component in components {
                    if component.get_screen() != i as u8 {
                        continue;
                    }
                    component.draw(display, values.clone())?;
                }

                drop(values);

                let (black, _red) = display.get_fixed_buffer();

                interface
                    .fast(black)
                    .await
                    .expect("Error sending to main thread");
            }
        }
        sleep(Duration::from_millis(10)).await;
    }
    // interface2
    //     .tx
    //     .send(EInkCommand::LED { color: 2 })
    //     .await
    //     .expect("Error setting LED");

    // Run the main thread
    // if false {
    // main_thread(interface1).await;
    // }
}
