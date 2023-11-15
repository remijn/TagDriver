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

use embedded_graphics::{image::Image, prelude::DrawTarget, Drawable};
use embedded_icon::mdi::size32px::{
    Brightness1, Brightness2, Brightness3, Brightness4, Brightness5, Brightness6, Brightness7,
    VolumeHigh, VolumeLow, VolumeMedium, VolumeVariantOff,
};
use embedded_icon::NewIcon;

// impl Into<IconObj<T> for Icon<C, T> {}

use itertools::Itertools;
use serialport::{DataBits, Parity, StopBits};
use tokio::{sync::mpsc, time::sleep};
use tokio_serial::{SerialPort, SerialPortBuilderExt};

use dbus::{BusType, DBusPropertyAdress, DBusProxyAdress};

use crate::display::{components::DisplayAreaType, COLOR_FG};

fn spawn_eink_thread(
    port_str: &'static str,
    baud: u32,
    width: u32,
    height: u32,
    flip: bool,
) -> Result<EInkInterface, Box<dyn Error>> {
    // Create the serial port
    let mut port = tokio_serial::new(port_str, baud)
        .timeout(Duration::from_millis(1000))
        .open_native_async()
        .expect(format!("Failed to connect to device {}", port_str).as_str());

    port.set_exclusive(true)?;

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
        _port: port_str,
        flip,
    });
}

const SCREEN_COUNT: u8 = 3;
const BG_COLOR: BWRColor = BWRColor::Off;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let interface1: EInkInterface = spawn_eink_thread(
        "/dev/serial/by-id/usb-RemijnPi_Eink_Driver_DE6270431F67292B-if00",
        912600,
        250,
        128,
        false,
    )?;

    let interface2: EInkInterface = spawn_eink_thread(
        "/dev/serial/by-id/usb-RemijnPi_Eink_Driver_DE6270431F67292B-if04",
        912600,
        250,
        128,
        true,
    )?;
    let interface3: EInkInterface = spawn_eink_thread(
        "/dev/serial/by-id/usb-RemijnPi_Eink_Driver_DE6270431F67292B-if02",
        912600,
        300,
        400,
        false,
    )?;

    let mut screens: [(BWRDisplay, EInkInterface); SCREEN_COUNT as usize] =
        [interface1, interface2, interface3].map(|interface| {
            (
                BWRDisplay::new(interface.width, interface.height, interface.flip),
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

    const COLOR: BWRColor = COLOR_FG;

    const BRIGHTNESS_ICON_COUNT: u32 = 6;
    let mut brightness_dialog = BarDialog::new(
        "brightness dialog",
        brightness_property,
        0,
        Box::new(|target: &mut BWRDisplay, val, center| {
            // const color = BWRColor::Off;
            match (val * BRIGHTNESS_ICON_COUNT as f64).floor() as u32 {
                6 => Image::with_center(&Brightness7::new(COLOR), center)
                    .draw(target)
                    .ok(),
                5 => Image::with_center(&Brightness6::new(COLOR), center)
                    .draw(target)
                    .ok(),
                4 => Image::with_center(&Brightness5::new(COLOR), center)
                    .draw(target)
                    .ok(),
                3 => Image::with_center(&Brightness4::new(COLOR), center)
                    .draw(target)
                    .ok(),
                2 => Image::with_center(&Brightness3::new(COLOR), center)
                    .draw(target)
                    .ok(),
                1 => Image::with_center(&Brightness2::new(COLOR), center)
                    .draw(target)
                    .ok(),
                0 | _ => Image::with_center(&Brightness1::new(COLOR), center)
                    .draw(target)
                    .ok(),
            };
        }),
    ); //[].to_vec());
    display_components.push(&mut brightness_dialog);
    // display_components.push(&mut brightness_dialog);

    let player_proxy: DBusProxyAdress = DBusProxyAdress::new(
        BusType::SESSION,
        "org.mpris.MediaPlayer2.playerctld",
        "/org/mpris/MediaPlayer2",
    );
    let player_volume_property: DBusPropertyAdress =
        DBusPropertyAdress::new(player_proxy, "org.mpris.MediaPlayer2.Player", "Volume");

    const PLAYER_VOLUME_ICON_COUNT: u32 = 3;
    let mut player_volume_dialog = BarDialog::new(
        "player volume dialog",
        player_volume_property,
        1,
        Box::new(|target: &mut BWRDisplay, val, center| {
            match (val * PLAYER_VOLUME_ICON_COUNT as f64).ceil() as u16 {
                3 => Image::with_center(&VolumeHigh::new(COLOR), center)
                    .draw(target)
                    .ok(),
                2 => Image::with_center(&VolumeMedium::new(COLOR), center)
                    .draw(target)
                    .ok(),
                1 => Image::with_center(&VolumeLow::new(COLOR), center)
                    .draw(target)
                    .ok(),
                0 | _ => Image::with_center(&VolumeVariantOff::new(COLOR), center)
                    .draw(target)
                    .ok(),
            };
        }),
    );

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

    let initial = dbus_interface.init(properties).await?; //.expect("Could not init DBus");

    tokio::spawn(async move {
        dbus_interface.run().await;
    });

    for component in display_components.iter_mut() {
        if let Some(dbus) = component.dbus_mut() {
            dbus.set_initial(&initial);
        }
        // drop(component);
    }

    // ----- Our Main Program Loop -----
    loop {
        // Proccess dmesg updates

        let mut screen_needs_refresh: Vec<bool> = [false; SCREEN_COUNT as usize].to_vec().clone();

        while let Ok(_has_new) = dbus_rx.try_recv() {
            let values = &dbus_values.lock().await;

            for component in display_components.iter() {
                let mut component_needs_refresh = false;

                if let Some(dbus) = component.dbus() {
                    // Is updated needed by dbus?
                    component_needs_refresh = dbus.needs_refresh(&values);
                }
                // TODO: Add refresh for non dbus components. ie. logo

                if component_needs_refresh {
                    println!(
                        "Component \"{}\" requests update on screen {}",
                        component.get_name(),
                        component.get_screen()
                    )
                }
                screen_needs_refresh[component.get_screen() as usize] |= component_needs_refresh;
            }
        }
        for (i, (display, interface)) in screens.iter_mut().enumerate() {
            if screen_needs_refresh[i] {
                // Screen I needs an update, lets wrender

                println!("Rendering screen {}", i);

                // clear the screen
                display.clear(BG_COLOR)?;

                let values = Box::new(dbus_values.lock().await.clone());

                // list of components filtered by the current screen, mapped to zindex, and then sorted
                let components = display_components
                    .iter_mut()
                    .filter(|component| component.get_screen() == i as u8)
                    .map(|component| {
                        let index = component.get_z_index(&values);
                        (component, index)
                    })
                    .sorted_by(|a, b| Ord::cmp(&b.1, &a.1));

                // Draw the components to the screen's framebuffer
                for component in components {
                    println!("Render Z:{} {}", component.1, component.0.get_name());
                    component.0.draw(display, values.clone())?;

                    match component.0.get_type() {
                        DisplayAreaType::Dialog | DisplayAreaType::Fullscreen => break,
                        _ => continue,
                    }
                }

                drop(values);

                let (black, _red4) = display.get_fixed_buffer();

                // Stupid hack to force full-refresh the right screen
                if interface.flip {
                    interface
                        .full(black)
                        .await
                        .expect("Error sending to main thread");
                } else {
                    interface
                        .fast(black)
                        .await
                        .expect("Error sending to main thread");
                }
            }
        }
        sleep(Duration::from_millis(10)).await;
    }
}
